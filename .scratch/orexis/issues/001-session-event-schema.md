# 001 — Session event schema and migration format

- **Type:** grilling
- **Blocks:** M1
- **Status:** open
- **Claimed by:** —

## Question

What is the canonical session event schema, how is it versioned, and how are migrations applied so that replay is byte-identical across versions?

## Evidence

- Plan §4.5 invariants 1–2: model-visible means logged; `derive_messages()` is pure; format versioned with adjacent migrations; committed generations immutable.
- DeepSeek Harness: versioned immutable generations, migrations adjacent to the format.
- Harness `crates/cli/src/session.rs`: SQLite sessions with event sequence; no versioning.
- Hermes `state.db`: schema versions; messages, system prompts, generations.
- opencode: event-sourced sessions with branch/revert/fork.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
