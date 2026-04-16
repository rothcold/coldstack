use actix_web::{HttpResponse, web};
use chrono::Utc;
use rusqlite::params;

use crate::db::AppState;
use crate::errors::AppError;
use crate::models::*;

pub async fn get_agents(data: web::Data<AppState>) -> HttpResponse {
    let result = (|| -> Result<Vec<Agent>, AppError> {
        let conn = data.db.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, cli, system_prompt, work_dir, model, max_concurrency, created_at FROM agents ORDER BY created_at DESC",
        )?;

        let agents = stmt
            .query_map([], |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    cli: row.get(2)?,
                    system_prompt: row.get(3)?,
                    work_dir: row.get(4)?,
                    model: row.get(5)?,
                    max_concurrency: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(agents)
    })();

    match result {
        Ok(agents) => HttpResponse::Ok().json(agents),
        Err(e) => e.to_response(),
    }
}

pub async fn create_agent(data: web::Data<AppState>, item: web::Json<CreateAgent>) -> HttpResponse {
    let result = (|| -> Result<Agent, AppError> {
        let conn = data.db.get()?;
        let created_at = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO agents (name, cli, system_prompt, work_dir, model, max_concurrency, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.name,
                item.cli,
                item.system_prompt.as_deref().unwrap_or(""),
                item.work_dir.as_deref().unwrap_or("."),
                item.model,
                item.max_concurrency.unwrap_or(1),
                created_at,
            ],
        ).map_err(|e| match e {
            rusqlite::Error::SqliteFailure(ref err, _) if err.extended_code == 2067 => {
                AppError::Conflict("Agent name already exists".to_string())
            }
            other => AppError::Db(other),
        })?;

        let id = conn.last_insert_rowid();
        Ok(Agent {
            id,
            name: item.name.clone(),
            cli: item.cli.clone(),
            system_prompt: item.system_prompt.clone().unwrap_or_default(),
            work_dir: item.work_dir.clone().unwrap_or_else(|| ".".to_string()),
            model: item.model.clone(),
            max_concurrency: item.max_concurrency.unwrap_or(1),
            created_at,
        })
    })();

    match result {
        Ok(agent) => HttpResponse::Created().json(agent),
        Err(e) => e.to_response(),
    }
}

pub async fn update_agent(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<UpdateAgent>,
) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<Agent, AppError> {
        let conn = data.db.get()?;

        let existing = conn
            .query_row(
                "SELECT id, name, cli, system_prompt, work_dir, model, max_concurrency, created_at FROM agents WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Agent {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        cli: row.get(2)?,
                        system_prompt: row.get(3)?,
                        work_dir: row.get(4)?,
                        model: row.get(5)?,
                        max_concurrency: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
                other => AppError::Db(other),
            })?;

        let new_name = item.name.clone().unwrap_or(existing.name);
        let new_cli = item.cli.clone().unwrap_or(existing.cli);
        let new_system_prompt = item.system_prompt.clone().unwrap_or(existing.system_prompt);
        let new_work_dir = item.work_dir.clone().unwrap_or(existing.work_dir);
        let new_model = match item.model.clone() {
            Some(m) => m,
            None => existing.model,
        };
        let new_max_concurrency = item.max_concurrency.unwrap_or(existing.max_concurrency);

        conn.execute(
            "UPDATE agents SET name = ?1, cli = ?2, system_prompt = ?3, work_dir = ?4, model = ?5, max_concurrency = ?6 WHERE id = ?7",
            params![new_name, new_cli, new_system_prompt, new_work_dir, new_model, new_max_concurrency, id],
        ).map_err(|e| match e {
            rusqlite::Error::SqliteFailure(ref err, _) if err.extended_code == 2067 => {
                AppError::Conflict("Agent name already exists".to_string())
            }
            other => AppError::Db(other),
        })?;

        Ok(Agent {
            id,
            name: new_name,
            cli: new_cli,
            system_prompt: new_system_prompt,
            work_dir: new_work_dir,
            model: new_model,
            max_concurrency: new_max_concurrency,
            created_at: existing.created_at,
        })
    })();

    match result {
        Ok(agent) => HttpResponse::Ok().json(agent),
        Err(e) => e.to_response(),
    }
}

pub async fn delete_agent(data: web::Data<AppState>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();

    let result = (|| -> Result<(), AppError> {
        let conn = data.db.get()?;
        let affected = conn.execute("DELETE FROM agents WHERE id = ?1", params![id])?;
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
