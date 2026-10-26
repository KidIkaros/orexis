# ADR-0011 — Approval overrides as session-scoped policy data

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** ADR-0002 (providers), M2-10 audit finding

## Context

The M2-02 audit found that "always" was implemented as a daemon-wide
`HashSet<String>` inside `ApprovalHub` — a policy decision in the transport
layer. It leaked across sessions and had no revocation path.

## Decision

1. **`ApprovalOverrides` is a port** (`orexis-ports`):
   `allows(session, tool)`, `remember(session, tool)`, `clear(session)`.
2. **Overrides are session-scoped by default.** `HubGate` carries the session
   id; the daemon builds one gate per turn from the session runtime.
3. **`Approve { always }` remembers the tool for that session only.**
4. **Revocation is explicit:** `ClientMsg::ClearOverrides` clears the session's
   overrides and appends an info event.
5. **`ApprovalRequest` carries `session_id`;** each connection forwards only
   its session's requests.
6. **Policy is still `PermissionPolicy`.** An override only turns an `Ask`
   into `allow` for the same tool in that session; it never changes `Deny` or
   plan mode.

`MemoryOverrides` is the in-memory implementation, owned by the daemon.

## Consequences

- No cross-session leakage; the M2-10 test asserts a second session still asks.
- Revocation exists; clearing overrides returns to normal policy.
- Overrides are **in-memory only**; restart clears them. Persistence is a later
  decision.
- The desktop "always" button is now truthful: it allows that tool for this
  session.

## Alternatives considered

- **Keep the daemon-wide set** — rejected: cross-session policy leak.
- **Mutate `PermissionPolicy` in place** — rejected: policy is shared and
  immutable by design; overrides are a separate, revocable layer.
- **Persist overrides immediately** — deferred: no requirement yet, and
  persistence needs a store decision.
