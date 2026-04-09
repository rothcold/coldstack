import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { buildSystemPrompt, buildTaskPrompt } from "./prompt.js";
import { executeClaude, executeGemini, setupGeminiMcp } from "./executor.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

// --- Config ---
const BASE = process.env.COLDSTACK_URL || "http://127.0.0.1:8080";
const POLL_INTERVAL = Number(process.env.POLL_INTERVAL_MS) || 5000;
const MCP_SERVER_PATH = resolve(__dirname, process.env.MCP_SERVER_PATH || "../mcp/index.js");

const config = { coldstackUrl: BASE, mcpServerPath: MCP_SERVER_PATH };

// --- Logging ---
function log(agent, msg, level = "INFO") {
  const ts = new Date().toISOString();
  process.stderr.write(`[${ts}] [${level}] [${agent}] ${msg}\n`);
}

// --- API helpers ---
async function api(path, options = {}) {
  const res = await fetch(`${BASE}/api${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  const text = await res.text();
  if (!res.ok) throw new Error(`${res.status}: ${text}`);
  return text ? JSON.parse(text) : null;
}

async function getTasks() {
  return api("/tasks");
}

async function getAgents() {
  return api("/agents");
}

async function updateTask(id, body) {
  return api(`/tasks/${id}`, { method: "PUT", body: JSON.stringify(body) });
}

// --- Executor dispatch ---
const executors = {
  claude: executeClaude,
  gemini: executeGemini,
};

// --- Per-agent state ---
const inFlight = new Map();   // agentName -> Set<taskId>
const retryCounts = new Map(); // taskId -> number

function getInFlight(name) {
  if (!inFlight.has(name)) inFlight.set(name, new Set());
  return inFlight.get(name);
}

// --- Task execution lifecycle ---
async function runTask(task, agent) {
  const flying = getInFlight(agent.name);
  flying.add(task.id);

  try {
    // 1. Mark as Doing
    await updateTask(task.id, { status: "Doing" });
    log(agent.name, `Picked up task ${task.id} "${task.title}"`);

    // 2. Build prompts (agent.system_prompt is the custom role from DB)
    const systemPrompt = buildSystemPrompt(task, agent);
    const taskPrompt = buildTaskPrompt(task);

    // 3. Execute
    const executor = executors[agent.cli];
    if (!executor) throw new Error(`Unknown CLI: ${agent.cli}`);

    const result = await executor(systemPrompt, taskPrompt, agent, config);

    // 4. Check current status (agent may have updated it via MCP)
    const tasks = await getTasks();
    const current = tasks.find((t) => t.id === task.id);
    const finalStatus = current?.status;

    if (result.code !== 0) {
      throw new Error(`CLI exited with code ${result.code}\n${result.stderr}`);
    }

    if (finalStatus === "Done" || finalStatus === "Finished") {
      log(agent.name, `Task ${task.id} already marked ${finalStatus} by agent`);
    } else {
      await updateTask(task.id, { status: "Finished" });
      log(agent.name, `Task ${task.id} completed → Finished`);
    }

    retryCounts.delete(task.id);
  } catch (err) {
    const retries = (retryCounts.get(task.id) || 0) + 1;
    retryCounts.set(task.id, retries);

    if (retries >= 3) {
      await updateTask(task.id, { status: "Reviewing" }).catch(() => {});
      log(agent.name, `Task ${task.id} failed ${retries}x, set to Reviewing: ${err.message}`, "ERROR");
      retryCounts.delete(task.id);
    } else {
      await updateTask(task.id, { status: "Pending" }).catch(() => {});
      log(agent.name, `Task ${task.id} failed (retry ${retries}/3): ${err.message}`, "WARN");
    }
  } finally {
    flying.delete(task.id);
  }
}

// --- Polling loop ---
async function poll() {
  try {
    const [tasks, agents] = await Promise.all([getTasks(), getAgents()]);
    const agentMap = new Map(agents.map((a) => [a.name, a]));

    for (const agent of agents) {
      const flying = getInFlight(agent.name);
      const pending = tasks.filter(
        (t) => t.assignee === agent.name && t.status === "Pending" && !flying.has(t.id)
      );

      const slots = (agent.max_concurrency || 1) - flying.size;
      const toRun = pending.slice(0, Math.max(0, slots));

      for (const task of toRun) {
        runTask(task, agent);
      }
    }
  } catch (err) {
    log("system", `Poll error: ${err.message}`, "WARN");
  }
}

// --- Startup ---
let shuttingDown = false;
let pollTimer;

async function main() {
  console.error(`Coldstack Agent Runner`);
  console.error(`API: ${BASE} | Poll interval: ${POLL_INTERVAL}ms`);
  console.error(`MCP server: ${MCP_SERVER_PATH}\n`);

  // Setup Gemini MCP once
  try {
    const agents = await getAgents();
    const hasGemini = agents.some((a) => a.cli === "gemini");
    if (hasGemini) {
      log("system", "Setting up Gemini MCP server...");
      const ok = await setupGeminiMcp(config);
      if (!ok) log("system", "Gemini MCP setup failed — gemini agents may not work", "WARN");
    }
    console.error(`Found ${agents.length} agent(s): ${agents.map((a) => a.name).join(", ") || "(none)"}\n`);
  } catch (err) {
    console.error(`Warning: Could not fetch agents from API: ${err.message}`);
    console.error("Will retry on next poll cycle.\n");
  }

  // Start polling
  poll();
  pollTimer = setInterval(() => {
    if (!shuttingDown) poll();
  }, POLL_INTERVAL);
}

// --- Graceful shutdown ---
function shutdown() {
  if (shuttingDown) return;
  shuttingDown = true;
  console.error("\nShutting down...");
  clearInterval(pollTimer);

  const allFlying = () => [...inFlight.values()].reduce((n, s) => n + s.size, 0);
  const start = Date.now();
  const check = setInterval(() => {
    if (allFlying() === 0 || Date.now() - start > 30_000) {
      clearInterval(check);
      if (allFlying() > 0) console.error("Force exit — some tasks still running");
      process.exit(0);
    }
  }, 500);
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
