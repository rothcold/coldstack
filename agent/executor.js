import { spawn } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

function spawnAsync(cmd, args, options = {}) {
  return new Promise((res, rej) => {
    const proc = spawn(cmd, args, {
      ...options,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    proc.stdout.on("data", (d) => {
      stdout += d;
      if (stdout.length > 1024 * 1024) stdout = stdout.slice(-512 * 1024);
    });
    proc.stderr.on("data", (d) => {
      stderr += d;
      if (stderr.length > 1024 * 1024) stderr = stderr.slice(-512 * 1024);
    });

    const timeout = options.timeoutMs || 300_000;
    const timer = setTimeout(() => {
      proc.kill("SIGTERM");
      rej(new Error(`Process timed out after ${timeout}ms`));
    }, timeout);

    proc.on("error", (err) => {
      clearTimeout(timer);
      rej(err);
    });
    proc.on("close", (code) => {
      clearTimeout(timer);
      res({ code, stdout, stderr });
    });
  });
}

export async function executeClaude(systemPrompt, taskPrompt, agent, config) {
  const mcpPath = resolve(__dirname, config.mcpServerPath);
  const mcpConfig = JSON.stringify({
    mcpServers: {
      "coldstack": {
        command: "node",
        args: [mcpPath],
        env: { COLDSTACK_URL: config.coldstackUrl },
      },
    },
  });

  const args = [
    "-p",
    "--output-format", "text",
    "--mcp-config", mcpConfig,
    "--system-prompt", systemPrompt,
  ];
  if (agent.model) args.push("--model", agent.model);
  args.push(taskPrompt);

  return spawnAsync("claude", args, { cwd: agent.work_dir || "." });
}

export async function executeGemini(systemPrompt, taskPrompt, agent, config) {
  // Gemini has no --system-prompt flag, so prepend it
  const combinedPrompt = `${systemPrompt}\n\n---\n\n${taskPrompt}`;
  const args = ["-p", combinedPrompt];

  return spawnAsync("gemini", args, { cwd: agent.work_dir || "." });
}

export async function setupGeminiMcp(config) {
  const mcpPath = resolve(__dirname, config.mcpServerPath);
  try {
    await spawnAsync("gemini", ["mcp", "add", "coldstack", "--", "node", mcpPath], {
      timeoutMs: 15_000,
    });
    return true;
  } catch {
    return false;
  }
}
