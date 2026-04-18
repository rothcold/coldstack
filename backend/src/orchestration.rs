use actix_web::web;
use chrono::Utc;
use rusqlite::params;
use tokio::time::{Duration, MissedTickBehavior};

use crate::db::AppState;
use crate::errors::AppError;
use crate::handlers::employees::{launch_assignment, prepare_assignment};
use crate::models::{TransitionTaskRequest, WorkflowAction, WorkflowActorType, WorkflowStatus};
use crate::workflow::{next_workflow_status, workflow_role_for_status};

const AUTO_HANDOFF_SCAN_INTERVAL_SECONDS: u64 = 5;

pub fn start_auto_handoff_scanner(state: web::Data<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(AUTO_HANDOFF_SCAN_INTERVAL_SECONDS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            let _ = process_pending_auto_handoffs(state.clone(), true).await;
        }
    });
}

pub async fn process_execution_success(
    state: web::Data<AppState>,
    execution_id: i64,
    launch_execution: bool,
) -> Result<(), AppError> {
    let mut conn = state.db.get()?;
    let (task_id, employee_id, employee_name, workflow_role, task_status): (
        i64,
        i64,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT te.task_id, e.id, e.name, e.workflow_role, t.status
             FROM task_executions te
             JOIN ai_employees e ON e.id = te.employee_id
             JOIN tasks t ON t.id = te.task_id
             WHERE te.id = ?1",
            params![execution_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
            other => AppError::Db(other),
        })?;

    let current_status = WorkflowStatus::from_str(&task_status);
    let expected_role = workflow_role_for_status(current_status);
    let actual_role = crate::models::WorkflowRole::from_str(&workflow_role);

    let should_attempt = if expected_role != Some(actual_role) {
        false
    } else if let Some(next_status) = next_workflow_status(current_status) {
        crate::workflow::transition_task(
            &mut conn,
            task_id,
            TransitionTaskRequest {
                actor_type: WorkflowActorType::Employee,
                actor_id: Some(employee_id),
                actor_label: Some(employee_name),
                from_status: current_status,
                to_status: Some(next_status),
                action: WorkflowAction::Advance,
                note: None,
                evidence_text: None,
            },
        )?;

        let should_auto_handoff = workflow_role_for_status(next_status).is_some();
        if should_auto_handoff {
            conn.execute(
                "UPDATE tasks
                 SET auto_handoff_pending = 1, auto_handoff_claimed_at = NULL
                 WHERE id = ?1",
                params![task_id],
            )?;
        }
        should_auto_handoff
    } else {
        false
    };
    drop(conn);

    if should_attempt {
        attempt_auto_handoff(state, task_id, launch_execution).await?;
    }

    Ok(())
}

pub async fn process_pending_auto_handoffs(
    state: web::Data<AppState>,
    launch_execution: bool,
) -> Result<(), AppError> {
    let task_ids = {
        let conn = state.db.get()?;
        let mut stmt = conn.prepare(
            "SELECT id
             FROM tasks
             WHERE auto_handoff_pending = 1
             ORDER BY id",
        )?;
        stmt.query_map([], |row| row.get::<_, i64>(0))?
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
    };

    for task_id in task_ids {
        let _ = attempt_auto_handoff(state.clone(), task_id, launch_execution).await;
    }

    Ok(())
}

pub async fn attempt_auto_handoff(
    state: web::Data<AppState>,
    task_id: i64,
    launch_execution: bool,
) -> Result<bool, AppError> {
    if !claim_task_for_handoff(&state, task_id)? {
        return Ok(false);
    }

    let result = attempt_auto_handoff_with_claim(state.clone(), task_id, launch_execution).await;

    if result.is_err() {
        release_handoff_claim(&state, task_id)?;
    }

    result
}

async fn attempt_auto_handoff_with_claim(
    state: web::Data<AppState>,
    task_id: i64,
    launch_execution: bool,
) -> Result<bool, AppError> {
    let employees = {
        let conn = state.db.get()?;
        let status = conn.query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get::<_, String>(0),
        )?;

        let workflow_role = match workflow_role_for_status(WorkflowStatus::from_str(&status)) {
            Some(role) => role.as_str().to_string(),
            None => {
                conn.execute(
                    "UPDATE tasks
                     SET auto_handoff_pending = 0, auto_handoff_claimed_at = NULL
                     WHERE id = ?1",
                    params![task_id],
                )?;
                return Ok(false);
            }
        };

        let mut stmt = conn.prepare(
            "SELECT id, agent_backend
             FROM ai_employees
             WHERE workflow_role = ?1 AND status = 'idle'
             ORDER BY id ASC",
        )?;
        let employees = stmt
            .query_map(params![workflow_role], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        employees
    };

    for (employee_id, backend) in employees {
        if !state.adapters.is_available(&backend) {
            continue;
        }

        let launch = {
            let mut conn = state.db.get()?;
            match prepare_assignment(&state, &mut conn, employee_id, task_id) {
                Ok(launch) => Some(launch),
                Err(AppError::Conflict(_)) => None,
                Err(err) => return Err(err),
            }
        };

        if let Some(launch) = launch {
            if launch_execution {
                launch_assignment(state, launch);
            }
            return Ok(true);
        }
    }

    release_handoff_claim(&state, task_id)?;
    Ok(false)
}

fn claim_task_for_handoff(state: &web::Data<AppState>, task_id: i64) -> Result<bool, AppError> {
    let conn = state.db.get()?;
    let affected = conn.execute(
        "UPDATE tasks
         SET auto_handoff_claimed_at = ?1
         WHERE id = ?2
           AND auto_handoff_pending = 1
           AND auto_handoff_claimed_at IS NULL",
        params![Utc::now().to_rfc3339(), task_id],
    )?;
    Ok(affected == 1)
}

fn release_handoff_claim(state: &web::Data<AppState>, task_id: i64) -> Result<(), AppError> {
    let conn = state.db.get()?;
    conn.execute(
        "UPDATE tasks
         SET auto_handoff_claimed_at = NULL
         WHERE id = ?1",
        params![task_id],
    )?;
    Ok(())
}
