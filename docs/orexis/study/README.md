# The Orexis study

Eight repositories, read against the same twelve dimensions, to decide what Orexis adopts, adapts, or rejects.

## Order

**Internal first** — you know the code and its scars:

1. [Kindred](kindred.md) — the entity substrate, orchestration, MCP/UTCP clients
2. [Harness](harness.md) — the tool loop, codec, providers, eval
3. [Familiar](familiar.md) — the failure analysis

**Then the external five:**

4. [DeepSeek Harness](deepseek-harness.md) — seams, event domains, logged invariant, versioned log, PTC
5. [opencode](opencode.md) — client/server, code-mode, sessions, permissions, TUI
6. [Hermes](hermes.md) — provider breadth, plugin API, skills, gateways, ACP
7. [MUE-X](mue-x.md) — evolution, drives, PAD, memory lattice
8. [Mercury](mercury.md) — installer, budget, typed memory, channels

## Method

- One document per repo, using [`_template.md`](_template.md).
- Every claim cites a file, a line, or a URL. No vibes.
- Each document ends with **adopt / adapt / reject**, with a reason per item.
- Where a repo contradicts another, record the conflict in [`synthesis.md`](synthesis.md) rather than resolving it in the repo doc.

## Research

Each repo gets one `/research` artifact. Research feeds the doc; it does not replace reading the load-bearing modules.

## Synthesis

[`synthesis.md`](synthesis.md) collects:

- the pattern catalog (adopt / adapt / reject, with sources),
- the conflict list,
- the porting ledger,
- the architecture decisions that follow.

The synthesis is the input to `docs/orexis/adr/` and to the map's *Decisions so far*.
