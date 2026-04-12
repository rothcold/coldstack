# Changelog

All notable changes to this project will be documented in this file.

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
