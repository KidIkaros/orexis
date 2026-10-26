# M7-06 — ACP server: dispatch, stdio bin, session mapping

- **Blocked by:** M7-01
- **Blocks:** M7-07, M7-11
- **Status:** resolved (devin, 2026-10-26)
- **Claimed by:** devin, 2026-10-26

## Goal

`orexis acp` (daemon-crate bin, wired through composition) speaks ACP over
stdio; the sans-io dispatch maps ACP methods onto the session loop
(ADR-0017 §5).

## Acceptance

- [x] `initialize` → capabilities; `session/new` → session; `session/prompt`
      → the turn loop; `session/cancel` → interrupt.
- [x] `session/update` notifications: say deltas → message chunks; tool
      calls → tool_call/tool_call_update.
- [x] Conformance fixtures: recorded ACP transcripts against the sans-io
      dispatch.
- [x] The bin constructs nothing itself; composition builds the runtime
      (ADR-0006).

## Notes

Three pieces:

1. **`orexis-adapters-protocol-acp`** (sans-io): `parse_incoming` dispatches
   initialize/session-new/session-prompt/session-cancel/unhandled;
   response builders; the bridge maps daemon `ServerMsg` → `session/update`
   (deltas → agentMessageChunk, ToolCall → toolCall in_progress with kind
   mapping, Observation → toolCallUpdate completed, unpaired Error → a
   visible `[error]` chunk) and extracts stop reasons (Done → end_turn,
   Interrupted → cancelled). Depends on `orexis-protocol` (ADR-0007 allows
   it); `SessionEvent` is now re-exported there.
2. **`AcpServer`** (daemon lib): the stdio loop, session tracking, and the
   drive. The drive runs as a task with a shared writer so the read loop
   keeps consuming — `session/cancel` arrives mid-prompt and the prompt
   ends `cancelled`, not hung. The drive subscribes before spawning (a
   fast cancel must not outrun the subscription) and handles the
   select/JoinHandle completion race via `is_finished()`.
3. **Wiring**: `spawn_turn` extracted from the socket prompt handler (both
   paths share identity, router, gate, MCP specs, and the framing line);
   `daemon_config_from_env` extracted for both bins; the `acp` bin is thin.

Six conformance fixtures: initialize/session-new shapes, prompt streaming
(chunk + tool_call + toolCallUpdate + end_turn), cancel-mid-prompt →
cancelled, unknown method → -32601, malformed line → -32603, unknown
session → error. 82 workspace suites green.

Manual (not verified here): an ACP client driving `orexis acp` (M7-11).
