# M7-01 — `orexis-adapters-jsonrpc`

- **Blocked by:** —
- **Blocks:** M7-02, M7-06, M7-10
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

The JSON-RPC 2.0 envelope shared by MCP, ACP, and A2A, as shared adapter
infrastructure (ADR-0005 rules, ADR-0017 §2).

## Acceptance

- [x] Envelope types: request, response, error, notification; serde round-trip.
- [x] ndjson framing helpers: bytes → messages, messages → bytes; malformed
      line is an error, not a panic.
- [x] Request/response correlation table (id → pending), sync, no I/O.
- [x] Import-lint allows `adapters-* -> adapters-jsonrpc`; still forbids
      `core`/`ports`/`usecases` -> `adapters-*`.
- [x] Fixtures: recorded transcripts; no live peers.

## Notes

Pure data and pure functions only. No domain types (ADR-0005 rule 3) — the
crate depends only on `serde_json` and `thiserror`, not on `orexis-core`.

Parse rules: strict `jsonrpc: "2.0"`; method + absent/null id →
notification; method + id → request; result/error + id → response; null id
allowed only on error responses (unmatchable in correlation); fractional ids
rejected; batches (arrays) rejected — no corpus protocol uses them.

Correlation: `insert` returns `false` on a duplicate pending id and keeps
the first; `take` removes. 24 tests, all recorded-transcript fixtures.
