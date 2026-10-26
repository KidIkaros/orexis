# ADR-0005 — Shared adapter infrastructure

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/issues/../m1/M1-15-adr-shared-http.md`

## Context

The plan's Dependency Rule says adapters depend on `core` + `ports`, never on each other. But the OpenAI-compatible and Anthropic adapters both need HTTP with streaming, headers, and error mapping. Duplicating that in each adapter is worse than one shared implementation; moving HTTP into `ports` would leak a transport detail into the inner circle, which the rule exists to prevent.

## Decision

A **shared adapter-infrastructure crate** may be depended on by adapters. It must:

1. live under `orexis-adapters-*` (here, `orexis-adapters-http`);
2. depend only on external libraries, not on domain crates;
3. expose no domain types — only transport primitives (`Transport`, `ByteStream`, `TransportError`);
4. never be depended on by `core`, `ports`, or `usecases`.

Provider adapters may depend on shared adapter infrastructure, but **not on each other**. `usecases` may not depend on any `orexis-adapters-*`.

`orexis-adapters-http` is the first such crate: `ReqwestTransport`, `MockTransport`, and `FailingTransport`.

## Consequences

- One transport implementation, tested once, used by both wires.
- The Dependency Rule still holds for `core`, `ports`, and `usecases`.
- Import-lint allows `adapters-* -> adapters-http` and still forbids `core/ports/usecases -> adapters-*`.
- A future shared adapter (an SSE parser, a shared tool-call accumulator) follows the same rule.
- The exception is narrow: infrastructure only, no domain types, no provider-to-provider dependency.

## Alternatives considered

- **Duplicate transport per adapter** — rejected: two copies of streaming, headers, and error mapping drift.
- **Move `Transport` into `ports`** — rejected: it leaks HTTP into the inner circle.
- **A `support` crate outside the adapter namespace** — the same exception, less discoverable; rejected in favour of the `adapters-*` namespace.
- **A provider base class** — Rust has no inheritance, and it would couple provider adapters to each other; rejected.
