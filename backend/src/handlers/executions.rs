use actix_web::web::Bytes;
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::Utc;
use rusqlite::params;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
use tokio::io::AsyncBufReadExt;
use tokio::sync::broadcast;

use crate::adapters::{EmployeeConfig, TaskInfo};
use crate::db::{AppState, OutputEvent};
use crate::errors::AppError;
use crate::models::*;

/// Spawn an agent process for an execution. Called after DB records are created.
pub async fn spawn_execution(
    state: web::Data<AppState>,
    execution_id: i64,
    task_info: TaskInfo,
    employee_config: EmployeeConfig,
    backend: String,
) {
    let adapter = match state.adapters.get(&backend) {
        Some(a) => a,
        None => {
            mark_execution_failed(&state, execution_id, "Adapter not available").await;
            return;
        }
    };

    let process = match adapter.execute(&task_info, &employee_config).await {
        Ok(p) => p,
        Err(e) => {
            mark_execution_failed(&state, execution_id, &e).await;
            return;
        }
    };

    let child_id = process.child.id().unwrap_or(0);

    // Store PID in DB
    if let Ok(conn) = state.db.get() {
        let _ = conn.execute(
            "UPDATE task_executions SET pid = ?1 WHERE id = ?2",
            params![child_id, execution_id],
        );
    }

    let (cancel_tx, _) = broadcast::channel::<()>(1);
    let (output_tx, _) = broadcast::channel::<OutputEvent>(256);

    {
        let mut running = state.running.lock().await;
        running.insert(
            execution_id,
            crate::db::RunningExecution {
                cancel_tx: cancel_tx.clone(),
                output_tx: output_tx.clone(),
            },
        );
    }

    // Spawn a background task to read stdout and store chunks
    let state_clone = state.clone();
    let output_tx_clone = output_tx.clone();
    let mut cancel_rx = cancel_tx.subscribe();

    tokio::spawn(async move {
        let mut child = process.child;
        let mut stdout_lines = process.stdout.lines();
        let mut stderr_lines = process.stderr.lines();
        let seq = Arc::new(AtomicI64::new(0));
        let chunks_count = Arc::new(AtomicI64::new(0));
        let max_chunks = 10000;
        let mut cancelled = false;
        let mut stdout_done = false;
        let mut stderr_done = false;

        let emit_chunk = |line: String,
                          is_stderr: bool,
                          seq: &Arc<AtomicI64>,
                          chunks_count: &Arc<AtomicI64>| {
            let seq_value = seq.fetch_add(1, Ordering::Relaxed) + 1;
            let now = Utc::now().to_rfc3339();
            let chunk = if is_stderr {
                format!("[stderr] {}", line)
            } else {
                line
            };

            if chunks_count.fetch_add(1, Ordering::Relaxed) < max_chunks {
                if let Ok(conn) = state_clone.db.get() {
                    let _ = conn.execute(
                        "INSERT INTO output_chunks (execution_id, seq, chunk, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![execution_id, seq_value, &chunk, &now],
                    );
                }

                let _ = output_tx_clone.send(OutputEvent::Output {
                    seq: seq_value,
                    data: format!(
                        "{{\"execution_id\":{},\"chunk\":{},\"seq\":{},\"ts\":\"{}\"}}",
                        execution_id,
                        serde_json::to_string(&chunk).unwrap_or_else(|_| "\"\"".to_string()),
                        seq_value,
                        &now
                    ),
                });
            } else if chunks_count.load(Ordering::Relaxed) == max_chunks + 1 {
                if let Ok(conn) = state_clone.db.get() {
                    let _ = conn.execute(
                        "INSERT INTO output_chunks (execution_id, seq, chunk, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![execution_id, seq_value + 1, "\n\n[OUTPUT TRUNCATED - LIMIT EXCEEDED]\n", now],
                    );
                }
            }
        };

        loop {
            tokio::select! {
                line = stdout_lines.next_line(), if !stdout_done => {
                    match line {
                        Ok(Some(line)) => emit_chunk(line, false, &seq, &chunks_count),
                        Ok(None) => stdout_done = true,
                        Err(_) => stdout_done = true,
                    }
                }
                line = stderr_lines.next_line(), if !stderr_done => {
                    match line {
                        Ok(Some(line)) => emit_chunk(line, true, &seq, &chunks_count),
                        Ok(None) => stderr_done = true,
                        Err(_) => stderr_done = true,
                    }
                }
                _ = cancel_rx.recv() => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    cancelled = true;
                    let status_event = format!(
                        "{{\"execution_id\":{},\"status\":\"cancelled\"}}",
                        execution_id
                    );
                    let _ = output_tx_clone.send(OutputEvent::Status(status_event));
                    break;
                }
            }

            if stdout_done && stderr_done {
                break;
            }
        }

        if !cancelled {
            // Process finished naturally
            let exit_status = child.wait().await;
            let mut exit_code = exit_status.ok().and_then(|s| s.code()).unwrap_or(-1);
            let mut git_finalize_error = None;

            if exit_code == 0 {
                match crate::task_source::finalize_workspace(
                    &task_info.task_id,
                    &task_info.branch_name,
                    &task_info.title,
                )
                .await
                {
                    Ok(_) => {}
                    Err(error) => {
                        exit_code = -1;
                        git_finalize_error = Some(error);
                    }
                }
            }

            let finished_at = Utc::now().to_rfc3339();
            let status = if exit_code == 0 {
                "completed"
            } else {
                "failed"
            };
            let emp_status = if exit_code == 0 { "idle" } else { "error" };

            if let Ok(conn) = state_clone.db.get() {
                let _ = conn.execute(
                    "UPDATE task_executions SET status = ?1, exit_code = ?2, finished_at = ?3 WHERE id = ?4 AND status = 'running'",
                    params![status, exit_code, finished_at, execution_id],
                );

                if let Ok(employee_id) = conn.query_row(
                    "SELECT employee_id FROM task_executions WHERE id = ?1",
                    params![execution_id],
                    |r| r.get::<_, i64>(0),
                ) {
                    let _ = conn.execute(
                        "UPDATE ai_employees SET status = ?1 WHERE id = ?2",
                        params![emp_status, employee_id],
                    );
                }
            }

            if exit_code == 0 {
                let _ = crate::orchestration::process_execution_success(
                    state_clone.clone(),
                    execution_id,
                    true,
                )
                .await;
            } else if let Some(error) = git_finalize_error {
                if let Ok(conn) = state_clone.db.get() {
                    let _ = conn.execute(
                        "INSERT INTO output_chunks (execution_id, seq, chunk, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            execution_id,
                            seq.fetch_add(1, Ordering::Relaxed) + 1,
                            format!("Git finalize failed: {}", error),
                            Utc::now().to_rfc3339()
                        ],
                    );
                }
            }

            let status_event = format!(
                "{{\"execution_id\":{},\"status\":\"{}\",\"exit_code\":{}}}",
                execution_id, status, exit_code
            );
            let _ = output_tx_clone.send(OutputEvent::Status(status_event));
        }

        // Remove from running map
        let mut running = state_clone.running.lock().await;
        running.remove(&execution_id);
    });
}

async fn mark_execution_failed(state: &web::Data<AppState>, execution_id: i64, error: &str) {
    let finished_at = Utc::now().to_rfc3339();
    if let Ok(conn) = state.db.get() {
        let _ = conn.execute(
            "UPDATE task_executions SET status = 'failed', finished_at = ?1, exit_code = -1 WHERE id = ?2",
            params![finished_at, execution_id],
        );

        if let Ok(employee_id) = conn.query_row(
            "SELECT employee_id FROM task_executions WHERE id = ?1",
            params![execution_id],
            |r| r.get::<_, i64>(0),
        ) {
            let _ = conn.execute(
                "UPDATE ai_employees SET status = 'error' WHERE id = ?1",
                params![employee_id],
            );
        }

        // Store error as first output chunk
        let _ = conn.execute(
            "INSERT INTO output_chunks (execution_id, seq, chunk, created_at) VALUES (?1, 1, ?2, ?3)",
            params![execution_id, format!("Error: {}", error), finished_at],
        );
    }
}

/// SSE stream endpoint: GET /api/executions/{id}/stream
pub async fn stream_execution(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    req: HttpRequest,
) -> HttpResponse {
    let execution_id = path.into_inner();

    // Verify execution exists
    let exec_status = {
        let conn = match data.db.get() {
            Ok(c) => c,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };
        match conn.query_row(
            "SELECT status FROM task_executions WHERE id = ?1",
            params![execution_id],
            |r| r.get::<_, String>(0),
        ) {
            Ok(s) => s,
            Err(_) => return HttpResponse::NotFound().finish(),
        }
    };

    // Get existing chunks from DB
    let existing_chunks: Vec<(i64, String, String)> = {
        let conn = match data.db.get() {
            Ok(c) => c,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

        // Support Last-Event-ID for reconnection
        let last_seq: i64 = req
            .headers()
            .get("Last-Event-ID")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mut stmt = match conn
            .prepare("SELECT seq, chunk, created_at FROM output_chunks WHERE execution_id = ?1 AND seq > ?2 ORDER BY seq")
        {
            Ok(s) => s,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

        let chunks: Vec<(i64, String, String)> = match stmt
            .query_map(params![execution_id, last_seq], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            }) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };
        chunks
    };

    // Build SSE response
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(64);

    // If execution is still running, subscribe to live broadcast
    let live_rx = {
        let running = data.running.lock().await;
        running.get(&execution_id).map(|e| e.output_tx.subscribe())
    };

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        // Send existing chunks first
        for (seq, chunk, ts) in existing_chunks {
            let event = format!(
                "id:{}\nevent:output\ndata:{{\"execution_id\":{},\"chunk\":{},\"seq\":{},\"ts\":\"{}\"}}\n\n",
                seq,
                execution_id,
                serde_json::to_string(&chunk).unwrap_or_else(|_| "\"\"".to_string()),
                seq,
                ts
            );
            if tx_clone.send(Ok(Bytes::from(event))).await.is_err() {
                return;
            }
        }

        if let Some(mut rx) = live_rx {
            // Stream live chunks until process finishes
            loop {
                match rx.recv().await {
                    Ok(OutputEvent::Status(status)) => {
                        let status_event = format!("event:status\ndata:{}\n\n", status);
                        let _ = tx_clone.send(Ok(Bytes::from(status_event))).await;
                        break;
                    }
                    Ok(OutputEvent::Output { seq, data }) => {
                        let sse_event = format!("id:{}\nevent:output\ndata:{}\n\n", seq, data);
                        if tx_clone.send(Ok(Bytes::from(sse_event))).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        } else {
            // If execution is already done, send final status and close
            let status_event = format!(
                "event:status\ndata:{{\"execution_id\":{},\"status\":\"{}\"}}\n\n",
                execution_id, exec_status
            );
            let _ = tx_clone.send(Ok(Bytes::from(status_event))).await;
        }
    });

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .append_header(("Cache-Control", "no-cache"))
        .append_header(("X-Accel-Buffering", "no"))
        .streaming(tokio_stream::wrappers::ReceiverStream::new(rx))
}

/// Get output chunks for an execution: GET /api/executions/{id}/output
pub async fn get_execution_output(data: web::Data<AppState>, path: web::Path<i64>) -> HttpResponse {
    let execution_id = path.into_inner();

    let result = (|| -> Result<Vec<OutputChunk>, AppError> {
        let conn = data.db.get()?;

        // Verify execution exists
        conn.query_row(
            "SELECT id FROM task_executions WHERE id = ?1",
            params![execution_id],
            |_| Ok(()),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
            other => AppError::Db(other),
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, execution_id, seq, chunk, created_at FROM output_chunks WHERE execution_id = ?1 ORDER BY seq",
        )?;

        let chunks = stmt
            .query_map(params![execution_id], |row| {
                Ok(OutputChunk {
                    id: row.get(0)?,
                    execution_id: row.get(1)?,
                    seq: row.get(2)?,
                    chunk: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(chunks)
    })();

    match result {
        Ok(chunks) => HttpResponse::Ok().json(chunks),
        Err(e) => e.to_response(),
    }
}
