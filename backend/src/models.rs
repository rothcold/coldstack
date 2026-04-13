use serde::{Deserialize, Serialize};

/*
Workflow state machine

Plan -> Design -> Coding -> Review -> QA -> NeedsHuman -> Done
                  ^         |         |        |            |
                  |         | reject  | reject | reject     |
                  +---------+---------+--------+------------+

Archived is not a workflow stage. It is a human-only visibility flag that removes a
completed task from the active workspace.
*/
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkflowStatus {
    Plan,
    Design,
    Coding,
    Review,
    QA,
    NeedsHuman,
    Done,
}

impl WorkflowStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowStatus::Plan => "Plan",
            WorkflowStatus::Design => "Design",
            WorkflowStatus::Coding => "Coding",
            WorkflowStatus::Review => "Review",
            WorkflowStatus::QA => "QA",
            WorkflowStatus::NeedsHuman => "NeedsHuman",
            WorkflowStatus::Done => "Done",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Design" => WorkflowStatus::Design,
            "Coding" => WorkflowStatus::Coding,
            "Doing" => WorkflowStatus::Coding,
            "Review" => WorkflowStatus::Review,
            "Finished" | "Reviewing" => WorkflowStatus::Review,
            "QA" => WorkflowStatus::QA,
            "NeedsHuman" => WorkflowStatus::NeedsHuman,
            "Done" => WorkflowStatus::Done,
            _ => WorkflowStatus::Plan,
        }
    }

    pub fn board_group(&self) -> &'static str {
        match self {
            WorkflowStatus::Plan => "Plan",
            WorkflowStatus::Design | WorkflowStatus::Coding => "Build",
            WorkflowStatus::Review => "Review",
            WorkflowStatus::QA => "QA",
            WorkflowStatus::NeedsHuman => "Human",
            WorkflowStatus::Done => "Done",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRole {
    Planner,
    Designer,
    Coder,
    Reviewer,
    Qa,
    Human,
}

impl WorkflowRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowRole::Planner => "planner",
            WorkflowRole::Designer => "designer",
            WorkflowRole::Coder => "coder",
            WorkflowRole::Reviewer => "reviewer",
            WorkflowRole::Qa => "qa",
            WorkflowRole::Human => "human",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "designer" => WorkflowRole::Designer,
            "coder" => WorkflowRole::Coder,
            "reviewer" => WorkflowRole::Reviewer,
            "qa" => WorkflowRole::Qa,
            "human" => WorkflowRole::Human,
            _ => WorkflowRole::Planner,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowAction {
    Advance,
    Reject,
    Archive,
}

impl WorkflowAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowAction::Advance => "advance",
            WorkflowAction::Reject => "reject",
            WorkflowAction::Archive => "archive",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActorType {
    Employee,
    Human,
}

impl WorkflowActorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowActorType::Employee => "employee",
            WorkflowActorType::Human => "human",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "human" => WorkflowActorType::Human,
            _ => WorkflowActorType::Employee,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subtask {
    pub id: i64,
    pub task_id: i64,
    pub title: String,
    pub completed: bool,
    pub status: WorkflowStatus,
    pub assignee: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: i64,
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub archived: bool,
    pub status: WorkflowStatus,
    pub assignee: Option<String>,
    pub created_at: String,
    pub subtasks: Vec<Subtask>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoardTaskSummary {
    pub id: i64,
    pub task_id: String,
    pub title: String,
    pub status: WorkflowStatus,
    pub board_group: String,
    pub assignee: Option<String>,
    pub archived: bool,
    pub needs_attention: bool,
    pub waiting_for_human: bool,
    pub rejection_count: i64,
    pub latest_event_summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowEvent {
    pub id: i64,
    pub task_id: i64,
    pub from_status: WorkflowStatus,
    pub to_status: WorkflowStatus,
    pub actor_type: WorkflowActorType,
    pub actor_id: Option<i64>,
    pub actor_label: String,
    pub action: WorkflowAction,
    pub note: Option<String>,
    pub evidence_text: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskDetail {
    pub task: Task,
    pub events: Vec<WorkflowEvent>,
    pub current_action_label: String,
    pub current_action_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    #[serde(default)]
    pub task_id: Option<String>,
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
    #[serde(default, with = "double_option")]
    pub assignee: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TransitionTaskRequest {
    pub actor_type: WorkflowActorType,
    pub actor_id: Option<i64>,
    pub actor_label: Option<String>,
    pub from_status: WorkflowStatus,
    pub to_status: Option<WorkflowStatus>,
    pub action: WorkflowAction,
    pub note: Option<String>,
    pub evidence_text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransitionTaskResponse {
    pub task: TaskDetail,
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
    pub status: Option<WorkflowStatus>,
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeStatus {
    Idle,
    Working,
    Error,
}

impl EmployeeStatus {
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
    pub workflow_role: WorkflowRole,
    pub department: String,
    pub agent_backend: String,
    pub backend_available: bool,
    pub custom_prompt: Option<String>,
    pub system_prompt: String,
    pub status: EmployeeStatus,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployee {
    pub name: String,
    pub role: String,
    pub workflow_role: Option<WorkflowRole>,
    pub department: String,
    pub agent_backend: String,
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployee {
    pub name: Option<String>,
    pub role: Option<String>,
    pub workflow_role: Option<WorkflowRole>,
    pub department: Option<String>,
    pub agent_backend: Option<String>,
    #[serde(default, with = "double_option")]
    pub custom_prompt: Option<Option<String>>,
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
pub struct CurrentExecution {
    pub execution_id: i64,
    pub task_id: i64,
    pub task_key: String,
    pub task_title: String,
    pub started_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputChunk {
    pub id: i64,
    pub execution_id: i64,
    pub seq: i64,
    pub chunk: String,
    pub created_at: String,
}
