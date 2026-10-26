# M3-01 — Core tools and path sandbox

- **Blocked by:** —
- **Blocks:** M3-04, M3-05, M3-07, M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Implement `read_file`, `write_file`, `edit_file` (exact), `list_dir`, and `search` in `orexis-adapters-tools`, with every path resolved inside the project root.

## Acceptance

- [x] Read/write/edit/list/search work on a temp project.
- [x] `edit_file` rejects zero and multiple matches with a clear error.
- [x] Relative `..`, absolute-outside, and symlink escapes are rejected.
- [x] Read output truncates at 64 KB with a marker; search caps at 100 hits.
- [x] Unknown tools and malformed args return a typed error.
- [x] Composition wires `LocalTools` with the project root.
- [x] Eight tests cover each tool and every escape.

## Notes

- `LocalTools::resolve` normalizes, canonicalizes, and re-checks the root; symlinks are caught by canonicalization.
- Search skips `.git`, `target`, and `node_modules`, caps depth at 20 and file size at 1 MB.
- `LocalTools::specs()` provides the tool schemas for the codec prompt.
