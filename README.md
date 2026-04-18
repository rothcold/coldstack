# Coldstack

A self-hosted, single-binary AI workflow control plane. Coldstack lets you register role-specific AI agents, assign them staged tasks, and watch handoffs move from planning to QA from one Rust binary that embeds the web UI.

## What's in the box

- **`backend/`** — Rust + Actix-web server (`coldstack-server` crate, binary name `coldstack`). SQLite via `rusqlite` (bundled), serves the embedded React SPA on the same port as the API.
- **`frontend/`** — React 19 + TypeScript + Vite (`coldstack-web` package). Compiled to static assets that get embedded into the Rust binary at build time via `rust-embed`.
- **`mcp/`** — MCP server exposing Coldstack's task/agent API as MCP tools, so any MCP-aware AI client can manage tasks directly.
- **`agent_workspaces/`** — Per-task git workspaces created from each task's configured source repository and source branch.
- **`COLDSTACK_SKILL.md`** — API reference for AI agents integrating against Coldstack.

## Quick start

### Run the prebuilt binary

```bash
./build_and_package.sh           # builds frontend, then backend, outputs to release/coldstack
./release/coldstack              # serves http://127.0.0.1:8080
```

The frontend **must** be built before the backend — `rust-embed` compiles `frontend/dist/` into the binary at compile time.

### Docker

```bash
docker compose up --build
```

The container exposes `:8080` and mounts a `db-data` volume at `/data` so `tasks.db` survives restarts.

### Development

Backend:

```bash
cd backend
cargo run                        # http://127.0.0.1:8080
```

Frontend (with hot reload, proxies `/api` to the backend):

```bash
cd frontend
pnpm install
pnpm run dev                     # http://localhost:5173
```

## Running agents

Register employees through the UI or API, then assign them workflow tasks. Each task now carries:

- a git source, local path or remote repository
- a source branch, defaulting to `main`
- a dedicated human-readable task branch

When an agent starts work, Coldstack clones the source into `agent_workspaces/<task-id>/`, checks out the task branch, and commits task output locally. The task detail panel exposes an explicit `Push Branch` action when you want to publish that branch back to the source repository.

## MCP integration

Point any MCP client at `mcp/index.js` to expose Coldstack's tools (`list_tasks`, `create_task`, `update_task`, `add_subtask`, `list_agents`, …). Set `COLDSTACK_URL` if the server is not on `127.0.0.1:8080`.

## API

All routes are under `/api`. See [`COLDSTACK_SKILL.md`](./COLDSTACK_SKILL.md) for the full reference. Highlights:

- `GET /api/tasks` — list workflow board summaries
- `POST /api/tasks` — create task (`task_id` must be unique → `409 Conflict` otherwise)
- `PUT /api/tasks/{id}` — partial update
- `POST /api/tasks/{id}/publish` — push the task branch to the configured source repository
- `DELETE /api/tasks/{id}` — cascades to subtasks
- `GET /api/agents`, `POST /api/agents`, `PUT /api/agents/{id}`, `DELETE /api/agents/{id}`
- `GET /api/employees`, `POST /api/employees/{id}/assign/{task_id}`, …

`{id}` always refers to the **internal integer `id`**, not the user-facing `task_id` string.

**Task statuses:** `Plan` → `Design` → `Coding` → `Review` → `QA` → `NeedsHuman` → `Done`

Successful executions can automatically hand tasks to the next matching idle workflow-role agent. If no downstream agent is idle, the task stays in the next status and is marked as waiting for auto handoff until the background scanner can claim it.

## Project layout

```
coldstack/
├── backend/              # Rust server (coldstack-server)
│   ├── src/
│   │   ├── main.rs       # Actix-web entrypoint, embeds frontend/dist
│   │   ├── handlers/     # API routes
│   │   ├── adapters/     # Agent CLI adapters (claude, gemini)
│   │   ├── db.rs
│   │   └── models.rs
│   └── Cargo.toml
├── frontend/             # React SPA (coldstack-web)
│   └── src/App.tsx
├── mcp/                  # MCP server exposing the Coldstack API
├── agent/                # Polling runner that drives CLIs
├── build_and_package.sh  # Frontend → backend → release/coldstack
├── Dockerfile
└── docker-compose.yml
```
