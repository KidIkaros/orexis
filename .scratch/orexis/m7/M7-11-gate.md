# M7-11 — M7 gate

- **Blocked by:** M7-03 … M7-10
- **Blocks:** —
- **Status:** resolved
- **Claimed by:** devin, 2026-10-26

## Goal

`scripts/m7-gate.sh` proves the milestone's automated half; the manual half
is one real peer per protocol.

## Acceptance

- [x] fmt, clippy `-D warnings`, workspace tests, boundaries.
- [x] Protocol suites: jsonrpc, mcp, acp, utcp, a2a.
- [x] Conformance fixture inventory: every protocol has recorded-transcript
      fixtures; none requires a live peer.
- [x] Manual checklist printed: real stdio MCP server; ACP client driving
      `orexis acp`; real HTTP UTCP manual; real A2A sample agent.

## Landed

- `scripts/m7-gate.sh`: chains the M6 gate (which runs the full automated
  baseline: fmt, clippy, workspace tests, boundaries, seam-graph diff),
  then the five protocol suites, then a fixture inventory — each protocol
  crate checked against the fixture style it actually uses (inline wire
  strings, scripted mock server, or scripted transports). Ends with the
  manual checklist, one real peer per protocol.
- `daemon/tests/restart.rs`: the per-read timeout (5s) and connect budget
  (3s) were too tight when the gate chain loads the machine — recovery
  itself takes well under a second in isolation, so the budgets proved
  machine speed, not recovery. Raised to 30s; the test still fails when
  the daemon is truly broken.

## Notes

Cut rule applies: two missed gates make M6 the release candidate.
