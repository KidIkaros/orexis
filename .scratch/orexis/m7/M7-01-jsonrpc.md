# M7-01 — `orexis-adapters-jsonrpc`

- **Blocked by:** —
- **Blocks:** M7-02, M7-06, M7-10
- **Status:** open

## Goal

The JSON-RPC 2.0 envelope shared by MCP, ACP, and A2A, as shared adapter
infrastructure (ADR-0005 rules, ADR-0017 §2).

## Acceptance

- [ ] Envelope types: request, response, error, notification; serde round-trip.
- [ ] ndjson framing helpers: bytes → messages, messages → bytes; malformed
      line is an error, not a panic.
- [ ] Request/response correlation table (id → pending), sync, no I/O.
- [ ] Import-lint allows `adapters-* -> adapters-jsonrpc`; still forbids
      `core`/`ports`/`usecases` -> `adapters-*`.
- [ ] Fixtures: recorded transcripts; no live peers.

## Notes

Pure data and pure functions only. No domain types (ADR-0005 rule 3).
