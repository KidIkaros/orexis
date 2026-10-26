# M7-02 — MCP sans-io core

- **Blocked by:** M7-01
- **Blocks:** M7-03, M7-05
- **Status:** open

## Goal

Port Kindred `mcp.rs` as a sans-io core in `orexis-adapters-protocol-mcp`:
initialize handshake, tools/list, tools/call, resources/read, prompts/get,
correlation over an injected transport.

## Acceptance

- [ ] `initialize` handshake with capability exchange.
- [ ] `tools/list` and `tools/call` with structured content and errors.
- [ ] `resources/read` and `prompts/get`.
- [ ] Server-initiated requests and notifications handled without deadlock.
- [ ] Conformance fixtures from recorded MCP transcripts; sans-io (no I/O in
      the core).
- [ ] Ported with its tests from Kindred (one-way ledger, LICENSE owned).

## Notes

The core never spawns a process or opens a socket; shells do (M7-03, M7-05).
