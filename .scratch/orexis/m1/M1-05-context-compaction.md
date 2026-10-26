# M1-05 — Context assembly and compaction

- **Blocked by:** M1-01, M1-02
- **Blocks:** M1-06, M1-07, M1-08
- **Status:** open

## Goal

Assemble the model-visible request deterministically — system prompt, tool docs, project context files, and history — and compact at a fixed threshold with a verbatim tail and an incremental summary.

## Acceptance

- [ ] Assembly is a pure function of the profile, tools, context files, and events.
- [ ] The prompt prefix is stable within a session; only compression changes it.
- [ ] `AGENTS.md` and nested context files are discovered and injected by scope.
- [ ] Compaction fires at a fixed buffer below the model's context; the last N turns stay verbatim.
- [ ] The summary merges incrementally; a `Compaction` event references the range.
- [ ] A request never exceeds the model's context; overflow triggers compaction, not truncation.
- [ ] Golden tests cover assembly order and compaction boundaries.

## Notes

- No vector RAG; context files and history only.
- Compaction is a generation trigger (ADR-0001).
