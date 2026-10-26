# Mercury — study

**Source:** `github.com/cosmicstack-labs/mercury-agent` · **Language:** TypeScript · **Size:** moderate · **License:** MIT · **Last seen:** upstream main

**Why it is in the study:** it ships the user-facing basics Orexis needs — a one-line installer, a daily token budget with cheap-model routing, typed memory with confidence and conflict resolution, per-folder permissions, and a multi-channel daemon. It also fixed the exact TUI ceiling Orexis will hit.

## Known surface

| Area | What it does |
|---|---|
| Installer | One-line install, checksum, attach UX |
| Soul | Markdown persona |
| Memory | SQLite "Second Brain": typed facts, confidence, conflict resolution, FTS |
| Safety | Per-folder permissions, shell blocklists, approvals, SSRF guards |
| Budget | Daily token budget, cheaper-model routing |
| Daemon | Background process with CLI, Telegram, Discord, Slack, Signal, web |
| Code | "Mercury Code" only reports completion after tests/build/typecheck |
| Bots | Persistent bots with separate persona/model/memory/budget/schedules |
| TUI | Ink fork with `<Static itemKey>`, freeze gate, live-region guard, IME cursor anchor |

## Dimensions to fill

1. **Process model** — daemon + channels; how a client attaches.
2. **Turn loop** — one loop per bot?
3. **Session model** — what is persisted per bot and per channel.
4. **Tool protocol** — native tool calls.
5. **Context management** — what the Second Brain injects.
6. **Provider abstraction** — routing by cost; fallbacks.
7. **Entity substrate** — soul markdown; no drives/affect.
8. **Evolution** — none.
9. **Interop** — skills registry; Hermes-compatible skills.
10. **Surfaces** — CLI, channels, web, TUI.
11. **Quality practice** — verify-before-done; installer tests.
12. **Failure modes** — TS stack; cloud-first assumptions.

## Known evidence

- The installer and attach UX are the adoption lesson: distribution beats architecture.
- The daily budget with cheap-model routing is the cost-control pattern Orexis needs.
- Typed memory with confidence and conflict resolution is a stronger model than a flat fact list.
- The Ink fixes (`<Static itemKey>`, freeze gate, live-region guard, IME anchor) are the exact TUI problems Orexis's ratatui renderer will face.
- Mercury's skills registry explicitly curates for Hermes — interoperability, not competition.

## Adopt

- **One-line installer + checksum + attach UX.**
- **Daily token budget + cheap-model routing.**
- **Typed memory with confidence and conflict resolution.**
- **Per-folder permissions and SSRF guards.**
- **Verify-before-done** ("Mercury Code" discipline).
- **Ink fixes as reference** for ratatui's render strategy.

## Adapt

- The budget model into Orexis's per-profile and per-worker budgets.
- The bot model into workers (not separate entities).

## Reject

- The TypeScript runtime.
- Cloud-first assumptions.
- Channel breadth in v1.
- Separate persona/memory per bot (Orexis workers share one entity).

## Open questions

- How is the daily budget accounted and enforced across channels?
- How does conflict resolution pick a winner between contradictory facts?
- How does the daemon attach a new client without replaying everything?
