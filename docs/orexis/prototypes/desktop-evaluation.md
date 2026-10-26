# Desktop evaluation protocol — M2

Status: **not run**. The wireframe cycle needs participants; the clickable
shell cycle needs a Tauri build, which this environment cannot produce
(webkit2gtk/gtk/trunk/tauri-cli are absent). Nothing below is a result.

## Cycle 1 — wireframes (3 developers)

- Materials: `desktop-layout.md`, the three-zone wireframes.
- Tasks: steer (prompt → approve → review diff), recover (failed verify →
  read error → retry or skip), inspect (find a worker/task, open its session).
- Method: think-aloud, 30–40 minutes each.
- Record: task completion, approval comprehension, panel findability,
  qualitative friction.

## Cycle 2 — clickable Tauri shell (5 developers)

- Materials: the built shell against a local daemon and a scratch repo.
- Tasks: the same three, plus keyboard-only completion.
- Metrics: the usability spec in `desktop-usability.md`.
- Record: time to first reviewed diff, keyboard-only completion, error
  recovery, satisfaction (1–7).

## Decision log

| Finding | Decision | Ticket |
|---|---|---|
| — | — | — |

A miss is recorded with its attribute, the observed value, and the redesign
decision. No row is closed without one of those.
