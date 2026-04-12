mod adapters;
mod db;
mod errors;
mod handlers;
mod models;
mod workflow;

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
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
            .route("/tasks/{id}", web::get().to(handlers::tasks::get_task_detail))
            .route("/tasks/{id}", web::put().to(handlers::tasks::update_task))
            .route("/tasks/{id}", web::delete().to(handlers::tasks::delete_task))
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
            .route("/agents/{id}", web::put().to(handlers::agents::update_agent))
            .route(
                "/agents/{id}",
                web::delete().to(handlers::agents::delete_agent),
            )
            .route("/employees", web::get().to(handlers::employees::get_employees))
            .route("/employees", web::post().to(handlers::employees::create_employee))
            .route("/employees/{id}", web::get().to(handlers::employees::get_employee))
            .route("/employees/{id}", web::put().to(handlers::employees::update_employee))
            .route("/employees/{id}", web::delete().to(handlers::employees::delete_employee))
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
    use actix_web::{test, web, App};
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

        let mut stmt = conn.prepare("PRAGMA table_info(ai_employees)").unwrap();
        let employee_cols: Vec<String> = stmt
            .query_map([], |r| r.get(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(employee_cols.contains(&"workflow_role".to_string()));
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
                "assignee": "alice"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["archived"], false);
        assert_eq!(body["status"], "Plan");
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
                "description": ""
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
                "description": "Trace me"
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
        assert!(body["events"].as_array().unwrap().is_empty());
        assert!(body["current_action_label"].is_string());
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
                "description": ""
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
                "description": ""
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
                "description": ""
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
                "description": ""
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute("UPDATE tasks SET status = 'Review' WHERE id = ?1", [task_id])
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
                "description": ""
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
                "assignee": "Alice"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute("UPDATE tasks SET status = 'Review' WHERE id = ?1", [task_id])
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
                "description": ""
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
                "description": ""
            }))
            .to_request();
        let task_resp = test::call_service(&app, task_req).await;
        let task: serde_json::Value = test::read_body_json(task_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        {
            let conn = state.db.get().unwrap();
            conn.execute("UPDATE tasks SET status = 'Review' WHERE id = ?1", [task_id])
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
                "description": ""
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
