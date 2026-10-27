# M8-05 — Strategies as use cases (plan/build/review/mine)

- **Blocked by:** M8-01
- **Blocks:** M8-06, M8-07
- **Status:** open

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
