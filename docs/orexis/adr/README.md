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
