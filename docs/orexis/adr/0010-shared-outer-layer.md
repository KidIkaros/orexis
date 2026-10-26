# ADR-0010 — Shared outer-layer crates

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin

## Context

Two crates now sit outside the original graph:

- `orexis-client` — the synchronous Unix-socket `DaemonClient`, extracted from
  the TUI so the desktop can reuse it.
- `orexis-desktop-core` — the desktop `AppState` and `DesktopBridge`, testable
  without Tauri.

Both are outer-layer code. `orexis-client` performs I/O; `orexis-desktop-core`
holds UI state and a daemon bridge. Neither is a port or an adapter.

## Decision

- `orexis-client` is a **shared I/O client**: it may depend on
  `orexis-protocol` and `orexis-core`; adapters and apps may depend on it;
  nothing inward (`core`, `ports`, `usecases`) may.
- `orexis-desktop-core` is an **app-layer state/bridge**: it may depend on
  `orexis-client`, `orexis-protocol`, and `orexis-core`; nothing inward may
  depend on it.
- Neither crate constructs provider/tool/session adapters.
- Import-lint checks that `core`, `ports`, and `usecases` do not depend on
  either crate.

## Consequences

- The TUI and desktop share one daemon client; no duplicated framing.
- The desktop state is testable without Tauri.
- The Dependency Rule still holds: both are outer ring, never imported inward.
- A future CLI client follows the same rule.
