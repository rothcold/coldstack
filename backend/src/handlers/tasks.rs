use actix_web::{HttpResponse, web};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::db::AppState;
use crate::errors::AppError;
use crate::models::*;
use crate::workflow;

fn allocate_branch_name(
    conn: &rusqlite::Connection,
    title: &str,
    description: &str,
    attempt: usize,
) -> Result<String, AppError> {
    let base = crate::task_source::default_branch_name(title, description);
    let mut candidate = if attempt == 0 {
        base.clone()
    } else {
        format!("{}-{}", base, attempt + 1)
    };
    let mut suffix = attempt + 2;

    loop {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM tasks WHERE branch_name = ?1 LIMIT 1",
                params![candidate],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{}-{}", base, suffix);
        suffix += 1;
    }
}

fn get_subtasks_for_task(
    conn: &rusqlite::Connection,
    task_id: i64,
) -> Result<Vec<Subtask>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, title, completed, status, assignee FROM subtasks WHERE task_id = ?1",
    )?;
    let subtasks = stmt
        .query_map(params![task_id], |row| {
            let status_str: String = row.get(4)?;
            Ok(Subtask {
                id: row.get(0)?,
                task_id: row.get(1)?,
                title: row.get(2)?,
                completed: row.get::<_, i32>(3)? == 1,
                status: WorkflowStatus::from_str(&status_str),
                assignee: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(subtasks)
}

fn load_task(conn: &rusqlite::Connection, id: i64) -> Result<Task, AppError> {
    let (
        task_id,
        title,
        description,
        source,
        source_branch,
        branch_name,
        archived,
        status_str,
        assignee,
        created_at,
    ) = conn
        .query_row(
            "SELECT task_id, title, description, source, source_branch, branch_name, archived, status, assignee, created_at FROM tasks WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i32>(6)? == 1,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
            other => AppError::Db(other),
        })?;

    Ok(Task {
        id,
        task_id,
        title,
        description,
        source,
        source_branch,
        branch_name,
        archived,
        status: WorkflowStatus::from_str(&status_str),
        assignee,
        created_at,
        subtasks: get_subtasks_for_task(conn, id)?,
    })
}

pub async fn get_tasks(data: web::Data<AppState>) -> HttpResponse {
    let result = (|| -> Result<Vec<BoardTaskSummary>, AppError> {
        let conn = data.db.get()?;
        let mut stmt = conn.prepare(
            "SELECT
                t.id,
                t.task_id,
                t.title,
                t.status,
                t.assignee,
                t.archived,
                CASE
                    WHEN t.status = 'Coding' AND (
                        SELECT e.action
                        FROM task_workflow_events e
                        WHERE e.task_id = t.id
                        ORDER BY e.created_at DESC, e.id DESC
                        LIMIT 1
                    ) = 'reject' THEN 1
                    ELSE 0
                END AS needs_attention,
                CASE WHEN t.status = 'NeedsHuman' THEN 1 ELSE 0 END AS waiting_for_human,
                t.auto_handoff_pending,
                (
                    SELECT COUNT(*) FROM task_workflow_events e
                    WHERE e.task_id = t.id AND e.action = 'reject'
                ) AS rejection_count,
                CASE
                    WHEN t.auto_handoff_pending = 1 THEN
                        CASE t.status
                            WHEN 'Plan' THEN 'Waiting for next idle planner agent'
                            WHEN 'Design' THEN 'Waiting for next idle designer agent'
                            WHEN 'Coding' THEN 'Waiting for next idle coder agent'
                            WHEN 'Review' THEN 'Waiting for next idle reviewer agent'
                            WHEN 'QA' THEN 'Waiting for next idle qa agent'
                            ELSE 'Waiting for next agent'
                        END
                    ELSE (
                        SELECT CASE
                            WHEN e.action = 'reject' THEN COALESCE(e.note, 'Returned for changes')
                            WHEN e.action = 'archive' THEN 'Archived by human'
                            ELSE 'Moved to ' || e.to_status
                        END
                        FROM task_workflow_events e
                        WHERE e.task_id = t.id
                        ORDER BY e.created_at DESC, e.id DESC
                        LIMIT 1
                    )
                END AS latest_event_summary
             FROM tasks t
             WHERE t.archived = 0
             ORDER BY t.created_at DESC",
        )?;

        let tasks = stmt
            .query_map([], |row| {
                let status = WorkflowStatus::from_str(&row.get::<_, String>(3)?);
                Ok(BoardTaskSummary {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    title: row.get(2)?,
                    status,
                    board_group: status.board_group().to_string(),
                    assignee: row.get(4)?,
                    archived: row.get::<_, i32>(5)? == 1,
                    needs_attention: row.get::<_, i32>(6)? == 1,
                    waiting_for_human: row.get::<_, i32>(7)? == 1,
                    waiting_for_agent: row.get::<_, i32>(8)? == 1,
                    rejection_count: row.get(9)?,
                    latest_event_summary: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tasks)
    })();

    match result {
        Ok(tasks) => HttpResponse::Ok().json(tasks),
        Err(e) => e.to_response(),
    }
}

pub async fn get_task_detail(data: web::Data<AppState>, path: web::Path<i64>) -> HttpResponse {
    let result = (|| -> Result<TaskDetail, AppError> {
        let conn = data.db.get()?;
        workflow::load_task_detail(&conn, path.into_inner())
    })();

    match result {
        Ok(task) => HttpResponse::Ok().json(task),
        Err(e) => e.to_response(),
    }
}

pub async fn create_task(data: web::Data<AppState>, item: web::Json<CreateTask>) -> HttpResponse {
    let result = (|| -> Result<Task, AppError> {
        let conn = data.db.get()?;
        let created_at_str = Utc::now().to_rfc3339();
        let default_status = WorkflowStatus::Plan;
        let source = item.source.trim();
        if source.is_empty() {
            return Err(AppError::BadRequest(
                "Task source is required".to_string(),
            ));
        }
        let source_branch = item
            .source_branch
            .clone()
            .unwrap_or_else(|| "main".to_string())
            .trim()
            .to_string();
        if source_branch.is_empty() {
            return Err(AppError::BadRequest(
                "Task source branch is required".to_string(),
            ));
        }
        let task_id = item
            .task_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("T-{}", uuid::Uuid::new_v4()));

        let mut branch_name = String::new();
        let mut inserted = false;

        for attempt in 0..10 {
            branch_name = allocate_branch_name(&conn, &item.title, &item.description, attempt)?;
            match conn.execute(
                "INSERT INTO tasks (task_id, title, description, source, source_branch, branch_name, archived, status, assignee, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?9)",
                params![task_id, item.title, item.description, source, source_branch, branch_name, default_status.as_str(), item.assignee, created_at_str],
            ) {
                Ok(_) => {
                    inserted = true;
                    break;
                }
                Err(rusqlite::Error::SqliteFailure(ref err, _)) if err.extended_code == 2067 => {
                    let task_id_exists: bool = conn
                        .query_row(
                            "SELECT 1 FROM tasks WHERE task_id = ?1 LIMIT 1",
                            params![task_id],
                            |_| Ok(true),
                        )
                        .optional()?
                        .unwrap_or(false);
                    if task_id_exists {
                        return Err(AppError::Conflict("Task ID already exists".to_string()));
                    }
                }
                Err(other) => return Err(AppError::Db(other)),
            }
        }

        if !inserted {
            return Err(AppError::Conflict(
                "Could not allocate a unique task branch".to_string(),
            ));
        }

        let id = conn.last_insert_rowid();
        Ok(Task {
            id,
            task_id,
            title: item.title.clone(),
            description: item.description.clone(),
            source: Some(source.to_string()),
            source_branch: Some(source_branch),
            branch_name: Some(branch_name),
            archived: false,
            status: default_status,
            assignee: item.assignee.clone(),
            created_at: created_at_str,
            subtasks: Vec::new(),
        })
    })();

    match result {
        Ok(task) => HttpResponse::Created().json(task),
        Err(e) => e.to_response(),
    }
}

pub async fn update_task(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<UpdateTask>,
) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<Task, AppError> {
        let conn = data.db.get()?;
        let existing = load_task(&conn, id)?;

        let new_task_id = item.task_id.clone().unwrap_or(existing.task_id);
        let new_title = item.title.clone().unwrap_or(existing.title);
        let new_desc = item.description.clone().unwrap_or(existing.description);
        let new_source = match item.source.clone() {
            Some(source) => {
                let trimmed = source.trim();
                if trimmed.is_empty() {
                    return Err(AppError::BadRequest(
                        "Task source cannot be empty".to_string(),
                    ));
                }
                Some(trimmed.to_string())
            }
            None => existing.source,
        };
        let new_source_branch = match item.source_branch.clone() {
            Some(source_branch) => {
                let trimmed = source_branch.trim();
                if trimmed.is_empty() {
                    return Err(AppError::BadRequest(
                        "Task source branch cannot be empty".to_string(),
                    ));
                }
                Some(trimmed.to_string())
            }
            None => existing.source_branch,
        };
        let branch_name = existing.branch_name.clone().or_else(|| {
            allocate_branch_name(&conn, &new_title, &new_desc, 0).ok()
        });
        let new_assignee = item.assignee.clone().unwrap_or(existing.assignee);

        conn.execute(
            "UPDATE tasks SET task_id = ?1, title = ?2, description = ?3, source = ?4, source_branch = ?5, branch_name = ?6, assignee = ?7 WHERE id = ?8",
            params![new_task_id, new_title, new_desc, new_source, new_source_branch, branch_name, new_assignee, id],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(ref err, _) if err.extended_code == 2067 => {
                AppError::Conflict("Task ID already exists".to_string())
            }
            other => AppError::Db(other),
        })?;

        load_task(&conn, id)
    })();

    match result {
        Ok(task) => HttpResponse::Ok().json(task),
        Err(e) => e.to_response(),
    }
}

pub async fn publish_task_branch(data: web::Data<AppState>, path: web::Path<i64>) -> HttpResponse {
    let task_id = path.into_inner();

    let result = (|| -> Result<(String, String), AppError> {
        let conn = data.db.get()?;
        let (task_key, branch_name): (String, Option<String>) = conn
            .query_row(
                "SELECT task_id, branch_name FROM tasks WHERE id = ?1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
                other => AppError::Db(other),
            })?;

        let branch_name = branch_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Conflict("Task branch is missing".to_string()))?;

        Ok((task_key, branch_name))
    })();

    match result {
        Ok((task_key, branch_name)) => {
            match crate::task_source::publish_workspace(&task_key, &branch_name).await {
                Ok(()) => HttpResponse::Ok().json(PublishTaskBranchResponse { branch_name }),
                Err(error) => AppError::Conflict(error).to_response(),
            }
        }
        Err(e) => e.to_response(),
    }
}

pub async fn transition_task(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<TransitionTaskRequest>,
) -> HttpResponse {
    let task_id = path.into_inner();

    let result = (|| -> Result<TransitionTaskResponse, AppError> {
        let mut conn = data.db.get()?;
        let detail = workflow::transition_task(&mut conn, task_id, item.into_inner())?;
        Ok(TransitionTaskResponse { task: detail })
    })();

    match result {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => e.to_response(),
    }
}

pub async fn delete_task(data: web::Data<AppState>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<(), AppError> {
        let conn = data.db.get()?;
        let affected = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    })();

    match result {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => e.to_response(),
    }
}

pub async fn add_subtask(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<CreateSubtask>,
) -> HttpResponse {
    let task_id = path.into_inner();
    let default_status = WorkflowStatus::Plan;

    let result = (|| -> Result<Subtask, AppError> {
        let conn = data.db.get()?;

        let parent_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = ?1",
            params![task_id],
            |r| r.get(0),
        )?;
        if parent_exists == 0 {
            return Err(AppError::NotFound);
        }

        conn.execute(
            "INSERT INTO subtasks (task_id, title, completed, status, assignee) VALUES (?1, ?2, 0, ?3, ?4)",
            params![task_id, item.title, default_status.as_str(), item.assignee],
        )?;

        let id = conn.last_insert_rowid();

        Ok(Subtask {
            id,
            task_id,
            title: item.title.clone(),
            completed: false,
            status: default_status,
            assignee: item.assignee.clone(),
        })
    })();

    match result {
        Ok(subtask) => HttpResponse::Created().json(subtask),
        Err(e) => e.to_response(),
    }
}

pub async fn toggle_subtask(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> HttpResponse {
    let (task_id, subtask_id) = path.into_inner();

    let result = (|| -> Result<(), AppError> {
        let conn = data.db.get()?;
        let affected = conn.execute(
            "UPDATE subtasks SET completed = 1 - completed WHERE id = ?1 AND task_id = ?2",
            params![subtask_id, task_id],
        )?;
        if affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    })();

    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(e) => e.to_response(),
    }
}

pub async fn update_subtask(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    item: web::Json<UpdateSubtask>,
) -> HttpResponse {
    let (task_id, subtask_id) = path.into_inner();

    let result = (|| -> Result<(), AppError> {
        let conn = data.db.get()?;

        let sub_data = conn
            .query_row(
                "SELECT title, completed, status, assignee FROM subtasks WHERE id = ?1 AND task_id = ?2",
                params![subtask_id, task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i32>(1)? == 1,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
                other => AppError::Db(other),
            })?;

        let new_title = item.title.clone().unwrap_or(sub_data.0);
        let new_completed = item.completed.unwrap_or(sub_data.1);
        let new_status = item.status.unwrap_or(WorkflowStatus::from_str(&sub_data.2));
        let new_assignee = item.assignee.clone().unwrap_or(sub_data.3);

        conn.execute(
            "UPDATE subtasks SET title = ?1, completed = ?2, status = ?3, assignee = ?4 WHERE id = ?5",
            params![new_title, new_completed as i32, new_status.as_str(), new_assignee, subtask_id],
        )?;

        Ok(())
    })();

    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(e) => e.to_response(),
    }
}
