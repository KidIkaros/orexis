# M7-05 — MCP streamable HTTP transport

- **Blocked by:** M7-02
- **Blocks:** M7-11
- **Status:** resolved
- **Claimed by:** devin, 2026-10-26

## Goal

The streamable HTTP shell over `orexis-adapters-http`: POST JSON-RPC, parse
`application/json` and SSE responses, session id header handling.

## Acceptance

- [x] JSON and SSE responses both parse through the same core.
- [x] Server-sent requests inside an SSE stream are answered.
- [x] Reconnect/expire semantics: a lost session is an error event, not a
      silent re-initialize.
- [x] Conformance fixture with a recorded HTTP transcript.
- [ ] Manual: one real HTTP MCP server end-to-end. (moved to M7-11 gate)

## Landed

- `adapters-http`: response-aware `post_response`/`get_response` — status
  and headers travel with the body, so a shell decides what a 404 means
  (missing card, expired session) before treating the body as an error.
  `MockResponse` scripts full responses (status, headers, body).
- `adapters-jsonrpc`: `SseDecoder` — events, multi-line data, chunk
  boundaries, comments, CR tolerance.
- `protocol-mcp/src/http.rs`: `McpHttpClient` adapts streamable HTTP onto
  the sans-io `McpCore`. `Mcp-Session-Id` absorbed at initialization and
  carried on later requests; `mcp-protocol-version` recorded the moment
  the initialize result arrives. A 404 on a session-carrying request is
  `McpError::SessionExpired` — the caller decides whether to start over.
- `McpSource` enum: a server config with `url` is HTTP, with `command` is
  stdio; `load_server_configs` parses both; composition wires
  `ReqwestTransport`.
- Tests: 7 HTTP fixtures (handshake + session + version headers, tool call
  round trip, SSE through the same core, server request answered, expired
  session, HTTP error status), plus config parsing.

## Notes

Auth follows the profile's auth kind; unimplemented auth kinds error per
ADR-0016.
