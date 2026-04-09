# Skill: Coldstack Integration

Expert guidance for AI agents to interact with the Coldstack API to manage tasks, subtasks, statuses, and assignees.

## API Specification

All endpoints are prefixed with `/api`. The server runs on `http://127.0.0.1:8080` by default.

### Data Models

#### Task Status
- `Pending`, `Doing`, `Finished`, `Reviewing`, `Done`

#### Task Object
- `id`: Internal database ID (integer)
- `task_id`: Unique user-defined string ID (e.g., "TASK-101")
- `title`: String
- `description`: String
- `completed`: Boolean
- `status`: TaskStatus
- `assignee`: String (optional)
- `created_at`: ISO 8601 string
- `subtasks`: Array of Subtask objects

#### Subtask Object
- `id`: Internal ID
- `task_id`: Parent Task's internal ID
- `title`: String
- `completed`: Boolean
- `status`: TaskStatus
- `assignee`: String (optional)

---

## Core Operations

### 1. Task Management

| Action | Method | Endpoint | Description |
| :--- | :--- | :--- | :--- |
| List Tasks | `GET` | `/api/tasks` | Returns all tasks with subtasks. |
| Create Task | `POST` | `/api/tasks` | Body: `{ "task_id": string, "title": string, "description": string, "assignee": string \| null }` |
| Update Task | `PUT` | `/api/tasks/{id}` | Body: `{ "task_id": string?, "title": string?, "description": string?, "completed": bool?, "status": TaskStatus?, "assignee": string? }` |
| Delete Task | `DELETE` | `/api/tasks/{id}` | Deletes task and all its subtasks. |

### 2. Subtask Management

| Action | Method | Endpoint | Description |
| :--- | :--- | :--- | :--- |
| Add Subtask | `POST` | `/api/tasks/{id}/subtasks` | Body: `{ "title": string, "assignee": string \| null }` |
| Toggle Subtask | `POST` | `/api/tasks/{id}/subtasks/{subid}/toggle` | Flips the completion status. |
| Update Subtask | `PUT` | `/api/tasks/{id}/subtasks/{subid}` | Body: `{ "title": string?, "completed": bool?, "status": TaskStatus?, "assignee": string? }` |

---

## Instructions for AI Agents

### Workflow: Create and Organize a Project
1. **Initialize**: Create a main Task with a unique `task_id`.
2. **Decompose**: Add subtasks for specific action items.
3. **Assign**: Set assignees at either the task or subtask level.
4. **Track**: Use `PUT` to update `status` as work progresses.

### Constraints & Validation
- **Unique Task ID**: You MUST check if a `task_id` exists before creating a new one, or handle the `409 Conflict` error.
- **Hierarchical Status**: When all subtasks are `Done`, consider updating the parent Task status to `Finished` or `Done`.
- **Assignment**: Assignees are plain text. Use consistent naming (e.g., "@username").

### Example Tool Call (Python/Requests)
```python
import requests

def create_full_task(task_id, title, desc, subtasks):
    # 1. Create Task
    res = requests.post("http://127.0.0.1:8080/api/tasks", json={
        "task_id": task_id, "title": title, "description": desc
    })
    task = res.json()
    
    # 2. Add Subtasks
    for st in subtasks:
        requests.post(f"http://127.0.0.1:8080/api/tasks/{task['id']}/subtasks", json={"title": st})
```
