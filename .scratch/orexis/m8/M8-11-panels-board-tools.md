# M8-11 — Real fleet/board panels and the model's board tools

- **Blocked by:** M8-09
- **Blocks:** M8-10
- **Status:** open

## Goal

The oversight surface for team mode (ADR-0012's contract): the
TUI/desktop fleet, board, and artifacts panels read the real domain when
team mode is on (live workers, board tasks, blackboard artifacts),
projections when off. The model's write surface for the board: tools
(`board_add`, `board_list`, …) so the plan strategy can produce a task
breakdown — through the same permission gate.

## Acceptance

- [ ] The fleet panel shows live workers (id, strategy, task, status).
- [ ] The board panel shows real tasks with statuses and edges.
- [ ] The artifacts panel reads the blackboard.
- [ ] The model's board tools exist, pass the gate, and are capped.
