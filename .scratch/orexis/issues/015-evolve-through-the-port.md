# 015 — Serve `/evolve` through the `EvolutionStore` port

- **Type:** feature
- **Blocks:** —
- **Status:** open
- **Claimed by:** —

## Question

The daemon's `evolution()` handler reads the ledger JSONL directly from a
path hardcoded in `serve.rs` (`.orexis/evolution/ledger.jsonl`), duplicating
path/format knowledge the `GitCorridor` adapter owns. The `EvolutionStore`
port already has `entries()` and `export_jsonl()` — the handler bypasses
both. If the corridor's layout changes, the overlay silently shows "no
evolution ledger".

## Findings (clean-code review, 2026-10-26)

1. **Port bypass** — `handlers/session.rs::evolution` uses
   `std::fs::read_to_string` on a path handed in via `DaemonConfig`, while
   the port method exists. Fix: `DaemonConfig.evolution_ledger:
   Option<PathBuf>` becomes `Option<Arc<dyn EvolutionStore>>`, constructed
   by composition (rule 3), and the handler calls `export_jsonl()`.
2. **Oldest-not-newest** — `.take(500)` keeps the first 500 lines; once the
   ledger exceeds 500 entries, new entries never appear. Fix: keep the last
   500.
3. **Unused work in fake mode** — `serve.rs` computes `cwd`/`project_root`
   unconditionally; fake mode uses neither, and `current_dir()?` becomes a
   new failure path. Fix: compute inside the real branch.

## Acceptance

- [ ] Daemon holds `Option<Arc<dyn EvolutionStore>>`; no ledger path in
      `serve.rs`.
- [ ] Overlay shows the most recent entries (last 500).
- [ ] Fake runtime serves "no evolution ledger" as today.
- [ ] `m6-gate.sh` green.

## Notes

Found by the clean-code review pass before M7; not a regression — the
surface is read-only and correct for small ledgers.
