# M7-09 — UTCP transports: HTTP, CLI, MCP

- **Blocked by:** M7-03, M7-08
- **Blocks:** M7-11
- **Status:** open

## Goal

Transport shells for the UTCP core (ADR-0017 §4).

## Acceptance

- [ ] HTTP transport over `orexis-adapters-http`; auth per manual.
- [ ] CLI transport executes through the sandboxed `ToolExecutor` — never a
      raw process spawn; output delimited and capped.
- [ ] MCP transport composes the M7-03 client.
- [ ] Tool names namespaced `utcp/<provider>/<tool>`; same gate, delimiting,
      and caps as MCP tools.
- [ ] Manual: one real HTTP manual called end-to-end.

## Notes

Same integration rules as M7-04; no second policy system.
