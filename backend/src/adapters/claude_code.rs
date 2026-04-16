use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::BufReader;
use tokio::process::Command;

use super::{AgentAdapter, AgentProcess, EmployeeConfig, TaskInfo};

pub struct ClaudeCodeAdapter;

fn sanitize_path(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

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
        let binary =
            Self::find_claude_binary().ok_or_else(|| "claude CLI not found in PATH".to_string())?;

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

        let workspace: PathBuf =
            PathBuf::from("agent_workspaces").join(sanitize_path(&task.task_id));
        std::fs::create_dir_all(&workspace)
            .map_err(|e| format!("Failed to create workspace {:?}: {}", workspace, e))?;

        let mut cmd = Command::new(&binary);
        cmd.current_dir(&workspace)
            .arg("-p")
            .arg(&prompt)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions")
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
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr".to_string())?;

        Ok(AgentProcess {
            child,
            stdout: BufReader::new(stdout),
            stderr: BufReader::new(stderr),
        })
    }

    fn is_available(&self) -> bool {
        Self::find_claude_binary().is_some()
    }
}
