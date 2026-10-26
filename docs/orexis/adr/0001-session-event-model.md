# ADR-0001 — Session event model and versioning

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/issues/001-session-event-schema.md`

## Context

Orexis must satisfy two invariants: **model-visible means logged** and **replay is byte-identical**. It must also survive format changes without rewriting history, support fork and compaction, and keep secrets out of the log. Prior art diverges: DeepSeek uses immutable versioned generations with adjacent migrations; Hermes uses SQLite `state.db` with schema versions and lineage compaction; Harness uses SQLite sessions with a sequence but no versioning; opencode is event-sourced with branch/revert/fork.

## Decision

**Event granularity.** Durable medium. Events are `SessionStart`, `ModelRequest`, `ModelResponse`, `ToolCall`, `Observation`, `ApprovalRequest`, `ApprovalDecision`, `Compaction`, `Fork`, `ModelSwitch`, `Error`, `SessionEnd`. Token deltas are ephemeral and are never stored as events; the UI may stream them, but replay reconstructs from `ModelResponse`.

**Versioning.** One `format_version` per session; every event carries `kind` + `version`.

**Migrations.** Lazy read-time migration for reads; an explicit `orexis migrate` (and session close) rewrites to the current format. Generations are immutable; old readers can still open old sessions.

**Generation triggers.** Compaction, fork, model switch, and schema migration each start a new generation.

**Storage.** SQLite with WAL is the canonical log. JSONL is the export/import and inspection format. One writer per session; WAL allows concurrent readers.

**Identity.** A monotonic `seq` per session orders events and drives replay; a ULID `event_id` dedupes across exports and imports.

**Model-visible request.** The assembled request is stored verbatim as a `ModelRequest` event, with a hash and the provider profile id. Replay compares byte-for-byte and catches drift.

**Compaction.** Append a `Compaction` event referencing the compacted range and carrying the summary. The log is never destructively rewritten; derivation applies the compaction.

**Fork.** A new session id with `parent_id` + `fork_seq`; the prefix is copied so exports are self-contained.

**Secrets.** Redacted at write; a hash and a redaction marker preserve audit. Raw secrets never enter the log.

**Retention.** Indefinite by default; only an explicit `prune` removes sessions.

**Durability.** One transaction per event batch; fsync before acknowledging; resume from the last `seq` after a crash.

**Shape:**

```
session(id, parent_id?, fork_seq?, format_version, profile_id, model_id, created_at, updated_at)
event(session_id, seq, event_id, kind, version, generation_id, payload_json, created_at)
generation(session_id, generation_id, starts_at_seq, reason)
```

`derive_messages(session_id, up_to_seq)` is pure: it applies `Compaction` and `ModelSwitch` events and never reads telemetry.

## Consequences

- Replay is exact and auditable; a provider change is visible in the log.
- Old sessions remain readable after a format change; no big-bang migration.
- Compaction and fork are additive, so history is never lost.
- Storage grows indefinitely unless the user prunes; that is deliberate (continuity is the entity's point).
- Token-delta replay is not stored; the UI must reconstruct streaming from `ModelResponse`.
- Fork copies the prefix; storage cost is the price of self-contained exports.

## Alternatives considered

- **Coarse per-turn events** — fewer rows, but replay cannot isolate a single model call or tool call.
- **Fine events with token deltas** — exact streaming replay, at a large storage cost; rejected because deltas are derivable.
- **Per-event versioning only** — no session-level migration target; rejected.
- **Eager migration on open** — simple, but mutates on read and blocks opening old sessions.
- **Destructive compaction** — smaller log, but violates the immutability and audit requirements.
- **Reference-only forks** — storage-efficient, but retention and export must chase references.
