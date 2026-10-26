# M7-11 — M7 gate

- **Blocked by:** M7-03 … M7-10
- **Blocks:** —
- **Status:** open

## Goal

`scripts/m7-gate.sh` proves the milestone's automated half; the manual half
is one real peer per protocol.

## Acceptance

- [ ] fmt, clippy `-D warnings`, workspace tests, boundaries.
- [ ] Protocol suites: jsonrpc, mcp, acp, utcp, a2a.
- [ ] Conformance fixture inventory: every protocol has recorded-transcript
      fixtures; none requires a live peer.
- [ ] Manual checklist printed: real stdio MCP server; ACP client driving
      `orexis acp`; real HTTP UTCP manual; real A2A sample agent.

## Notes

Cut rule applies: two missed gates make M6 the release candidate.
