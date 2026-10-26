# Kindred — study

**Source:** `/home/ikaaros/Coding/Gold/kindred` · **Language:** Rust · **Size:** 78,931 LOC · **License:** MIT (SPDX headers) · **Last seen:** local checkout

**Why it is in the study:** it already implements the entity substrate — soul, drives, glial modulation, pulse, immune system, genes, encrypted memory — plus orchestration and MCP/UTCP clients. It is the largest single source of portable code.

## Known surface

| Area | Modules |
|---|---|
| Entity | `soul.rs` (`Drives`: connection, curiosity, mastery, coherence, rest), `glial.rs` (`DriveMultipliers`, `modulate_drives`), `pulse.rs` (`Pulse`, `PulseTick`, `PulseAction`), `soul_metric.rs`, `behavioral.rs`, `intent.rs` |
| Evolution | `evolve.rs`, `soul_corridor.rs`, `immune.rs` |
| Memory | `memory/`, `memory.rs`, `recall.rs`, `consolidation.rs`, `curator.rs` |
| Storage | `db.rs` (rusqlite, bundled), `crypto.rs` (ChaCha20-BLAKE3 AEAD, Argon2id, per-purpose keys) |
| Safety | `permission.rs`, `gate.rs`, `guard.rs`, `consent.rs` |
| Tools | `hands.rs`, `hands_stack.rs`, `action_transport.rs` |
| Orchestration | `orchestration.rs` (`OrchestrationMode`, `ApprovalGate`, `RiskLevel`, `OrchestrationPlan`), `tasks.rs` (planner → executor → verifier), `proposal.rs` (durable approval queue) |
| Interop | `mcp.rs` (MCP client, stdio JSON-RPC), `utcp.rs` (UTCP-style universal client: HTTP, CLI, MCP transports) |
| Surfaces | `tui.rs` (ratatui companion surface, 1,150 LOC), `cli/` (init, onboard, doctor, status, task, pulse, reflect, evolve, proposals, soul, data, lifecycle, daemon_life, tui) |
| Process | `daemon.rs`, `lifecycle.rs`, `watchdog.rs`, `reaper.rs`, `lock.rs` |

## Dimensions to fill

1. **Process model** — pulse-driven daemon; how the pulse decides actions; how the CLI attaches.
2. **Turn loop** — is there one? How do `tasks.rs` and `pulse.rs` relate?
3. **Session model** — what is persisted, how it is replayed, whether there is fork/rewind.
4. **Tool protocol** — `action_transport.rs` schemas; how tools are described and dispatched.
5. **Context management** — prompt assembly; what memory is injected; compaction.
6. **Provider abstraction** — `voice.rs`; OpenAI-compatible only? Local support?
7. **Entity substrate** — the five drives vs the design doc's six; glial equations; pulse cadence.
8. **Evolution** — what `soul_corridor` may mutate; validation; rollback; immune rules.
9. **Interop** — MCP client completeness; UTCP transport coverage; no ACP.
10. **Surfaces** — why the TUI answers "how is the entity" rather than "what is the system doing".
11. **Quality practice** — test coverage, dogfood examples, ratchets.
12. **Failure modes** — what the design doc admits; what the code leaves unfinished.

## Known evidence

- Five drives in `soul.rs::Drives`; the design doc describes six (adds growth). **Unresolved source-of-truth conflict.**
- `mcp.rs` is a real MCP client (stdio, dynamic discovery, tools/resources/prompts).
- `utcp.rs` subsumes MCP as one transport and calls HTTP/CLI tools directly.
- `crypto.rs` uses an app-level AEAD, not SQLCipher.
- The TUI is a companion surface: relational status bar, conversation, input. Not a coding workspace.
- Companion product features exist and are out of scope for Orexis: `relationship.rs`, `people.rs`, `advocacy.rs`, `anti_dependency.rs`, `audition.rs`.

## Adopt

- <to fill>

## Adapt

- <to fill>

## Reject

- <to fill>

## Open questions

- How does the pulse choose an action, and what stops it?
- What exactly may `soul_corridor` mutate, and how is a bad mutation rolled back?
- How do the five drives interact with `glial` modulation at runtime?
- Is there a single-agent turn loop, or is every action pulse-scheduled?
