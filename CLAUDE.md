# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Frontend (`/frontend`)

```bash
pnpm install          # Install dependencies
pnpm run dev          # Start Vite dev server (http://localhost:5173)
pnpm run build        # TypeScript check + production build to dist/
pnpm run lint         # ESLint with TypeScript support
pnpm run preview      # Preview production build
```

### Backend (`/backend`)

```bash
cargo build          # Debug build
cargo build --release  # Release build
cargo run            # Run server (http://127.0.0.1:8080)
```

### Full Build (single binary)

```bash
./build_and_package.sh  # Builds frontend, then backend embeds frontend/dist/, outputs to release/task-manager
```

> The frontend **must** be built before the backend when doing a full build, because rust-embed compiles `frontend/dist/` into the binary at compile time.

## Architecture

This is a **single-binary full-stack app**: the Rust backend embeds the compiled React frontend using `rust-embed`, serving it as static assets alongside the REST API.

### Backend (`backend/src/main.rs`)

- Actix-web server on `127.0.0.1:8080`
- CORS configured for dev (`localhost:5173`)
- All API routes scoped under `/api`
- Unmatched routes fall through to the embedded SPA (`index.html`)
- SQLite database via `rusqlite` with foreign key cascade on subtask deletion

**Database schema:**

- `tasks`: `id`, `task_id` (unique string), `title`, `description`, `completed`, `status`, `assignee`, `created_at`
- `subtasks`: `id`, `task_id` (FK → tasks.id), `title`, `completed`, `status`, `assignee`

**TaskStatus values:** `Pending`, `Doing`, `Finished`, `Reviewing`, `Done`

### Frontend (`frontend/src/App.tsx`)

- Single-component React app managing all state via `useState`/`useEffect`
- Fetches from `/api` (proxied to backend in dev via Vite config)
- All styling is inline CSS — no external CSS framework

## API Reference

See `TASK_MANAGER_SKILL.md` for the full API spec used by AI agent integrations. Key points:

- `{id}` in task routes refers to the **internal integer `id`**, not `task_id`
- `task_id` must be unique; POST returns `409 Conflict` if already exists
- Subtask routes: `/api/tasks/{id}/subtasks` and `/api/tasks/{id}/subtasks/{subid}`
- Update endpoints accept partial bodies (all fields optional)

## Skill routing

When the user's request matches an available skill, ALWAYS invoke it using the Skill
tool as your FIRST action. Do NOT answer directly, do NOT use other tools first.
The skill has specialized workflows that produce better results than ad-hoc answers.

Key routing rules:
- Product ideas, "is this worth building", brainstorming → invoke office-hours
- Bugs, errors, "why is this broken", 500 errors → invoke investigate
- Ship, deploy, push, create PR → invoke ship
- QA, test the site, find bugs → invoke qa
- Code review, check my diff → invoke review
- Update docs after shipping → invoke document-release
- Weekly retro → invoke retro
- Design system, brand → invoke design-consultation
- Visual audit, design polish → invoke design-review
- Architecture review → invoke plan-eng-review
- Save progress, checkpoint, resume → invoke checkpoint
- Code quality, health check → invoke health
