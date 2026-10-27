# 016 — Delimit built-in tool output too

- **Type:** feature
- **Blocks:** —
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

M7-04 implemented untrusted-content delimiting for external tools (mcp/,
utcp/) at the executor boundary. Built-in tool output (read_file, search,
shell, list_dir) still enters the model context raw — capped and
hardline-filtered, but not delimited. A repo file containing
"ignore previous instructions" is the same injection class as hostile MCP
output (AGENTS.md security: "tool output … is delimited").

## Evidence

- AGENTS.md security section and PLAN invariant 22: tool output is
  untrusted content, generally.
- M4 adversarial fixtures cover caps and hardline commands, not delimiting.
- M7-04's `<mcp_result>` wrapper is the working pattern, including
  delimiter-breakout escaping.

## Resolution

Built-in tool output is delimited at the executor boundary, in
`LocalTools::execute` — the same layer that owns the external wrappers.
Every built-in observation enters the model context as
`<tool_result name="…">…</tool_result>`, with `</tool_result` inside the
output escaped to `<\/tool_result` so a repo file cannot close the region
early. The daemon's untrusted-content note is now unconditional and
covers all tool output (a repo file is as capable of carrying injected
instructions as a hostile MCP server), instead of appearing only when
external tools are configured.

Replay stays byte-identical: the delimiter is inside the logged
`Observation` event, and `verify_replay` hashes only event-derived
messages. `SessionEvent::Error` is not model-visible, so verify output
(never an Observation) needs no wrapper — the executor boundary covers
every model-visible tool surface.

## Landed

- `adapters-tools/src/lib.rs`: `delimit()` + wrap in `LocalTools::execute`.
- `daemon/handlers/session.rs`: the untrusted-content note is
  unconditional and generalized to all tool output.
- Tests: delimiting + breakout fixtures in `tests/tools.rs` and
  `tests/adversarial.rs` (`injected_instructions_render_delimited`); the
  one exact-equality assertion updated to the delimited form.

## Follow-ups

None opened. The TUI renders observations verbatim, so built-in rows now
show the `<tool_result>` wrapper — consistent with how external tool
rows already render; eliding wrappers in the TUI is a display polish
question for the M2 evaluation cycles, not a correctness one.
