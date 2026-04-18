# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0.0] - 2026-04-18

### Added
- Added task source repository support, including editable source branch selection, human-readable task branch names, and workspace cloning from either local git paths or remote repositories.
- Added an explicit `Push Branch` task action so finished task work can be pushed to the source repository on demand instead of implicitly during execution finalization.
- Added automatic workflow handoff orchestration that advances successful tasks to the next stage, assigns the next idle matching agent, and falls back to a pending queue when no downstream agent is available.
- Added an in-process background scanner that retries pending auto-handoffs when a matching idle agent later becomes available.

### Changed
- Changed agent execution startup so adapters always receive repository source, source branch, and target branch context before work begins.
- Changed task detail and board UI to show waiting-for-agent state, source metadata, and branch metadata directly in the workflow surface.
- Changed local task workspace cloning to use isolated git object storage for local repositories, reducing bleed-through from task workspaces back into source repos.

### Fixed
- Fixed assignment error handling so assigning a task to a missing employee still returns `404` after the assignment flow refactor.
- Fixed backend and frontend test coverage around task source branch handling, publish branch behavior, and automatic workflow handoff visibility.

## [0.1.0.0] - 2026-04-13

### Added
- Added a workflow board that moves tasks through Plan, Design, Coding, Review, QA, Human approval, and archive-ready completion.
- Added task detail timelines, explicit transition actions, rejection notes, and human-only archive approval so every handoff is visible in the UI.
- Added workflow transition APIs, board summary/detail payloads, workflow event storage, and role-aware validation on the Rust backend.
- Added roster management for workflow roles, backend availability feedback, and live execution stream reuse across employee cards.
- Added Vitest component coverage for the workflow UI and Playwright E2E coverage for the golden path and mobile detail sheet flow.

### Changed
- Changed task semantics from legacy `completed`/`Doing` style states to workflow-first `archived` and staged status handling across backend and frontend models.
- Changed the default app navigation so the workflow board is primary and the company roster acts as the control plane for agent setup.
- Changed employee management to use structured workflow roles and stricter backend validation for supported adapters.
- Changed frontend theming, layout, and responsive behavior to support desktop lane groups and mobile full-screen detail sheets.

### Fixed
- Fixed execution streaming so stdout and stderr share one ordered event stream and current execution state stays visible on the roster.
- Fixed role/status naming drift between backend and frontend and surfaced missing adapter availability directly in the UI.
- Fixed local runtime artifacts leaking into git by ignoring `tasks.db`, `backend/tasks.db`, and `frontend/test-results/`.
