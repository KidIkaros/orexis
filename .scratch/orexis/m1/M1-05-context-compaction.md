# M1-05 — Context assembly and compaction

- **Blocked by:** M1-01, M1-02
- **Blocks:** M1-06, M1-07, M1-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Assemble the model-visible request deterministically — system prompt, tool docs, project context files, and history — and compact at a fixed threshold with a verbatim tail and an incremental summary.

## Acceptance

- [x] Assembly is a pure function of the profile, tools, context files, and events.
- [x] The prompt prefix is stable within a session; only compression changes it.
- [x] `AGENTS.md`, `CLAUDE.md`, and `.orexis/CONTEXT.md` are discovered from root to cwd and injected by scope.
- [x] Compaction fires at a fixed buffer (8,192 tokens) below the model's context; the last 8 events stay verbatim.
- [x] The summary merges incrementally (it folds a prior `Compaction` summary in); the event references the range.
- [x] Compaction is the only overflow path; hard truncation is deliberately not implemented.
- [x] Golden tests cover assembly order, compaction boundaries, and short-history skip.

## Notes

- `ExtractiveSummarizer` is deterministic for M1; an LLM summarizer replaces it behind the same `Summarizer` port.
- The assembled system prompt is prepended as a `System` message; OpenAI maps it to `system`, Anthropic to its top-level `system` field.
- `FileContextSource` lives in `orexis-adapters-context-fs`; discovery walks root → cwd in order.
