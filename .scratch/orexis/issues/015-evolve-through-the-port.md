# 015 — Serve `/evolve` through the `EvolutionStore` port

- **Type:** feature
- **Blocks:** —
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

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

## Resolution

All three findings fixed. `DaemonConfig` holds
`Option<Arc<dyn EvolutionStore>>`; the handler calls `export_jsonl()` and
keeps the last 500 lines; a ledger read error is a startup-visible
warning, not a silent empty overlay. Composition constructs the corridor
only when `.orexis/evolution` exists (ADR-0015: evolution is off by
default, so a fresh install wires nothing and gains no side effects),
with the signing seed from `OREXIS_EVOLUTION_KEY` (hex, ADR-0009 env
fallback); a corridor without a key is a visible warning. Fake mode
passes `None` and serves "no evolution ledger" as before. `env.rs` now
computes `cwd`/`project_root` only in the real branch.

## Landed

- `composition`: `Runtime.evolution`, `BuildConfig.evolution_key`,
  `evolution_source()`; `orexis-adapters-evolution-git` moved from
  dev-dep to dep (composition constructs adapters).
- `adapters-evolution-git`: `Ed25519Signer::from_hex` — the adapter owns
  its key format.
- `daemon`: `DaemonConfig.evolution`; handler through the port, last 500;
  `env.rs` restructured (no `expect` on construction invariants).
- Tests: `evolution_returns_the_ledger` (through a real corridor),
  `evolution_shows_the_most_recent_entries` (600 lines → last 500),
  `an_existing_corridor_is_wired_through_the_port` (no corridor → None;
  corridor without key → warning; corridor with key → wired).
- `docs/orexis/seams.md`: the composition → evolution-git edge.

## Follow-ups

Evolution key provisioning (keyring entry, an `evolution init` flow) is
fog — the env fallback is the ADR-0009 floor, not the full story.
