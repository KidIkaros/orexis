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
9. [Native adapter timing: Gemini / Bedrock / Vertex](009-native-adapter-timing.md)
10. [Packaging and signing](010-packaging-signing.md)
11. ~~[Eight per-repo study decisions](011-per-repo-study-decisions.md)~~ — resolved; synthesis drafted.
