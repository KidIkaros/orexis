# M7-02 — MCP sans-io core

- **Blocked by:** M7-01
- **Blocks:** M7-03, M7-05
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

Port Kindred `mcp.rs` as a sans-io core in `orexis-adapters-protocol-mcp`:
initialize handshake, tools/list, tools/call, resources/read, prompts/get,
correlation over an injected transport.

## Acceptance

- [x] `initialize` handshake with capability exchange.
- [x] `tools/list` and `tools/call` with structured content and errors.
- [x] `resources/read` and `prompts/get`.
- [x] Server-initiated requests and notifications handled without deadlock.
- [x] Conformance fixtures from recorded MCP transcripts; sans-io (no I/O in
      the core).
- [x] Ported with its tests from Kindred (one-way ledger, LICENSE owned).

## Notes

Ported from Kindred `src/mcp.rs` (MIT). The blocking stdio client became a
caller-driven state machine (`McpCore`): Start → Initializing → Ready;
`initialize`/`list_tools`/… enqueue outgoing messages, `pop_outgoing`
drains them, `receive(line)` returns typed events.

The port fixes Kindred's interleaving bug: its blocking `request()` read
one line and assumed it was the response, so a server-initiated message
corrupted the stream. Here every response is correlated by id
(`Correlations<Pending>`); a stray response is an event, not an error.

Server-initiated requests become `ServerRequest` events; the caller answers
with `respond(id, result)` or `refuse(id)` (-32601) — the core never waits,
so interleaved traffic cannot deadlock. `ping` supported; list methods are
cursor-aware (`nextCursor`). Listed tools map onto `orexis_core::ToolSpec`
(missing inputSchema defaults to an empty object schema); MCP annotations
are not carried yet — fog until a permission use needs them.

25 tests: handshake, state gating, pagination, tool content/isError,
resources, prompts, server requests, interleaving (the Kindred regression),
stray responses, list-changed notifications, malformed lines.
