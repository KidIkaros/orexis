# Context — Orexis ubiquitous language

Every document, ticket, and commit uses these words in exactly these senses. If a word is missing, add it here before using it.

## Position

| Term | Means | Never means |
|---|---|---|
| **Orexis** | The product and default agent name | The upstream projects it borrows from |
| **Sovereign** | Your state, identity, memory, sessions, and keys live locally and export fully; no telemetry; no account required | Local-only inference |
| **Model-agnostic** | Inference runs wherever the user points it: any configured provider, remote or local | One blessed provider |
| **Living entity** | The persistent identity layer: identity, drives, affect, memory, evolution | A persona file; a companion/relationship product |

## Architecture

| Term | Means | Never means |
|---|---|---|
| **Entity** | The singular identity layer (identity, drives, affect, memory) | A worker |
| **Worker** | A generic execution instance with its own context, model, toolset, budget | A role |
| **Strategy** | A use case: `plan`, `build`, `review`, `mine` | An agent type; an execution posture |
| **Drive** | A slow [0,1] pressure that proposes work: connection, curiosity, mastery, coherence, rest, growth | An emotion; a prompt instruction |
| **Affect** | Fast PAD state (valence, arousal, dominance) that modulates posture | A mood label; a prompt |
| **Posture** | Turn-boundary execution parameters: model tier, thinking, repair budget, verification, risk appetite, exploration | A Strategy (use case) |
| **Mood** | A display-only label derived from drives and affect | Persisted state |
| **Glial** | Deterministic affect-to-drive/posture modulation | A separate agent |
| **Board** | Durable tasks with owner, required strategy, blocking edges (M8) | A prompt |
| **Fleet** | The set of workers in team mode (M8); in M2, the session projection | A single worker |
| **Artifact** | A durable output: diff, log, generated file (M8 blackboard) | A transient tool result |
| **Session projection** | An oversight view derived from sessions (M2 fleet/board/changes) | A Worker, Task, or Artifact |
| **Approval override** | A session-scoped allowance for one tool (ADR-0011) | A policy rewrite |
| **Bus** | Direct and room messages, persisted as events | Ephemeral chat |
| **Blackboard** | Shared workspace, artifacts, and leases | A shared prompt |
| **Seam** | A capability's definition + provider + consumer | A plugin |
| **Port** | A trait declared by the consumer side | An adapter |
| **Adapter** | A concrete implementation of a port (provider, store, sandbox, protocol, UI) | A port |
| **ProviderProfile** | Provider data: endpoint, auth, dialect, cache, thinking, models | An adapter |
| **Session event** | A durable fact in the session log; model-visible if the model saw it | Telemetry |
| **Entity event** | A durable fact in the entity log (drives, affect, diary, traits, memory) | A session event |
| **Memory lattice** | Episodic, semantic, procedural, and identity memory; lexical recall | A vector store |
| **Trait** | An evidence-backed identity statement with confidence and decay | An agent-authored soul file |
| **Diary** | The append-only identity narrative traits are distilled from | A prompt |
| **Entity store** | The local append-only entity log and projections | A session store |
| **Telemetry** | Non-truth analytics; never consulted for correctness | Session events |
| **Sans-io** | A parser or state machine with no I/O and no runtime | An async module |

## Process

| Term | Means | Never means |
|---|---|---|
| **Tier** | Evolution scope: `off`, `skills`, `quarantine`, `core` | A pricing tier |
| **Milestone** | A staged deliverable with its own exit gate (M1–M9) | A release |
| **Release candidate** | The milestone that ships when the next one misses its gate twice | A preview |
| **Frontier** | The open, unblocked decisions in the map, in order | The roadmap |
| **Fog** | In-scope work not yet sharp enough to ticket | Out of scope |
| **Out of scope** | Work ruled beyond the destination; never graduates | Fog |
| **ADR** | A recorded, hard-to-reverse decision | A plan revision |
| **Ticket** | One decision, sized to one session | A build task |
