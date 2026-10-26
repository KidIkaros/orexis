# M7-03 — MCP stdio client end-to-end

- **Blocked by:** M7-02
- **Blocks:** M7-04, M7-09
- **Status:** open

## Goal

The stdio shell: spawn a server process, frame ndjson over its stdin/stdout,
and drive the core from a client use case (connect, list, call, read).

## Acceptance

- [ ] Server lifecycle: spawn, handshake, shutdown, kill-on-drop; stderr is
      captured and capped, never interleaved with the wire.
- [ ] Scripted-peer conformance fixture (a fake server process or an
      in-memory transport) runs the full loop.
- [ ] Transport failure surfaces as an error event, never a hang.
- [ ] Manual: one real stdio MCP server listed and called.

## Notes

Child process ownership follows the sandbox adapter's process discipline.
