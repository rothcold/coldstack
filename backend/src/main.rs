mod adapters;
mod db;
mod errors;
mod handlers;
mod models;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
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
            .route("/tasks/{id}", web::put().to(handlers::tasks::update_task))
            .route("/tasks/{id}", web::delete().to(handlers::tasks::delete_task))
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
    use rusqlite::params;

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

    mod unit {
        use super::super::models::TaskStatus;

        #[test]
        fn test_task_status_from_str_defaults_to_pending() {
            assert_eq!(TaskStatus::from_str("unknown"), TaskStatus::Pending);
            assert_eq!(TaskStatus::from_str(""), TaskStatus::Pending);
        }

        #[test]
        fn test_task_status_roundtrip() {
            for status in [
                TaskStatus::Pending,
                TaskStatus::Doing,
                TaskStatus::Finished,
                TaskStatus::Reviewing,
                TaskStatus::Done,
            ] {
                assert_eq!(TaskStatus::from_str(status.as_str()), status);
            }
        }

        #[test]
        fn test_init_db_creates_tables() {
            let pool = super::setup_pool();
            let conn = pool.get().unwrap();
            let task_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
                .unwrap();
            assert_eq!(task_count, 0);
            let sub_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM subtasks", [], |r| r.get(0))
                .unwrap();
            assert_eq!(sub_count, 0);
        }

        #[test]
        fn test_init_db_creates_correct_columns() {
            let pool = super::setup_pool();
            let conn = pool.get().unwrap();

            let mut stmt = conn.prepare("PRAGMA table_info(tasks)").unwrap();
            let cols: Vec<String> = stmt
                .query_map([], |r| r.get(1))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            for expected in [
                "id",
                "task_id",
                "title",
                "description",
                "completed",
                "status",
                "assignee",
                "created_at",
            ] {
                assert!(
                    cols.contains(&expected.to_string()),
                    "tasks missing column: {}",
                    expected
                );
            }

            let mut stmt = conn.prepare("PRAGMA table_info(subtasks)").unwrap();
            let cols: Vec<String> = stmt
                .query_map([], |r| r.get(1))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            for expected in ["id", "task_id", "title", "completed", "status", "assignee"] {
                assert!(
                    cols.contains(&expected.to_string()),
                    "subtasks missing column: {}",
                    expected
                );
            }
        }

        #[test]
        fn test_init_db_enables_foreign_keys() {
            let pool = super::setup_pool();
            let conn = pool.get().unwrap();
            let fk_enabled: i64 = conn
                .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
                .unwrap();
            assert_eq!(fk_enabled, 1);
        }

        #[test]
        fn test_init_db_is_idempotent() {
            let pool = super::setup_pool();
            let conn = pool.get().unwrap();
            super::super::db::init_db(&conn).unwrap();
        }
    }

    #[actix_web::test]
    async fn test_get_tasks_returns_empty_list() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::get().uri("/api/tasks").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert!(body.is_empty());
    }

    #[actix_web::test]
    async fn test_create_task_returns_created_with_correct_data() {
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
        assert_eq!(body["task_id"], "T-001");
        assert_eq!(body["title"], "Build feature");
        assert_eq!(body["description"], "Implement login");
        assert_eq!(body["assignee"], "alice");
        assert_eq!(body["completed"], false);
        assert_eq!(body["status"], "Pending");
        assert!(body["subtasks"].as_array().unwrap().is_empty());
        assert!(body["id"].as_i64().unwrap() > 0);
    }

    #[actix_web::test]
    async fn test_create_task_without_assignee() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({
                "task_id": "T-002",
                "title": "No assignee task",
                "description": ""
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["assignee"].is_null());
    }

    #[actix_web::test]
    async fn test_create_task_duplicate_id_returns_409() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let payload = serde_json::json!({
            "task_id": "T-DUP",
            "title": "First",
            "description": ""
        });

        let req1 = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(&payload)
            .to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), 201);

        let req2 = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(&payload)
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), 409);
    }

    #[actix_web::test]
    async fn test_get_tasks_returns_all_tasks_ordered_by_created_at_desc() {
        let pool = setup_pool();
        {
            let conn = pool.get().unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, completed, status, created_at) VALUES ('T-OLDER', 'Older', '', 0, 'Pending', '2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
            conn.execute(
                "INSERT INTO tasks (task_id, title, description, completed, status, created_at) VALUES ('T-NEWER', 'Newer', '', 0, 'Pending', '2024-01-02T00:00:00Z')",
                [],
            ).unwrap();
        }
        let state = make_state(pool);
        let app = test_app!(state);

        let req = test::TestRequest::get().uri("/api/tasks").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 2);
        assert_eq!(body[0]["task_id"], "T-NEWER");
        assert_eq!(body[1]["task_id"], "T-OLDER");
    }

    #[actix_web::test]
    async fn test_get_tasks_includes_subtasks() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-S", "title": "Parent", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Sub step"}))
            .to_request();
        test::call_service(&app, sub_req).await;

        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let tasks: Vec<serde_json::Value> = test::read_body_json(list_resp).await;

        let parent = tasks.iter().find(|t| t["task_id"] == "T-S").unwrap();
        let subtasks = parent["subtasks"].as_array().unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0]["title"], "Sub step");
    }

    #[actix_web::test]
    async fn test_update_task_fields_are_persisted() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-U", "title": "Old title", "description": "old"}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let id = task["id"].as_i64().unwrap();

        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", id))
            .set_json(serde_json::json!({
                "title": "New title",
                "description": "new desc",
                "status": "Doing",
                "assignee": "bob",
                "completed": true
            }))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(update_resp).await;
        assert_eq!(body["title"], "New title");
        assert_eq!(body["description"], "new desc");
        assert_eq!(body["status"], "Doing");
        assert_eq!(body["assignee"], "bob");
        assert_eq!(body["completed"], true);
        assert_eq!(body["id"], id);

        let get_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let tasks: Vec<serde_json::Value> =
            test::read_body_json(test::call_service(&app, get_req).await).await;
        let persisted = tasks.iter().find(|t| t["id"] == id).unwrap();
        assert_eq!(persisted["title"], "New title");
        assert_eq!(persisted["status"], "Doing");
        assert_eq!(persisted["assignee"], "bob");
        assert_eq!(persisted["completed"], true);
    }

    #[actix_web::test]
    async fn test_update_task_partial_update_preserves_unchanged_fields() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-P", "title": "Keep me", "description": "keep desc", "assignee": "carol"}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let id = task["id"].as_i64().unwrap();

        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", id))
            .set_json(serde_json::json!({"status": "Reviewing"}))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(update_resp).await;
        assert_eq!(body["title"], "Keep me");
        assert_eq!(body["description"], "keep desc");
        assert_eq!(body["assignee"], "carol");
        assert_eq!(body["status"], "Reviewing");
    }

    #[actix_web::test]
    async fn test_update_task_not_found_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::put()
            .uri("/api/tasks/9999")
            .set_json(serde_json::json!({"title": "ghost"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_update_task_duplicate_task_id_returns_409() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        for (id, title) in [("T-X", "X"), ("T-Y", "Y")] {
            let req = test::TestRequest::post()
                .uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": id, "title": title, "description": ""}))
                .to_request();
            test::call_service(&app, req).await;
        }

        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let tasks: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        let tx = tasks.iter().find(|t| t["task_id"] == "T-X").unwrap();
        let id = tx["id"].as_i64().unwrap();

        let req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", id))
            .set_json(serde_json::json!({"task_id": "T-Y"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409);
    }

    #[actix_web::test]
    async fn test_delete_task_removes_it_from_list() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-DEL", "title": "To delete", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let id = task["id"].as_i64().unwrap();

        let del_req = test::TestRequest::delete()
            .uri(&format!("/api/tasks/{}", id))
            .to_request();
        let del_resp = test::call_service(&app, del_req).await;
        assert_eq!(del_resp.status(), 204);

        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let tasks: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        assert!(!tasks.iter().any(|t| t["task_id"] == "T-DEL"));
    }

    #[actix_web::test]
    async fn test_delete_task_not_found_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::delete()
            .uri("/api/tasks/9999")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_delete_task_cascades_subtasks() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-CASCADE", "title": "Parent", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Child"}))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        let sub: serde_json::Value = test::read_body_json(sub_resp).await;
        let sub_id = sub["id"].as_i64().unwrap();

        let del_req = test::TestRequest::delete()
            .uri(&format!("/api/tasks/{}", task_id))
            .to_request();
        test::call_service(&app, del_req).await;

        let conn = state.db.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtasks WHERE id = ?1",
                params![sub_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[actix_web::test]
    async fn test_add_subtask_returns_created_with_correct_data() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-S2", "title": "Parent", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Write tests", "assignee": "dave"}))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        assert_eq!(sub_resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(sub_resp).await;
        assert_eq!(body["title"], "Write tests");
        assert_eq!(body["assignee"], "dave");
        assert_eq!(body["completed"], false);
        assert_eq!(body["status"], "Pending");
        assert_eq!(body["task_id"], task_id);
        assert!(body["id"].as_i64().unwrap() > 0);
    }

    #[actix_web::test]
    async fn test_toggle_subtask_flips_completed() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-TOG", "title": "P", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Toggle me"}))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        let sub: serde_json::Value = test::read_body_json(sub_resp).await;
        let sub_id = sub["id"].as_i64().unwrap();

        let toggle_req = test::TestRequest::post()
            .uri(&format!(
                "/api/tasks/{}/subtasks/{}/toggle",
                task_id, sub_id
            ))
            .to_request();
        let toggle_resp = test::call_service(&app, toggle_req).await;
        assert_eq!(toggle_resp.status(), 200);

        {
            let conn = state.db.get().unwrap();
            let completed: i32 = conn
                .query_row(
                    "SELECT completed FROM subtasks WHERE id = ?1",
                    params![sub_id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(completed, 1);
        }

        let toggle_req2 = test::TestRequest::post()
            .uri(&format!(
                "/api/tasks/{}/subtasks/{}/toggle",
                task_id, sub_id
            ))
            .to_request();
        test::call_service(&app, toggle_req2).await;

        let conn = state.db.get().unwrap();
        let completed2: i32 = conn
            .query_row(
                "SELECT completed FROM subtasks WHERE id = ?1",
                params![sub_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(completed2, 0);
    }

    #[actix_web::test]
    async fn test_update_subtask_persists_changes() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-US", "title": "P", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Original"}))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        let sub: serde_json::Value = test::read_body_json(sub_resp).await;
        let sub_id = sub["id"].as_i64().unwrap();

        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}/subtasks/{}", task_id, sub_id))
            .set_json(serde_json::json!({
                "title": "Updated",
                "completed": true,
                "status": "Done",
                "assignee": "eve"
            }))
            .to_request();
        let update_resp = test::call_service(&app, update_req).await;
        assert_eq!(update_resp.status(), 200);

        let conn = state.db.get().unwrap();
        let (title, completed, status, assignee): (String, i32, String, Option<String>) = conn
            .query_row(
                "SELECT title, completed, status, assignee FROM subtasks WHERE id = ?1",
                params![sub_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();

        assert_eq!(title, "Updated");
        assert_eq!(completed, 1);
        assert_eq!(status, "Done");
        assert_eq!(assignee, Some("eve".to_string()));
    }

    #[actix_web::test]
    async fn test_update_subtask_partial_preserves_unchanged_fields() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-UPP", "title": "P", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Keep title", "assignee": "frank"}))
            .to_request();
        let sub_resp = test::call_service(&app, sub_req).await;
        let sub: serde_json::Value = test::read_body_json(sub_resp).await;
        let sub_id = sub["id"].as_i64().unwrap();

        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}/subtasks/{}", task_id, sub_id))
            .set_json(serde_json::json!({"status": "Doing"}))
            .to_request();
        test::call_service(&app, update_req).await;

        let conn = state.db.get().unwrap();
        let (title, assignee, status): (String, Option<String>, String) = conn
            .query_row(
                "SELECT title, assignee, status FROM subtasks WHERE id = ?1",
                params![sub_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();

        assert_eq!(title, "Keep title");
        assert_eq!(assignee, Some("frank".to_string()));
        assert_eq!(status, "Doing");
    }

    #[actix_web::test]
    async fn test_update_subtask_not_found_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::put()
            .uri("/api/tasks/1/subtasks/9999")
            .set_json(serde_json::json!({"title": "ghost"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_add_subtask_nonexistent_parent_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks/9999/subtasks")
            .set_json(serde_json::json!({"title": "Orphan"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_toggle_subtask_nonexistent_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks/1/subtasks/9999/toggle")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_toggle_subtask_wrong_task_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let t1: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-TOG1", "title": "P1", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let t2: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-TOG2", "title": "P2", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let t1_id = t1["id"].as_i64().unwrap();
        let t2_id = t2["id"].as_i64().unwrap();

        let sub: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri(&format!("/api/tasks/{}/subtasks", t1_id))
                    .set_json(serde_json::json!({"title": "Sub"}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let sub_id = sub["id"].as_i64().unwrap();

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!(
                    "/api/tasks/{}/subtasks/{}/toggle",
                    t2_id, sub_id
                ))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_update_subtask_wrong_task_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let t1: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-OWN1", "title": "P1", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let t2: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-OWN2", "title": "P2", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let t1_id = t1["id"].as_i64().unwrap();
        let t2_id = t2["id"].as_i64().unwrap();

        let sub: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri(&format!("/api/tasks/{}/subtasks", t1_id))
                    .set_json(serde_json::json!({"title": "Sub"}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let sub_id = sub["id"].as_i64().unwrap();

        let resp = test::call_service(
            &app,
            test::TestRequest::put()
                .uri(&format!("/api/tasks/{}/subtasks/{}", t2_id, sub_id))
                .set_json(serde_json::json!({"title": "Hijacked"}))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_update_task_clear_assignee() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-CLR", "title": "T", "description": "", "assignee": "alice"}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let id = task["id"].as_i64().unwrap();
        assert_eq!(task["assignee"], "alice");

        let resp = test::call_service(
            &app,
            test::TestRequest::put()
                .uri(&format!("/api/tasks/{}", id))
                .set_json(serde_json::json!({"assignee": null}))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);

        let tasks: Vec<serde_json::Value> = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::get().uri("/api/tasks").to_request(),
            )
            .await,
        )
        .await;
        let updated = tasks.iter().find(|t| t["id"] == id).unwrap();
        assert!(updated["assignee"].is_null());
    }

    #[actix_web::test]
    async fn test_update_subtask_clear_assignee() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-SCLR", "title": "P", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        let sub: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri(&format!("/api/tasks/{}/subtasks", task_id))
                    .set_json(serde_json::json!({"title": "Sub", "assignee": "bob"}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let sub_id = sub["id"].as_i64().unwrap();

        let resp = test::call_service(
            &app,
            test::TestRequest::put()
                .uri(&format!("/api/tasks/{}/subtasks/{}", task_id, sub_id))
                .set_json(serde_json::json!({"assignee": null}))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);

        let conn = state.db.get().unwrap();
        let assignee: Option<String> = conn
            .query_row(
                "SELECT assignee FROM subtasks WHERE id = ?1",
                params![sub_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(assignee.is_none());
    }

    #[actix_web::test]
    async fn test_create_task_missing_required_field_returns_400() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-BAD", "description": "no title"}))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 400);
    }

    // ── Employee CRUD ───────────────────────────────────────────────────────

    fn setup_pool_with_seed() -> db::DbPool {
        let pool = setup_pool();
        let conn = pool.get().unwrap();
        db::seed_employees(&conn).unwrap();
        pool
    }

    #[actix_web::test]
    async fn test_get_employees_returns_seeded() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let req = test::TestRequest::get().uri("/api/employees").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 5);
        assert_eq!(body[0]["name"], "Alice");
        assert_eq!(body[0]["status"], "idle");
        assert_eq!(body[0]["agent_backend"], "claude_code");
    }

    #[actix_web::test]
    async fn test_get_employee_by_id() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let req = test::TestRequest::get().uri("/api/employees/1").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["name"], "Alice");
    }

    #[actix_web::test]
    async fn test_get_employee_not_found() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::get().uri("/api/employees/999").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_create_employee() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/employees")
            .set_json(serde_json::json!({
                "name": "Frank",
                "role": "Security Engineer",
                "department": "Security",
                "agent_backend": "claude_code",
                "system_prompt": "You are Frank."
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["name"], "Frank");
        assert_eq!(body["status"], "idle");
        assert!(body["id"].as_i64().unwrap() > 0);
    }

    #[actix_web::test]
    async fn test_update_employee() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let req = test::TestRequest::put()
            .uri("/api/employees/1")
            .set_json(serde_json::json!({"role": "Staff Engineer"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["name"], "Alice");
        assert_eq!(body["role"], "Staff Engineer");
    }

    #[actix_web::test]
    async fn test_delete_employee() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let resp = test::call_service(
            &app,
            test::TestRequest::delete().uri("/api/employees/5").to_request(),
        )
        .await;
        assert_eq!(resp.status(), 204);

        let resp = test::call_service(
            &app,
            test::TestRequest::get().uri("/api/employees/5").to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_delete_employee_not_found() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let resp = test::call_service(
            &app,
            test::TestRequest::delete().uri("/api/employees/999").to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);
    }

    // ── Task assignment ─────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_assign_task_to_employee() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        // Create a task first
        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-ASSIGN", "title": "Do work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        // Assign to Alice (id=1)
        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/employees/1/assign/{}", task_id))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["task_id"], task_id);
        assert_eq!(body["employee_id"], 1);
        assert_eq!(body["status"], "running");

        // Alice should now be "working"
        let emp: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::get().uri("/api/employees/1").to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(emp["status"], "working");
    }

    #[actix_web::test]
    async fn test_assign_task_to_busy_employee_returns_409() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-BUSY", "title": "Work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        // First assignment succeeds
        test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/employees/1/assign/{}", task_id))
                .to_request(),
        )
        .await;

        // Second assignment should fail
        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/employees/1/assign/{}", task_id))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 409);
    }

    #[actix_web::test]
    async fn test_assign_nonexistent_task_returns_404() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/employees/1/assign/9999")
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);
    }

    // ── Cancel execution ────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_cancel_execution() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        // Create task and assign
        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-CANCEL", "title": "Work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        let exec: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri(&format!("/api/employees/1/assign/{}", task_id))
                    .to_request(),
            )
            .await,
        )
        .await;
        let exec_id = exec["id"].as_i64().unwrap();

        // Cancel it
        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/executions/{}/cancel", exec_id))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "cancelled");
        assert!(!body["finished_at"].is_null());

        // Employee should be idle again
        let emp: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::get().uri("/api/employees/1").to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(emp["status"], "idle");
    }

    #[actix_web::test]
    async fn test_cancel_already_completed_returns_409() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-DONE", "title": "Work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        let exec: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri(&format!("/api/employees/1/assign/{}", task_id))
                    .to_request(),
            )
            .await,
        )
        .await;
        let exec_id = exec["id"].as_i64().unwrap();

        // Cancel once (succeeds)
        test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/executions/{}/cancel", exec_id))
                .to_request(),
        )
        .await;

        // Cancel again (fails)
        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/executions/{}/cancel", exec_id))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 409);
    }

    // ── Execution history ───────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_executions_for_employee() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        // Create task and assign
        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-HIST", "title": "Work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/employees/1/assign/{}", task_id))
                .to_request(),
        )
        .await;

        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/employees/1/executions")
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 200);

        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0]["task_id"], task_id);
        assert_eq!(body[0]["status"], "running");
    }

    #[actix_web::test]
    async fn test_get_executions_nonexistent_employee_returns_404() {
        let state = make_state(setup_pool());
        let app = test_app!(state);

        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/employees/999/executions")
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), 404);
    }

    // ── Seeding ─────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_seed_is_idempotent() {
        let pool = setup_pool();
        {
            let conn = pool.get().unwrap();
            db::seed_employees(&conn).unwrap();
            db::seed_employees(&conn).unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM ai_employees", [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 5);
        }
    }

    // ── Startup recovery ────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_startup_recovery_marks_running_as_failed() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        // Create task and assign
        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-REC", "title": "Work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/employees/1/assign/{}", task_id))
                .to_request(),
        )
        .await;

        // Simulate server restart
        {
            let conn = state.db.get().unwrap();
            db::startup_recovery(&conn).unwrap();
        }

        // Execution should be failed
        let conn = state.db.get().unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM task_executions WHERE task_id = ?1",
                params![task_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "failed");

        // Employee should be idle
        let emp_status: String = conn
            .query_row(
                "SELECT status FROM ai_employees WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(emp_status, "idle");
    }

    // ── Delete employee cancels running execution ───────────────────────────

    #[actix_web::test]
    async fn test_delete_employee_cancels_execution() {
        let state = make_state(setup_pool_with_seed());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri("/api/tasks")
                    .set_json(serde_json::json!({"task_id": "T-EDEL", "title": "Work", "description": ""}))
                    .to_request(),
            )
            .await,
        )
        .await;
        let task_id = task["id"].as_i64().unwrap();

        let exec: serde_json::Value = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::post()
                    .uri(&format!("/api/employees/1/assign/{}", task_id))
                    .to_request(),
            )
            .await,
        )
        .await;
        let exec_id = exec["id"].as_i64().unwrap();

        // Delete employee
        let resp = test::call_service(
            &app,
            test::TestRequest::delete().uri("/api/employees/1").to_request(),
        )
        .await;
        assert_eq!(resp.status(), 204);

        // Execution should be cancelled (cascade will delete it, but let's check before cascade)
        // Actually, ON DELETE CASCADE means the execution row is gone too
        let conn = state.db.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_executions WHERE id = ?1",
                params![exec_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
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
