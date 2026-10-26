# M7-04 — MCP tools in the tool surface

- **Blocked by:** M7-03
- **Blocks:** M7-11
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

Discovered MCP tools become first-class tools in the existing surface
(ADR-0017 §3).

## Acceptance

- [x] Namespacing `mcp/<server>/<tool>`; no collision with built-ins.
- [x] Servers connect at session start; specs register into the session; a
      failed server is a visible session event.
- [x] Calls pass the permission gate; default `ask`; ruleset may allow per
      server or per tool; the floor survives YOLO.
- [x] Tool output is delimited and capped (invariant 22).
- [x] Server tool descriptions are delimited when injected into the prompt.
- [x] Failing test first: an MCP tool call without a ruleset entry asks.

## Notes

Five units:

1. **core** — the policy matcher gained namespace prefix rules
   (`mcp/fs/*` matches `mcp/fs/…`, not `mcp/fs-other/…`; deny still wins
   among rules); `is_external_tool` in core; plan mode denies external
   tools regardless of rules.
2. **adapters-tools** — `RouterTools` dispatches `mcp/`/`utcp/` names to
   the external arm, everything else to the built-in arm.
3. **adapters-protocol-mcp** — `McpTools` (connect once, paginated specs
   with 500-char descriptions, execute with `<mcp_result>` delimiting,
   delimiter-breakout escaping, and the 64KB cap); `load_server_configs`
   merges dirs with project-wins and reports problems.
4. **ports** — new `ExternalTools` port (connect_all + specs, supertrait of
   `ToolExecutor`); the daemon sees only the port; composition constructs
   the concrete source (ADR-0006).
5. **daemon** — `McpState` per session; `attach` connects once at session
   start (failures appended as session `Error` events before replay); the
   prompt handler extends `context_tools` with connected specs, routes
   through `RouterTools` under the gate, and appends the untrusted-data
   framing line to the system prompt when external tools exist.

Delimiting for structured fields (tool descriptions inside a JSON tool
definition) is the framing line plus the description cap — a block
delimiter cannot wrap a schema field.

Tests: core policy (5 new), router (4), McpTools fixtures with the mock
server (6), composition wiring (2), daemon E2E (2: specs register into the
session; a failed server is a visible session event). 78 workspace suites
green.
