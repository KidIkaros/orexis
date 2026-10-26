# Desktop usability — process audit and specification

Source: `/developing-user-interfaces` (Hix & Hartson), process-audit and
usability-spec passes. Scope: the M2 desktop, not the TUI.

## Process audit

Usability = effectiveness + efficiency + satisfaction, produced by the process.

| Step | Status | Consequence | Fix |
|---|---|---|---|
| **Analysis first** (users, tasks, context) | Missing for desktop | Six feature-shaped panels become six places to look | Task analysis in ticket 006: one user class, three oversight tasks |
| **Designs representable** (UAN-style) | Partial | TUI has ADR-0004 + prototype; desktop has only a question | Wireframes, focus model, keymap, action → feedback table |
| **Usability specified measurably** | Missing | "Usable" is opinion; no M2 acceptance | Spec table below, adopted into M2 |
| **Prototypes exist** (fidelity ladder) | Partial | Desktop decisions would be made in code | Wireframe → clickable Tauri shell → evaluated build |
| **The loop runs** (evaluate → redesign) | Partial | Decisions are taste, not evidence | Two cycles: wireframe (3), clickable shell (5) |
| **Organizational backing** | Missing/partial | Evaluation is dropped under pressure | Both cycles scheduled in M2; owner is the usability engineer |

## User class and tasks

**User class:** a developer working on a local repository, comfortable in a
terminal, using the desktop for oversight rather than typing.

| Task | Steps |
|---|---|
| **Steer** | Start a session, send a prompt, approve a shell command, review the diff |
| **Recover** | A failed verify: read the error, retry or skip |
| **Inspect** | Find a worker and a task in fleet/board, inspect the entity, return to the session |

**Context:** local repo, provider configured, one user, keyboard-first, no web.

## Usability specification

| Attribute | Instrument | Worst acceptable | Planned target |
|---|---|---|---|
| Core task completion (steer) | Task script + observation | 4/5 | 5/5 |
| Time to first reviewed diff | Screen/keystroke log | ≤ 5 min | ≤ 3 min |
| Approval comprehension (what + reversible) | Think-aloud + post-approval question | 4/5 | 5/5 |
| Keyboard-only completion | Input log | 5/5 | 5/5 |
| Error recovery (failed verify) | Task script | 4/5 | 5/5 |
| Panel findability (fleet, board, self) | Task timing, < 10 s each | 4/5 | 5/5 |
| Satisfaction (1–7) | Post-task questionnaire | median 5 | median 6 |

**Participants:** 3–5 per cycle; two cycles before the M2 gate.

## Evaluation plan

1. **Cycle 1 — wireframes.** Analytic heuristic pass + 3 developers,
   think-aloud on the three tasks. Findings → layout decisions.
2. **Cycle 2 — clickable Tauri shell.** 5 developers, the three tasks,
   metrics against the spec. Findings → M2 redesign.
3. **M2 gate.** Spec rows met at the planned target, or the miss is recorded
   with a redesign decision.

## Tells to avoid

- "We'll usability-test at the end" — evaluation is inside every cycle.
- "The six panels are the spec" — panels are a means; tasks are the unit.
- "Keyboard-first is obvious" — measure it; it is a 5/5 requirement.
