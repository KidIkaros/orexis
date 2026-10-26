# M7-05 — MCP streamable HTTP transport

- **Blocked by:** M7-02
- **Blocks:** M7-11
- **Status:** open

## Goal

The streamable HTTP shell over `orexis-adapters-http`: POST JSON-RPC, parse
`application/json` and SSE responses, session id header handling.

## Acceptance

- [ ] JSON and SSE responses both parse through the same core.
- [ ] Server-sent requests inside an SSE stream are answered.
- [ ] Reconnect/expire semantics: a lost session is an error event, not a
      silent re-initialize.
- [ ] Conformance fixture with a recorded HTTP transcript.

## Notes

Auth follows the profile's auth kind; unimplemented auth kinds error per
ADR-0016.
