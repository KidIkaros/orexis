# M3-03 — Sandboxed shell

- **Blocked by:** M3-02
- **Blocks:** M3-06, M3-08
- **Status:** open

## Goal

A `shell` tool confined to the project root: timeout, per-stream output caps, process group, no orphans, no inherited secrets.

## Acceptance

- [ ] cwd is the project root; commands run in a new process group.
- [ ] Timeout (120 s default) kills the group; no orphan survives.
- [ ] stdout/stderr cap at 64 KB each with a marker.
- [ ] A deny list and the permission ruleset gate execution.
- [ ] Environment is scrubbed of provider keys and tokens.
- [ ] Tests: timeout, output bomb, group kill, deny.
