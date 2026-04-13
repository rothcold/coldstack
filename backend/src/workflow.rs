use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::errors::AppError;
use crate::models::{
    Task, TaskDetail, TransitionTaskRequest, WorkflowAction, WorkflowActorType, WorkflowEvent,
    WorkflowRole, WorkflowStatus,
};

/*
Transition pipeline

request
  -> load task + actor
  -> validate from_status matches DB
  -> validate action/role/status transition
  -> write task update
  -> write workflow event
  -> reload detail payload
*/

pub fn infer_workflow_role(display_role: &str) -> WorkflowRole {
    let normalized = display_role.trim().to_lowercase();
    if normalized.contains("review") {
        WorkflowRole::Reviewer
    } else if normalized.contains("qa") || normalized.contains("test") {
        WorkflowRole::Qa
    } else if normalized.contains("design") {
        WorkflowRole::Designer
    } else if normalized.contains("plan") || normalized.contains("product") {
        WorkflowRole::Planner
    } else {
        WorkflowRole::Coder
    }
}

pub fn default_prompt_for_role(role: WorkflowRole) -> &'static str {
    match role {
        WorkflowRole::Planner => {
            "You are the planner. Produce an implementation plan only. Clarify scope, break work into concrete steps, call out risks, and do not write code."
        }
        WorkflowRole::Designer => {
            "You are the designer. Focus only on design standards, interaction quality, hierarchy, accessibility, and visual consistency. Do not write production code."
        }
        WorkflowRole::Coder => {
            "You are the coder. Write and update code only. Keep changes focused, correct, and minimal. Do not spend time on planning or review commentary."
        }
        WorkflowRole::Reviewer => {
            "You are the reviewer. Review code changes only. Find bugs, regressions, missing edge cases, and test gaps. Do not rewrite the feature or implement unrelated changes."
        }
        WorkflowRole::Qa => {
            "You are QA. Test behavior only. Verify flows, reproduce failures, and report precise results. Do not redesign the feature or make unrelated code changes."
        }
        WorkflowRole::Human => {
            "You are the human approver. Make the final decision, give concise direction, and close the loop when the work is actually done."
        }
    }
}

pub fn normalize_custom_prompt(custom_prompt: Option<String>) -> Option<String> {
    custom_prompt.and_then(|prompt| {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn compose_employee_prompt(role: WorkflowRole, custom_prompt: Option<&str>) -> String {
    match custom_prompt.map(str::trim).filter(|prompt| !prompt.is_empty()) {
        Some(custom_prompt) => format!("{}\n\n{}", default_prompt_for_role(role), custom_prompt),
        None => default_prompt_for_role(role).to_string(),
    }
}

pub fn workflow_hint(status: WorkflowStatus) -> Option<String> {
    match status {
        WorkflowStatus::NeedsHuman => Some("Only a human can close this loop.".to_string()),
        WorkflowStatus::Review => Some("Review should approve or send this back.".to_string()),
        WorkflowStatus::QA => Some("QA should verify or reject with evidence.".to_string()),
        _ => None,
    }
}

pub fn current_action_label(status: WorkflowStatus) -> String {
    match status {
        WorkflowStatus::Plan => "Planning in progress".to_string(),
        WorkflowStatus::Design => "Design in progress".to_string(),
        WorkflowStatus::Coding => "Coding in progress".to_string(),
        WorkflowStatus::Review => "Needs review".to_string(),
        WorkflowStatus::QA => "Needs QA".to_string(),
        WorkflowStatus::NeedsHuman => "Needs your decision".to_string(),
        WorkflowStatus::Done => "Ready to archive".to_string(),
    }
}

fn validate_actor(
    role: WorkflowRole,
    actor_type: WorkflowActorType,
    action: WorkflowAction,
    from_status: WorkflowStatus,
    to_status: Option<WorkflowStatus>,
) -> Result<WorkflowStatus, AppError> {
    if action == WorkflowAction::Archive {
        if actor_type != WorkflowActorType::Human {
            return Err(AppError::Forbidden(
                "Only a human can archive tasks".to_string(),
            ));
        }
        if from_status != WorkflowStatus::Done {
            return Err(AppError::Conflict(
                "Only completed tasks can be archived".to_string(),
            ));
        }
        return Ok(WorkflowStatus::Done);
    }

    let target = to_status.ok_or_else(|| {
        AppError::BadRequest("to_status is required for non-archive transitions".to_string())
    })?;

    if action == WorkflowAction::Reject && from_status == target {
        return Err(AppError::BadRequest(
            "Reject transitions must move the task backward".to_string(),
        ));
    }

    if action == WorkflowAction::Reject && (target != WorkflowStatus::Coding) {
        return Err(AppError::BadRequest(
            "Rejected work currently returns to Coding".to_string(),
        ));
    }

    let allowed = match (from_status, target, action, role, actor_type) {
        (WorkflowStatus::Plan, WorkflowStatus::Design, WorkflowAction::Advance, WorkflowRole::Planner, WorkflowActorType::Employee) => true,
        (WorkflowStatus::Design, WorkflowStatus::Coding, WorkflowAction::Advance, WorkflowRole::Designer, WorkflowActorType::Employee) => true,
        (WorkflowStatus::Coding, WorkflowStatus::Review, WorkflowAction::Advance, WorkflowRole::Coder, WorkflowActorType::Employee) => true,
        (WorkflowStatus::Review, WorkflowStatus::QA, WorkflowAction::Advance, WorkflowRole::Reviewer, WorkflowActorType::Employee) => true,
        (WorkflowStatus::Review, WorkflowStatus::Coding, WorkflowAction::Reject, WorkflowRole::Reviewer, WorkflowActorType::Employee) => true,
        (WorkflowStatus::QA, WorkflowStatus::NeedsHuman, WorkflowAction::Advance, WorkflowRole::Qa, WorkflowActorType::Employee) => true,
        (WorkflowStatus::QA, WorkflowStatus::Coding, WorkflowAction::Reject, WorkflowRole::Qa, WorkflowActorType::Employee) => true,
        (WorkflowStatus::NeedsHuman, WorkflowStatus::Done, WorkflowAction::Advance, WorkflowRole::Human, WorkflowActorType::Human) => true,
        (WorkflowStatus::NeedsHuman, WorkflowStatus::Coding, WorkflowAction::Reject, WorkflowRole::Human, WorkflowActorType::Human) => true,
        _ => false,
    };

    if !allowed {
        return Err(AppError::Forbidden(format!(
            "Transition {:?} -> {:?} is not allowed for {:?}",
            from_status, target, role
        )));
    }

    Ok(target)
}

pub fn load_task(conn: &Connection, id: i64) -> Result<Task, AppError> {
    conn.query_row(
        "SELECT id, task_id, title, description, archived, status, assignee, created_at FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            let status_str: String = row.get(5)?;
            Ok(Task {
                id: row.get(0)?,
                task_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                archived: row.get::<_, i32>(4)? == 1,
                status: WorkflowStatus::from_str(&status_str),
                assignee: row.get(6)?,
                created_at: row.get(7)?,
                subtasks: Vec::new(),
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
        other => AppError::Db(other),
    })
}

pub fn load_workflow_events(conn: &Connection, task_id: i64) -> Result<Vec<WorkflowEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, from_status, to_status, actor_type, actor_id, actor_label, action, note, evidence_text, created_at
         FROM task_workflow_events
         WHERE task_id = ?1
         ORDER BY created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(params![task_id], |row| {
        Ok(WorkflowEvent {
            id: row.get(0)?,
            task_id: row.get(1)?,
            from_status: WorkflowStatus::from_str(&row.get::<_, String>(2)?),
            to_status: WorkflowStatus::from_str(&row.get::<_, String>(3)?),
            actor_type: WorkflowActorType::from_str(&row.get::<_, String>(4)?),
            actor_id: row.get(5)?,
            actor_label: row.get(6)?,
            action: match row.get::<_, String>(7)?.as_str() {
                "reject" => WorkflowAction::Reject,
                "archive" => WorkflowAction::Archive,
                _ => WorkflowAction::Advance,
            },
            note: row.get(8)?,
            evidence_text: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;

    Ok(rows.filter_map(Result::ok).collect())
}

pub fn load_task_detail(conn: &Connection, task_id: i64) -> Result<TaskDetail, AppError> {
    let mut task = load_task(conn, task_id)?;

    let mut stmt = conn.prepare(
        "SELECT id, task_id, title, completed, status, assignee FROM subtasks WHERE task_id = ?1",
    )?;
    let subtasks = stmt
        .query_map(params![task_id], |row| {
            Ok(crate::models::Subtask {
                id: row.get(0)?,
                task_id: row.get(1)?,
                title: row.get(2)?,
                completed: row.get::<_, i32>(3)? == 1,
                status: WorkflowStatus::from_str(&row.get::<_, String>(4)?),
                assignee: row.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    task.subtasks = subtasks;

    Ok(TaskDetail {
        current_action_label: current_action_label(task.status),
        current_action_hint: workflow_hint(task.status),
        events: load_workflow_events(conn, task_id)?,
        task,
    })
}

pub fn transition_task(conn: &mut Connection, task_id: i64, req: TransitionTaskRequest) -> Result<TaskDetail, AppError> {
    let tx = conn.transaction()?;
    let task = load_task(&tx, task_id)?;

    if task.status != req.from_status {
        return Err(AppError::Conflict(
            "Task status changed, refresh and try again".to_string(),
        ));
    }

    let actor_label = req.actor_label.unwrap_or_else(|| match req.actor_type {
        WorkflowActorType::Human => "Human".to_string(),
        WorkflowActorType::Employee => "AI Agent".to_string(),
    });

    let role = if req.actor_type == WorkflowActorType::Human {
        WorkflowRole::Human
    } else {
        let actor_id = req
            .actor_id
            .ok_or_else(|| AppError::BadRequest("actor_id is required for employees".to_string()))?;
        tx.query_row(
            "SELECT workflow_role FROM ai_employees WHERE id = ?1",
            params![actor_id],
            |row| Ok(WorkflowRole::from_str(&row.get::<_, String>(0)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound)?
    };

    if req.action == WorkflowAction::Reject
        && req
            .note
            .as_ref()
            .map(|note| note.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(AppError::BadRequest(
            "Reject transitions require a note".to_string(),
        ));
    }

    let next_status = validate_actor(role, req.actor_type, req.action, task.status, req.to_status)?;
    let archived = if req.action == WorkflowAction::Archive { 1 } else { task.archived as i32 };
    let status = if req.action == WorkflowAction::Archive {
        WorkflowStatus::Done
    } else {
        next_status
    };

    tx.execute(
        "UPDATE tasks SET status = ?1, archived = ?2 WHERE id = ?3",
        params![status.as_str(), archived, task_id],
    )?;

    tx.execute(
        "INSERT INTO task_workflow_events (task_id, from_status, to_status, actor_type, actor_id, actor_label, action, note, evidence_text, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            task_id,
            task.status.as_str(),
            status.as_str(),
            req.actor_type.as_str(),
            req.actor_id,
            actor_label,
            req.action.as_str(),
            req.note.map(|s| s.trim().to_string()),
            req.evidence_text.map(|s| s.trim().to_string()),
            Utc::now().to_rfc3339(),
        ],
    )?;

    tx.commit()?;
    load_task_detail(conn, task_id)
}
