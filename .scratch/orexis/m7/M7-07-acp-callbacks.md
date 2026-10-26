# M7-07 — ACP fs and permission callbacks

- **Blocked by:** M7-06
- **Blocks:** M7-11
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

Client callbacks served by Orexis's own surfaces, through the gate.

## Acceptance

- [x] `fs/read_text_file` / `fs/write_text_file` served by the workspace
      adapter through the permission gate — one fs path (ADR-0017 §5).
- [x] Approval requests map to `session/request_permission`; the editor's
      answer resolves the approval hub; denial produces a denied observation.
- [x] Fixture: permission round-trip and fs round-trip over recorded
      transcripts.

## Notes

**fs — one path, deliberately.** ACP's `fs/*` methods are agent→client
requests for agents without fs access. Orexis is sovereign: its tools use
the workspace directly, so the agent never sends fs callbacks. The fixture
proves the workspace path under ACP (a read_file turn completes with the
file's body, no permission round-trip, no client callback). A client that
sends `fs/*` anyway gets -32601 (defensive; the dispatch marks them
Unhandled).

**Permissions — the round-trip.** A server-level forwarder subscribes to
the approval hub: `ApprovalRequest` → `session/request_permission` (fixed
option ids: allow_once/allow_always/reject_once) with the request id
registered in a `Correlations` table. The read loop dispatches the client's
`ClientResponse` (the dispatch no longer rejects responses) →
`outcome_from` maps the option id to (yes, always) → `hub.resolve` fires
the gate's oneshot. Denial → the gate errors → the `[error]` chunk from
M7-06 makes it visible; nothing was written.

Two races closed with tests: the answer may be read before the forwarder
registers the request (bounded wait for the registration), and the
forwarder's writer clone must be reaped after abort before the run loop
hands the writer back.

Fixtures: permission allow round-trip (file written), denial (error chunk,
no file), fs-through-workspace, client-fs-unhandled, plus the M7-06 set.
82 workspace suites green.

Manual (not verified here): an ACP client approving a real permission
request (M7-11).
