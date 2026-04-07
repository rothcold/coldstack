use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::BufReader;
use tokio::process::Command;

use super::{AgentAdapter, AgentProcess, EmployeeConfig, TaskInfo};

pub struct ClaudeCodeAdapter;

impl ClaudeCodeAdapter {
    fn find_claude_binary() -> Option<String> {
        // Check common locations
        for name in &["claude"] {
            if std::process::Command::new("which")
                .arg(name)
                .output()
                .ok()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(name.to_string());
            }
        }
        None
    }
}

#[async_trait]
impl AgentAdapter for ClaudeCodeAdapter {
    async fn execute(
        &self,
        task: &TaskInfo,
        employee: &EmployeeConfig,
    ) -> Result<AgentProcess, String> {
        let binary = Self::find_claude_binary()
            .ok_or_else(|| "claude CLI not found in PATH".to_string())?;

        let prompt = if let Some(ref sys_prompt) = employee.system_prompt {
            format!(
                "{}\n\nTask: {} ({})\n\n{}",
                sys_prompt, task.title, task.task_id, task.description
            )
        } else {
            format!(
                "Task: {} ({})\n\n{}",
                task.title, task.task_id, task.description
            )
        };

        let mut cmd = Command::new(&binary);
        cmd.arg("-p")
            .arg(&prompt)
            .arg("--output-format")
            .arg("stream-json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", binary, e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;

        Ok(AgentProcess {
            child,
            stdout: BufReader::new(stdout),
        })
    }

    fn is_available(&self) -> bool {
        Self::find_claude_binary().is_some()
    }

    fn backend_name(&self) -> &str {
        "claude_code"
    }
}
