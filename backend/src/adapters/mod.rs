pub mod claude_code;

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Child;
use tokio::io::BufReader;

pub struct TaskInfo {
    pub title: String,
    pub description: String,
    pub task_id: String,
}

pub struct EmployeeConfig {
    pub system_prompt: Option<String>,
}

pub struct AgentProcess {
    pub child: Child,
    pub stdout: BufReader<tokio::process::ChildStdout>,
    pub stderr: BufReader<tokio::process::ChildStderr>,
}

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    async fn execute(&self, task: &TaskInfo, employee: &EmployeeConfig) -> Result<AgentProcess, String>;
    fn is_available(&self) -> bool;
}

pub struct AdapterRegistry {
    adapters: HashMap<String, Box<dyn AgentAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: HashMap::new(),
        };
        let claude = claude_code::ClaudeCodeAdapter;
        if claude.is_available() {
            registry.adapters.insert("claude_code".to_string(), Box::new(claude));
        }
        registry
    }

    pub fn get(&self, backend: &str) -> Option<&dyn AgentAdapter> {
        self.adapters.get(backend).map(|a| a.as_ref())
    }

    pub fn is_available(&self, backend: &str) -> bool {
        self.adapters.contains_key(backend)
    }
}
