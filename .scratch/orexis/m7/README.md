# M7 — Protocols

All tickets resolved (2026-10-26); `scripts/m7-gate.sh` green. Manual
remaining: one real peer per protocol (see M7-11).

| Ticket | Goal | Blocked by |
|---|---|---|
| [M7-01](M7-01-jsonrpc.md) | `adapters-jsonrpc`: envelope, framing, correlation | — |
| [M7-02](M7-02-mcp-core.md) | MCP sans-io core + fixtures | M7-01 |
| [M7-03](M7-03-mcp-stdio.md) | MCP stdio client end-to-end | M7-02 |
| [M7-04](M7-04-mcp-tools.md) | MCP tools in the tool surface | M7-03 |
| [M7-05](M7-05-mcp-http.md) | MCP streamable HTTP transport | M7-02 |
| [M7-06](M7-06-acp-server.md) | ACP server: dispatch, bin, session mapping | M7-01 |
| [M7-07](M7-07-acp-callbacks.md) | ACP fs and permission callbacks | M7-06 |
| [M7-08](M7-08-utcp-core.md) | UTCP sans-io core | — |
| [M7-09](M7-09-utcp-transports.md) | UTCP transports: HTTP, CLI, MCP | M7-03, M7-08 |
| [M7-10](M7-10-a2a-client.md) | A2A client | M7-01 |
| [M7-11](M7-11-gate.md) | M7 gate | M7-03 … M7-10 |

Parallel tracks: (M7-01) → MCP and ACP and A2A; UTCP core is independent
until transports need the MCP client.
