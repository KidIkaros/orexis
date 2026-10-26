# ADR-0008 — Desktop surface contract

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/issues/006-desktop-panel-layout.md`

## Context

The desktop is the oversight workspace: steer a session, approve and review
changes, and inspect the fleet, board, artifacts, and entity. It shares the
daemon with the TUI over the Unix socket. The plan names six panels but not
their hierarchy, focus, or state model. A process audit found no task analysis,
no precise interaction spec, no measurable usability targets, and no evaluation
loop for the desktop.

## Decision

**Layout.** Three zones:

| Tier | Panels | Where |
|---|---|---|
| Primary | Chat / streaming | Center |
| Secondary | Approvals, Diff, Preview | Right contextual panel |
| Tertiary | Sessions, Board, Fleet, Artifacts, Files | Left rail |
| Modal | Settings, palette | Overlay |

The six named panels map task-first: chat center; approvals and diff right;
fleet, artifacts, and files left. They are not six equal tabs.

**Focus.** Composer first. `Tab` cycles composer → transcript → left rail →
right panel; `⌘1`…`⌘6` focus panels; `⌘⇧A` jumps to approvals; `Esc` closes an
overlay or returns to the composer. An approval never steals focus; a badge
announces it, and the right card is answered with `y` / `n` / `d` / `a`.

**Approvals.** Inline right card, non-blocking. No modal.

**State.** One `AppState` signal (connection, sessions, active, transcript,
pending approval, diff, left/right tabs, settings, palette, status). The daemon
client lives on the Tauri Rust side: commands (`attach`, `prompt`, `approve`,
`undo`, `interrupt`, `list_sessions`) and `orexis://event` for `ServerMsg`. No
HTTP server; the webview never talks to the network.

**Evaluation.** Two formative cycles: wireframes with 3 developers, then a
clickable Tauri shell with 5. The usability spec in
`docs/orexis/prototypes/desktop-usability.md` is the M2 acceptance table.

## Consequences

- The desktop and TUI share the protocol and the daemon, not the rendering.
- The layout is task-driven; a new panel must earn a tier.
- Approvals stay visible without blocking the composer.
- The M2 gate is measured, not asserted.
- A future multi-window mode is a separate ADR.

## Alternatives considered

- **Transcript-primary with overlays** — rejected: approvals and diffs would
  cover the conversation during the task that needs both.
- **Six equal tabs** — rejected: it makes every panel equally prominent and
  hides the task hierarchy.
- **Blocking approval modal** — rejected: it interrupts reading and typing, and
  the TUI already proved a non-blocking card works.
- **One evaluation cycle** — rejected: layout mistakes would be found after
  implementation.
