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

- **Five drives** — connection, curiosity, mastery, coherence, rest — as the base model (`src/soul.rs:48-62`).
- **Glial modulation** — `DriveMultipliers` and `modulate_drives` as the affect-to-drive bridge (`src/glial.rs:127,275`).
- **Pulse** — `Pulse`, `PulseTick`, `PulseAction` as a scheduling heartbeat, not a hidden autonomy loop (`src/pulse.rs:60-76`).
- **Immune system** — danger patterns, graded responses, cytokine-storm detection (`src/immune.rs:20-117,333`).
- **Reflection tiers** — micro/macro/deep selection plus the metamorphic invariant check (`src/evolve.rs:38-48,346`).
- **Immutable soul corridor** — encrypted, append-only identity records with reasoned redaction (`src/soul_corridor.rs:2-13,266,363`).
- **Orchestration modes and gates** — `GraphFirst`/`ModelDriven`; `ApprovalGate` Auto/Inform/Confirm/Refuse (`src/orchestration.rs:29-45,69-96`).
- **Planner → executor → verifier** — `TaskPlan`, `PlannerDecision::Clarify` instead of guess-calling (`src/tasks.rs:107-140`).
- **MCP client** — connect, list tools/resources/prompts, call, read (`src/mcp.rs:202-453`).
- **UTCP universal client** — HTTP/CLI/MCP transports, `search_tools`/`find_tool` (`src/utcp.rs:129,226,313,498,516`).
- **Crypto and storage** — ChaCha20-BLAKE3 AEAD, Argon2id, rusqlite bundled (`src/crypto.rs`, `src/db.rs`).

## Adapt

- The **soul corridor** becomes the identity store, separate from the mutable gene pool and from memory.
- The **immune system** becomes the evolution safety layer, not a runtime affect.
- **Orchestration gates** fold into the permission ruleset (`deny > ask > allow`) rather than a second policy system.
- The **tasks pipeline** becomes the `build` strategy's internal plan → execute → verify loop.
- The **TUI** contributes the ratatui/crossterm foundation and event-to-render discipline; the layout is rewritten (ADR-0004).
- The **five-vs-six drives** question is deferred to ticket 007; growth is not assumed.

## Reject

- **Companion product features**: `relationship.rs`, `people.rs`, `advocacy.rs`, `anti_dependency.rs`, `audition.rs`.
- **Companion tone** in prompts and surfaces.
- **Pulse as an always-on hidden autonomy loop**; it may schedule, but it must be visible and bounded.

## Open questions

- How does the pulse choose an action, and what stops it?
- What exactly may `soul_corridor` mutate, and how is a bad mutation rolled back?
- How do the five drives interact with `glial` modulation at runtime?
- Is there a single-agent turn loop, or is every action pulse-scheduled?
