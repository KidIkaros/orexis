# M8-05 — Strategies as use cases (plan/build/review/mine)

- **Blocked by:** M8-01
- **Blocks:** M8-06, M8-07
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

Strategy definitions (ADR-0020, invariant 16): `{id, prompt fragment,
toolset filter, verify policy, budget defaults}` for plan, build, review,
mine. The `build` strategy carries Kindred's planner → executor →
verifier shape with `Clarify` instead of guess-calling. A strategy's
toolset filter can only narrow the permission gate's verdicts, never
widen.

## Acceptance

- [ ] The four strategies defined as data + use cases.
- [ ] The toolset filter narrows only (a test proves the floor survives).
- [ ] The prompt fragments are bounded (no unbounded prompt growth).

## Landed

- `usecases/src/strategy.rs` (new): `StrategyId` (plan/build/review/mine,
  parse + name), `Strategy` (prompt fragment, toolset, verify policy,
  budget defaults), `strategy(id)` — the four definitions. The `build`
  strategy is the open one; the others narrow to read-shaped toolsets.
- `core/src/team.rs`: `ToolsetFilter` — the filter is domain data (core),
  so adapters can enforce it without a usecases dependency.
- `adapters-tools/src/strategy_tools.rs` (new): `StrategyTools` — the
  narrowing wrapper over any executor; a tool outside the allowlist is
  refused before the gate is asked; the gate's verdicts are untouched.
- Tests: the four strategies are bounded; parse round-trips and refuses
  unknowns; the filter narrows by name and namespace prefix (with a
  lookalike rejection); an open filter passes through; the wrapper
  refuses narrowed tools.
