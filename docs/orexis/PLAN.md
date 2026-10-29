---
agent: devin-local
session: jolly-flannel
created: 2026-10-25T21:40:25Z
---
# Orexis — sovereign, model-agnostic coding harness with a living entity

Build Orexis in Rust as a sovereign, model-agnostic coding harness with a living entity: local state and identity, inference wherever the user points it (broad provider catalog), full coding surface, single-agent default with a benchmark-gated team mode, and MCP + ACP interop.

> Kindred is the soul. Harness is the hands. Familiar is the lesson. The corpus is the map.

A **sovereign, model-agnostic coding harness with a living entity**: your state, identity, memory, sessions, and keys live on your machine and export fully; inference goes wherever you point it; the agent actually edits code, runs tests, uses git, and reviews diffs — and it persists, remembers, and evolves.

**Breadth principle.** Breadth is a goal, not a risk. Every capability is added behind a seam; we cut only when a seam cannot carry it. This plan lists what ships; it does not pre-emptively trim.

---

## Map

### Destination

**A shipped Orexis 1.0 that passes the §5.1 checklist** — installed from a signed artifact, selecting a provider at init, completing a multi-file coding task with verify and diff review, with entity state surviving restarts and model switches, and full export/import.

### Notes

- Domain: local-first agent harness; Rust; TUI + desktop; no web surface.
- Quality order: architecture → behaviour → ownership → breadth → safety.
- Breadth principle: cut only when a seam cannot carry the capability.
- Skills every session consults: `grilling`, `domain-modeling`, `clean-architecture`, `designing-interfaces`, `rust-patterns`, `tdd`, `code-review`.
- Standing preference: **no wholesale rewrites**. Decisions are appended to the index; the body changes only when the frontier or the fog changes. One revision per milestone.

### Decisions so far

| Decision | Detail |
|---|---|
| Sovereign state + model-agnostic inference + living entity | §0, §1.5 |
| Rust; fresh repo; one-way porting ledger from Kindred and Harness | §2, §6 |
| Ports consumer-side; adapters only in composition; `tokio` banned inward | §4.1–4.3 |
| Async ports; runtime ports; fresh-process PTC default | §4.2 |
| Provider catalog: profiles as data; one compatible adapter; native only where the wire differs; inference chosen at init | §4.4 |
| Event log; versioned format; model-visible means logged | §4.5 |
| Coding surface: exact + fuzzy edits, shell, search, git, verify, diffs/checkpoints, LSP | §5, M3 |
| Safety floor: policy-as-code, untrusted-content delimiting, OS sandbox, skill trust tiers | §5, M3/M4 |
| Single-agent default; team opt-in; strategies are use cases; workers share identity; provider-bounded concurrency | §9 |
| Desktop: three-zone oversight workspace; task-driven panels; inline approvals; two evaluation cycles | ADR-0008; ticket 006 |
| MCP + ACP in v1; A2A/UTCP in M7 | §5, M7 |
| Entity model: six drives (connection, curiosity, mastery, coherence, rest, growth) + PAD affect; drives propose, affect modulates posture; never prompt mid-session | ADR-0013; ticket 007 |
| Entity identity is external/persistent/model-agnostic; model persona is internal/model-specific; neuron alignment/steering is research, not M5 | ADR-0014; issue 012 |
| Evolution: default off; skills opt-in after the ladder; quarantine stage-only; core signed + human; sealed set; signed git ledger; rollback triggers | ADR-0015; issue 008 |
| Lexical memory; no code RAG | §7 |
| Tiered, git-backed, signed evolution | §4.5 |
| Milestones M1–M9 with per-milestone cut rule | §5 |
| Session event model: durable medium events, session + event versioning, lazy migration, four generation triggers, SQLite + JSONL, `seq` + ULID, verbatim request + hash, append-only compaction, fork copies prefix, redact at write, indefinite retention, batch + fsync | ADR-0001; ticket 001 |
| Provider catalog: rich profile + `extra`, vendored snapshot + refresh, shipped < user < project < CLI, keyring + env, conservative unknown defaults, profile-declared capacity, per-profile fallbacks, adapter-owned OAuth | ADR-0002; ticket 002 |
| Codec set: `native`, `xml-manual`, `json`, `lenient`; code-mode is a program surface, not a codec; automatic selection by capability with `--codec` override; sans-io fixtures | ADR-0003; ticket 003 |
| TUI surface: four fixed regions, composer-first focus, inline tool expansion + bottom sheet for large diffs, adaptive overlays and status bar, virtualized transcript, full keymap, eleven states | ADR-0004; ticket 004 |
| Study: eight repos read to adopt/adapt/reject; synthesis drafted with pattern catalog, resolved conflicts, ADR map, confidence, and remaining verification | `docs/orexis/study/synthesis.md`; ticket 011 |
| Study gate cleared: synthesis + ADRs 0001–0004 + vertical slice (six crates) replaying byte-identically | `docs/orexis/slice.md` |
| Native adapters are on-demand (wire/fidelity/auth triggers), not milestone-scheduled; M7 stays protocols-only | ADR-0016; ticket 009 |
| Provider conformance fixtures are capability transcripts with a three-state manifest (expressed / gap / wire-inexpressible); open_ai + anthropic first; thinking and message-cache gaps are adapter tickets, not native triggers | ADR-0018; ticket 013 |
| Packaging: one `orexis` binary, four targets, minisign-signed artifacts (one keypair with the Tauri updater), GitHub Releases primary, verify-before-execute; upgrade preservation is structural | ADR-0019; ticket 010 |
| Team-mode domain: workers are in-process turns over sessions sharing the entity; board/bus/blackboard are ports with sqlite/fs adapters; the scheduler asks the provider for capacity; strategies are use cases; the supervisor is a watchdog; opt-in, benchmark-gated | ADR-0020; M8 spec |
| Team-mode gate measured: paired breadth run on Qwen3.5-2B — pass 4/5 → 5/5 (the one flip is M8-16's listing trap), wall −1,720%, tokens 1.80×; the team arm was the driver's serial work while the plan worker stalled on an unanswerable `board_add` approval (M8-17) ⇒ **team mode stays experimental** | §9; M8-10; `bench-results-qwen35-gate.jsonl` |
| Data portability: `export --all` / `import` bundle round-trips all sessions, generations, and entity store without loss | §5.1, M9 |
| v0.1.0 released: multi-target signed release pipeline (minisign, 4 targets), live `install.sh` verified on GitHub (KidIkaros/orexis) | §5, §5.1, M9 |

### Frontier

Open decisions, in order. Each is one session's work; resolving it appends a row above and clears the fog it unblocks.

1. ~~**Session event schema and migration format**~~ — resolved; ADR-0001.
2. ~~**`ProviderProfile` schema and catalog source** (models.dev snapshot vs live)~~ — resolved; ADR-0002.
3. ~~**Codec set beyond the ported four**~~ — resolved; ADR-0003.
4. ~~**TUI layout and keymap**~~ — resolved; ADR-0004.
5. ~~**Edit-tool contract per model tier**~~ — resolved; issue 005.
6. ~~**Desktop panel layout in Leptos**~~ — resolved; ADR-0008.
7. ~~**Entity model: five vs six drives, affect dimensions**~~ — resolved; ADR-0013.
8. ~~**Evolution tier defaults and validation ladder**~~ — resolved; ADR-0015.
9. ~~**Native adapter timing: Gemini / Bedrock / Vertex**~~ — resolved; ADR-0016.
10. ~~**Packaging and signing**~~ — resolved; ADR-0019.
11. ~~**Eight per-repo study decisions**~~ — resolved; synthesis drafted.
12. **Desktop workspace release packaging (v0.2.0, M2)** — ticket 023: Tauri v2 desktop bundles (Linux `.deb`/AppImage, macOS `.dmg`), IPC wiring with `desktop-core`, side-by-side diff inspector, evaluation cycles 1 & 2.
13. **Team mode multi-model benchmark graduation (v0.3.0, §9/§10)** — run breadth suite on remote frontier (Claude 3.7 / GPT-4.5) to test team vs serial token and wall-clock ratios for GA graduation; provider rate-limit QoS.
14. **Zero-config MCP auto-discovery and rival state importers (v0.4.0)** — discover local configs (Claude, Cursor, OpenCode); `orexis import --format aider|cline|opencode`; ACP editor plugins.
15. **Local neuron steering research (M5-R01) & zero-vector memory lattice (v0.5.0, M5)** — activation steering probes on local Qwen/Llama (H-Neuron sycophancy reduction, zero-token cognitive posture); lexical full-text memory lattice.
16. **100-cycle evolution gate & long-term stability guarantees (v1.0.0 LTS, M6)** — 100 autonomous self-mutation cycles with zero unrecoverable rollbacks.

### Not yet specified

Neuron activation alignment/steering for local open weights — research track (M5-R01); model-specific, not entity identity (ADR-0014) · MAS scheduling policy after the breadth benchmark on frontier models · evolution calibration after 100 cycles · A2A/UTCP adoption after M7 · SDK/marketplace timing · multi-user collaboration · mobile/web.

### Out of scope

Web UI (desktop only) · relationship/companion features · local-only inference as a constraint · agentic frameworks · vector RAG over code · harness identity mimicry · hosted cloud service · SDK and marketplace in v1.

> The sections below are the reference material behind this map: market evidence, architecture, provider breadth, design rules, milestones, and the porting ledger. The map is the index; the detail lives below.

---

## 0. Market landscape (why this position)

| Signal | Value | Source |
|---|---|---|
| Enterprise AI coding agent market | **$14.18B (2026) → $45.83B (2031)**, 26.44% CAGR | Mordor Intelligence |
| Gartner estimate | ~$9.8B+ | Gartner |
| AI coding spend (2025) | ~$4B | Menlo Ventures |
| Devs using agents weekly / daily | **90% / 68%** | JetBrains 2026 |
| Usage share | Claude Code **65.5%**, Copilot 58.7%, Codex 29.5%, Cursor 25.5%, **OpenCode 13.6%**, Cline 4.5%, Aider 2.1% | Stack Overflow 2026 |
| Revenue | Cursor $2B→$6B ARR (sold $60B); Cognition ~$900M ($48B) | TechCrunch / Cognition |
| Tool protocol | **MCP: 49,226 servers, +39%/month; SDK downloads 19:1 over A2A** | MCP census 2026 |
| Interface protocol | **ACP in 6/11 harnesses**; A2A in 1/11 | arXiv 2609.00006 |
| Multi-agent cost | **~15× tokens**; 8/9 harnesses keep sub-agents in-process | Hadfield et al.; corpus |
| Provider breadth | Hermes ships **43 model-provider profiles**; OpenCode uses models.dev; Codex uses a server-delivered catalog | Source |

**Convergence:** seven subsystems (loop, LLM integration, tools, memory/context, safety/permissions, orchestration, extensibility) plus interface and session surfaces. The floor is ~100 lines; production investment goes to safety, recovery, cost, extensibility, and platform.

**Two absences validate our architecture:** 0/11 harnesses use an agentic framework; 0/11 use vector RAG for code. Hand-rolled loops plus ripgrep/tree-sitter/JIT context are the production answer.

**Platform turn:** harnesses are becoming frameworks and platforms — SDKs, marketplaces, importers for rival state, enterprise governance. Distribution and switching costs now matter alongside the loop.

**Security reality:** Clinejection (~4,000 machines), TrustIssues (CVSS 10), IDEsaster (100% of 10 tools vulnerable), slopsquatting. Safety is a product surface.

**Where Orexis competes:** the giants own cloud-first breadth. Orexis wins on **sovereign state, model-agnostic inference, a persistent entity, and real self-evolution** — while still being a first-class coding harness.

---

## 1. Quality definition

| Dimension | Bar | Judged by |
|---|---|---|
| **Architecture** | Dependencies inward; ports consumer-side; adapters only in composition; no `tokio` inward; no shared `utils` | Generated dependency graph; import-lint |
| **Behaviour** | Match or beat the harness benchmark across a **model matrix**; zero lost turns on crash | `orexis bench` + replay + crash matrix |
| **Ownership** | Every line ours or MIT/Apache with attribution | Dependency audit; `NOTICE` |
| **Breadth** | Providers, tools, skills, and protocols are catalogs, not hand-picked lists | Catalog completeness; profile coverage |
| **Safety** | Untrusted-content delimiting, policy-as-code with a floor, OS sandbox, skill trust tiers | Adversarial fixtures; supply-chain audit |

---

## 1.5 What the entity is

The entity is the persistent identity layer — not a persona file, and not a separate agent per role.

| Layer | What it is | What the user observes |
|---|---|---|
| Identity | Traits distilled from an append-only diary; evidence-backed, confidence-scored, decaying | `/self`, `IDENTITY.md`, behaviour over time |
| Drives | A small set of motives that propose work (Kindred's five; MUE-X's seven reconciled in the study) | Occasional nudges, goal proposals |
| Affect | A PAD-like state modulating routing, risk, and retry — never the prompt mid-session | Mood in `/self`, routing changes |
| Memory | Six-layer lexical lattice with crystallisation into skills | Better recall, skills that persist |
| Evolution | Tiered, git-backed, sealed, signed mutation of genes and strategies | `/evolve`, ledger, rollback |

It is **not** a companion/relationship product (Kindred's relationship phases, advocacy, and anti-dependency are rejected), a role-play persona, or a separate agent per strategy.

## 1.6 Glossary

| Term | Means |
|---|---|
| Entity | The singular identity layer (identity, drives, affect, memory) |
| Worker | A generic execution instance with its own context, model, toolset, budget |
| Strategy | A use case: `plan`, `build`, `review`, `mine` |
| Board | Durable tasks with owner, required strategy, blocking edges |
| Bus | Direct and room messages, persisted as events |
| Blackboard | Shared workspace, artifacts, and leases |
| Seam | A capability's definition + provider + consumer |
| ProviderProfile | Provider data: endpoint, auth, dialect, cache, thinking, models |
| Tier | Evolution scope: `off`, `skills`, `quarantine`, `core` |
| Milestone | A staged deliverable with its own exit gate |
| Release candidate | The milestone that ships when the next one misses its gate twice |
| Session event | A durable fact in the event log; model-visible if the model saw it |
| Telemetry | Non-truth analytics; never consulted for correctness |

---

## 2. Prior art we own

| Project | Language | Size | Gives us | Warns us about |
|---|---|---|---|---|
| **Kindred** | Rust | **78,931 LOC** | Entity substrate (soul, five drives, glial, pulse, immune, genes/corridor, encrypted SQLite, curator, consolidation, recall); orchestration; tasks; proposal queue; **MCP client** (945); **UTCP client** (1,182); permissions; ratatui TUI; daemon/lifecycle/watchdog | Companion framing; TUI answers "how is the entity"; five drives vs six in the design doc |
| **Harness** | Rust | **21,665 LOC** | Tool loop: four codec arms, providers, sandbox/gate/exec, genes, probe/compare eval, sessions, serve/attach | Single-model assumptions; sync executor + `block_in_place` |
| **Familiar** | Go | 158k LOC, 73 packages | Event bus, IPC, skills, curator, self-evolution ideas | **The failure mode**: "73 packages… buried under infrastructure built to imitate competitors" |

---

## 3. The study (Phase S)

### S0 — Wayfinder map

`/wayfinder` in the new repo: decision tickets in `.scratch/orexis/issues/`.

### S0.5 — Extraction schema

Twelve dimensions; each doc ends in **adopt / adapt / reject** with citations: process model · turn loop · session model · tool protocol · context management · provider abstraction · entity substrate · evolution · interop · surfaces · quality practice · failure modes.

One **`/research`** artifact per repo, plus `docs/orexis/research/harness-corpus.md` (the eleven-system study) and `docs/orexis/research/market.md`.

### S1 — Internal first

Kindred (substrate + orchestration + clients; reject companion product) · Harness (loop + codec; reject `block_in_place`) · Familiar (failure analysis).

### S2 — External five

DeepSeek Harness (seams, event domains, logged invariant, versioned log, PTC, team limits) · opencode (client/server, code-mode, sessions, permissions, TUI, models.dev) · Hermes (43 provider profiles, plugin breadth, skills, gateways, ACP, cache invariant) · MUE-X (evolution, drives, PAD, lattice) · Mercury (installer, budget, typed memory, channels).

### S3 — Synthesis

Pattern catalog · conflict resolution · porting ledger · architecture spec + ADRs.

### S4 — Vertical slice

**prompt → tool call → session event → TUI render → replay**, adapters from composition.

**Status: done 2026-10-25.** `crates/slice` (six crates) runs the loop, renders the transcript, and asserts `replay: byte-identical`. See `docs/orexis/slice.md`.

**Study gate:** map cleared, synthesis + ledger + ADRs, slice replays byte-identically. **Cleared 2026-10-25.**

---

## 4. Architecture

### 4.1 The Dependency Rule, as crates

```
orexis-core            entities + data types (no IO, no ports, no tokio)
orexis-ports           traits + runtime abstractions (no tokio)
orexis-usecases        loop · session · permissions · strategies · scheduler
                       entity · memory · evolution · identity
orexis-adapters-*      provider-* · session-sqlite · board-sqlite · artifacts-fs
                       sandbox-* · codec-* · protocol-{mcp,acp,utcp,a2a}
                       memory-sqlite · vcs-git · lsp-* · tui-ratatui
                       desktop-tauri · gateway-* · reflector-llm · runtime-tokio
orexis-composition     the only crate that constructs adapters (Main)
orexis                 binary: cli / serve / attach / bench
```

Rules: core/ports depend on `std` + `futures` only; use cases never name adapters or `tokio`; adapters depend on core + ports; composition is the only constructor; no shared `utils`; CI enforces.

### 4.2 Async and the runtime boundary

`async` is language; `tokio` is framework. Ports may be `async fn`; nothing inward of adapters names `tokio`. Streaming via `futures::Stream`. Framing/parsing is **sync, sans-io**. `Clock`, `Spawner`, `Cancellation` are ports. `ToolExecutor` is async; the sandbox adapter owns `spawn_blocking`.

### 4.3 Ports (consumer-side)

`Provider` (+ `capacity()`, `capabilities()`) · `Codec` · `ToolExecutor` · `SessionAppend`/`SessionRead`/`SessionFork` · `BoardStore` · `MessageStore` · `ArtifactStore` · `EventSink` · `Vcs` · `MemoryStore` · `Reflector` · `Lsp` · `Clock`/`Spawner`/`Cancellation`.

### 4.4 Provider breadth and inference

**A provider is data plus, rarely, an adapter.**

| Layer | Contents |
|---|---|
| `ProviderProfile` | base URL · auth kind (key/OAuth/ADC/SigV4) · wire dialect · tool-call format · cache dialect · thinking support · context/pricing metadata · model list |
| `openai-compatible` adapter | Covers most profiles: OpenAI, DeepSeek, OpenRouter, Fireworks, Together, Groq, xAI, Mistral, vLLM, llama-server, LM Studio, Ollama, OpenCode Zen, Nous, and the rest |
| Native adapters | Only where the wire differs: Anthropic (messages/tool blocks), Gemini (generateContent), Bedrock/Vertex (SigV4/ADC), Copilot/Qwen (OAuth device flow) |
| Local providers | llama.cpp, Ollama, LM Studio as profiles — first-class, not privileged |
| Catalog | models.dev metadata + Hermes's 43 profiles + local GGUF registry; user profiles override (last-writer-wins) |
| Selection | Chosen at `init`; switchable per session; fallback chains and credential pools |

**Per-provider conditioning** (paid once, centrally): cache breakpoints, thinking/reasoning effort, tool-call serialization, streaming quirks, context limits, pricing. A **model-family prompt matrix** and **cache-dialect fanout** are design rules, not afterthoughts.

**Privacy:** remote inference means data leaves the machine. Per-provider data policy, redaction of secrets, no telemetry, and an explicit "what leaves" panel in `/self`.

### 4.5 Invariants (each an ADR)

1. Model-visible means logged; `derive_messages()` pure; replay byte-identical.
2. Session format versioned; migrations adjacent; generations immutable.
3. Prompt prefix stable within a session; only compression breaks it.
4. Program execution defaults to a fresh process under sandbox.
5. Every capability declares a seam; ≥2 providers.
6. Permissions are a ruleset at the tool layer, with a floor that survives YOLO.
7. Evolution is tiered, git-backed, sealed, signed; quarantine never auto-applies Python.
8. Identity is emergent from a diary; never agent-authored into `SOUL.md`.
9. No code reuse from unlicensed repos.
10. No silent refactors.
11. Ports consumer-side; adapters only in composition.
12. No adapter type crosses a port.
13. `tokio` banned inward of adapters.
14. Protocols are sans-io.
15. Every TUI/desktop state has a wayfinding answer and an escape hatch.
16. A strategy is a use case, never an agent type.
17. Concurrency is provider-bounded; the scheduler asks the provider for capacity.
18. A worker shares the entity's identity and memory; only context, model, toolset, and budget are per-worker.
19. Single-agent is the default; team mode is opt-in and benchmark-gated.
20. No vector RAG over code; lexical FTS5 + ripgrep + tree-sitter only.
21. MCP for integrations, Skills for capability templates; trust tiers for third-party skills.
22. Untrusted content is delimited and never treated as instructions.
23. **Inference location is a per-profile choice, never a product assumption.**
24. **Provider breadth is a catalog: a new provider is data, an adapter only when the wire differs.**

---

## 5. Milestones

| Milestone | Deliverable | Usable as | Gate |
|---|---|---|---|
| **M1** | Workspace, core/ports/fakes, daemon, **provider registry + catalog**, single-agent loop, TUI, replay | A working harness on any provider | `cargo test`; lint; replay byte-identical; OpenAI-compatible + Anthropic + llama.cpp live |
| **M2** | Desktop (Tauri + Leptos): chat, fleet, approvals, diffs, artifacts, files, settings | The oversight workspace | Six panels render; shared daemon attach |
| **M3** | Coding surface + safety floor: edit (exact + fuzzy), shell, search, git, **verify loop**, **diffs/checkpoints**, **LSP**, permissions ruleset, plan gate, sandbox | A real, safe coding harness | Edit-drift fixtures; verify-on-stop; LSP diagnostics; shell denied without a ruleset |
| **M4** | Recovery + supply-chain hardening: crash recovery, audit trail, skill trust tiers and quarantine | A durable harness | kill -9 loses nothing; adversarial fixtures; quarantine enforced |
| **M5** | Entity: soul, drives, glial, memory lattice, identity distillation | The differentiator | Identity never leaks mid-session; evidence-backed traits |
| **M6** | Evolution: corridor, mutation, validation, tiers, signed pool | The moat | 100 cycles, zero unrecoverable states |
| **M7** | Protocols: **MCP client + ACP server**; UTCP; A2A | Ecosystem interop | Conformance fixtures; one real peer each |
| **M8** | Team mode (opt-in): workers, strategies, board, bus, blackboard, scheduler | The breadth engine | Team beats serial on a breadth task at acceptable token cost |
| **M9** | Hardening and 1.0 | Shipping | Stranger installs and completes a multi-tool task |

**M1 status (2026-10-26):** implemented; `scripts/m1-gate.sh` green. Providers verified via OpenRouter free, Refuseless, and local llama.cpp; the live-model repository task passed. Direct Anthropic remains manual.

**M2 status (2026-10-26):** implemented in source; M2-01 … M2-06 wired; fleet/board/changes are session projections per ADR-0012; `scripts/m2-gate.sh` green for automated checks. Tauri shell build, evaluation cycles, and usability measurement remain manual.

**M4 status (2026-10-26):** automated gate green; M4-01 … M4-04 resolved. Manual: SIGKILL a real repository turn and restart.

**M6 status (2026-10-26):** automated gate green; M6-01 … M6-05 resolved. Manual: `/evolve` inspection and evolution calibration. Decision 008 resolved (ADR-0015).

**M5 status (2026-10-26):** automated gate green; M5-01 … M5-06 resolved. Manual: `/self` against a real repo and a live model switch. Decision 007 resolved (ADR-0013); neuron alignment is a separate research track (ADR-0014, M5-R01).

**M7 status (2026-10-26):** automated gate green (`scripts/m7-gate.sh`); M7-01 … M7-11 resolved. Protocols: MCP client (stdio + streamable HTTP), ACP server (stdio, fs + permission callbacks), UTCP client (HTTP/CLI/MCP transports), A2A client (card, send, tasks, SSE). Manual: one real peer per protocol. A2A/UTCP adoption beyond fixture-level is decided on M7's evidence.

**M8 status (2026-10-28):** team domain implemented (M8-01 … M8-15 resolved; M8-16 and M8-17 open); `scripts/m8-gate.sh` green on the automated suites. The §9 benchmark says **experimental**: one local model, five breadth pairs, team 5/5 against serial 4/5 — the single flip is M8-16's trap, not a team win — at ≈18× mean wall clock and 1.80× tokens. The team arm was not parallel work: the driver did each task while the plan worker stalled on `board_add`, whose approval no client can answer (M8-17), so the wall column is that 300 s wait. Team mode stays opt-in; the next gate run needs M8-17, M8-16, and the rest of the §10 model matrix.

**Ordering (2026-10-26): M3 before M2.** The M1 real-task gate needs the coding surface, and the desktop should display a working agent rather than a fake one.

**Cut rule:** if a milestone misses its gate twice, the previous milestone becomes the release candidate and the missed one moves to 1.1. No milestone silently absorbs another.

### 5.1 1.0 checklist

- Installs from a signed release artifact on Linux and macOS. (Achieved: v0.1.0)
- `orexis init` selects a provider; there is no silent default. (Achieved: v0.1.0)
- Completes a multi-file coding task with edit, shell, git, verify, and diff review. (Achieved: v0.1.0)
- Entity state persists across restarts and model switches. (Achieved: v0.1.0)
- `export --all` / `import` round-trips without loss. (Achieved: v0.1.0)
- Upgrade preserves sessions and the entity. (Achieved: v0.1.0)
- No telemetry; `/self` shows exactly what leaves the machine. (Achieved: v0.1.0)

### 5.2 Post-v0.1.0 Roadmap (v0.2.0 through v1.0.0 LTS)

| Version | Milestone | Focus | Deliverables | Gate |
|---|---|---|---|---|
| **v0.2.0** | **M2** | Desktop Oversight Workspace | Tauri v2 bundles (`.deb`, AppImage, `.dmg`), side-by-side split diff component, hunk-level approvals, shared socket attach | `scripts/m2-gate.sh` green; usability cycles 1 & 2 completed; multi-platform desktop release artifacts signed |
| **v0.3.0** | **M8 / §9** | Team Mode GA & Fleet Concurrency | Fix M8-16 (listing trap) & M8-17 (board approvals); frontier benchmark matrix (Claude 3.7 / GPT-4.5); provider rate-limit QoS | Breadth benchmark beats serial: $\ge 15\%$ higher pass rate or $\ge 25\%$ lower wall clock at $\le 2\times$ tokens across frontier models |
| **v0.4.0** | **M7** | Ecosystem Interop & Migration Bridge | Zero-config MCP auto-discovery (Claude, Cursor, OpenCode); universal state importer (`aider`, `cline`, `opencode`); ACP editor plugins | Conformance test suite across 3 rival formats with zero event loss; zero-config discovery verified across 5 MCP servers |
| **v0.5.0** | **M5 / R01** | Living Entity & Cognitive Alignment | Production zero-vector lexical memory lattice (FTS5 + entity graph); Research Track M5-R01 local neuron steering prototype (H-Neuron sycophancy reduction) | `/self` stability across 10 sessions & 3 model switches; standalone reproducible prototype of activation steering on open weights without runtime coupling (ADR-0014) |
| **v1.0.0 LTS** | **M6** | Sovereign Autonomous Agent | 100-cycle self-evolution gate; signed tamper-evident git ledger; sealed core guarantees; long-term API and protocol stability | 100 autonomous self-mutation cycles with zero unrecoverable rollbacks; full multi-platform security and reproducibility audit |

---

## 6. Porting ledger

| Module | Source | Decision | Target |
|---|---|---|---|
| `orchestration.rs`, `tasks.rs`, `proposal.rs` | Kindred | **Adapt** | `usecases::strategy`, `usecases::scheduler`, `usecases::approvals` |
| `mcp.rs` (client, stdio) | Kindred | **Port**, sans-io | `adapters-protocol-mcp` |
| `utcp.rs` | Kindred | **Port**, sans-io | `adapters-protocol-utcp` |
| `hands.rs`, `action_transport.rs` | Kindred | **Port** | `usecases::tools` + `adapters-sandbox-*` |
| `permission.rs`, `gate.rs`, `guard.rs`, `consent.rs` | Kindred | **Adapt** | `usecases::permissions` |
| `soul.rs`, `glial.rs`, `pulse.rs` | Kindred | **Port** | `usecases::entity` |
| `immune.rs`, `evolve.rs`, `soul_corridor.rs` | Kindred | **Adapt** | `usecases::evolution` |
| `curator.rs`, `consolidation.rs`, `recall.rs`, `memory/` | Kindred | **Port** | `usecases::memory` |
| `crypto.rs`, `db.rs` | Kindred | **Adapt** | `adapters-session-sqlite`, `adapters-memory-sqlite` |
| `daemon.rs`, `lifecycle.rs`, `watchdog.rs`, `reaper.rs` | Kindred | **Adapt** | `orexis-composition`, `orexis serve` |
| `tui.rs` | Kindred | **Framework only** | `adapters-tui-ratatui` — layout rewritten |
| `relationship.rs`, `people.rs`, `advocacy.rs`, `anti_dependency.rs` | Kindred | **Reject** | — |
| codec arms (4) | Harness | **Port**, sans-io | `adapters-codec-*` |
| providers | Harness | **Port** + expand to catalog | `adapters-provider-*` |
| sandbox/gate/exec | Harness | **Port**; executor async | `adapters-sandbox-*` |
| genes | Harness + Kindred | **Merge** | `usecases::evolution` |
| eval probe/compare | Harness | **Port** | `orexis-bench` |
| core types/assemble/compact | Harness | **Adapt** | `orexis-core` + `usecases` |
| `serve`/`attach` proto | Harness | **Adapt** | `orexis-ports::protocol` + daemon |
| `block_in_place` | Harness | **Reject** | async `ToolExecutor` |
| Familiar (any code) | Familiar | **Reject** | failure analysis only |

---

## 7. Design rules from the corpus

| Rule | Why |
|---|---|
| Linear loop first; middleware only for 3+ orthogonal policies | Loop sophistication does not predict success |
| Bash first; add tools on observed failure; deferred loading above ~15 tools | Mini-SWE-Agent floor; Claude/Codex/Hermes convergence |
| Exact unique-substring edits for frontier models; fuzzy cascade for weaker; never line numbers | Drift happens on line numbers |
| Hierarchical Markdown context; JIT nested injection | All eleven systems converged |
| Threshold compaction: fixed buffer, verbatim tail, incremental summaries | Preserves early decisions |
| No code RAG; ripgrep + tree-sitter + glob + FTS5 | 0/11 use vector code retrieval |
| Policy-as-code with a floor under YOLO | Hermes hardline patterns |
| Untrusted-content delimiting | Clinejection class |
| Turn-level checkpoints + session-tree version control | OpenCode/Hermes/Pi |
| Model-family prompt matrix + cache-dialect fanout | Codex/OpenCode |
| Syntax-aware command permissioning | OpenCode/Hermes |
| Skills for capability templates, MCP for integrations | Skills 9/11, MCP 8/11 |
| Outer verification loop (verify-on-stop) | OpenHands/Hermes |
| Agent-maintained memory, human-gated | Codex/Gemini |
| ACP server; sub-agents in-process | 6/11 ACP; 8/9 in-process |
| Single-agent until breadth-first work proves parallel isolation | ~15× tokens |
| Provider profiles + per-model metadata | Hermes 43 profiles; models.dev |
| No frameworks; no 1-to-1 SaaS wrappers; cheap stuck caps | Twin absences; Aizawa et al. |

**Anti-patterns:** runtime frameworks, vector RAG over code, line-number edits, unbounded tool output, confirmation where undo would do, harness identity mimicry.

---

## 8. Surfaces

### 8.1 One daemon, two clients

`orexis serve` owns sessions, the board, the bus, and the event log. The TUI attaches over Unix-socket NDJSON; the desktop (Tauri + Leptos) attaches over the same socket through its Rust side. No HTTP server for UI clients; outbound HTTPS to providers is expected.

### 8.2 TUI (fast path, SSH)

Fixed regions: status (session · worker · model · provider · ctx · cost · plan), transcript, composer, keyhints. Overlays: fleet, board, room, self-state, todo, interop. Keymap: `Enter` send, `⇧Enter` newline, `Esc` cancel, `Ctrl-C` interrupt, `Ctrl-P` palette, `Ctrl-R` search, `g` open file, `Ctrl-D` detach.

### 8.3 Desktop (oversight workspace)

Six panels: **chat/streaming**, **fleet**, **approvals**, **diff viewer**, **artifacts**, **file browser**. Settings: model/provider, persona, permissions, privacy ("what leaves this machine").

### 8.4 States

Empty · loading · streaming · tool running · approval · error · budget exhausted · disconnected · queued. Every state has a wayfinding answer and an escape hatch.

---

## 9. Team mode (opt-in, benchmark-gated)

| Concept | Is | Is not |
|---|---|---|
| **Entity** | The singular soul: identity, drives, affect, memory | A worker |
| **Worker** | A generic execution instance: own context, model, toolset, budget | A role |
| **Strategy** | A use case: `plan`, `build`, `review`, `mine` | An agent type |
| **Board** | Durable tasks with required strategy, owner, blocking edges | A prompt |
| **Bus** | Direct + room messages, persisted as events | Ephemeral chat |
| **Blackboard** | Workspace, artifacts, leases | A shared prompt |
| **Scheduler** | Admits tasks under provider capacity and budget | A queue alone |
| **Supervisor** | Restart, stall detection, escalation | The loop |

**Gate:** team mode ships only if, on the breadth benchmark, it achieves **≥15% higher pass rate or ≥25% lower wall-clock** than serial single-agent at **≤2× tokens**. Otherwise it remains experimental. The 15× token figure is the prior to beat.

| Limit | Value | Source |
|---|---|---|
| Local provider generations | 1 | Harness semaphore; hardware |
| Remote concurrency | per provider profile | API limits |
| Roster size | 16 | Hermes live sessions; DeepSeek members |
| Board tasks | 256 | DeepSeek |
| Spawn depth | 1 (flat) | Hermes default |

Profile caps are defaults; a team may tighten, never exceed. Exhaustion produces a visible queue.

---

## 10. Benchmark

Reuse harness probe/compare. Add coding tasks (edit drift, multi-file refactor, verify loop), entity tasks (identity across model switches, trait evidence, memory recall, evolution rollback), and team tasks (breadth-first parallel work, restart-resume, budget exhaustion). **Run the matrix across providers and model tiers**, not one model. Minimum floor: one local small (≤4B), one local medium (8–14B), one remote frontier, one remote cheap. Metrics: pass, round trips, tool calls, tokens, wall, stop reason, cost. Paired bootstrap + noise band. UI metrics: keystroke p95, token→render p95, resume(100).

---

## 11. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Repeating Familiar** — breadth before architecture | Fatal | M1–M9 gates; cut rule; each milestone usable |
| Provider matrix maintenance | High | Profiles as data; one compatible adapter; native only where the wire differs; conformance fixtures per dialect |
| MAS costs 15× tokens and may not beat serial | High | Opt-in + benchmark gate; serial default |
| Entity features read as novelty | High | Identity must improve task outcomes (recall, preference adherence); measure it |
| Security incident (Clinejection class) | Fatal to trust | Untrusted-content delimiting; skill trust tiers; policy floor; OS sandbox; supply-chain audit |
| Remote inference leaks data | High | Per-provider data policy, redaction, "what leaves" panel, no telemetry |
| Giants own cloud-first breadth | High | Compete on sovereignty, persistence, evolution — not on gateway count |
| Porting creates a third architecture | High | One-way ledger; named targets; layouts rewritten |
| Runtime leaks inward | High | `tokio` ban; runtime ports + fakes |
| Desktop scope creep | Medium | Six panels only in M2 |
| Drives discrepancy (5 vs 6) | Medium | Study ticket; `/domain-modeling` |
| Benchmark gaming | Medium | Held-out tasks; judges resistant to lucky runs |

---

## 12. Skills and workflow

**Study:** `wayfinder` · `research` · `market-research` · `domain-modeling` · `codebase-design` · `system-design` · `agent-harness-construction` · `eval-harness`/`agent-eval` · `architecture-decision-records` · `clean-architecture` at every gate

**Spec → tickets:** `to-spec` · `to-tickets` · `blueprint`

**Build:** `implement` (`tdd`) · `code-review` · `rust-patterns` · `rust-best-practices` · `rust-testing` · `verification-loop` · `lint` · `test` · `refactor`

**Critical modules** (session log, scheduler, evolution, permissions): `santa-method` · `security-review` · `safety-guard`

**UI:** `designing-interfaces` · `gui-bloopers` · `designing-with-the-mind-in-mind` · `make-interfaces-feel-better` · `frontend-design` · `accessibility` · `web-perf`

**Protocols:** `mcp-server-patterns` · `api-design` · `security-review` · `computer-networking`

**Long sessions / shipping:** `context-budget` · `strategic-compact` · `handoff` · `github-workflows` · `deployment-patterns` · `docker-patterns` · `wizard` · `writing-for-agents` · `documentation-lookup`

**Per-ticket gates:** failing test → smallest green → two-axis review → `verification-loop` → `santa-method` on critical modules.

---

## 13. Process

`CONTEXT.md` · `AGENTS.md` · per-repo study docs + synthesis + ADRs · tickets in `.scratch/orexis/issues/` · failing test first · two-axis review · repro-command-first bugs · architecture review (`clean-architecture`) at every gate · UI audit (`designing-interfaces`) at M1/M2 · market review (`market-research`) at M6 and M9.

---

## 14. Decided and open

**Decided, not re-litigable:** Rust; async ports over a runtime-agnostic core; sans-io protocols; `tokio` banned inward; ports consumer-side; adapters only in composition; strategies are use cases; workers share the entity's identity; concurrency is provider-bounded; single-agent default, team opt-in; MCP + ACP in v1; no code RAG; **inference location is per-profile, chosen at init**; **provider breadth via profiles and a catalog**; **sovereign state + model-agnostic inference** as the position.

**Open (study outputs):** session schema; codec set beyond the ported four; TUI/desktop layout details; entity model specifics; evolution tier defaults; A2A/UTCP timing; packaging.

---

## 15. Sources

Mordor Intelligence (enterprise AI coding agent market); Gartner; Stack Overflow Developer Survey 2026; JetBrains AI adoption 2026; TechCrunch (Cursor, Cognition); arXiv 2609.00006 (eleven-system harness study); MCP adoption census 2026; A2A v1.0; CSA research (Clinejection, supply chain, Claude Code action); Pillar Security (TrustIssues); OWASP GenAI Q1 2026; IDEsaster; Anthropic *Building Effective Agents*; Rajasekaran et al.; Hadfield et al.; Lin et al.; repos `NousResearch/hermes-agent`, `anomalyco/opencode`, `deepseek-ai/deepseek-harness`, `cosmicstack-labs/mercury-agent`, `KorroAi/mue-x`, `openai/codex`, `google-gemini/gemini-cli`, `aaif-goose/goose`, `OpenHands/OpenHands`.
