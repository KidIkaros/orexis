# 019 — `orexis` binary consolidation

- **Type:** feature
- **Blocks:** M9, 020, 021, 014
- **Status:** open
- **Claimed by:** —

## Question

The workspace ships separate binaries today (`serve`, `acp` in the daemon
crate; the TUI as a library driven by a client). M9's packaging (ADR-0019)
needs one `orexis` binary with subcommands. Which subcommands, how the
clap-free argument surface is structured (the workspace uses no CLI
parser today), and what happens to the old entry points?

## Evidence

- ADR-0019: one `orexis` binary; the M7 spec deferred consolidation
  ("`acp` joins `serve` in the daemon crate until the `orexis` binary
  consolidation").
- 1.0 checklist: `orexis init`, `export --all` / `import`, `providers`.
- PLAN §4.1: `orexis` is the binary crate: cli / serve / attach / bench.

## Acceptance

- [ ] One `orexis` binary; `serve`, `attach`, `acp`, `tui`, `bench`,
      `init`, `providers`, `export`, `import`, `upgrade` subcommands (the
      last four may stub with a "not yet" error until their tickets land).
- [ ] No new dependency unless unavoidable; the argument surface is
      hand-rolled or a minimal parser.
- [ ] The old entry points keep working (compat shims or documented
      removal).
- [ ] The gates still pass from the consolidated binary.

## Notes

`init` and `providers` are where ticket 014's auth-kind visibility lands.
