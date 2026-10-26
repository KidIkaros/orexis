# ADR-0017 — Protocol surfaces and integration boundary

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** PLAN §4.1, §5 (M7), invariants 14/21/22, ADR-0005, ADR-0007, issue 009, synthesis (MCP/UTCP, ACP/A2A conflicts)

## Context

M7 ships ecosystem interop: an MCP client, an ACP server, a UTCP client, and
an A2A client. Three of the four wires are JSON-RPC 2.0. The corpus evidence:
MCP first (49k servers, 19:1 SDK downloads over A2A), ACP server (6/11
harnesses), UTCP ported from Kindred (`utcp.rs`, 1,182 LOC), A2A client
(1/11 — client only). Kindred's `mcp.rs` (945 LOC, stdio, tools/resources/
prompts) is the porting source for MCP.

## Decision

### 1. One adapter per protocol; sans-io cores; thin I/O shells

`orexis-adapters-protocol-{mcp,acp,utcp,a2a}` each contain a sans-io core
(message parsing, method dispatch, correlation — sync, no I/O, no runtime)
plus thin I/O shells. Invariant 14 applies: conformance fixtures run against
the cores with recorded wire transcripts, never against live peers.

### 2. JSON-RPC 2.0 is shared adapter infrastructure

MCP, ACP, and A2A all speak JSON-RPC 2.0. A new shared crate
`orexis-adapters-jsonrpc` holds the envelope types, ndjson framing helpers,
and request/response correlation — pure data and pure functions, no I/O.
It follows ADR-0005's rules exactly (lives under `adapters-*`, external
libraries only, no domain types, never depended on by `core`/`ports`/
`usecases`). Import-lint gains the `adapters-* -> adapters-jsonrpc` edge.

`orexis-protocol` stays what ADR-0007 made it: the daemon↔client contract.
External protocol wire types live in their own adapters.

### 3. External tools enter through the existing tool surface

MCP and UTCP tools are namespaced `mcp/<server>/<tool>` and
`utcp/<provider>/<tool>` and dispatch through the existing `ToolExecutor`
port and permission gate. No new tool path, no second policy system.

- Servers connect at session start; discovered tool specs register into the
  session; a server that fails to start is a visible session event, never a
  silent drop.
- Default permission for external tools is `ask`; the ruleset may allow per
  server or per tool. The floor survives YOLO (invariant 6).
- Tool output is untrusted content: delimited (invariant 22) and capped.
  **Tool descriptions from servers are untrusted too** — they are delimited
  when injected into the prompt (the tool-description injection class).
- Deferred loading above ~15 tools applies uniformly (§7).

### 4. UTCP's CLI transport goes through the sandbox

UTCP manuals may declare a CLI transport. Those calls execute through the
sandboxed `ToolExecutor`, never a raw process spawn. HTTP transport uses
`orexis-adapters-http`; an MCP-transport manual composes the M7 MCP client.

### 5. ACP server maps the session loop; fs callbacks stay sovereign

`orexis acp` (a daemon-crate bin, wired through composition) speaks ACP over
stdio: `initialize` → capabilities, `session/new` → a session, `session/
prompt` → the turn loop, `session/update` → streaming deltas (say → message
chunk, tool call → tool_call/update), approvals → `session/request_permission`,
`session/cancel` → interrupt. ACP `fs/*` callbacks are served by Orexis's own
workspace through the permission gate — one fs path, not two.

### 6. A2A is client-only in M7

Agent card discovery, `message/send`, `tasks/get`, SSE streaming where the
peer supports it. An A2A server (Orexis as a remote agent) is fog until team
mode (M8) proves a need.

### 7. Transport set for M7

| Protocol | Transports |
|---|---|
| MCP client | stdio, streamable HTTP |
| ACP server | stdio |
| UTCP client | HTTP, CLI (sandboxed), MCP |
| A2A client | HTTP (+ SSE) |

## Consequences

- One JSON-RPC implementation, tested once, used by three protocols.
- External tool trust is enforced at the existing gate; no new permission
  surface to audit.
- MCP server configuration is user/project config (`.orexis/mcp/`, user dir;
  project wins). No shipped default servers — shipping third-party defaults
  is a supply-chain decision, not a default.
- Conformance fixtures per protocol are the M7 gate's automated half; one
  real peer each is the manual half.

## Alternatives considered

- **JSON-RPC types in `orexis-protocol`** — rejected: that crate is the
  daemon↔client contract, not a general wire library; ADR-0005 already has
  the right mechanism.
- **A dedicated MCP tool path with its own permissions** — rejected: a second
  policy system is the Kindred orchestration-gates mistake.
- **MCP server in Orexis (inbound)** — rejected for M7: no corpus harness
  needs it in v1; revisit with the SDK/marketplace fog.
- **A2A server in M7** — rejected: client-only satisfies the gate; the server
  is team-mode-shaped work.
- **Plain tool names without namespacing** — rejected: collisions between
  built-in and external tools are a correctness bug, not a style choice.
