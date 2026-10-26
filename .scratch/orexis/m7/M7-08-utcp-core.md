# M7-08 — UTCP sans-io core

- **Blocked by:** —
- **Blocks:** M7-09
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

Port Kindred `utcp.rs` as a sans-io core in `orexis-adapters-protocol-utcp`:
provider manuals, call templates, `search_tools`/`find_tool`.

## Acceptance

- [x] Manual parsing: providers with transports, auth, call templates;
      malformed manual is an error.
- [x] `search_tools`/`find_tool` over registered manuals.
- [x] Call template substitution (path, query, body, auth) is pure and
      fixture-tested.
- [x] Ported with its tests from Kindred (one-way ledger).

## Notes

The core stops at *instructions* — the transport shells (M7-09) execute
them:

- `Provider` (name, type cli/http/mcp, endpoint, command, command_args,
  auth_token) with structural validation (reachable endpoint/command, no
  '/' in the name — it would break the tool namespace).
- `parse_providers` accepts an array or `{providers: []}`; malformed or
  structurally invalid manuals are errors.
- `UtcpCore`: `load_providers` (append), `discover_instructions` (HTTP GET
  `{endpoint}/tools` + Bearer; CLI `--list-tools`; MCP command),
  `record_discovery` (replaces that provider's entries), `search_tools`
  (name matches rank before description matches, case-insensitive),
  `find_tool` (exact), `call_instruction` (HTTP POST `{endpoint}/tools/
  {tool}` + Bearer + body; CLI argv + tool + args-JSON; MCP command + tool
  + args).

Kindred's I/O transports (blocking std::process, `block_in_place` HTTP)
are rejected per the ledger; their *shapes* become the instruction enums.
19 tests ported/extended. 84 workspace suites green.
