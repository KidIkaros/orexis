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
| [M1-09](M1-09-codec-integration.md) | Wire codecs into the loop | M1-10, M1-11 |
| [M1-10](M1-10-fallible-ports.md) | Fallible provider/tool ports + ProviderResponse | — |
| [M1-11](M1-11-tool-pairing.md) | Tool-call pairing (`call_id`) | — |
| [M1-12](M1-12-split-usecases.md) | Split `usecases` into modules | M1-09 |
| [M1-13](M1-13-split-sqlite.md) | Split the SQLite session store | — |
| [M1-14](M1-14-compaction-range.md) | Correct the compaction range | — |
| [M1-15](M1-15-adr-shared-http.md) | ADR: shared adapter infrastructure | — |
| [M1-16](M1-16-tui-interactive.md) | TUI interactive loop and polish | M1-07 |
| [M1-17](M1-17-composition-correctness.md) | Composition correctness | — |
| [M1-18](M1-18-daemon-boundary.md) | Daemon composition boundary | — |
| [M1-19](M1-19-lint-hardening.md) | Boundary lint hardening | — |
| [M1-20](M1-20-split-tui.md) | Split the TUI modules | — |
| [M1-21](M1-21-split-catalog.md) | Split the provider catalog | M1-17 |
| [M1-22](M1-22-docs-status.md) | Refresh README and AGENTS status | — |
| [M1-23](M1-23-models-dev-import.md) | models.dev catalog import | — |
| [M1-24](M1-24-hermes-provider-overrides.md) | Hermes provider overrides | M1-23 |

## Per-ticket loop

1. Failing test that encodes the acceptance check.
2. Smallest green.
3. Two-axis review: Standards (`AGENTS.md`, `CONTEXT.md`) and Spec (this ticket).
4. `verification-loop` before closing.

## Frontier order

M1-01 and M1-02 are both unblocked and independent. Start either; they converge at M1-05.
