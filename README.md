# Coldstack

A self-hosted, single-binary AI employee dashboard. Coldstack lets you register AI agents (Claude, Gemini, …), assign them tasks, and watch them work — all from one Rust binary that embeds the entire web UI.

## What's in the box

- **`backend/`** — Rust + Actix-web server (`coldstack-server` crate, binary name `coldstack`). SQLite via `rusqlite` (bundled), serves the embedded React SPA on the same port as the API.
- **`frontend/`** — React 19 + TypeScript + Vite (`coldstack-web` package). Compiled to static assets that get embedded into the Rust binary at build time via `rust-embed`.
- **`mcp/`** — MCP server exposing Coldstack's task/agent API as MCP tools, so any MCP-aware AI client can manage tasks directly.
- **`agent/`** — Node.js polling runner that picks up `Pending` tasks, dispatches them to the configured CLI (`claude` / `gemini`), and reports back via the API.
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

Start the polling runner alongside the server:

```bash
cd agent
node index.js
```

Environment variables:

- `COLDSTACK_URL` — server URL (default `http://127.0.0.1:8080`)
- `POLL_INTERVAL_MS` — poll interval in ms (default `5000`)
- `MCP_SERVER_PATH` — path to the MCP server (default `../mcp/index.js`)

The runner picks up any task whose `assignee` matches a registered agent's `name` and whose `status` is `Pending`. Failures retry up to 3× before flipping the task to `Reviewing`.

## MCP integration

Point any MCP client at `mcp/index.js` to expose Coldstack's tools (`list_tasks`, `create_task`, `update_task`, `add_subtask`, `list_agents`, …). Set `COLDSTACK_URL` if the server is not on `127.0.0.1:8080`.

## API

All routes are under `/api`. See [`COLDSTACK_SKILL.md`](./COLDSTACK_SKILL.md) for the full reference. Highlights:

- `GET /api/tasks` — list tasks (with subtasks)
- `POST /api/tasks` — create task (`task_id` must be unique → `409 Conflict` otherwise)
- `PUT /api/tasks/{id}` — partial update
- `DELETE /api/tasks/{id}` — cascades to subtasks
- `GET /api/agents`, `POST /api/agents`, `PUT /api/agents/{id}`, `DELETE /api/agents/{id}`
- `GET /api/employees`, `POST /api/employees/{id}/assign/{task_id}`, …

`{id}` always refers to the **internal integer `id`**, not the user-facing `task_id` string.

**Task statuses:** `Pending` → `Doing` → `Finished` → `Reviewing` → `Done`

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
