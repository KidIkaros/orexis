# M1 — implementation tickets

Spec: [`docs/orexis/specs/M1.md`](../../../docs/orexis/specs/M1.md).

Work blockers-first. Claim a ticket before starting. Each ticket closes only when its acceptance checklist is green.

| Ticket | Goal | Blocked by |
|---|---|---|
| [M1-01](M1-01-session-store.md) | SQLite session store (ADR-0001) | — |
| [M1-02](M1-02-provider-catalog-openai.md) | Provider catalog + OpenAI-compatible adapter (ADR-0002) | — |
| [M1-03](M1-03-anthropic-llamacpp.md) | Anthropic + llama.cpp adapters | M1-02 |
| [M1-04](M1-04-codecs.md) | Codecs + selection + fixtures (ADR-0003) | M1-02 |
| [M1-05](M1-05-context-compaction.md) | Context assembly + compaction | M1-01, M1-02 |
| [M1-06](M1-06-tui.md) | ratatui TUI (ADR-0004) | M1-01, M1-02, M1-05 |
| [M1-07](M1-07-daemon-attach.md) | Daemon + attach | M1-01, M1-02, M1-06 |
| [M1-08](M1-08-gate.md) | Import-lint, seam graph, M1 gate | M1-01 … M1-07 |

## Per-ticket loop

1. Failing test that encodes the acceptance check.
2. Smallest green.
3. Two-axis review: Standards (`AGENTS.md`, `CONTEXT.md`) and Spec (this ticket).
4. `verification-loop` before closing.

## Frontier order

M1-01 and M1-02 are both unblocked and independent. Start either; they converge at M1-05.
