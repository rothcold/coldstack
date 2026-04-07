# TODOS

## PID Reuse Verification on Startup Recovery
**Why:** On server restart, stored PIDs may have been reused by unrelated processes. Without verification, startup recovery could kill a random process.
**What:** Before killing a stored PID during startup recovery, verify via `/proc/{pid}/cmdline` (Linux) that the process is actually a `claude`/`gemini`/`codex` process.
**Depends on:** Phase 3 (agent adapter + process management)

## SSE Last-Event-ID Reconnection
**Why:** When an SSE connection drops, the client reconnects and currently receives all chunks from the start. With Last-Event-ID support, it resumes from where it left off.
**What:** Add `id:` field to SSE events using output_chunks.seq. Accept `Last-Event-ID` header on the stream endpoint and filter chunks with seq > that value.
**Depends on:** Phase 3 (SSE streaming)

## Output Chunk Size Limits
**Why:** A runaway agent process could produce megabytes of output, filling the SQLite database.
**What:** Cap output at ~10MB per execution (~10,000 chunks). On exceed, stop writing new chunks but keep the process running. Frontend shows "output truncated" warning.
**Depends on:** Phase 3

## Frontend Component Extraction
**Why:** Backend gets full module extraction, but frontend stays as a growing monolith in App.tsx (663 lines, will grow past 1000 with Company view). Inconsistent.
**What:** Extract into: App.tsx (router/layout), TasksView.tsx, CompanyView.tsx, EmployeeCard.tsx, EmployeeDetail.tsx, LiveTerminal.tsx.
**Depends on:** Phase 4 (frontend)

## Shared CSS Variables File
**Why:** The Tasks view and Company view use completely different visual styles (light task list vs dark terminal). Without shared design tokens, they look like different apps.
**What:** Create a CSS variables file with color tokens (bg, text, status colors), spacing scale (4/8/16/24/32px), typography (sans + mono), terminal colors, and border radius. Both views reference these variables.
**Depends on:** Start of Phase 4 (before building Company view)

## Keyboard Navigation for Company View
**Why:** Without keyboard nav, keyboard-only users can't use the Company view at all. Tab between cards, Enter to select, Escape to close panel.
**What:** Add tabindex to employee cards, Enter/Space to open detail panel, Escape to close. Terminal region as `role="log"` with `aria-live="polite"`. ARIA labels on status indicators ("Alice, idle" not just a colored dot). Color is not the only signal for status (dot + text label).
**Depends on:** Phase 4 (frontend)

## Full Design System (DESIGN.md)
**Why:** CSS variables are a band-aid. A proper design system (aesthetic, typography, color palette, component vocabulary) prevents the Company view from drifting into generic dashboard territory.
**What:** Run /design-consultation to create DESIGN.md with: aesthetic direction, font choices, full color palette, spacing scale, motion guidelines, component patterns. Then migrate existing inline CSS to use the system.
**Depends on:** Post-MVP, before scaling past 2 views
