# opencode — study

**Source:** `github.com/anomalyco/opencode` · **Language:** TypeScript · **Size:** large monorepo · **License:** MIT · **Last seen:** upstream main

**Why it is in the study:** it is the best open-source example of a client/server harness with code-mode, event-sourced sessions, permission rulesets, and a model catalog — the interaction patterns Orexis's TUI and desktop need.

## Known surface

| Area | What it does |
|---|---|
| Client/server | `opencode serve` embeds an HTTP server with an OpenAPI spec and generated SDK; TUI, desktop, web, IDE, CI are clients |
| ACP | `opencode acp` serves the agent side over stdio |
| Code-mode | `packages/codemode`: a restricted interpreter lets the model make multiple tool calls in one program, cutting round trips |
| Sessions | Event-sourced; branch, revert, fork; child sessions for sub-agents |
| Permissions | Rulesets; syntax-aware command permissioning (tree-sitter, argument arity) |
| Agents | Build/plan/general personas; permission-derived child capabilities |
| Models | `models.dev` catalog; nine model-family prompts; cache-dialect fanout |
| TUI | Ink; performance fixes shared with Mercury's fork |

## Dimensions to fill

1. **Process model** — server + clients; how state is shared; how a client reconnects.
2. **Turn loop** — log-as-queue; how a session drains events.
3. **Session model** — event schema; branch/revert/fork; shadow-git checkpoints per step.
4. **Tool protocol** — native calls plus code-mode; how a program's calls are admitted.
5. **Context management** — anchored incremental summaries; cache dialect per provider.
6. **Provider abstraction** — models.dev; transforms; nine prompt families.
7. **Entity substrate** — absent; personas are configuration.
8. **Evolution** — none.
9. **Interop** — ACP server; MCP; reads `~/.claude/skills`.
10. **Surfaces** — TUI, desktop, web, IDE, SDK.
11. **Quality practice** — generated API/SDK contracts; tests.
12. **Failure modes** — Effect/TS complexity; permissive permissions by default.

## Known evidence

- The client/server split is the model for Orexis's daemon + TUI + desktop.
- Code-mode is a real round-trip reducer; it belongs in Orexis as a program surface, not the default.
- Session branch/revert/fork and per-step shadow-git are the patterns behind `/undo`, `/branch`, `/rollback`.
- The model catalog and prompt matrix are the provider-breadth reference.

## Adopt

- **Client/server with a thin client** and a generated protocol contract.
- **Event-sourced sessions** with branch/revert/fork.
- **Permission rulesets** with syntax-aware command parsing.
- **Model catalog + model-family prompt matrix + cache-dialect fanout.**
- **Code-mode** as an opt-in program surface.
- **ACP server.**

## Adapt

- The session event schema, made versioned and replayable.
- Code-mode, sandboxed per Orexis's fresh-process default.

## Reject

- The Effect/TypeScript runtime.
- Permissive-by-default permissions.

## Open questions

- How does code-mode sandbox the interpreter, and what can a program reach?
- How are branch/revert/fork represented in the event log?
- How does the server admit concurrent sessions on one provider?
