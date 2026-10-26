# M3-11 — Split adapters-tools

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

Split `adapters-tools/src/lib.rs` (964 lines) into fs, search, shell, git, and gate modules, each under 300 lines.

## Acceptance

- [ ] `lib.rs` is the struct, constructor, specs, resolve, and execute only.
- [ ] `fs.rs`, `search.rs`, `shell.rs`, `git.rs`, `gate.rs` own their methods.
- [ ] Public API unchanged; all tests stay green.
- [ ] No behaviour change.
