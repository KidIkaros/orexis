# 019 — `orexis` binary consolidation

- **Type:** feature
- **Blocks:** M9, 020, 021, 014
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

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
- Found while sizing: **nothing launched the TUI** — `adapters-tui` is a
  library with a `run()` that no binary called; the M1 gate's manual text
  referenced a launcher that did not exist.

## Resolution

**A new `crates/orexis` binary crate** — the outermost layer, depending
on daemon, adapters-tui, client, and protocol. Hand-rolled dispatch (no
CLI parser; the subcommands are few):

- `serve` / `acp` — the daemon's subcommand bodies moved into
  `daemon::cli::{serve_main, acp_main}`; the old bins remain as compat
  shims (and keep the daemon tests' `CARGO_BIN_EXE_serve` working).
- `tui [--socket P] [SESSION]` — launches the TUI (the previously
  missing launcher).
- `attach [--socket P] [SESSION]` — a pipe client: stdin lines are
  prompts, server messages print as NDJSON; `/stop` interrupts.
  Scriptable oversight without a terminal UI.
- `init | providers | export | import | upgrade | bench` — `not yet`
  stubs (exit 2), their tickets pending (014, 020, 021).
- `--socket` defaults to `OREXIS_SOCKET` or `/tmp/orexis.sock`.

The runtime is built explicitly for the async subcommands; the TUI runs
sync on the main thread.

## Landed

- `crates/orexis` (new): dispatch, pipe client, usage, unit tests
  (routing, stubs, usage).
- `daemon::cli`: shared subcommand bodies; the bins are shims.
- `docs/orexis/seams.md`: regenerated (orexis → tui/client/daemon/
  protocol).
- `scripts/m1-gate.sh`, `scripts/m3-gate.sh`: the manual text now names
  `orexis tui`.

## Follow-ups

- Ticket 014 lands `init`/`providers` here (auth-kind visibility).
- Ticket 020 (release pipeline) packages this binary.
- Ticket 021 (`orexis upgrade`) replaces its stub.
