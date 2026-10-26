# M3-11 — Split adapters-tools

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Split `adapters-tools/src/lib.rs` (964 lines) into fs, search, shell, git, and gate modules, each under 300 lines.

## Acceptance

- [x] `lib.rs` is the struct, constructor, specs, resolve, and execute only (202 lines).
- [x] `files.rs` (135), `search.rs` (92), `shell.rs` (177), `git.rs` (58), `gate.rs` (107) own their methods.
- [x] Public API unchanged; all tests stay green.
- [x] No behaviour change.

## Notes

- The file module is `files.rs`, not `fs.rs`, to avoid shadowing `std::fs`.
- Unit tests moved to `tests/tools.rs` (256 lines); adversarial tests remain separate.
- `edit.rs` (267), `checkpoints.rs` (144), and `verify.rs` (73) were already focused.
