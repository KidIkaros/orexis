# M3 — implementation tickets

Spec: [`docs/orexis/specs/M3.md`](../../../docs/orexis/specs/M3.md).

Work blockers-first. Claim a ticket before starting. Each ticket closes only when its acceptance checklist is green.

| Ticket | Goal | Blocked by |
|---|---|---|
| [M3-01](M3-01-core-tools.md) | Core tools + path sandbox | — |
| [M3-02](M3-02-permissions.md) | Permission ruleset + plan gate | — |
| [M3-03](M3-03-shell.md) | Sandboxed shell | M3-02 |
| [M3-04](M3-04-fuzzy-edit.md) | Fuzzy edit cascade | M3-01 |
| [M3-05](M3-05-git-checkpoints.md) | Git tools + turn checkpoints | M3-01 |
| [M3-06](M3-06-verify.md) | Verify-on-stop | M3-03, M3-05 |
| [M3-07](M3-07-lsp.md) | LSP diagnostics | M3-01 |
| [M3-08](M3-08-gate.md) | M3 gate | M3-01 … M3-07 |

## Start

M3-01 and M3-02 are both unblocked. M3-01 shipped; M3-02 is next.
