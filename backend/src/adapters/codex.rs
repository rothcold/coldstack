use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::BufReader;
use tokio::process::Command;

use super::{AgentAdapter, AgentProcess, EmployeeConfig, TaskInfo, find_binary};

pub struct CodexAdapter;

#[async_trait]
impl AgentAdapter for CodexAdapter {
    async fn execute(
        &self,
        task: &TaskInfo,
        employee: &EmployeeConfig,
    ) -> Result<AgentProcess, String> {
        let binary = find_binary(&["codex"])
            .ok_or_else(|| "codex CLI not found in PATH".to_string())?;

        let prompt = if let Some(ref sys_prompt) = employee.system_prompt {
            format!(
                "{}\n\nRepository source: {}\nSource branch: {}\nTarget branch: {}\n\nTask: {} ({})\n\n{}",
                sys_prompt, task.source, task.source_branch, task.branch_name, task.title, task.task_id, task.description
            )
        } else {
            format!(
                "Repository source: {}\nSource branch: {}\nTarget branch: {}\n\nTask: {} ({})\n\n{}",
                task.source, task.source_branch, task.branch_name, task.title, task.task_id, task.description
            )
        };

        let workspace = crate::task_source::ensure_workspace(
            &task.task_id,
            &task.source,
            &task.source_branch,
            &task.branch_name,
        )
        .await?;

        let mut child = Command::new(&binary)
            .current_dir(&workspace)
            .arg("exec")
            .arg(&prompt)
            .arg("-s")
            .arg("danger-full-access")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false)
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", binary, e))?;

        let stdout = child.stdout.take().ok_or_else(|| "Failed to capture stdout".to_string())?;
        let stderr = child.stderr.take().ok_or_else(|| "Failed to capture stderr".to_string())?;

        Ok(AgentProcess {
            child,
            stdout: BufReader::new(stdout),
            stderr: BufReader::new(stderr),
        })
    }

    fn is_available(&self) -> bool {
        find_binary(&["codex"]).is_some()
    }
}
