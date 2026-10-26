# 001 — Session event schema and migration format

- **Type:** grilling
- **Blocks:** M1
- **Status:** resolved
- **Claimed by:** devin, 2026-10-25

## Question

What is the canonical session event schema, how is it versioned, and how are migrations applied so that replay is byte-identical across versions?

## Evidence

- Plan §4.5 invariants 1–2: model-visible means logged; `derive_messages()` is pure; format versioned with adjacent migrations; committed generations immutable.
- DeepSeek Harness: versioned immutable generations, migrations adjacent to the format.
- Harness `crates/cli/src/session.rs`: SQLite sessions with event sequence; no versioning.
- Hermes `state.db`: schema versions; messages, system prompts, generations.
- opencode: event-sourced sessions with branch/revert/fork.

## Resolution

**Decided 2026-10-25.** See `docs/orexis/adr/0001-session-event-model.md` for the full record.

- **Granularity:** durable medium. Events: `SessionStart`, `ModelRequest`, `ModelResponse`, `ToolCall`, `Observation`, `ApprovalRequest`, `ApprovalDecision`, `Compaction`, `Fork`, `ModelSwitch`, `Error`, `SessionEnd`. Token deltas are ephemeral and never stored as events.
- **Versioning:** one `format_version` per session; every event carries `kind` + `version`.
- **Migrations:** lazy read-time migration; explicit `orexis migrate` (and close) rewrites to the current format. Generations are immutable.
- **Generation triggers:** compaction, fork, model switch, schema migration.
- **Storage:** SQLite (WAL) is canonical; JSONL is export/import and inspection.
- **Identity:** monotonic `seq` per session for ordering; ULID `event_id` for dedupe and export.
- **Model-visible request:** the assembled request is stored verbatim as a `ModelRequest` event, with a hash and the provider profile id.
- **Compaction:** append a `Compaction` event referencing the compacted range and carrying the summary; the log is never destructively rewritten.
- **Fork:** new session id with `parent_id` + `fork_seq`; the prefix is copied so exports are self-contained.
- **Secrets:** redacted at write; a hash and redaction marker preserve audit.
- **Retention:** indefinite by default; explicit `prune` only.
- **Durability:** one transaction per event batch, fsync before acknowledging, resume from the last `seq`.

**Shape:**

```
session(id, parent_id?, fork_seq?, format_version, profile_id, model_id, created_at, updated_at)
event(session_id, seq, event_id, kind, version, generation_id, payload_json, created_at)
generation(session_id, generation_id, starts_at_seq, reason)
```

`derive_messages(session_id, up_to_seq)` is pure: it applies `Compaction` and `ModelSwitch` events and never reads telemetry.

## Follow-ups

- ADR-0001 written.
- M1 implements the schema and the derivation function.
- Fog cleared: nothing new; storage and lifecycle were the remaining unknowns.
