# 016 — Delimit built-in tool output too

- **Type:** feature
- **Blocks:** —
- **Status:** open
- **Claimed by:** —

## Question

M7-04 implemented untrusted-content delimiting for external tools (mcp/,
utcp/) at the executor boundary. Built-in tool output (read_file, search,
shell, list_dir) still enters the model context raw — capped and
hardline-filtered, but not delimited. A repo file containing
"ignore previous instructions" is the same injection class as hostile MCP
output (AGENTS.md security: "tool output … is delimited").

## Evidence

- AGENTS.md security section and PLAN invariant 22: tool output is
  untrusted content, generally.
- M4 adversarial fixtures cover caps and hardline commands, not delimiting.
- M7-04's `<mcp_result>` wrapper is the working pattern, including
  delimiter-breakout escaping.

## Acceptance

- [ ] Built-in tool output is delimited at the executor boundary (the log
      carries what the model saw; replay stays byte-identical).
- [ ] Breakout escaping applies to the built-in delimiter too.
- [ ] Adversarial fixture: a repo file with injected instructions renders
      delimited in the model context.
- [ ] Replay fixtures still byte-identical (the delimiter is inside the
      logged observation).

## Notes

Found during M7-04; the generalization is a model-context behavior change
for built-ins, so it is its own ticket, not a silent refactor.
