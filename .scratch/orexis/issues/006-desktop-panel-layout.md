# 006 — Desktop panel layout in Leptos

- **Type:** prototype
- **Blocks:** M2
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Question

What is the Leptos layout and state model for the six panels — chat/streaming, fleet, approvals, diff viewer, artifacts, file browser — over the shared daemon socket?

## Evidence

- Plan §8.3: six panels; settings for model/provider, persona, permissions, privacy.
- Plan §8.1: desktop attaches over the same Unix socket through its Rust side; no HTTP server.
- opencode: client/server with generated contracts.
- Hermes desktop: Bot Mode, slash palette.

## Prototype

- `docs/orexis/prototypes/desktop-layout.md` — wireframes at 1280×800 and
  1920×1080, panel hierarchy, focus model, states, UAN table, Leptos state model.
- `docs/orexis/prototypes/desktop-usability.md` — process audit, usability spec,
  evaluation plan.
- `docs/orexis/specs/M2.md` — scope, usability acceptance, tickets.

## Proposed resolution

- **Three-zone layout:** left rail (sessions/board/fleet/artifacts/files),
  center transcript + composer, right contextual panel (approvals/diff/preview).
- **Task-driven panel model:** the six named panels map to the zones; they are
  not six equal tabs.
- **Composer-first focus:** approvals badge, never steal focus.
- **Tauri Rust bridge:** commands + `orexis://event`; no HTTP.
- **Two evaluation cycles:** wireframes (3), clickable shell (5).

## Decisions

| Question | Decision |
|---|---|
| Layout | Three-zone: left rail, center transcript, right contextual panel |
| Panel model | Task-driven: chat center, approvals/diff right, fleet/artifacts/files left |
| Approvals | Inline right card, non-blocking; badge announces |
| Evaluation | Two cycles: wireframes (3), clickable shell (5) |

## Resolution

The desktop is a three-zone oversight workspace over the shared daemon:
composer-first focus, task-driven panel mapping, non-blocking approval card,
and a Rust-side Tauri bridge with commands + `orexis://event` (no HTTP). The
layout and state model are in `docs/orexis/prototypes/desktop-layout.md`; the
usability spec and evaluation plan are in `desktop-usability.md`. ADR-0008
records the contract. M2-01 through M2-06 implement it.

## Follow-ups

- M2-01 … M2-06 (`.scratch/orexis/m2/`).
- A multi-window mode, if ever, is a separate ADR.

## Follow-ups

<new tickets or fog this resolution creates>
