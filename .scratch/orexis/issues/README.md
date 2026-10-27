# Decision tickets

One file per decision on the map's **Frontier**. A ticket is a question whose resolution is a decision — not a slice of a build.

## Naming

`NNN-short-name.md`, numbered in frontier order (e.g. `001-session-event-schema.md`).

## Format

```markdown
# NNN — Title

- **Type:** research | prototype | grilling | task
- **Blocks:** M1, M2, ...
- **Status:** open | claimed | resolved
- **Claimed by:** <name/date>

## Question

<the decision this ticket resolves, sized to one session>

## Evidence

<links, file:line references, research artifacts>

## Resolution

<filled when resolved: the decision, the reasoning, and what it unblocks>

## Follow-ups

<new tickets or fog this resolution creates>
```

## Rules

- **One ticket per session.** Claim it before working.
- Resolving a ticket **appends one row** to the map's *Decisions so far* and closes the ticket.
- A resolution that creates new work graduates fog into new tickets; it does not reopen this one.
- A ticket that turns out to sit beyond the destination is closed and listed under *Out of scope*.
- Never resolve a ticket by rewriting the plan. The plan changes only when the frontier or fog changes.

## Frontier (from `docs/orexis/PLAN.md` §Map)

1. ~~[Session event schema and migration format](001-session-event-schema.md)~~ — resolved; ADR-0001.
2. ~~[`ProviderProfile` schema and catalog source](002-provider-profile-schema.md)~~ — resolved; ADR-0002.
3. ~~[Codec set beyond the ported four](003-codec-set.md)~~ — resolved; ADR-0003.
4. ~~[TUI layout and keymap](004-tui-layout-keymap.md)~~ — resolved; ADR-0004.
5. [Edit-tool contract per model tier](005-edit-tool-contract.md)
6. [Desktop panel layout in Leptos](006-desktop-panel-layout.md)
7. [Entity model: five vs six drives, affect dimensions](007-entity-model.md)
8. [Evolution tier defaults and validation ladder](008-evolution-tier-defaults.md)
9. ~~[Native adapter timing: Gemini / Bedrock / Vertex](009-native-adapter-timing.md)~~ — resolved; ADR-0016.
10. ~~[Packaging and signing](010-packaging-signing.md)~~ — resolved; ADR-0019.
11. ~~[Eight per-repo study decisions](011-per-repo-study-decisions.md)~~ — resolved; synthesis drafted.
12. ~~[Neuron alignment/steering](012-neuron-alignment.md)~~ — research track; ADR-0014.
13. ~~[Per-dialect provider conformance fixtures](013-provider-wire-conformance-fixtures.md)~~ — resolved; ADR-0018.
14. [`init` and `providers` surface implemented auth kinds](014-init-auth-kind-visibility.md)
15. ~~[Serve `/evolve` through the `EvolutionStore` port](015-evolve-through-the-port.md)~~ — resolved; the handler goes through the port, last 500 entries.
16. ~~[Delimit built-in tool output too](016-delimit-builtin-tool-output.md)~~ — resolved; invariant 22 now covers built-ins at the executor boundary.
17. ~~[Emit thinking configuration per profile](017-thinking-config.md)~~ — resolved; the port carries the thinking request, the posture is the source, adapters own the wire mapping.
18. ~~[Anthropic message-level cache breakpoints](018-anthropic-message-cache.md)~~ — resolved; the system prefix carries the breakpoint, the tail waits for measured evidence.
19. [`orexis` binary consolidation](019-binary-consolidation.md)
20. [Release pipeline](020-release-pipeline.md)
21. [`orexis upgrade`](021-upgrade-command.md)
