export function buildSystemPrompt(task, agent) {
  const subtaskLines = (task.subtasks || [])
    .map((s) => `- [${s.status}] (id:${s.id}) ${s.title}`)
    .join("\n");

  const roleBlock = agent.system_prompt
    ? `## Role\n${agent.system_prompt}\n\n`
    : "";

  return `You are an AI agent "${agent.name}" working in: ${agent.work_dir || "."}
You have MCP tools for the coldstack server. Use them to track your progress.

${roleBlock}## Task
- ID: ${task.id} | Task ID: ${task.task_id}
- Title: ${task.title}

## Description
${task.description || "(no description)"}

${subtaskLines ? `## Subtasks\n${subtaskLines}\n` : ""}
## Instructions
1. Execute the work described in the task.
2. Use update_subtask / add_subtask MCP tools to track progress on subtasks.
3. When all work is complete, use the update_task tool to set status to "Finished".
4. If you hit a blocker, set status to "Reviewing" and explain the issue in the description.`;
}

export function buildTaskPrompt(task) {
  return `Execute the following task: ${task.title}\n\n${task.description || ""}`;
}
