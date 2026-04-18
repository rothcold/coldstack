# TODOs

## Manual Agent Stop And Reset

### Backend stop and reset flow
- [x] Add a shared helper in `backend/src/handlers/employees.rs` to centralize which employee states can be reset to `idle`
- [x] Add `Reset Agent` backend API in `backend/src/handlers/employees.rs` for agents in `error` or stale `working`
- [x] Keep `Reset Agent` scoped to employee state only and do not rewrite historical execution rows
- [x] Keep `Stop Execution` on the existing execution cancel path and reject stop requests for non-running executions
- [x] Reject `Reset Agent` when the employee still has a running execution and instruct the caller to stop it first
- [x] Wire the new reset route in `backend/src/main.rs`

### Frontend stop and reset UX
- [x] Add `Stop Execution` action to the employee detail panel when the selected agent has a running execution
- [x] Add `Reset Agent` action to the employee detail panel when the selected agent is in `error` or stale `working` without a current execution
- [x] Surface clear error feedback for stop/reset conflicts instead of silently failing
- [x] Refresh the parent employee roster after stop/reset success so card status and detail state stay in sync
- [x] Refresh execution history/detail state after stop/reset success without adding a new global store

### Tests
- [x] Add backend tests for stopping a running execution, including employee status returning to `idle`
- [x] Add backend tests for stop conflicts and missing execution ids
- [x] Add backend tests for resetting an `error` agent and a stale `working` agent
- [x] Add backend tests for reset conflicts: missing employee, already idle, and running execution still attached
- [x] Add backend tests proving `Reset Agent` does not mutate existing execution history
- [x] Add frontend tests covering which stop/reset actions render for each visible employee state
- [x] Add frontend tests covering successful stop/reset actions and the required roster/detail refresh
- [x] Add frontend tests covering visible error messages for stop/reset conflict responses

## Completed

- [x] Add a persistent task-level orchestration marker, such as `auto_handoff_pending`, so only explicitly waiting tasks are picked up by automatic handoff
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add task-level claim logic so execution completion and the background scanner cannot both auto-assign the same task
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add a shared orchestration helper in `backend/src` that advances workflow state, manages pending markers, chooses the downstream agent, and starts the next execution
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Reuse the existing workflow state machine instead of adding a second transition system for automatic handoff
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] On successful execution completion, automatically advance the task to the next workflow status before attempting downstream assignment
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] When multiple downstream idle agents match the next workflow role, always choose the smallest employee id deterministically
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] When no downstream idle agent is available, keep the task at the new workflow status and mark it as waiting for auto handoff
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Never auto-assign `NeedsHuman` or `Done` tasks
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Never auto-handoff after failed or cancelled executions
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Start an in-process Rust background scanner with `tokio::interval` instead of an external cron job
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Make the scanner look only at tasks explicitly marked as waiting for auto handoff
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] When a matching downstream idle agent later becomes available, let the scanner auto-assign the task and clear the pending marker
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Keep scanner passes idempotent so repeated ticks do not create duplicate execution rows
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Show that a task is waiting for the next workflow-role agent when auto handoff cannot continue immediately
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Refresh task detail and board state after automatic downstream assignment so the UI reflects the new owner and stage
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Surface clear waiting reasons when no downstream idle agent or backend is available
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests for successful execution completion triggering immediate downstream auto handoff
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests for successful execution completion marking tasks pending when no downstream idle agent exists
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests for the background scanner picking up pending tasks after a matching agent becomes idle
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests for deterministic smallest-id downstream agent selection
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests proving failed and cancelled executions never trigger auto handoff
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests proving `NeedsHuman` and `Done` tasks are never auto-assigned
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add backend tests proving task-level claim logic prevents duplicate handoff assignments across completion and scanner paths
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add frontend tests covering visible waiting-for-next-agent state
  **Completed:** v0.2.0.0 (2026-04-18)
- [x] Add frontend tests covering UI refresh after automatic downstream assignment
  **Completed:** v0.2.0.0 (2026-04-18)

## Automatic Workflow Handoff

### Backend orchestration flow
- [ ] Add a persistent task-level orchestration marker, such as `auto_handoff_pending`, so only explicitly waiting tasks are picked up by automatic handoff
- [ ] Add task-level claim logic so execution completion and the background scanner cannot both auto-assign the same task
- [ ] Add a shared orchestration helper in `backend/src` that advances workflow state, manages pending markers, chooses the downstream agent, and starts the next execution
- [ ] Reuse the existing workflow state machine instead of adding a second transition system for automatic handoff
- [ ] On successful execution completion, automatically advance the task to the next workflow status before attempting downstream assignment
- [ ] When multiple downstream idle agents match the next workflow role, always choose the smallest employee id deterministically
- [ ] When no downstream idle agent is available, keep the task at the new workflow status and mark it as waiting for auto handoff
- [ ] Never auto-assign `NeedsHuman` or `Done` tasks
- [ ] Never auto-handoff after failed or cancelled executions

### Background scanner
- [ ] Start an in-process Rust background scanner with `tokio::interval` instead of an external cron job
- [ ] Make the scanner look only at tasks explicitly marked as waiting for auto handoff
- [ ] When a matching downstream idle agent later becomes available, let the scanner auto-assign the task and clear the pending marker
- [ ] Keep scanner passes idempotent so repeated ticks do not create duplicate execution rows

### Frontend visibility
- [ ] Show that a task is waiting for the next workflow-role agent when auto handoff cannot continue immediately
- [ ] Refresh task detail and board state after automatic downstream assignment so the UI reflects the new owner and stage
- [ ] Surface clear waiting reasons when no downstream idle agent or backend is available

### Tests
- [ ] Add backend tests for successful execution completion triggering immediate downstream auto handoff
- [ ] Add backend tests for successful execution completion marking tasks pending when no downstream idle agent exists
- [ ] Add backend tests for the background scanner picking up pending tasks after a matching agent becomes idle
- [ ] Add backend tests for deterministic smallest-id downstream agent selection
- [ ] Add backend tests proving failed and cancelled executions never trigger auto handoff
- [ ] Add backend tests proving `NeedsHuman` and `Done` tasks are never auto-assigned
- [ ] Add backend tests proving task-level claim logic prevents duplicate handoff assignments across completion and scanner paths
- [ ] Add frontend tests covering visible waiting-for-next-agent state
- [ ] Add frontend tests covering UI refresh after automatic downstream assignment
