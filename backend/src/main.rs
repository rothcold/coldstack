mod adapters;
mod db;
mod errors;
mod handlers;
mod models;
mod orchestration;
mod task_source;
mod workflow;

use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist"]
struct Asset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime.as_ref())
                .body(content.data.into_owned())
        }
        None => {
            let index = Asset::get("index.html").expect("index.html not found");
            HttpResponse::Ok()
                .content_type("text/html")
                .body(index.data.into_owned())
        }
    }
}

async fn index() -> impl Responder {
    handle_embedded_file("index.html")
}

async fn static_assets(path: web::Path<String>) -> impl Responder {
    handle_embedded_file(&path)
}

fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/tasks", web::get().to(handlers::tasks::get_tasks))
            .route("/tasks", web::post().to(handlers::tasks::create_task))
            .route(
                "/tasks/{id}",
                web::get().to(handlers::tasks::get_task_detail),
            )
            .route("/tasks/{id}", web::put().to(handlers::tasks::update_task))
            .route(
                "/tasks/{id}",
                web::delete().to(handlers::tasks::delete_task),
            )
            .route(
                "/tasks/{id}/publish",
                web::post().to(handlers::tasks::publish_task_branch),
            )
            .route(
                "/tasks/{id}/transition",
                web::post().to(handlers::tasks::transition_task),
            )
            .route(
                "/tasks/{id}/subtasks",
                web::post().to(handlers::tasks::add_subtask),
            )
            .route(
                "/tasks/{id}/subtasks/{subtask_id}/toggle",
                web::post().to(handlers::tasks::toggle_subtask),
            )
            .route(
                "/tasks/{id}/subtasks/{subtask_id}",
                web::put().to(handlers::tasks::update_subtask),
            )
            .route("/agents", web::get().to(handlers::agents::get_agents))
            .route("/agents", web::post().to(handlers::agents::create_agent))
            .route(
                "/agents/{id}",
                web::put().to(handlers::agents::update_agent),
            )
            .route(
                "/agents/{id}",
                web::delete().to(handlers::agents::delete_agent),
            )
            .route(
                "/employees",
                web::get().to(handlers::employees::get_employees),
            )
            .route(
                "/employees",
                web::post().to(handlers::employees::create_employee),
            )
            .route(
                "/employees/{id}",
                web::get().to(handlers::employees::get_employee),
            )
            .route(
                "/employees/{id}",
                web::put().to(handlers::employees::update_employee),
            )
            .route(
                "/employees/{id}",
                web::delete().to(handlers::employees::delete_employee),
            )
            .route(
                "/employees/{id}/reset",
                web::post().to(handlers::employees::reset_employee),
            )
            .route(
                "/employees/{id}/assign/{task_id}",
                web::post().to(handlers::employees::assign_task),
            )
            .route(
                "/employees/{id}/executions",
                web::get().to(handlers::employees::get_executions),
            )
            .route(
                "/employees/{id}/current_execution",
                web::get().to(handlers::employees::get_current_execution),
            )
            .route(
                "/executions/{id}/cancel",
                web::post().to(handlers::employees::cancel_execution),
            )
            .route(
                "/executions/{id}/stream",
                web::get().to(handlers::executions::stream_execution),
            )
            .route(
                "/executions/{id}/output",
                web::get().to(handlers::executions::get_execution_output),
            ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test, web};
    use rusqlite::Connection;

    fn setup_pool() -> db::DbPool {
        let pool = db::create_memory_pool().expect("Failed to create test pool");
        let conn = pool.get().expect("Failed to get test connection");
        db::init_db(&conn).expect("Failed to init test db");
        pool
    }

    fn make_state(pool: db::DbPool) -> web::Data<db::AppState> {
        web::Data::new(db::AppState::new(pool))
    }

    macro_rules! test_app {
        ($state:expr) => {
            test::init_service(
                App::new()
                    .app_data($state.clone())
                    .configure(configure_api_routes),
            )
            .await
        };
    }

    #[::core::prelude::v1::test]
    fn test_init_db_creates_workflow_columns() {
        let pool = setup_pool();
        let conn = pool.get().unwrap();

        let mut stmt = conn.prepare("PRAGMA table_info(tasks)").unwrap();
        let task_cols: Vec<String> = stmt
            .query_map([], |r| r.get(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(task_cols.contains(&"archived".to_string()));
        assert!(task_cols.contains(&"status".to_string()));
        assert!(task_cols.contains(&"source_branch".to_string()));
        let branch_index_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_tasks_branch_name'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(branch_index_exists, 1);

        let mut stmt = conn.prepare("PRAGMA table_info(ai_employees)").unwrap();
        let employee_cols: Vec<String> = stmt
            .query_map([], |r| r.get(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(employee_cols.contains(&"workflow_role".to_string()));
        assert!(employee_cols.contains(&"custom_prompt".to_string()));
        assert!(task_cols.contains(&"auto_handoff_pending".to_string()));
        assert!(task_cols.contains(&"auto_handoff_claimed_at".to_string()));
    }

    #[::core::prelude::v1::test]
    fn test_branch_name_unique_index_rejects_duplicates() {
        let pool = setup_pool();
        let conn = pool.get().unwrap();

        conn.execute(
            "INSERT INTO tasks (task_id, title, description, source, source_branch, branch_name, archived, status, created_at)
             VALUES ('T-ONE', 'One', '', '/tmp/project', 'main', 'task/shared-branch', 0, 'Plan', '2026-04-18T00:00:00Z')",
            [],
        )
        .unwrap();

        let error = conn
            .execute(
                "INSERT INTO tasks (task_id, title, description, source, source_branch, branch_name, archived, status, created_at)
                 VALUES ('T-TWO', 'Two', '', '/tmp/project', 'main', 'task/shared-branch', 0, 'Plan', '2026-04-18T00:00:01Z')",
                [],
            )
            .unwrap_err();

        assert!(matches!(error, rusqlite::Error::SqliteFailure(_, _)));
    }

    #[::core::prelude::v1::test]
    fn test_workflow_status_roundtrip() {
        for status in [
            models::WorkflowStatus::Plan,
            models::WorkflowStatus::Design,
            models::WorkflowStatus::Coding,
            models::WorkflowStatus::Review,
            models::WorkflowStatus::QA,
            models::WorkflowStatus::NeedsHuman,
            models::WorkflowStatus::Done,
        ] {
            assert_eq!(models::WorkflowStatus::from_str(status.as_str()), status);
        }
    }

    #[::core::prelude::v1::test]
    fn test_init_db_migrates_completed_to_archived() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                completed INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'Pending',
                assignee TEXT,
                created_at TEXT NOT NULL
             );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (task_id, title, description, completed, status, created_at) VALUES ('T-OLD', 'Legacy', '', 1, 'Doing', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        db::init_db(&conn).unwrap();

        let (archived, status): (i32, String) = conn
            .query_row(
                "SELECT archived, status FROM tasks WHERE task_id = 'T-OLD'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(archived, 1);
        assert_eq!(status, "Coding");
    }

    #[::core::prelude::v1::test]
    fn test_init_db_migrates_system_prompt_to_custom_prompt() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE ai_employees (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                role TEXT NOT NULL,
                department TEXT NOT NULL,
                agent_backend TEXT NOT NULL,
                system_prompt TEXT,
                status TEXT NOT NULL DEFAULT 'idle',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ai_employees (name, role, department, agent_backend, system_prompt, created_at) VALUES ('Legacy', 'Reviewer', 'QA', 'claude_code', 'legacy custom prompt', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        db::init_db(&conn).unwrap();
        db::init_db(&conn).unwrap();

        let (custom_prompt, workflow_role): (Option<String>, String) = conn
            .query_row(
                "SELECT custom_prompt, workflow_role FROM ai_employees WHERE name = 'Legacy'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(custom_prompt.as_deref(), Some("legacy custom prompt"));
        assert_eq!(workflow_role, "reviewer");
    }

    #[::core::prelude::v1::test]
    fn test_compose_employee_prompt_by_role() {
        let cases = [
            (models::WorkflowRole::Planner, "implementation plan only"),
            (models::WorkflowRole::Designer, "design standards"),
            (models::WorkflowRole::Coder, "Write and update code only"),
            (models::WorkflowRole::Reviewer, "Review code changes only"),
            (models::WorkflowRole::Qa, "Test behavior only"),
        ];

        for (role, expected) in cases {
            let prompt = workflow::compose_employee_prompt(role, None);
            assert!(
                prompt.contains(expected),
                "missing '{expected}' for {:?}",
                role
            );
        }
    }

    #[::core::prelude::v1::test]
    fn test_compose_employee_prompt_appends_custom_instructions() {
        let prompt = workflow::compose_employee_prompt(
            models::WorkflowRole::Coder,
            Some("Only touch backend/src"),
        );
        assert!(prompt.starts_with("You are the coder."));
        assert!(prompt.ends_with("Only touch backend/src"));
        assert!(prompt.contains("\n\nOnly touch backend/src"));
    }

    #[::core::prelude::v1::test]
    fn test_assign_path_builds_employee_config_from_composed_prompt() {
        let config = handlers::employees::build_employee_config(
            models::WorkflowRole::Reviewer,
            Some("Focus on regressions".to_string()),
        );
        let prompt = config.system_prompt.as_deref().unwrap();
        assert!(prompt.contains("Review code changes only"));
        assert!(prompt.ends_with("Focus on regressions"));
    }

    #[::core::prelude::v1::test]
    fn test_seed_employees_uses_supported_backend() {
        let pool = setup_pool();
        let conn = pool.get().unwrap();

        db::seed_employees(&conn).unwrap();

        let unsupported: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_employees WHERE agent_backend != 'claude_code'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unsupported, 0);
    }

    #[actix_web::test]
    async fn test_create_task_returns_workflow_task() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-001",
                "title": "Build feature",
                "description": "Implement login",
                "source": "/tmp/project",
                "assignee": "alice"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["archived"], false);
        assert_eq!(body["status"], "Plan");
        assert_eq!(body["source"], "/tmp/project");
        assert_eq!(body["source_branch"], "main");
        assert_eq!(body["branch_name"], "task/build-feature");
    }

    #[actix_web::test]
    async fn test_create_task_requires_source() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-NOSOURCE",
                "title": "Build feature",
                "description": "Implement login",
                "source": ""
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_get_tasks_returns_board_summary_shape() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-SUM",
                "title": "Board item",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), 201);

        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        assert_eq!(list_resp.status(), 200);

        let body: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0]["board_group"], "Plan");
        assert!(body[0].get("events").is_none());
        assert!(body[0].get("subtasks").is_none());
    }

    #[actix_web::test]
    async fn test_get_task_detail_returns_timeline_payload() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-DETAIL",
                "title": "Detail item",
                "description": "Trace me",
                "source": "/tmp/project"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let detail_req = test::TestRequest::get()
            .uri(&format!("/api/tasks/{}", task_id))
            .to_request();
        let detail_resp = test::call_service(&app, detail_req).await;
        assert_eq!(detail_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(detail_resp).await;
        assert_eq!(body["task"]["task_id"], "T-DETAIL");
        assert_eq!(body["task"]["source"], "/tmp/project");
        assert_eq!(body["task"]["source_branch"], "main");
        assert_eq!(body["task"]["branch_name"], "task/detail-item");
        assert!(body["events"].as_array().unwrap().is_empty());
        assert!(body["current_action_label"].is_string());
    }

    #[actix_web::test]
    async fn test_create_task_allocates_unique_human_readable_branch_name() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let first_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-BRANCH-1",
                "title": "Build weather forecast website",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let first_resp = test::call_service(&app, first_req).await;
        assert_eq!(first_resp.status(), 201);
        let first: serde_json::Value = test::read_body_json(first_resp).await;

        let second_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-BRANCH-2",
                "title": "Build weather forecast website",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let second_resp = test::call_service(&app, second_req).await;
        assert_eq!(second_resp.status(), 201);
        let second: serde_json::Value = test::read_body_json(second_resp).await;

        assert_eq!(first["branch_name"], "task/build-weather-forecast-website");
        assert_eq!(second["branch_name"], "task/build-weather-forecast-website-2");
    }

    #[actix_web::test]
    async fn test_create_task_accepts_custom_source_branch() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-SRC-BRANCH",
                "title": "Build feature",
                "description": "Implement login",
                "source": "/tmp/project",
                "source_branch": "develop"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["source_branch"], "develop");
    }

    #[actix_web::test]
    async fn test_transition_happy_path_writes_event() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Planner",
                "role": "Planner",
                "workflow_role": "planner",
                "department": "Ops",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-MOVE",
                "title": "Move it",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let transition_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "employee",
                "actor_id": employee_id,
                "from_status": "Plan",
                "to_status": "Design",
                "action": "advance"
            }))
            .to_request();
        let transition_resp = test::call_service(&app, transition_req).await;
        assert_eq!(transition_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(transition_resp).await;
        assert_eq!(body["task"]["task"]["status"], "Design");
        assert_eq!(body["task"]["events"].as_array().unwrap().len(), 1);
    }

    #[actix_web::test]
    async fn test_archive_requires_human_actor() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-ARCH",
                "title": "Archive me",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute("UPDATE tasks SET status = 'Done' WHERE id = ?1", [task_id])
                .unwrap();
        }

        let archive_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "employee",
                "actor_label": "automation",
                "from_status": "Done",
                "to_status": "Done",
                "action": "archive"
            }))
            .to_request();
        let archive_resp = test::call_service(&app, archive_req).await;
        assert_eq!(archive_resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_create_employee_persists_workflow_role() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Reviewer",
                "role": "Reviewer",
                "workflow_role": "reviewer",
                "department": "QA",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["workflow_role"], "reviewer");
        assert_eq!(body["custom_prompt"], serde_json::Value::Null);
        assert!(
            body["system_prompt"]
                .as_str()
                .unwrap()
                .contains("Review code changes only")
        );
    }

    #[actix_web::test]
    async fn test_create_employee_appends_custom_prompt() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Coder",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code",
                "custom_prompt": "Only edit backend/src"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["custom_prompt"], "Only edit backend/src");
        assert!(
            body["system_prompt"]
                .as_str()
                .unwrap()
                .ends_with("Only edit backend/src")
        );
    }

    #[actix_web::test]
    async fn test_update_employee_rebuilds_prompt_for_new_role() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Agent",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code",
                "custom_prompt": "Stay in src/"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let created: serde_json::Value = test::read_body_json(create_resp).await;

        let update_req = test::TestRequest::put()
            .uri(&format!(
                "/api/employees/{}",
                created["id"].as_i64().unwrap()
            ))
            .set_json(serde_json::json!({
                "role": "Reviewer",
                "workflow_role": "reviewer"
            }))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let updated: serde_json::Value = test::read_body_json(update_resp).await;
        assert_eq!(updated["workflow_role"], "reviewer");
        assert_eq!(updated["custom_prompt"], "Stay in src/");
        let prompt = updated["system_prompt"].as_str().unwrap();
        assert!(prompt.contains("Review code changes only"));
        assert!(!prompt.contains("Write and update code only"));
        assert!(prompt.ends_with("Stay in src/"));
    }

    #[actix_web::test]
    async fn test_update_employee_preserves_custom_prompt_when_omitted() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Planner",
                "role": "Planner",
                "workflow_role": "planner",
                "department": "Product",
                "agent_backend": "claude_code",
                "custom_prompt": "Call out rollout risks"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let created: serde_json::Value = test::read_body_json(create_resp).await;

        let update_req = test::TestRequest::put()
            .uri(&format!(
                "/api/employees/{}",
                created["id"].as_i64().unwrap()
            ))
            .set_json(serde_json::json!({
                "department": "Operations"
            }))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let updated: serde_json::Value = test::read_body_json(update_resp).await;
        assert_eq!(updated["custom_prompt"], "Call out rollout risks");
        assert!(
            updated["system_prompt"]
                .as_str()
                .unwrap()
                .ends_with("Call out rollout risks")
        );
    }

    #[actix_web::test]
    async fn test_create_employee_rejects_unsupported_backend() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Bad backend",
                "role": "Reviewer",
                "workflow_role": "reviewer",
                "department": "QA",
                "agent_backend": "codex"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_get_current_execution_returns_running_row() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Runner",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-CURRENT",
                "title": "Current execution",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, status) VALUES (?1, ?2, datetime('now'), 'running')",
                rusqlite::params![task_id, employee_id],
            )
            .unwrap();
        }

        let current_req = test::TestRequest::get()
            .uri(&format!("/api/employees/{}/current_execution", employee_id))
            .to_request();
        let current_resp = test::call_service(&app, current_req).await;
        assert_eq!(current_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(current_resp).await;
        assert_eq!(body["task_id"], task_id);
        assert_eq!(body["task_key"], "T-CURRENT");
    }

    #[actix_web::test]
    async fn test_assign_task_rejects_missing_source() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Coder",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_id = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, archived, status, created_at) VALUES ('T-LEGACY', 'Legacy task', '', 0, 'Coding', datetime('now'))",
                [],
            )
            .unwrap();
            conn.last_insert_rowid()
        };

        let req = test::TestRequest::post()
            .uri(&format!("/api/employees/{}/assign/{}", employee_id, task_id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_assign_task_returns_not_found_for_missing_employee() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-MISSING-EMPLOYEE",
                "title": "Assign target",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let req = test::TestRequest::post()
            .uri(&format!("/api/employees/999999/assign/{}", task_id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_cancel_execution_returns_employee_to_idle() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Stopper",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-STOP",
                "title": "Stop execution",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let execution_id = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE ai_employees SET status = 'working' WHERE id = ?1",
                [employee_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, status) VALUES (?1, ?2, datetime('now'), 'running')",
                rusqlite::params![task_id, employee_id],
            )
            .unwrap();
            conn.last_insert_rowid()
        };

        let cancel_req = test::TestRequest::post()
            .uri(&format!("/api/executions/{}/cancel", execution_id))
            .to_request();
        let cancel_resp = test::call_service(&app, cancel_req).await;
        assert_eq!(cancel_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(cancel_resp).await;
        assert_eq!(body["status"], "cancelled");

        let (execution_status, employee_status): (String, String) = {
            let conn = state.db.get().unwrap();
            (
                conn.query_row(
                    "SELECT status FROM task_executions WHERE id = ?1",
                    [execution_id],
                    |row| row.get(0),
                )
                .unwrap(),
                conn.query_row(
                    "SELECT status FROM ai_employees WHERE id = ?1",
                    [employee_id],
                    |row| row.get(0),
                )
                .unwrap(),
            )
        };
        assert_eq!(execution_status, "cancelled");
        assert_eq!(employee_status, "idle");
    }

    #[actix_web::test]
    async fn test_cancel_execution_rejects_non_running_execution() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Already done",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-STOP-CONFLICT",
                "title": "Conflict execution",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let execution_id = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, status, finished_at) VALUES (?1, ?2, datetime('now'), 'failed', datetime('now'))",
                rusqlite::params![task_id, employee_id],
            )
            .unwrap();
            conn.last_insert_rowid()
        };

        let cancel_req = test::TestRequest::post()
            .uri(&format!("/api/executions/{}/cancel", execution_id))
            .to_request();
        let cancel_resp = test::call_service(&app, cancel_req).await;
        assert_eq!(cancel_resp.status(), 409);
    }

    #[actix_web::test]
    async fn test_cancel_execution_returns_not_found_for_missing_id() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let cancel_req = test::TestRequest::post()
            .uri("/api/executions/9999/cancel")
            .to_request();
        let cancel_resp = test::call_service(&app, cancel_req).await;
        assert_eq!(cancel_resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_reset_employee_clears_error_status_without_rewriting_history() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Recoverable",
                "role": "Reviewer",
                "workflow_role": "reviewer",
                "department": "QA",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-RESET-ERROR",
                "title": "Reset error",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let execution_id = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE ai_employees SET status = 'error' WHERE id = ?1",
                [employee_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, finished_at, exit_code, status) VALUES (?1, ?2, datetime('now'), datetime('now'), 1, 'failed')",
                rusqlite::params![task_id, employee_id],
            )
            .unwrap();
            conn.last_insert_rowid()
        };

        let reset_req = test::TestRequest::post()
            .uri(&format!("/api/employees/{}/reset", employee_id))
            .to_request();
        let reset_resp = test::call_service(&app, reset_req).await;
        assert_eq!(reset_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(reset_resp).await;
        assert_eq!(body["status"], "idle");

        let (employee_status, execution_status): (String, String) = {
            let conn = state.db.get().unwrap();
            (
                conn.query_row(
                    "SELECT status FROM ai_employees WHERE id = ?1",
                    [employee_id],
                    |row| row.get(0),
                )
                .unwrap(),
                conn.query_row(
                    "SELECT status FROM task_executions WHERE id = ?1",
                    [execution_id],
                    |row| row.get(0),
                )
                .unwrap(),
            )
        };
        assert_eq!(employee_status, "idle");
        assert_eq!(execution_status, "failed");
    }

    #[actix_web::test]
    async fn test_reset_employee_clears_stale_working_status() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Stale worker",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE ai_employees SET status = 'working' WHERE id = ?1",
                [employee_id],
            )
            .unwrap();
        }

        let reset_req = test::TestRequest::post()
            .uri(&format!("/api/employees/{}/reset", employee_id))
            .to_request();
        let reset_resp = test::call_service(&app, reset_req).await;
        assert_eq!(reset_resp.status(), 200);

        let employee_status: String = {
            let conn = state.db.get().unwrap();
            conn.query_row(
                "SELECT status FROM ai_employees WHERE id = ?1",
                [employee_id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(employee_status, "idle");
    }

    #[actix_web::test]
    async fn test_reset_employee_rejects_running_execution() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Busy worker",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-RESET-BUSY",
                "title": "Busy reset",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE ai_employees SET status = 'working' WHERE id = ?1",
                [employee_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, status) VALUES (?1, ?2, datetime('now'), 'running')",
                rusqlite::params![task_id, employee_id],
            )
            .unwrap();
        }

        let reset_req = test::TestRequest::post()
            .uri(&format!("/api/employees/{}/reset", employee_id))
            .to_request();
        let reset_resp = test::call_service(&app, reset_req).await;
        assert_eq!(reset_resp.status(), 409);
    }

    #[actix_web::test]
    async fn test_reset_employee_rejects_idle_employee() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let employee_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Already idle",
                "role": "Planner",
                "workflow_role": "planner",
                "department": "Ops",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let employee_resp = test::call_service(&app, employee_req).await;
        let employee: serde_json::Value = test::read_body_json(employee_resp).await;
        let employee_id = employee["id"].as_i64().unwrap();

        let reset_req = test::TestRequest::post()
            .uri(&format!("/api/employees/{}/reset", employee_id))
            .to_request();
        let reset_resp = test::call_service(&app, reset_req).await;
        assert_eq!(reset_resp.status(), 409);
    }

    #[actix_web::test]
    async fn test_reset_employee_returns_not_found_for_missing_id() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let reset_req = test::TestRequest::post()
            .uri("/api/employees/9999/reset")
            .to_request();
        let reset_resp = test::call_service(&app, reset_req).await;
        assert_eq!(reset_resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_reviewer_can_reject_back_to_coding() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let reviewer_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Reviewer",
                "role": "Reviewer",
                "workflow_role": "reviewer",
                "department": "QA",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let reviewer_resp = test::call_service(&app, reviewer_req).await;
        let reviewer: serde_json::Value = test::read_body_json(reviewer_resp).await;

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-REJECT",
                "title": "Reject me",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE tasks SET status = 'Review' WHERE id = ?1",
                [task_id],
            )
            .unwrap();
        }

        let reject_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "employee",
                "actor_id": reviewer["id"],
                "from_status": "Review",
                "to_status": "Coding",
                "action": "reject",
                "note": "Needs implementation fixes"
            }))
            .to_request();
        let reject_resp = test::call_service(&app, reject_req).await;
        assert_eq!(reject_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(reject_resp).await;
        assert_eq!(body["task"]["task"]["status"], "Coding");
        assert_eq!(body["task"]["events"][0]["action"], "reject");
    }

    #[actix_web::test]
    async fn test_human_can_archive_done_task() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-DONE",
                "title": "Archive done task",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute("UPDATE tasks SET status = 'Done' WHERE id = ?1", [task_id])
                .unwrap();
        }

        let archive_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "human",
                "actor_label": "Project owner",
                "from_status": "Done",
                "to_status": "Done",
                "action": "archive"
            }))
            .to_request();
        let archive_resp = test::call_service(&app, archive_req).await;
        assert_eq!(archive_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(archive_resp).await;
        assert_eq!(body["task"]["task"]["archived"], true);
        assert_eq!(body["task"]["events"][0]["action"], "archive");
    }

    #[actix_web::test]
    async fn test_update_task_preserves_workflow_status() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-UPD",
                "title": "Editable task",
                "description": "before",
                "source": "/tmp/project",
                "assignee": "Alice"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE tasks SET status = 'Review' WHERE id = ?1",
                [task_id],
            )
            .unwrap();
        }

        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", task_id))
            .set_json(serde_json::json!({
                "title": "Editable task v2",
                "description": "after",
                "assignee": "Bob"
            }))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(update_resp).await;
        assert_eq!(body["title"], "Editable task v2");
        assert_eq!(body["description"], "after");
        assert_eq!(body["assignee"], "Bob");
        assert_eq!(body["status"], "Review");
    }

    #[actix_web::test]
    async fn test_update_task_cannot_archive_directly() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-NOARCHIVE",
                "title": "Keep visible",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", task_id))
            .set_json(serde_json::json!({
                "archived": true
            }))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(update_resp).await;
        assert_eq!(body["archived"], false);
    }

    #[actix_web::test]
    async fn test_get_tasks_clears_attention_after_advance() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let coder_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Coder",
                "role": "Coder",
                "workflow_role": "coder",
                "department": "Engineering",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let coder_resp = test::call_service(&app, coder_req).await;
        let coder: serde_json::Value = test::read_body_json(coder_resp).await;

        let reviewer_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Reviewer",
                "role": "Reviewer",
                "workflow_role": "reviewer",
                "department": "QA",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let reviewer_resp = test::call_service(&app, reviewer_req).await;
        let reviewer: serde_json::Value = test::read_body_json(reviewer_resp).await;

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-ATTN",
                "title": "Attention lifecycle",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE tasks SET status = 'Review' WHERE id = ?1",
                [task_id],
            )
            .unwrap();
        }

        let reject_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "employee",
                "actor_id": reviewer["id"],
                "from_status": "Review",
                "to_status": "Coding",
                "action": "reject",
                "note": "Still broken"
            }))
            .to_request();
        let reject_resp = test::call_service(&app, reject_req).await;
        assert_eq!(reject_resp.status(), 200);

        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let list_body: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        assert_eq!(list_body[0]["needs_attention"], true);

        let advance_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "employee",
                "actor_id": coder["id"],
                "from_status": "Coding",
                "to_status": "Review",
                "action": "advance"
            }))
            .to_request();
        let advance_resp = test::call_service(&app, advance_req).await;
        assert_eq!(advance_resp.status(), 200);

        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let list_body: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        assert_eq!(list_body[0]["needs_attention"], false);
    }

    #[actix_web::test]
    async fn test_qa_can_reject_back_to_coding() {
        let state = make_state(setup_pool());
        let app = test_app!(state.clone());

        let qa_req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "QA",
                "role": "QA",
                "workflow_role": "qa",
                "department": "QA",
                "agent_backend": "claude_code"
            }))
            .to_request();
        let qa_resp = test::call_service(&app, qa_req).await;
        let qa: serde_json::Value = test::read_body_json(qa_resp).await;

        let task_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-QA-REJECT",
                "title": "QA reject",
                "description": "",
                "source": "/tmp/project"
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute("UPDATE tasks SET status = 'QA' WHERE id = ?1", [task_id])
                .unwrap();
        }

        let reject_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/transition", task_id))
            .set_json(serde_json::json!({
                "actor_type": "employee",
                "actor_id": qa["id"],
                "from_status": "QA",
                "to_status": "Coding",
                "action": "reject",
                "note": "Regression still open"
            }))
            .to_request();
        let reject_resp = test::call_service(&app, reject_req).await;
        assert_eq!(reject_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(reject_resp).await;
        assert_eq!(body["task"]["task"]["status"], "Coding");
        assert_eq!(body["task"]["events"][0]["note"], "Regression still open");
    }

    #[actix_web::test]
    async fn test_execution_success_auto_handoffs_to_smallest_idle_downstream() {
        let state = make_state(setup_pool());
        if !state.adapters.is_available("claude_code") {
            return;
        }

        let (task_id, _planner_id, designer_small_id, designer_large_id, execution_id) = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, source, branch_name, status, created_at) VALUES ('T-HANDOFF', 'Auto handoff', '', '/tmp/project', 'task/auto-handoff', 'Plan', datetime('now'))",
                [],
            )
            .unwrap();
            let task_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Planner', 'Planner', 'planner', 'Ops', 'claude_code', 'idle', datetime('now'))",
                [],
            )
            .unwrap();
            let planner_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Designer A', 'Designer', 'designer', 'Design', 'claude_code', 'idle', datetime('now'))",
                [],
            )
            .unwrap();
            let designer_small_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Designer B', 'Designer', 'designer', 'Design', 'claude_code', 'idle', datetime('now'))",
                [],
            )
            .unwrap();
            let designer_large_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, finished_at, exit_code, status)
                 VALUES (?1, ?2, datetime('now'), datetime('now'), 0, 'completed')",
                rusqlite::params![task_id, planner_id],
            )
            .unwrap();
            let execution_id = conn.last_insert_rowid();
            (
                task_id,
                planner_id,
                designer_small_id,
                designer_large_id,
                execution_id,
            )
        };

        orchestration::process_execution_success(state.clone(), execution_id, false)
            .await
            .unwrap();

        let conn = state.db.get().unwrap();
        let (status, pending, running_employee): (String, i32, i64) = (
            conn.query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |row| row.get(0))
                .unwrap(),
            conn.query_row(
                "SELECT auto_handoff_pending FROM tasks WHERE id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT employee_id FROM task_executions WHERE task_id = ?1 AND id != ?2 ORDER BY id DESC LIMIT 1",
                rusqlite::params![task_id, execution_id],
                |row| row.get(0),
            )
            .unwrap(),
        );
        let designer_small_status: String = conn
            .query_row(
                "SELECT status FROM ai_employees WHERE id = ?1",
                [designer_small_id],
                |row| row.get(0),
            )
            .unwrap();
        let designer_large_status: String = conn
            .query_row(
                "SELECT status FROM ai_employees WHERE id = ?1",
                [designer_large_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(status, "Design");
        assert_eq!(pending, 0);
        assert_eq!(running_employee, designer_small_id);
        assert_eq!(designer_small_status, "working");
        assert_eq!(designer_large_status, "idle");
    }

    #[actix_web::test]
    async fn test_execution_success_marks_pending_when_no_downstream_idle_agent() {
        let state = make_state(setup_pool());
        if !state.adapters.is_available("claude_code") {
            return;
        }

        let (task_id, execution_id) = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, source, branch_name, status, created_at) VALUES ('T-PENDING', 'Pending handoff', '', '/tmp/project', 'task/pending-handoff', 'Plan', datetime('now'))",
                [],
            )
            .unwrap();
            let task_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Planner', 'Planner', 'planner', 'Ops', 'claude_code', 'idle', datetime('now'))",
                [],
            )
            .unwrap();
            let planner_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Designer', 'Designer', 'designer', 'Design', 'claude_code', 'working', datetime('now'))",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO task_executions (task_id, employee_id, started_at, finished_at, exit_code, status)
                 VALUES (?1, ?2, datetime('now'), datetime('now'), 0, 'completed')",
                rusqlite::params![task_id, planner_id],
            )
            .unwrap();
            (task_id, conn.last_insert_rowid())
        };

        orchestration::process_execution_success(state.clone(), execution_id, false)
            .await
            .unwrap();

        let conn = state.db.get().unwrap();
        let (status, pending, claimed_at, execution_count): (String, i32, Option<String>, i64) = (
            conn.query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |row| row.get(0))
                .unwrap(),
            conn.query_row(
                "SELECT auto_handoff_pending FROM tasks WHERE id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT auto_handoff_claimed_at FROM tasks WHERE id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM task_executions WHERE task_id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap(),
        );

        assert_eq!(status, "Design");
        assert_eq!(pending, 1);
        assert!(claimed_at.is_none());
        assert_eq!(execution_count, 1);
    }

    #[actix_web::test]
    async fn test_scanner_picks_up_pending_task_after_agent_becomes_idle() {
        let state = make_state(setup_pool());
        if !state.adapters.is_available("claude_code") {
            return;
        }

        let (task_id, reviewer_id) = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, source, branch_name, status, auto_handoff_pending, created_at)
                 VALUES ('T-SCAN', 'Scanner pickup', '', '/tmp/project', 'task/scanner-pickup', 'Review', 1, datetime('now'))",
                [],
            )
            .unwrap();
            let task_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Reviewer', 'Reviewer', 'reviewer', 'QA', 'claude_code', 'working', datetime('now'))",
                [],
            )
            .unwrap();
            let reviewer_id = conn.last_insert_rowid();
            (task_id, reviewer_id)
        };

        {
            let conn = state.db.get().unwrap();
            conn.execute(
                "UPDATE ai_employees SET status = 'idle' WHERE id = ?1",
                [reviewer_id],
            )
            .unwrap();
        }

        orchestration::process_pending_auto_handoffs(state.clone(), false)
            .await
            .unwrap();

        let conn = state.db.get().unwrap();
        let (pending, execution_count, employee_status): (i32, i64, String) = (
            conn.query_row(
                "SELECT auto_handoff_pending FROM tasks WHERE id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM task_executions WHERE task_id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap(),
            conn.query_row(
                "SELECT status FROM ai_employees WHERE id = ?1",
                [reviewer_id],
                |row| row.get(0),
            )
            .unwrap(),
        );

        assert_eq!(pending, 0);
        assert_eq!(execution_count, 1);
        assert_eq!(employee_status, "working");
    }

    #[actix_web::test]
    async fn test_auto_handoff_claim_prevents_duplicate_assignment() {
        let state = make_state(setup_pool());
        if !state.adapters.is_available("claude_code") {
            return;
        }

        let task_id = {
            let conn = state.db.get().unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, source, branch_name, status, auto_handoff_pending, created_at)
                 VALUES ('T-CLAIM', 'Claim once', '', '/tmp/project', 'task/claim-once', 'Review', 1, datetime('now'))",
                [],
            )
            .unwrap();
            let task_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO ai_employees (name, role, workflow_role, department, agent_backend, status, created_at)
                 VALUES ('Reviewer', 'Reviewer', 'reviewer', 'QA', 'claude_code', 'idle', datetime('now'))",
                [],
            )
            .unwrap();
            task_id
        };

        let (first, second) = tokio::join!(
            orchestration::attempt_auto_handoff(state.clone(), task_id, false),
            orchestration::attempt_auto_handoff(state.clone(), task_id, false)
        );

        let conn = state.db.get().unwrap();
        let execution_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_executions WHERE task_id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(execution_count, 1);
        assert_eq!(u8::from(first.unwrap()) + u8::from(second.unwrap()), 1);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = db::create_pool("tasks.db").expect("Failed to create connection pool");

    {
        let conn = pool.get().expect("Failed to get connection for init");
        db::init_db(&conn).expect("Failed to initialize database");
        db::seed_employees(&conn).expect("Failed to seed employees");
        db::startup_recovery(&conn).expect("Failed startup recovery");
    }

    let state = web::Data::new(db::AppState::new(pool));
    orchestration::start_auto_handoff_scanner(state.clone());

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
            ])
            .allowed_header(actix_web::http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .configure(configure_api_routes)
            .route("/", web::get().to(index))
            .route("/{path:.*}", web::get().to(static_assets))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
