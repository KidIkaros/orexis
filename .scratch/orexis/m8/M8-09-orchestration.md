# M8-09 — The team orchestration loop + daemon wiring + real panels

- **Blocked by:** M8-03, M8-04, M8-08
- **Blocks:** M8-10
- **Status:** open

## Goal

The team loop (ADR-0020): board → scheduler → workers → bus/blackboard,
composed in the daemon (ADR-0006); opt-in via `OREXIS_TEAM=1` or
settings; the TUI/desktop fleet, board, and artifacts panels read the
real domain when team mode is on (ADR-0012's contract), projections when
off. Worker output consumed by other workers is delimited and capped
(invariant 22).

## Acceptance

- [ ] Team mode off: today's single-agent behavior, unchanged.
- [ ] Team mode on: tasks flow board → workers → done; the fleet panel
      shows live workers.
- [ ] The entity sees worker outcomes as session outcomes.
- [ ] Composition constructs the adapters (rule 3); the daemon wires the
      loop.
