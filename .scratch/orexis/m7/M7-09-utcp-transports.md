# M7-09 — UTCP transports: HTTP, CLI, MCP

- **Blocked by:** M7-03, M7-08
- **Blocks:** M7-11
- **Status:** resolved
- **Claimed by:** devin, 2026-10-26

## Goal

Transport shells for the UTCP core (ADR-0017 §4).

## Acceptance

- [x] HTTP transport over `orexis-adapters-http`; auth per manual.
- [x] CLI transport executes through the sandboxed executor — never a
      raw process spawn; output delimited and capped.
- [x] MCP transport composes the M7-03 client.
- [x] Tool names namespaced `utcp/<provider>/<tool>`; same gate, delimiting,
      and caps as MCP tools.
- [ ] Manual: one real HTTP manual called end-to-end. (moved to M7-11 gate)

## Landed

- `adapters-http`: `Transport` gained plain `get`/`post` (Reqwest override;
  MockTransport scripts bodies, records method/url/headers).
- ports: `ExternalTools::kind()` (namespace stem for fanout + failure
  labels); `CommandRunner`/`CommandOutput` (argv-exact sandboxed run);
  `PeerFactory`/`PeerSession`/`PeerOutput` (peer tool servers by argv).
- `adapters-tools`: `CommandRunner` on `LocalTools` sharing `run_shell`'s
  sandbox (env scrub, cwd jail, process group, 64KB caps) — argv-exact so
  model-supplied args never touch a shell string. `NamespaceTools` fans
  `mcp/`/`utcp/` out to the right source behind `RouterTools`.
- `protocol-mcp`: `StdioPeerFactory` + `PeerSession for McpStdioClient`
  (composition injects; no adapter→adapter dep).
- `protocol-utcp`: `UtcpTools` (ExternalTools) — discovery/call for all
  three instruction kinds, `{success,content}` or raw-stdout CLI results,
  `<utcp_result>` delimiting with breakout escaping, 64KB cap, 500-char
  descriptions; `load_manuals` mirrors the MCP config loader.
- Core refinement: `call_instruction(provider, tool, args)` is
  provider-scoped — namespaced names pin the provider; `register()` added.
- daemon/composition generalized: `mcp: Option<ExternalTools>` →
  `external: Vec<ExternalTools>`; `ensure_external` connects all sources,
  failures surface as session events, specs union into the turn context.
- Tests: 30 utcp suite incl. real-process peer factory (mock_mcp_server),
  CommandRunner sandbox fixtures, namespace fanout, daemon E2E routing a
  `utcp/` call through the gate → router → fanout.

## Notes

Same integration rules as M7-04; no second policy system.
