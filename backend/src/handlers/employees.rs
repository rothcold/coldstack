use actix_web::{web, HttpResponse};
use chrono::Utc;
use rusqlite::params;

use crate::adapters::{EmployeeConfig, TaskInfo};
use crate::db::AppState;
use crate::errors::AppError;
use crate::models::*;

pub async fn get_employees(data: web::Data<AppState>) -> HttpResponse {
    let result = (|| -> Result<Vec<Employee>, AppError> {
        let conn = data.db.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, role, department, agent_backend, system_prompt, status, created_at FROM ai_employees ORDER BY id",
        )?;

        let employees = stmt
            .query_map([], |row| {
                let status_str: String = row.get(6)?;
                Ok(Employee {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    role: row.get(2)?,
                    department: row.get(3)?,
                    agent_backend: row.get(4)?,
                    system_prompt: row.get(5)?,
                    status: EmployeeStatus::from_str(&status_str),
                    created_at: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(employees)
    })();

    match result {
        Ok(employees) => HttpResponse::Ok().json(employees),
        Err(e) => e.to_response(),
    }
}

pub async fn get_employee(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<Employee, AppError> {
        let conn = data.db.get()?;
        conn.query_row(
            "SELECT id, name, role, department, agent_backend, system_prompt, status, created_at FROM ai_employees WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get(6)?;
                Ok(Employee {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    role: row.get(2)?,
                    department: row.get(3)?,
                    agent_backend: row.get(4)?,
                    system_prompt: row.get(5)?,
                    status: EmployeeStatus::from_str(&status_str),
                    created_at: row.get(7)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
            other => AppError::Db(other),
        })
    })();

    match result {
        Ok(employee) => HttpResponse::Ok().json(employee),
        Err(e) => e.to_response(),
    }
}

pub async fn create_employee(
    data: web::Data<AppState>,
    item: web::Json<CreateEmployee>,
) -> HttpResponse {
    let result = (|| -> Result<Employee, AppError> {
        let conn = data.db.get()?;
        let created_at = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO ai_employees (name, role, department, agent_backend, system_prompt, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![item.name, item.role, item.department, item.agent_backend, item.system_prompt, created_at],
        )?;

        let id = conn.last_insert_rowid();
        Ok(Employee {
            id,
            name: item.name.clone(),
            role: item.role.clone(),
            department: item.department.clone(),
            agent_backend: item.agent_backend.clone(),
            system_prompt: item.system_prompt.clone(),
            status: EmployeeStatus::Idle,
            created_at,
        })
    })();

    match result {
        Ok(employee) => HttpResponse::Created().json(employee),
        Err(e) => e.to_response(),
    }
}

pub async fn update_employee(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<UpdateEmployee>,
) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<Employee, AppError> {
        let conn = data.db.get()?;

        let existing = conn
            .query_row(
                "SELECT id, name, role, department, agent_backend, system_prompt, status, created_at FROM ai_employees WHERE id = ?1",
                params![id],
                |row| {
                    let status_str: String = row.get(6)?;
                    Ok(Employee {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        role: row.get(2)?,
                        department: row.get(3)?,
                        agent_backend: row.get(4)?,
                        system_prompt: row.get(5)?,
                        status: EmployeeStatus::from_str(&status_str),
                        created_at: row.get(7)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
                other => AppError::Db(other),
            })?;

        let new_name = item.name.clone().unwrap_or(existing.name);
        let new_role = item.role.clone().unwrap_or(existing.role);
        let new_department = item.department.clone().unwrap_or(existing.department);
        let new_backend = item.agent_backend.clone().unwrap_or(existing.agent_backend);
        let new_prompt = match item.system_prompt.clone() {
            Some(p) => p,
            None => existing.system_prompt,
        };

        conn.execute(
            "UPDATE ai_employees SET name = ?1, role = ?2, department = ?3, agent_backend = ?4, system_prompt = ?5 WHERE id = ?6",
            params![new_name, new_role, new_department, new_backend, new_prompt, id],
        )?;

        Ok(Employee {
            id,
            name: new_name,
            role: new_role,
            department: new_department,
            agent_backend: new_backend,
            system_prompt: new_prompt,
            status: existing.status,
            created_at: existing.created_at,
        })
    })();

    match result {
        Ok(employee) => HttpResponse::Ok().json(employee),
        Err(e) => e.to_response(),
    }
}

pub async fn delete_employee(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<Vec<i64>, AppError> {
        let conn = data.db.get()?;

        // Get running executions to cancel them
        let mut stmt = conn.prepare("SELECT id FROM task_executions WHERE employee_id = ?1 AND status = 'running'")?;
        let running_ids: Vec<i64> = stmt.query_map(params![id], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        // Cancel any running execution for this employee
        conn.execute(
            "UPDATE task_executions SET status = 'cancelled', finished_at = datetime('now') WHERE employee_id = ?1 AND status = 'running'",
            params![id],
        )?;

        let affected = conn.execute("DELETE FROM ai_employees WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(running_ids)
    })();

    match result {
        Ok(running_ids) => {
            // Signal running processes to stop
            let running = data.running.lock().await;
            for execution_id in running_ids {
                if let Some(entry) = running.get(&execution_id) {
                    let _ = entry.cancel_tx.send(());
                }
            }
            HttpResponse::NoContent().finish()
        }
        Err(e) => e.to_response(),
    }
}

pub async fn assign_task(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> HttpResponse {
    let (employee_id, task_id) = path.into_inner();

    let result = (|| -> Result<(Execution, TaskInfo, EmployeeConfig, String), AppError> {
        let mut conn = data.db.get()?;
        let tx = conn.transaction()?;

        // Get task info
        let (title, description, task_id_str): (String, String, String) = tx
            .query_row(
                "SELECT title, description, task_id FROM tasks WHERE id = ?1",
                params![task_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
                other => AppError::Db(other),
            })?;

        // Atomic update: only succeed if employee is idle
        let affected = tx.execute(
            "UPDATE ai_employees SET status = 'working' WHERE id = ?1 AND status = 'idle'",
            params![employee_id],
        )?;

        if affected == 0 {
            // Check if employee exists to distinguish between 404 and 409
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM ai_employees WHERE id = ?1",
                    params![employee_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if !exists {
                return Err(AppError::NotFound);
            }
            return Err(AppError::Conflict("Employee is not idle".to_string()));
        }

        // Get employee info for execution setup
        let (name, role, backend, system_prompt): (String, String, String, Option<String>) = tx
            .query_row(
                "SELECT name, role, agent_backend, system_prompt FROM ai_employees WHERE id = ?1",
                params![employee_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;

        let started_at = Utc::now().to_rfc3339();

        // Create execution
        tx.execute(
            "INSERT INTO task_executions (task_id, employee_id, started_at, status) VALUES (?1, ?2, ?3, 'running')",
            params![task_id, employee_id, started_at],
        )?;
        let execution_id = tx.last_insert_rowid();

        tx.commit()?;

        let execution = Execution {
            id: execution_id,
            task_id,
            employee_id,
            started_at,
            finished_at: None,
            exit_code: None,
            status: ExecutionStatus::Running,
        };

        let task_info = TaskInfo {
            title,
            description,
            task_id: task_id_str,
        };

        let employee_config = EmployeeConfig {
            name,
            role,
            system_prompt,
        };

        Ok((execution, task_info, employee_config, backend))
    })();

    match result {
        Ok((execution, task_info, employee_config, backend)) => {
            // Spawn agent process in background
            let execution_id = execution.id;
            tokio::spawn(super::executions::spawn_execution(
                data,
                execution_id,
                task_info,
                employee_config,
                backend,
            ));
            HttpResponse::Created().json(execution)
        }
        Err(e) => e.to_response(),
    }
}

pub async fn cancel_execution(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let execution_id = path.into_inner();

    let result = (|| -> Result<Execution, AppError> {
        let conn = data.db.get()?;

        // Get execution, must be running
        let (task_id, employee_id, started_at, status_str): (i64, i64, String, String) = conn
            .query_row(
                "SELECT task_id, employee_id, started_at, status FROM task_executions WHERE id = ?1",
                params![execution_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
                other => AppError::Db(other),
            })?;

        if status_str != "running" {
            return Err(AppError::Conflict(
                "Execution is not running".to_string(),
            ));
        }

        let finished_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE task_executions SET status = 'cancelled', finished_at = ?1 WHERE id = ?2",
            params![finished_at, execution_id],
        )?;

        // Reset employee to idle
        conn.execute(
            "UPDATE ai_employees SET status = 'idle' WHERE id = ?1",
            params![employee_id],
        )?;

        Ok(Execution {
            id: execution_id,
            task_id,
            employee_id,
            started_at,
            finished_at: Some(finished_at),
            exit_code: None,
            status: ExecutionStatus::Cancelled,
        })
    })();

    match result {
        Ok(execution) => {
            // Signal running process to stop
            let running = data.running.lock().await;
            if let Some(entry) = running.get(&execution_id) {
                let _ = entry.cancel_tx.send(());
            }
            HttpResponse::Ok().json(execution)
        }
        Err(e) => e.to_response(),
    }
}

pub async fn get_executions(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let employee_id = path.into_inner();

    let result = (|| -> Result<Vec<Execution>, AppError> {
        let conn = data.db.get()?;

        // Verify employee exists
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ai_employees WHERE id = ?1",
            params![employee_id],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Err(AppError::NotFound);
        }

        let mut stmt = conn.prepare(
            "SELECT id, task_id, employee_id, started_at, finished_at, exit_code, status FROM task_executions WHERE employee_id = ?1 ORDER BY started_at DESC",
        )?;

        let executions = stmt
            .query_map(params![employee_id], |row| {
                let status_str: String = row.get(6)?;
                Ok(Execution {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    employee_id: row.get(2)?,
                    started_at: row.get(3)?,
                    finished_at: row.get(4)?,
                    exit_code: row.get(5)?,
                    status: ExecutionStatus::from_str(&status_str),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(executions)
    })();

    match result {
        Ok(executions) => HttpResponse::Ok().json(executions),
        Err(e) => e.to_response(),
    }
}
