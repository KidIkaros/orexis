# The vertical slice

**Status:** done 2026-10-25 · **Run:** `cargo run -p orexis-slice` · **Result:** `replay: byte-identical`

## What it is

Six crates that prove the dependency direction and the session schema end to end:

```
orexis-core            Message, Action, Observation, SessionEvent, derive_messages
orexis-ports           Provider, ToolExecutor, SessionAppend, SessionRead, EventSink
orexis-usecases        run_turn (the loop), hash_messages, verify_replay
orexis-adapters-fake   ScriptedProvider, EchoTools, InMemorySession, RecordingSink, render_transcript
orexis-composition     build() — the only place concrete adapters are constructed
orexis-slice           the binary: prompt → tool call → event → render → replay
```

## What it proves

1. **Core is pure.** `orexis-core` has no dependencies at all — no I/O, no ports, no runtime.
2. **Ports are consumer-side.** `orexis-ports` declares the traits; no adapter is named there.
3. **Use cases depend only on core + ports.** `orexis-usecases` does not import any adapter crate.
4. **Adapters implement ports.** `orexis-adapters-fake` depends on core + ports, never on use cases.
5. **Composition is the only constructor.** `orexis-composition` is the sole place `ScriptedProvider`, `EchoTools`, `InMemorySession`, and `RecordingSink` are built.
6. **The session schema replays.** Every logged `ModelRequest` hash equals the pure `derive_messages` of the events before it; `verify_replay` asserts it.
7. **The render path is a projection.** `render_transcript` turns events into a transcript without owning state.

## Output

```
you › read src/lib.rs and summarize
orexis › reading the file
  ▸ read src/lib.rs
    read → ok(src/lib.rs)
✓ read and summarized
steps: 2  done: true
messages: 3  replay: byte-identical
```

## What it does not prove (and where it lands)

| Missing | Lands in |
|---|---|
| SQLite persistence, migrations, generations | M1 (ADR-0001) |
| Real providers (OpenAI-compatible, Anthropic, llama.cpp) | M1 (ADR-0002) |
| The four codecs | M1 (ADR-0003) |
| ratatui TUI | M1 (ADR-0004) |
| Permissions, sandbox, approvals | M3 |
| Real tools beyond an echo | M3 |
| Multi-session daemon, attach, replay over a socket | M1 |
| A real hash (blake3/sha256) instead of `DefaultHasher` | M1 |
| Import-lint enforcement of the Dependency Rule | M1 |

## Why this slice

The plan's anti-Familiar argument is that architecture comes before breadth. The slice is the smallest artifact that can falsify that claim: if the loop, the ports, the composition root, and the replay check did not fit together, better to know now than after the porting ledger.

They fit.
