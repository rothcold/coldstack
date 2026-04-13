use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};

use crate::adapters::AdapterRegistry;

pub type DbPool = Pool<SqliteConnectionManager>;

pub struct RunningExecution {
    pub cancel_tx: broadcast::Sender<()>,
    pub output_tx: broadcast::Sender<OutputEvent>,
}

#[derive(Clone, Debug)]
pub enum OutputEvent {
    Output { seq: i64, data: String },
    Status(String),
}

pub struct AppState {
    pub db: DbPool,
    pub adapters: AdapterRegistry,
    pub running: Mutex<HashMap<i64, RunningExecution>>,
}

impl AppState {
    pub fn new(db: DbPool) -> Self {
        Self {
            db,
            adapters: AdapterRegistry::new(),
            running: Mutex::new(HashMap::new()),
        }
    }
}
pub fn create_pool(path: &str) -> Result<DbPool, r2d2::Error> {
    let manager = SqliteConnectionManager::file(path).with_init(|conn| {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(())
    });
    Pool::builder().max_size(8).build(manager)
}

#[cfg(test)]
pub fn create_memory_pool() -> Result<DbPool, r2d2::Error> {
    let manager = SqliteConnectionManager::memory().with_init(|conn| {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(())
    });
    // Single connection for in-memory DB so all tests share state
    Pool::builder().max_size(1).build(manager)
}

pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            archived INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'Plan',
            assignee TEXT,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    let mut stmt = conn.prepare("PRAGMA table_info(tasks)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.contains(&"task_id".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN task_id TEXT", [])?;
        conn.execute(
            "UPDATE tasks SET task_id = 'T-' || id WHERE task_id IS NULL",
            [],
        )?;
    }
    // Ensure UNIQUE constraint on task_id for both fresh and migrated DBs.
    // ALTER TABLE ADD COLUMN cannot add UNIQUE, so use a unique index instead.
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_task_id ON tasks(task_id)",
        [],
    )?;
    if !columns.contains(&"status".to_string()) {
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN status TEXT NOT NULL DEFAULT 'Plan'",
            [],
        )?;
    }
    if !columns.contains(&"archived".to_string()) {
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !columns.contains(&"assignee".to_string()) {
        conn.execute("ALTER TABLE tasks ADD COLUMN assignee TEXT", [])?;
    }
    if columns.contains(&"completed".to_string()) {
        conn.execute(
            "UPDATE tasks SET archived = completed WHERE archived = 0 AND completed != 0",
            [],
        )?;
    }
    conn.execute(
        "UPDATE tasks
         SET status = CASE status
             WHEN 'Pending' THEN 'Plan'
             WHEN 'Doing' THEN 'Coding'
             WHEN 'Finished' THEN 'Review'
             WHEN 'Reviewing' THEN 'Review'
             ELSE status
         END",
        [],
    )?;

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

    let mut stmt = conn.prepare("PRAGMA table_info(subtasks)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.contains(&"status".to_string()) {
        conn.execute(
            "ALTER TABLE subtasks ADD COLUMN status TEXT NOT NULL DEFAULT 'Pending'",
            [],
        )?;
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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_employees (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            workflow_role TEXT NOT NULL DEFAULT 'planner',
            department TEXT NOT NULL,
            agent_backend TEXT NOT NULL,
            custom_prompt TEXT,
            system_prompt TEXT,
            status TEXT NOT NULL DEFAULT 'idle',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_executions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            employee_id INTEGER NOT NULL,
            pid INTEGER,
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            finished_at TEXT,
            exit_code INTEGER,
            status TEXT NOT NULL DEFAULT 'running',
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (employee_id) REFERENCES ai_employees(id) ON DELETE CASCADE
        )",
        [],
    )?;

    let mut stmt = conn.prepare("PRAGMA table_info(task_executions)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.contains(&"pid".to_string()) {
        conn.execute("ALTER TABLE task_executions ADD COLUMN pid INTEGER", [])?;
    }

    let mut stmt = conn.prepare("PRAGMA table_info(ai_employees)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !columns.contains(&"workflow_role".to_string()) {
        conn.execute(
            "ALTER TABLE ai_employees ADD COLUMN workflow_role TEXT NOT NULL DEFAULT 'planner'",
            [],
        )?;
    }
    if !columns.contains(&"custom_prompt".to_string()) {
        conn.execute("ALTER TABLE ai_employees ADD COLUMN custom_prompt TEXT", [])?;
    }
    if columns.contains(&"system_prompt".to_string()) {
        conn.execute(
            "UPDATE ai_employees
             SET custom_prompt = system_prompt
             WHERE custom_prompt IS NULL
               AND system_prompt IS NOT NULL
               AND TRIM(system_prompt) != ''",
            [],
        )?;
    }
    conn.execute(
        "UPDATE ai_employees SET agent_backend = 'claude_code' WHERE agent_backend IS NULL OR agent_backend = '' OR agent_backend NOT IN ('claude_code')",
        [],
    )?;
    conn.execute(
        "UPDATE ai_employees
         SET workflow_role = CASE LOWER(role)
             WHEN 'planner' THEN 'planner'
             WHEN 'design' THEN 'designer'
             WHEN 'designer' THEN 'designer'
             WHEN 'coding' THEN 'coder'
             WHEN 'coder' THEN 'coder'
             WHEN 'developer' THEN 'coder'
             WHEN 'review' THEN 'reviewer'
             WHEN 'reviewer' THEN 'reviewer'
             WHEN 'qa' THEN 'qa'
             WHEN 'human' THEN 'human'
             ELSE workflow_role
         END",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS output_chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            execution_id INTEGER NOT NULL,
            seq INTEGER NOT NULL,
            chunk TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (execution_id) REFERENCES task_executions(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_output_chunks_execution_seq ON output_chunks(execution_id, seq)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_executions_task_id ON task_executions(task_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_executions_employee_id ON task_executions(employee_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_workflow_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            from_status TEXT NOT NULL,
            to_status TEXT NOT NULL,
            actor_type TEXT NOT NULL,
            actor_id INTEGER,
            actor_label TEXT NOT NULL,
            action TEXT NOT NULL,
            note TEXT,
            evidence_text TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_workflow_events_task_created_at ON task_workflow_events(task_id, created_at)",
        [],
    )?;

    Ok(())
}

use std::fs;

pub fn startup_recovery(conn: &Connection) -> rusqlite::Result<()> {
    // Get PIDs of "running" executions
    let mut stmt = conn.prepare(
        "SELECT id, pid FROM task_executions WHERE status = 'running' AND pid IS NOT NULL",
    )?;
    let running_executions: Vec<(i64, u32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(Result::ok)
        .collect();

    for (_id, pid) in running_executions {
        if is_our_process(pid) {
            // It's likely our stale process from before the restart, kill it
            if let Ok(mut cmd) = std::process::Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .spawn()
            {
                let _ = cmd.wait();
            }
        }
    }

    // Mark any "running" executions as failed (stale from previous crash)
    conn.execute(
        "UPDATE task_executions SET status = 'failed', finished_at = datetime('now') WHERE status = 'running'",
        [],
    )?;
    // Reset any "working" employees to idle
    conn.execute(
        "UPDATE ai_employees SET status = 'idle' WHERE status = 'working'",
        [],
    )?;
    Ok(())
}

fn is_our_process(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        if let Ok(data) = fs::read(cmdline_path) {
            let contents = String::from_utf8_lossy(&data).to_lowercase();
            return contents.contains("claude")
                || contents.contains("gemini")
                || contents.contains("codex")
                || contents.contains("cursor");
        }
    }
    false
}

pub fn seed_employees(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM ai_employees", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let employees = [
        ("Alice", "Senior Frontend Engineer", "Frontend", "claude_code", "You are Alice, a senior frontend engineer. You specialize in React, TypeScript, and CSS. Write clean, accessible UI code."),
        ("Bob", "Backend Architect", "Backend", "claude_code", "You are Bob, a backend architect. You specialize in API design, database optimization, and system architecture."),
        ("Carol", "Quality Assurance Lead", "QA", "claude_code", "You are Carol, a QA lead. You write thorough test suites, find edge cases, and verify correctness."),
        ("Dave", "Infrastructure Engineer", "DevOps", "claude_code", "You are Dave, an infrastructure engineer. You handle CI/CD, Docker, deployment, and monitoring."),
        ("Eve", "Technical Writer", "Documentation", "claude_code", "You are Eve, a technical writer. You write clear documentation, API guides, and README files."),
    ];

    for (name, role, department, backend, prompt) in employees {
        conn.execute(
            "INSERT INTO ai_employees (name, role, department, agent_backend, custom_prompt) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, role, department, backend, prompt],
        )?;
    }

    Ok(())
}
