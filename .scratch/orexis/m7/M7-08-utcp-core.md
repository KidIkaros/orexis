# M7-08 — UTCP sans-io core

- **Blocked by:** —
- **Blocks:** M7-09
- **Status:** open

## Goal

Port Kindred `utcp.rs` as a sans-io core in `orexis-adapters-protocol-utcp`:
provider manuals, call templates, `search_tools`/`find_tool`.

## Acceptance

- [ ] Manual parsing: providers with transports, auth, call templates;
      malformed manual is an error.
- [ ] `search_tools`/`find_tool` over registered manuals.
- [ ] Call template substitution (path, query, body, auth) is pure and
      fixture-tested.
- [ ] Ported with its tests from Kindred (one-way ledger).

## Notes

No I/O in the core; transports land in M7-09.
