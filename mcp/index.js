import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const BASE_URL = process.env.COLDSTACK_URL || "http://127.0.0.1:8080";

async function api(path, options = {}) {
  const res = await fetch(`${BASE_URL}/api${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}: ${text}`);
  }
  return text ? JSON.parse(text) : null;
}

function text(data) {
  return {
    content: [{ type: "text", text: typeof data === "string" ? data : JSON.stringify(data, null, 2) }],
  };
}

const TaskStatus = z.enum(["Pending", "Doing", "Finished", "Reviewing", "Done"]);

const server = new McpServer({
  name: "coldstack",
  version: "1.0.0",
});

// --- Task tools ---

server.tool("list_tasks", "List all tasks with their subtasks", {}, async () => {
  const tasks = await api("/tasks");
  return text(tasks);
});

server.tool(
  "create_task",
  "Create a new task",
  {
    task_id: z.string().describe("Unique string ID, e.g. TASK-101"),
    title: z.string(),
    description: z.string().optional().default(""),
    assignee: z.string().optional().nullable().describe("Assignee name, e.g. @username"),
  },
  async ({ task_id, title, description, assignee }) => {
    const task = await api("/tasks", {
      method: "POST",
      body: JSON.stringify({ task_id, title, description, assignee: assignee ?? null }),
    });
    return text(task);
  }
);

server.tool(
  "update_task",
  "Update a task's fields (all fields optional)",
  {
    id: z.number().describe("Internal database ID of the task"),
    task_id: z.string().optional(),
    title: z.string().optional(),
    description: z.string().optional(),
    completed: z.boolean().optional(),
    status: TaskStatus.optional(),
    assignee: z.string().optional().nullable(),
  },
  async ({ id, ...body }) => {
    // Remove undefined fields
    const cleaned = Object.fromEntries(Object.entries(body).filter(([, v]) => v !== undefined));
    const task = await api(`/tasks/${id}`, {
      method: "PUT",
      body: JSON.stringify(cleaned),
    });
    return text(task);
  }
);

server.tool(
  "delete_task",
  "Delete a task and all its subtasks",
  {
    id: z.number().describe("Internal database ID of the task"),
  },
  async ({ id }) => {
    await api(`/tasks/${id}`, { method: "DELETE" });
    return text(`Task ${id} deleted.`);
  }
);

// --- Subtask tools ---

server.tool(
  "add_subtask",
  "Add a subtask to a task",
  {
    task_id: z.number().describe("Internal database ID of the parent task"),
    title: z.string(),
    assignee: z.string().optional().nullable(),
  },
  async ({ task_id, title, assignee }) => {
    const subtask = await api(`/tasks/${task_id}/subtasks`, {
      method: "POST",
      body: JSON.stringify({ title, assignee: assignee ?? null }),
    });
    return text(subtask);
  }
);

server.tool(
  "update_subtask",
  "Update a subtask's fields (all fields optional)",
  {
    task_id: z.number().describe("Internal database ID of the parent task"),
    subtask_id: z.number().describe("Internal database ID of the subtask"),
    title: z.string().optional(),
    completed: z.boolean().optional(),
    status: TaskStatus.optional(),
    assignee: z.string().optional().nullable(),
  },
  async ({ task_id, subtask_id, ...body }) => {
    const cleaned = Object.fromEntries(Object.entries(body).filter(([, v]) => v !== undefined));
    const subtask = await api(`/tasks/${task_id}/subtasks/${subtask_id}`, {
      method: "PUT",
      body: JSON.stringify(cleaned),
    });
    return text(subtask);
  }
);

server.tool(
  "toggle_subtask",
  "Toggle a subtask's completion status",
  {
    task_id: z.number().describe("Internal database ID of the parent task"),
    subtask_id: z.number().describe("Internal database ID of the subtask"),
  },
  async ({ task_id, subtask_id }) => {
    const subtask = await api(`/tasks/${task_id}/subtasks/${subtask_id}/toggle`, {
      method: "POST",
    });
    return text(subtask);
  }
);

// --- Agent tools ---

server.tool("list_agents", "List all registered agents", {}, async () => {
  const agents = await api("/agents");
  return text(agents);
});

server.tool(
  "create_agent",
  "Register a new agent",
  {
    name: z.string().describe("Unique agent name, used as assignee in tasks"),
    cli: z.enum(["claude", "gemini"]).describe("CLI backend to use"),
    system_prompt: z.string().optional().describe("Custom role/system prompt for this agent"),
    work_dir: z.string().optional().describe("Working directory for the agent"),
    model: z.string().optional().nullable().describe("Model override"),
    max_concurrency: z.number().optional().describe("Max concurrent tasks"),
  },
  async ({ name, cli, system_prompt, work_dir, model, max_concurrency }) => {
    const agent = await api("/agents", {
      method: "POST",
      body: JSON.stringify({ name, cli, system_prompt, work_dir, model: model ?? null, max_concurrency }),
    });
    return text(agent);
  }
);

server.tool(
  "update_agent",
  "Update an agent's configuration (all fields optional)",
  {
    id: z.number().describe("Internal database ID of the agent"),
    name: z.string().optional(),
    cli: z.enum(["claude", "gemini"]).optional(),
    system_prompt: z.string().optional(),
    work_dir: z.string().optional(),
    model: z.string().optional().nullable(),
    max_concurrency: z.number().optional(),
  },
  async ({ id, ...body }) => {
    const cleaned = Object.fromEntries(Object.entries(body).filter(([, v]) => v !== undefined));
    const agent = await api(`/agents/${id}`, {
      method: "PUT",
      body: JSON.stringify(cleaned),
    });
    return text(agent);
  }
);

server.tool(
  "delete_agent",
  "Delete an agent",
  {
    id: z.number().describe("Internal database ID of the agent"),
  },
  async ({ id }) => {
    await api(`/agents/${id}`, { method: "DELETE" });
    return text(`Agent ${id} deleted.`);
  }
);

// Start
const transport = new StdioServerTransport();
await server.connect(transport);
