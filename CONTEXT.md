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
| **Strategy** | A use case: `plan`, `build`, `review`, `mine` | An agent type |
| **Board** | Durable tasks with owner, required strategy, blocking edges | A prompt |
| **Bus** | Direct and room messages, persisted as events | Ephemeral chat |
| **Blackboard** | Shared workspace, artifacts, and leases | A shared prompt |
| **Seam** | A capability's definition + provider + consumer | A plugin |
| **Port** | A trait declared by the consumer side | An adapter |
| **Adapter** | A concrete implementation of a port (provider, store, sandbox, protocol, UI) | A port |
| **ProviderProfile** | Provider data: endpoint, auth, dialect, cache, thinking, models | An adapter |
| **Session event** | A durable fact in the event log; model-visible if the model saw it | Telemetry |
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
