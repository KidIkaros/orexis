# M2-05 — Artifacts and file browser

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** in progress (devin, 2026-10-26) — files/artifacts wired; session-log artifacts remain

## Goal

Browse files and artifacts inside the workspace without a separate editor.

## Acceptance

- [x] Workspace browsing is rooted at the project; traversal, absolute-outside, and symlink escapes use the existing sandbox. Tests cover escapes.
- [x] Preview with line numbers; 64 KB cap with a truncated marker.
- [x] Changes list modified files and the latest checkpoint diff; files open, diffs preview. *(Session-log artifacts and a durable Artifact store are M8 per ADR-0012.)*
- [x] `1`–`5` rail tabs; rows tabbable; file filter input.
