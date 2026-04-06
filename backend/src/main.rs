use actix_web::{web, App, HttpResponse, HttpServer, Responder, http::header};
use actix_cors::Cors;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection};
use chrono::{DateTime, Utc};
use std::sync::Mutex;

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
            // Fallback to index.html for SPA
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
enum TaskStatus {
    Pending,
    Doing,
    Finished,
    Reviewing,
    Done,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "Pending",
            TaskStatus::Doing => "Doing",
            TaskStatus::Finished => "Finished",
            TaskStatus::Reviewing => "Reviewing",
            TaskStatus::Done => "Done",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "Doing" => TaskStatus::Doing,
            "Finished" => TaskStatus::Finished,
            "Reviewing" => TaskStatus::Reviewing,
            "Done" => TaskStatus::Done,
            _ => TaskStatus::Pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Subtask {
    id: i64,
    task_id: i64,
    title: String,
    completed: bool,
    status: TaskStatus,
    assignee: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: i64,
    task_id: String,
    title: String,
    description: String,
    completed: bool,
    status: TaskStatus,
    assignee: Option<String>,
    created_at: String,
    subtasks: Vec<Subtask>,
}

#[derive(Debug, Deserialize)]
struct CreateTask {
    task_id: String,
    title: String,
    description: String,
    assignee: Option<String>,
}

mod double_option {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Ok(Some(Option::deserialize(d)?))
    }
}

#[derive(Debug, Deserialize)]
struct UpdateTask {
    task_id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    completed: Option<bool>,
    status: Option<TaskStatus>,
    #[serde(default, with = "double_option")]
    assignee: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct CreateSubtask {
    title: String,
    assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateSubtask {
    title: Option<String>,
    completed: Option<bool>,
    status: Option<TaskStatus>,
    #[serde(default, with = "double_option")]
    assignee: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Agent {
    id: i64,
    name: String,
    cli: String,
    system_prompt: String,
    work_dir: String,
    model: Option<String>,
    max_concurrency: i64,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateAgent {
    name: String,
    cli: String,
    system_prompt: Option<String>,
    work_dir: Option<String>,
    model: Option<String>,
    max_concurrency: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UpdateAgent {
    name: Option<String>,
    cli: Option<String>,
    system_prompt: Option<String>,
    work_dir: Option<String>,
    #[serde(default, with = "double_option")]
    model: Option<Option<String>>,
    max_concurrency: Option<i64>,
}

struct AppState {
    db: Mutex<Connection>,
}

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            completed INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'Pending',
            assignee TEXT,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    // Add columns if they don't exist (for existing databases)
    let mut stmt = conn.prepare("PRAGMA table_info(tasks)")?;
    let columns: Vec<String> = stmt.query_map([], |row| row.get(1))?.filter_map(|r| r.ok()).collect();
    
    if !columns.contains(&"task_id".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN task_id TEXT", [])?;
        // Set a default unique task_id for existing tasks
        conn.execute("UPDATE tasks SET task_id = 'T-' || id WHERE task_id IS NULL", [])?;
    }
    if !columns.contains(&"status".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN status TEXT NOT NULL DEFAULT 'Pending'", [])?;
    }
    if !columns.contains(&"assignee".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN assignee TEXT", [])?;
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS subtasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'Pending',
            assignee TEXT,
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Migration for subtasks
    let mut stmt = conn.prepare("PRAGMA table_info(subtasks)")?;
    let columns: Vec<String> = stmt.query_map([], |row| row.get(1))?.filter_map(|r| r.ok()).collect();
    
    if !columns.contains(&"status".to_string()) {
        conn.execute("ALTER TABLE subtasks ADD COLUMN status TEXT NOT NULL DEFAULT 'Pending'", [])?;
    }
    if !columns.contains(&"assignee".to_string()) {
        conn.execute("ALTER TABLE subtasks ADD COLUMN assignee TEXT", [])?;
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS agents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            cli TEXT NOT NULL DEFAULT 'claude',
            system_prompt TEXT NOT NULL DEFAULT '',
            work_dir TEXT NOT NULL DEFAULT '.',
            model TEXT,
            max_concurrency INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT ''
        )",
        [],
    )?;

    Ok(())
}

async fn get_tasks(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, task_id, title, description, completed, status, assignee, created_at FROM tasks ORDER BY created_at DESC")
        .unwrap();
    
    let task_rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i32>(4)? == 1,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .unwrap();

    let mut tasks = Vec::new();
    for row in task_rows {
        if let Ok((id, task_id, title, description, completed, status_str, assignee, created_at)) = row {
            let mut sub_stmt = conn.prepare("SELECT id, task_id, title, completed, status, assignee FROM subtasks WHERE task_id = ?1").unwrap();
            let subtasks: Vec<Subtask> = sub_stmt.query_map(params![id], |sub_row| {
                let status_str: String = sub_row.get(4)?;
                Ok(Subtask {
                    id: sub_row.get(0)?,
                    task_id: sub_row.get(1)?,
                    title: sub_row.get(2)?,
                    completed: sub_row.get::<_, i32>(3)? == 1,
                    status: TaskStatus::from_str(&status_str),
                    assignee: sub_row.get(5)?,
                })
            }).unwrap().filter_map(|r| r.ok()).collect();

            tasks.push(Task {
                id,
                task_id,
                title,
                description,
                completed,
                status: TaskStatus::from_str(&status_str),
                assignee,
                created_at,
                subtasks,
            });
        }
    }
    
    HttpResponse::Ok().json(tasks)
}

async fn create_task(
    data: web::Data<AppState>,
    item: web::Json<CreateTask>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let created_at: DateTime<Utc> = Utc::now();
    let created_at_str = created_at.to_rfc3339();
    let default_status = TaskStatus::Pending;
    
    let result = conn.execute(
        "INSERT INTO tasks (task_id, title, description, completed, status, assignee, created_at) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6)",
        params![item.task_id, item.title, item.description, default_status.as_str(), item.assignee, created_at_str],
    );

    match result {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            HttpResponse::Created().json(Task {
                id,
                task_id: item.task_id.clone(),
                title: item.title.clone(),
                description: item.description.clone(),
                completed: false,
                status: default_status,
                assignee: item.assignee.clone(),
                created_at: created_at_str,
                subtasks: Vec::new(),
            })
        },
        Err(_) => HttpResponse::Conflict().body("Task ID already exists"),
    }
}

async fn update_task(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<UpdateTask>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();
    
    // Get existing task
    let mut stmt = conn
        .prepare("SELECT id, task_id, title, description, completed, status, assignee, created_at FROM tasks WHERE id = ?1")
        .unwrap();
    
    let mut rows = stmt.query(params![id]).unwrap();
    
    let task_data = match rows.next().unwrap() {
        Some(row) => (
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, String>(3).unwrap(),
            row.get::<_, i32>(4).unwrap() == 1,
            row.get::<_, String>(5).unwrap(),
            row.get::<_, Option<String>>(6).unwrap(),
            row.get::<_, String>(7).unwrap(),
        ),
        None => return HttpResponse::NotFound().finish(),
    };
    
    let current_status = TaskStatus::from_str(&task_data.4);
    
    // Update fields
    let new_task_id = item.task_id.clone().unwrap_or(task_data.0);
    let new_title = item.title.clone().unwrap_or(task_data.1);
    let new_desc = item.description.clone().unwrap_or(task_data.2);
    let new_completed = item.completed.unwrap_or(task_data.3);
    let new_status = item.status.unwrap_or(current_status);
    let new_assignee = match item.assignee.clone() {
        Some(a) => a,
        None => task_data.5,
    };
    
    let result = conn.execute(
        "UPDATE tasks SET task_id = ?1, title = ?2, description = ?3, completed = ?4, status = ?5, assignee = ?6 WHERE id = ?7",
        params![new_task_id, new_title, new_desc, new_completed as i32, new_status.as_str(), new_assignee, id],
    );

    match result {
        Ok(_) => {
            // Fetch subtasks for the response
            let mut sub_stmt = conn.prepare("SELECT id, task_id, title, completed, status, assignee FROM subtasks WHERE task_id = ?1").unwrap();
            let subtasks: Vec<Subtask> = sub_stmt.query_map(params![id], |sub_row| {
                let status_str: String = sub_row.get(4)?;
                Ok(Subtask {
                    id: sub_row.get(0)?,
                    task_id: sub_row.get(1)?,
                    title: sub_row.get(2)?,
                    completed: sub_row.get::<_, i32>(3)? == 1,
                    status: TaskStatus::from_str(&status_str),
                    assignee: sub_row.get(5)?,
                })
            }).unwrap().filter_map(|r| r.ok()).collect();

            HttpResponse::Ok().json(Task {
                id,
                task_id: new_task_id,
                title: new_title,
                description: new_desc,
                completed: new_completed,
                status: new_status,
                assignee: new_assignee,
                created_at: task_data.6,
                subtasks,
            })
        },
        Err(_) => HttpResponse::Conflict().body("Task ID already exists"),
    }
}

async fn add_subtask(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<CreateSubtask>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let task_id = path.into_inner();
    let default_status = TaskStatus::Pending;

    let parent_exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", params![task_id], |r| r.get(0))
        .unwrap_or(0);
    if parent_exists == 0 {
        return HttpResponse::NotFound().finish();
    }

    conn.execute(
        "INSERT INTO subtasks (task_id, title, completed, status, assignee) VALUES (?1, ?2, 0, ?3, ?4)",
        params![task_id, item.title, default_status.as_str(), item.assignee],
    ).unwrap();
    
    let id = conn.last_insert_rowid();
    
    HttpResponse::Created().json(Subtask {
        id,
        task_id,
        title: item.title.clone(),
        completed: false,
        status: default_status,
        assignee: item.assignee.clone(),
    })
}

async fn toggle_subtask(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let (task_id, subtask_id) = path.into_inner();

    let affected = conn.execute(
        "UPDATE subtasks SET completed = 1 - completed WHERE id = ?1 AND task_id = ?2",
        params![subtask_id, task_id],
    ).unwrap();

    if affected == 0 {
        HttpResponse::NotFound().finish()
    } else {
        HttpResponse::Ok().finish()
    }
}

async fn update_subtask(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    item: web::Json<UpdateSubtask>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let (task_id, subtask_id) = path.into_inner();

    // Get existing subtask, enforcing task ownership
    let mut stmt = conn
        .prepare("SELECT title, completed, status, assignee FROM subtasks WHERE id = ?1 AND task_id = ?2")
        .unwrap();

    let sub_data = match stmt.query_row(params![subtask_id, task_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)? == 1,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    }) {
        Ok(d) => d,
        Err(_) => return HttpResponse::NotFound().finish(),
    };

    let new_title = item.title.clone().unwrap_or(sub_data.0);
    let new_completed = item.completed.unwrap_or(sub_data.1);
    let new_status = item.status.unwrap_or(TaskStatus::from_str(&sub_data.2));
    let new_assignee = match item.assignee.clone() {
        Some(a) => a,
        None => sub_data.3,
    };
    
    conn.execute(
        "UPDATE subtasks SET title = ?1, completed = ?2, status = ?3, assignee = ?4 WHERE id = ?5",
        params![new_title, new_completed as i32, new_status.as_str(), new_assignee, subtask_id],
    ).unwrap();
    
    HttpResponse::Ok().finish()
}

// --- Agent handlers ---

async fn get_agents(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, cli, system_prompt, work_dir, model, max_concurrency, created_at FROM agents ORDER BY created_at DESC")
        .unwrap();

    let agents: Vec<Agent> = stmt
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
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(agents)
}

async fn create_agent(
    data: web::Data<AppState>,
    item: web::Json<CreateAgent>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let created_at = Utc::now().to_rfc3339();

    let result = conn.execute(
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
    );

    match result {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            HttpResponse::Created().json(Agent {
                id,
                name: item.name.clone(),
                cli: item.cli.clone(),
                system_prompt: item.system_prompt.clone().unwrap_or_default(),
                work_dir: item.work_dir.clone().unwrap_or_else(|| ".".to_string()),
                model: item.model.clone(),
                max_concurrency: item.max_concurrency.unwrap_or(1),
                created_at,
            })
        }
        Err(_) => HttpResponse::Conflict().body("Agent name already exists"),
    }
}

async fn update_agent(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    item: web::Json<UpdateAgent>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();

    let mut stmt = conn
        .prepare("SELECT id, name, cli, system_prompt, work_dir, model, max_concurrency, created_at FROM agents WHERE id = ?1")
        .unwrap();

    let existing = match stmt.query_row(params![id], |row| {
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
    }) {
        Ok(a) => a,
        Err(_) => return HttpResponse::NotFound().finish(),
    };

    let new_name = item.name.clone().unwrap_or(existing.name);
    let new_cli = item.cli.clone().unwrap_or(existing.cli);
    let new_system_prompt = item.system_prompt.clone().unwrap_or(existing.system_prompt);
    let new_work_dir = item.work_dir.clone().unwrap_or(existing.work_dir);
    let new_model = match item.model.clone() {
        Some(m) => m,
        None => existing.model,
    };
    let new_max_concurrency = item.max_concurrency.unwrap_or(existing.max_concurrency);

    let result = conn.execute(
        "UPDATE agents SET name = ?1, cli = ?2, system_prompt = ?3, work_dir = ?4, model = ?5, max_concurrency = ?6 WHERE id = ?7",
        params![new_name, new_cli, new_system_prompt, new_work_dir, new_model, new_max_concurrency, id],
    );

    match result {
        Ok(_) => HttpResponse::Ok().json(Agent {
            id,
            name: new_name,
            cli: new_cli,
            system_prompt: new_system_prompt,
            work_dir: new_work_dir,
            model: new_model,
            max_concurrency: new_max_concurrency,
            created_at: existing.created_at,
        }),
        Err(_) => HttpResponse::Conflict().body("Agent name already exists"),
    }
}

async fn delete_agent(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();

    let affected = conn.execute("DELETE FROM agents WHERE id = ?1", params![id]);

    match affected {
        Ok(0) => HttpResponse::NotFound().finish(),
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/tasks", web::get().to(get_tasks))
            .route("/tasks", web::post().to(create_task))
            .route("/tasks/{id}", web::put().to(update_task))
            .route("/tasks/{id}", web::delete().to(delete_task))
            .route("/tasks/{id}/subtasks", web::post().to(add_subtask))
            .route("/tasks/{id}/subtasks/{subtask_id}/toggle", web::post().to(toggle_subtask))
            .route("/tasks/{id}/subtasks/{subtask_id}", web::put().to(update_subtask))
            .route("/agents", web::get().to(get_agents))
            .route("/agents", web::post().to(create_agent))
            .route("/agents/{id}", web::put().to(update_agent))
            .route("/agents/{id}", web::delete().to(delete_agent)),
    );
}

async fn delete_task(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let id = path.into_inner();
    
    let affected = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id]);
    
    match affected {
        Ok(0) => HttpResponse::NotFound().finish(),
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    fn make_state(conn: Connection) -> web::Data<AppState> {
        web::Data::new(AppState {
            db: Mutex::new(conn),
        })
    }

    macro_rules! test_app {
        ($state:expr) => {
            test::init_service(
                App::new()
                    .app_data($state.clone())
                    .configure(configure_api_routes)
            ).await
        };
    }

    // ── Pure unit tests (no async runtime needed) ────────────────────────────
    mod unit {
        use super::super::*;
        use rusqlite::Connection;

        #[test]
        fn test_task_status_from_str_defaults_to_pending() {
            assert_eq!(TaskStatus::from_str("unknown"), TaskStatus::Pending);
            assert_eq!(TaskStatus::from_str(""), TaskStatus::Pending);
        }

        #[test]
        fn test_task_status_roundtrip() {
            for status in [TaskStatus::Pending, TaskStatus::Doing, TaskStatus::Finished, TaskStatus::Reviewing, TaskStatus::Done] {
                assert_eq!(TaskStatus::from_str(status.as_str()), status);
            }
        }

        #[test]
        fn test_init_db_creates_tables() {
            let conn = Connection::open_in_memory().unwrap();
            init_db(&conn).unwrap();
            let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0)).unwrap();
            assert_eq!(task_count, 0);
            let sub_count: i64 = conn.query_row("SELECT COUNT(*) FROM subtasks", [], |r| r.get(0)).unwrap();
            assert_eq!(sub_count, 0);
        }

        #[test]
        fn test_init_db_creates_correct_columns() {
            let conn = Connection::open_in_memory().unwrap();
            init_db(&conn).unwrap();

            let mut stmt = conn.prepare("PRAGMA table_info(tasks)").unwrap();
            let cols: Vec<String> = stmt.query_map([], |r| r.get(1)).unwrap()
                .filter_map(|r| r.ok()).collect();
            for expected in ["id", "task_id", "title", "description", "completed", "status", "assignee", "created_at"] {
                assert!(cols.contains(&expected.to_string()), "tasks missing column: {}", expected);
            }

            let mut stmt = conn.prepare("PRAGMA table_info(subtasks)").unwrap();
            let cols: Vec<String> = stmt.query_map([], |r| r.get(1)).unwrap()
                .filter_map(|r| r.ok()).collect();
            for expected in ["id", "task_id", "title", "completed", "status", "assignee"] {
                assert!(cols.contains(&expected.to_string()), "subtasks missing column: {}", expected);
            }
        }

        #[test]
        fn test_init_db_enables_foreign_keys() {
            let conn = Connection::open_in_memory().unwrap();
            init_db(&conn).unwrap();
            let fk_enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
            assert_eq!(fk_enabled, 1);
        }

        #[test]
        fn test_init_db_is_idempotent() {
            let conn = Connection::open_in_memory().unwrap();
            init_db(&conn).unwrap();
            init_db(&conn).unwrap();
        }
    }

    // ── GET /api/tasks ───────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_tasks_returns_empty_list() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let req = test::TestRequest::get().uri("/api/tasks").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert!(body.is_empty());
    }

    // ── POST /api/tasks ──────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_create_task_returns_created_with_correct_data() {
        let state = make_state(setup_db());
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
        let state = make_state(setup_db());
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
        let state = make_state(setup_db());
        let app = test_app!(state);

        let payload = serde_json::json!({
            "task_id": "T-DUP",
            "title": "First",
            "description": ""
        });

        let req1 = test::TestRequest::post().uri("/api/tasks").set_json(&payload).to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), 201);

        let req2 = test::TestRequest::post().uri("/api/tasks").set_json(&payload).to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), 409);
    }

    // ── GET /api/tasks (with data) ───────────────────────────────────────────

    #[actix_web::test]
    async fn test_get_tasks_returns_all_tasks_ordered_by_created_at_desc() {
        let conn = setup_db();
        // Insert with explicit timestamps so ordering is deterministic
        conn.execute(
            "INSERT INTO tasks (task_id, title, description, completed, status, created_at) VALUES ('T-OLDER', 'Older', '', 0, 'Pending', '2024-01-01T00:00:00Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO tasks (task_id, title, description, completed, status, created_at) VALUES ('T-NEWER', 'Newer', '', 0, 'Pending', '2024-01-02T00:00:00Z')",
            [],
        ).unwrap();
        let state = make_state(conn);
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
        let state = make_state(setup_db());
        let app = test_app!(state);

        // Create task
        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-S", "title": "Parent", "description": ""}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let task_id = task["id"].as_i64().unwrap();

        // Add subtask
        let sub_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks", task_id))
            .set_json(serde_json::json!({"title": "Sub step"}))
            .to_request();
        test::call_service(&app, sub_req).await;

        // GET tasks and verify subtask is embedded
        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let tasks: Vec<serde_json::Value> = test::read_body_json(list_resp).await;

        let parent = tasks.iter().find(|t| t["task_id"] == "T-S").unwrap();
        let subtasks = parent["subtasks"].as_array().unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0]["title"], "Sub step");
    }

    // ── PUT /api/tasks/{id} ──────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_update_task_fields_are_persisted() {
        let state = make_state(setup_db());
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

        // Confirm changes actually landed in the DB via a fresh GET
        let get_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let tasks: Vec<serde_json::Value> = test::read_body_json(test::call_service(&app, get_req).await).await;
        let persisted = tasks.iter().find(|t| t["id"] == id).unwrap();
        assert_eq!(persisted["title"], "New title");
        assert_eq!(persisted["status"], "Doing");
        assert_eq!(persisted["assignee"], "bob");
        assert_eq!(persisted["completed"], true);
    }

    #[actix_web::test]
    async fn test_update_task_partial_update_preserves_unchanged_fields() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let create_req = test::TestRequest::post()
            .uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-P", "title": "Keep me", "description": "keep desc", "assignee": "carol"}))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let task: serde_json::Value = test::read_body_json(create_resp).await;
        let id = task["id"].as_i64().unwrap();

        // Only update status
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
        let state = make_state(setup_db());
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
        let state = make_state(setup_db());
        let app = test_app!(state);

        // Create two tasks
        for (id, title) in [("T-X", "X"), ("T-Y", "Y")] {
            let req = test::TestRequest::post()
                .uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": id, "title": title, "description": ""}))
                .to_request();
            test::call_service(&app, req).await;
        }

        // Get id of T-X
        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let tasks: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        let tx = tasks.iter().find(|t| t["task_id"] == "T-X").unwrap();
        let id = tx["id"].as_i64().unwrap();

        // Try to rename T-X to T-Y (already taken)
        let req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", id))
            .set_json(serde_json::json!({"task_id": "T-Y"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409);
    }

    // ── DELETE /api/tasks/{id} ───────────────────────────────────────────────

    #[actix_web::test]
    async fn test_delete_task_removes_it_from_list() {
        let state = make_state(setup_db());
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

        // Verify it is gone from the list
        let list_req = test::TestRequest::get().uri("/api/tasks").to_request();
        let list_resp = test::call_service(&app, list_req).await;
        let tasks: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
        assert!(!tasks.iter().any(|t| t["task_id"] == "T-DEL"));
    }

    #[actix_web::test]
    async fn test_delete_task_not_found_returns_404() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let req = test::TestRequest::delete().uri("/api/tasks/9999").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_delete_task_cascades_subtasks() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        // Create task + subtask
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

        // Delete parent
        let del_req = test::TestRequest::delete()
            .uri(&format!("/api/tasks/{}", task_id))
            .to_request();
        test::call_service(&app, del_req).await;

        // Verify subtask is gone from DB
        let db = state.db.lock().unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM subtasks WHERE id = ?1", params![sub_id], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── POST /api/tasks/{id}/subtasks ────────────────────────────────────────

    #[actix_web::test]
    async fn test_add_subtask_returns_created_with_correct_data() {
        let state = make_state(setup_db());
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

    // ── POST /api/tasks/{id}/subtasks/{subid}/toggle ─────────────────────────

    #[actix_web::test]
    async fn test_toggle_subtask_flips_completed() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        // Create task + subtask
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

        // Toggle once → completed = true
        let toggle_req = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks/{}/toggle", task_id, sub_id))
            .to_request();
        let toggle_resp = test::call_service(&app, toggle_req).await;
        assert_eq!(toggle_resp.status(), 200);

        let completed: i32 = state.db.lock().unwrap()
            .query_row("SELECT completed FROM subtasks WHERE id = ?1", params![sub_id], |r| r.get(0))
            .unwrap();
        assert_eq!(completed, 1);

        // Toggle again → completed = false
        let toggle_req2 = test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks/{}/toggle", task_id, sub_id))
            .to_request();
        test::call_service(&app, toggle_req2).await;

        let completed2: i32 = state.db.lock().unwrap()
            .query_row("SELECT completed FROM subtasks WHERE id = ?1", params![sub_id], |r| r.get(0))
            .unwrap();
        assert_eq!(completed2, 0);
    }

    // ── PUT /api/tasks/{id}/subtasks/{subid} ─────────────────────────────────

    #[actix_web::test]
    async fn test_update_subtask_persists_changes() {
        let state = make_state(setup_db());
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

        let db = state.db.lock().unwrap();
        let (title, completed, status, assignee): (String, i32, String, Option<String>) = db
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
        let state = make_state(setup_db());
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

        // Only update status
        let update_req = test::TestRequest::put()
            .uri(&format!("/api/tasks/{}/subtasks/{}", task_id, sub_id))
            .set_json(serde_json::json!({"status": "Doing"}))
            .to_request();
        test::call_service(&app, update_req).await;

        let db = state.db.lock().unwrap();
        let (title, assignee, status): (String, Option<String>, String) = db
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
        let state = make_state(setup_db());
        let app = test_app!(state);

        let req = test::TestRequest::put()
            .uri("/api/tasks/1/subtasks/9999")
            .set_json(serde_json::json!({"title": "ghost"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    // ── add_subtask with non-existent parent ─────────────────────────────────

    #[actix_web::test]
    async fn test_add_subtask_nonexistent_parent_returns_404() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks/9999/subtasks")
            .set_json(serde_json::json!({"title": "Orphan"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    // ── toggle_subtask edge cases ────────────────────────────────────────────

    #[actix_web::test]
    async fn test_toggle_subtask_nonexistent_returns_404() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let req = test::TestRequest::post()
            .uri("/api/tasks/1/subtasks/9999/toggle")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_toggle_subtask_wrong_task_returns_404() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        // Create two tasks, add subtask under task 1
        let t1: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-TOG1", "title": "P1", "description": ""}))
                .to_request()).await
        ).await;
        let t2: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-TOG2", "title": "P2", "description": ""}))
                .to_request()).await
        ).await;
        let t1_id = t1["id"].as_i64().unwrap();
        let t2_id = t2["id"].as_i64().unwrap();

        let sub: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post()
                .uri(&format!("/api/tasks/{}/subtasks", t1_id))
                .set_json(serde_json::json!({"title": "Sub"}))
                .to_request()).await
        ).await;
        let sub_id = sub["id"].as_i64().unwrap();

        // Toggle using wrong task_id
        let resp = test::call_service(&app, test::TestRequest::post()
            .uri(&format!("/api/tasks/{}/subtasks/{}/toggle", t2_id, sub_id))
            .to_request()).await;
        assert_eq!(resp.status(), 404);
    }

    // ── update_subtask ownership enforcement ─────────────────────────────────

    #[actix_web::test]
    async fn test_update_subtask_wrong_task_returns_404() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let t1: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-OWN1", "title": "P1", "description": ""}))
                .to_request()).await
        ).await;
        let t2: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-OWN2", "title": "P2", "description": ""}))
                .to_request()).await
        ).await;
        let t1_id = t1["id"].as_i64().unwrap();
        let t2_id = t2["id"].as_i64().unwrap();

        let sub: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post()
                .uri(&format!("/api/tasks/{}/subtasks", t1_id))
                .set_json(serde_json::json!({"title": "Sub"}))
                .to_request()).await
        ).await;
        let sub_id = sub["id"].as_i64().unwrap();

        let resp = test::call_service(&app, test::TestRequest::put()
            .uri(&format!("/api/tasks/{}/subtasks/{}", t2_id, sub_id))
            .set_json(serde_json::json!({"title": "Hijacked"}))
            .to_request()).await;
        assert_eq!(resp.status(), 404);
    }

    // ── assignee can be cleared via update ───────────────────────────────────

    #[actix_web::test]
    async fn test_update_task_clear_assignee() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-CLR", "title": "T", "description": "", "assignee": "alice"}))
                .to_request()).await
        ).await;
        let id = task["id"].as_i64().unwrap();
        assert_eq!(task["assignee"], "alice");

        let resp = test::call_service(&app, test::TestRequest::put()
            .uri(&format!("/api/tasks/{}", id))
            .set_json(serde_json::json!({"assignee": null}))
            .to_request()).await;
        assert_eq!(resp.status(), 200);

        let tasks: Vec<serde_json::Value> = test::read_body_json(
            test::call_service(&app, test::TestRequest::get().uri("/api/tasks").to_request()).await
        ).await;
        let updated = tasks.iter().find(|t| t["id"] == id).unwrap();
        assert!(updated["assignee"].is_null());
    }

    #[actix_web::test]
    async fn test_update_subtask_clear_assignee() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        let task: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
                .set_json(serde_json::json!({"task_id": "T-SCLR", "title": "P", "description": ""}))
                .to_request()).await
        ).await;
        let task_id = task["id"].as_i64().unwrap();

        let sub: serde_json::Value = test::read_body_json(
            test::call_service(&app, test::TestRequest::post()
                .uri(&format!("/api/tasks/{}/subtasks", task_id))
                .set_json(serde_json::json!({"title": "Sub", "assignee": "bob"}))
                .to_request()).await
        ).await;
        let sub_id = sub["id"].as_i64().unwrap();

        let resp = test::call_service(&app, test::TestRequest::put()
            .uri(&format!("/api/tasks/{}/subtasks/{}", task_id, sub_id))
            .set_json(serde_json::json!({"assignee": null}))
            .to_request()).await;
        assert_eq!(resp.status(), 200);

        let db = state.db.lock().unwrap();
        let assignee: Option<String> = db
            .query_row("SELECT assignee FROM subtasks WHERE id = ?1", params![sub_id], |r| r.get(0))
            .unwrap();
        assert!(assignee.is_none());
    }

    // ── malformed requests ───────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_create_task_missing_required_field_returns_400() {
        let state = make_state(setup_db());
        let app = test_app!(state);

        // Missing title
        let resp = test::call_service(&app, test::TestRequest::post().uri("/api/tasks")
            .set_json(serde_json::json!({"task_id": "T-BAD", "description": "no title"}))
            .to_request()).await;
        assert_eq!(resp.status(), 400);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize database
    let conn = Connection::open("tasks.db").expect("Failed to open database");
    init_db(&conn).expect("Failed to initialize database");
    
    let state = web::Data::new(AppState {
        db: Mutex::new(conn),
    });
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![actix_web::http::header::AUTHORIZATION, actix_web::http::header::ACCEPT])
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
