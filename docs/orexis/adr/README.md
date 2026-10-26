# Architecture Decision Records

An ADR records a decision that is hard to reverse, cross-cutting, or a reversal of a settled decision. It is not a plan revision and not a meeting note.

## When to write one

- The decision changes a dependency direction, a port, a storage format, or a security boundary.
- The decision reverses something in the map's *Decisions so far*.
- The decision is expensive to undo after code exists.

## Format

`ADR-NNNN-short-title.md`:

```markdown
# ADR-NNNN — Title

- **Status:** proposed | accepted | superseded by ADR-NNNN
- **Date:** YYYY-MM-DD
- **Deciders:** <names>

## Context

What forces are at play, and what evidence do we have?

## Decision

What we will do, stated so a reader can act on it.

## Consequences

What becomes easier, what becomes harder, and what we are explicitly not doing.

## Alternatives considered

What else was on the table, and why it lost.
```

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-session-event-model.md) | Session event model and versioning | accepted |
| [0002](0002-provider-catalog.md) | Provider catalog and profile | accepted |
| [0003](0003-codec-set.md) | Codec set and selection | accepted |
| [0004](0004-tui-surface.md) | TUI surface contract | accepted |
| [0005](0005-shared-adapter-infrastructure.md) | Shared adapter infrastructure | accepted |
| [0006](0006-daemon-app-layer.md) | The daemon is an app layer | accepted |
| [0007](0007-shared-contract-crate.md) | Shared contract crate | accepted |
| [0008](0008-desktop-surface.md) | Desktop surface contract | accepted |
| [0009](0009-secrets-crypto.md) | Secrets and crypto boundary | accepted |
| [0010](0010-shared-outer-layer.md) | Shared outer-layer crates | accepted |
| [0011](0011-approval-overrides.md) | Approval overrides as session-scoped policy data | accepted |
| [0012](0012-oversight-projections.md) | M2 oversight projections vs the M8 team domain | accepted |
| [0013](0013-entity-model.md) | Entity model: six drives and PAD affect | accepted |
| [0014](0014-identity-vs-persona.md) | Entity identity is not model persona | accepted |
