use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Pending,
    Doing,
    Finished,
    Reviewing,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "Pending",
            TaskStatus::Doing => "Doing",
            TaskStatus::Finished => "Finished",
            TaskStatus::Reviewing => "Reviewing",
            TaskStatus::Done => "Done",
        }
    }

    pub fn from_str(s: &str) -> Self {
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
pub struct Subtask {
    pub id: i64,
    pub task_id: i64,
    pub title: String,
    pub completed: bool,
    pub status: TaskStatus,
    pub assignee: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: i64,
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub status: TaskStatus,
    pub assignee: Option<String>,
    pub created_at: String,
    pub subtasks: Vec<Subtask>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub assignee: Option<String>,
}

pub mod double_option {
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
pub struct UpdateTask {
    pub task_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub status: Option<TaskStatus>,
    #[serde(default, with = "double_option")]
    pub assignee: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubtask {
    pub title: String,
    pub assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubtask {
    pub title: Option<String>,
    pub completed: Option<bool>,
    pub status: Option<TaskStatus>,
    #[serde(default, with = "double_option")]
    pub assignee: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub cli: String,
    pub system_prompt: String,
    pub work_dir: String,
    pub model: Option<String>,
    pub max_concurrency: i64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgent {
    pub name: String,
    pub cli: String,
    pub system_prompt: Option<String>,
    pub work_dir: Option<String>,
    pub model: Option<String>,
    pub max_concurrency: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgent {
    pub name: Option<String>,
    pub cli: Option<String>,
    pub system_prompt: Option<String>,
    pub work_dir: Option<String>,
    #[serde(default, with = "double_option")]
    pub model: Option<Option<String>>,
    pub max_concurrency: Option<i64>,
}

// --- AI Employee models ---

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeStatus {
    Idle,
    Working,
    Error,
}

impl EmployeeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmployeeStatus::Idle => "idle",
            EmployeeStatus::Working => "working",
            EmployeeStatus::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "working" => EmployeeStatus::Working,
            "error" => EmployeeStatus::Error,
            _ => EmployeeStatus::Idle,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Running => "running",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "completed" => ExecutionStatus::Completed,
            "failed" => ExecutionStatus::Failed,
            "cancelled" => ExecutionStatus::Cancelled,
            _ => ExecutionStatus::Running,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub role: String,
    pub department: String,
    pub agent_backend: String,
    pub system_prompt: Option<String>,
    pub status: EmployeeStatus,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployee {
    pub name: String,
    pub role: String,
    pub department: String,
    pub agent_backend: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployee {
    pub name: Option<String>,
    pub role: Option<String>,
    pub department: Option<String>,
    pub agent_backend: Option<String>,
    #[serde(default, with = "double_option")]
    pub system_prompt: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Execution {
    pub id: i64,
    pub task_id: i64,
    pub employee_id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub status: ExecutionStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputChunk {
    pub id: i64,
    pub execution_id: i64,
    pub seq: i64,
    pub chunk: String,
    pub created_at: String,
}
