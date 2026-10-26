# M7-04 — MCP tools in the tool surface

- **Blocked by:** M7-03
- **Blocks:** M7-11
- **Status:** open

## Goal

Discovered MCP tools become first-class tools in the existing surface
(ADR-0017 §3).

## Acceptance

- [ ] Namespacing `mcp/<server>/<tool>`; no collision with built-ins.
- [ ] Servers connect at session start; specs register into the session; a
      failed server is a visible session event.
- [ ] Calls pass the permission gate; default `ask`; ruleset may allow per
      server or per tool; the floor survives YOLO.
- [ ] Tool output is delimited and capped (invariant 22).
- [ ] Server tool descriptions are delimited when injected into the prompt.
- [ ] Failing test first: an MCP tool call without a ruleset entry asks.

## Notes

No new permission surface; the M3 ruleset gains external-tool entries.
