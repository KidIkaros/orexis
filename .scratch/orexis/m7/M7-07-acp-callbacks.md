# M7-07 — ACP fs and permission callbacks

- **Blocked by:** M7-06
- **Blocks:** M7-11
- **Status:** open

## Goal

Client callbacks served by Orexis's own surfaces, through the gate.

## Acceptance

- [ ] `fs/read_text_file` / `fs/write_text_file` served by the workspace
      adapter through the permission gate — one fs path (ADR-0017 §5).
- [ ] Approval requests map to `session/request_permission`; the editor's
      answer resolves the approval hub; denial produces a denied observation.
- [ ] Fixture: permission round-trip and fs round-trip over recorded
      transcripts.

## Notes

The client is untrusted for fs *content* the same as any tool output.
