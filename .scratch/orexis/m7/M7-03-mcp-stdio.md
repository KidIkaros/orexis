# M7-03 — MCP stdio client end-to-end

- **Blocked by:** M7-02
- **Blocks:** M7-04, M7-09
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

The stdio shell: spawn a server process, frame ndjson over its stdin/stdout,
and drive the core from a client use case (connect, list, call, read).

## Acceptance

- [x] Server lifecycle: spawn, handshake, shutdown, kill-on-drop; stderr is
      captured and capped, never interleaved with the wire.
- [x] Scripted-peer conformance fixture (a fake server process or an
      in-memory transport) runs the full loop.
- [x] Transport failure surfaces as an error event, never a hang.
- [ ] Manual: one real stdio MCP server listed and called.

## Notes

`McpStdioClient` owns the child (`kill_on_drop`), drives `McpCore`, and
services one round trip at a time. While waiting for a correlated response,
a server-initiated request is refused immediately (-32601) so the wire keeps
flowing; EOF and timeouts are `McpError::Transport`/`Timeout`, never hangs.
Server stderr is captured into a 64KB tail buffer (char-boundary safe, same
cap as the tools adapter) via a background task.

`mock_mcp_server` (the mock_lsp pattern) is the scripted peer: scenarios
`notify-first`, `server-request` (sends sampling/createMessage and waits for
the refusal), `stderr`. Six process-level fixtures cover the full loop,
spawn failure, timeout, stderr capture, interleaved notifications, and the
refusal round trip.

Manual (not verified here): one real stdio MCP server listed and called.
