# M7-06 — ACP server: dispatch, stdio bin, session mapping

- **Blocked by:** M7-01
- **Blocks:** M7-07, M7-11
- **Status:** open

## Goal

`orexis acp` (daemon-crate bin, wired through composition) speaks ACP over
stdio; the sans-io dispatch maps ACP methods onto the session loop
(ADR-0017 §5).

## Acceptance

- [ ] `initialize` → capabilities; `session/new` → session; `session/prompt`
      → the turn loop; `session/cancel` → interrupt.
- [ ] `session/update` notifications: say deltas → message chunks; tool
      calls → tool_call/tool_call_update.
- [ ] Conformance fixtures: recorded ACP transcripts against the sans-io
      dispatch.
- [ ] The bin constructs nothing itself; composition builds the runtime
      (ADR-0006).

## Notes

Stop reasons map to ACP stop reasons; budget exhaustion is a stop reason,
not an error.
