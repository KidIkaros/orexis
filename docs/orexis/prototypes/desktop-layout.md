# Desktop layout prototype (ticket 006)

Question: what is the Leptos layout and state model for the six panels over
the shared daemon socket?

## Task analysis first

The panels are a means, not the spec. The desktop exists for three oversight
tasks (see `desktop-usability.md`): **steer**, **recover**, **inspect**.

| Task | What must be on screen at once |
|---|---|
| Steer | Transcript, composer, the pending approval, the diff it will produce |
| Recover | The failed verify output, the composer, the diff, undo |
| Inspect | Fleet and board, the active session, the entity state |

## Layout at 1280×800

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ orexis  s12 · builder 1/2 · claude-sonnet · anthropic · ctx 42% · $0.07 · plan:off │
├───────────────┬──────────────────────────────────────────────┬───────────────┤
│ SESSIONS      │  you › refactor the store                    │ APPROVALS (1) │
│ ● s12 running │                                              │ ⚠ shell       │
│ ○ s11 idle    │  orexis › I'll read the store first.         │ cargo test    │
│               │  ▸ read_file src/store.rs         ✓ 0.3s     │ risk: low     │
│ BOARD         │  ▾ edit_file src/store.rs         ✓ 0.2s     │ reversible: y │
│ #12 rewrite   │    - let rows = load();                      │               │
│ #13 review    │    + let rows = load_all();                  │ [y] approve   │
│ #14 migrate   │                                              │ [n] deny      │
│               │  ▸ shell cargo test              … 4.1s      │ [d] diff      │
│ FLEET         │                                              │ [a] always    │
│ ● 1 builder   │                                              │               │
│ ● 2 reviewer  │                                              │ DIFF          │
│               │                                              │ src/store.rs  │
│ ARTIFACTS     │                                              │ - load()      │
│ FILES         │                                              │ + load_all()  │
├───────────────┴──────────────────────────────────────────────┴───────────────┤
│ › ask orexis…                                                               │
│ Enter send · ⇧Enter newline · ⌘K palette · ⌘⇧A approvals · ⌘, settings       │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Layout at 1920×1080

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ orexis  s12 · builder 1/2 · claude-sonnet · anthropic · ctx 42% · $0.07 · plan:off       │
├──────────────────┬──────────────────────────────────────────────────┬──────────────────┤
│ SESSIONS         │  you › refactor the store                        │ APPROVALS (1)    │
│ ● s12 running    │                                                  │ ⚠ shell           │
│ ○ s11 idle       │  orexis › I'll read the store first.             │ cargo test        │
│                  │  ▸ read_file src/store.rs         ✓ 0.3s         │ risk: low         │
│ BOARD            │  ▾ edit_file src/store.rs         ✓ 0.2s         │ reversible: yes   │
│ #12 rewrite      │    - let rows = load();                          │                   │
│ #13 review       │    + let rows = load_all();                      │ [y] approve       │
│ #14 migrate      │                                                  │ [n] deny          │
│                  │  ▸ shell cargo test              … 4.1s          │ [d] diff          │
│ FLEET            │                                                  │ [a] always        │
│ ● 1 builder      │                                                  │                   │
│ ● 2 reviewer     │                                                  │ DIFF  src/store.rs│
│ ○ 3 planner      │                                                  │ - let rows = load()│
│                  │                                                  │ + let rows = load_all()│
│ ARTIFACTS        │                                                  │                   │
│ FILES            │                                                  │ PREVIEW           │
│                  │                                                  │ (artifact/file)   │
├──────────────────┴──────────────────────────────────────────────────┴──────────────────┤
│ › ask orexis…                                                                         │
│ Enter send · ⇧Enter newline · ⌘K palette · ⌘⇧A approvals · ⌘, settings                 │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

## Panel hierarchy

| Tier | Panels | Where |
|---|---|---|
| Primary | Chat / streaming | Center, always visible |
| Secondary | Approvals, Diff, Preview | Right panel, contextual tabs |
| Tertiary | Sessions, Board, Fleet, Artifacts, Files | Left rail tabs |
| Modal | Settings, palette | Overlay, never resizes the layout |

The six named panels map as: chat (center), approvals (right), diff (right),
fleet (left), artifacts (left), files (left). Settings is a modal.

## Focus model

- Composer first; `Tab` cycles composer → transcript → left rail → right panel.
- `⌘1`…`⌘6` focus the six panels; `⌘⇧A` jumps to approvals.
- `Esc` closes an overlay or returns to the composer.
- Approval keys `y` / `n` / `d` / `a` apply when the right panel is focused.
- The right panel never steals focus; a badge announces a pending approval.

## States

| Domain | States |
|---|---|
| Connection | connecting, attached, disconnected |
| Session | empty, running, tool, approval, plan gate, error, budget, queued, compacting |
| Right panel | approvals, diff, preview |
| Left rail | sessions, board, fleet, artifacts, files |
| Overlay | none, settings, palette |

## Action → feedback (UAN-style)

| User action | System feedback |
|---|---|
| Type in composer | Character echo; Enter sends |
| Send | Composer clears; user entry appears; status "running"; stream starts |
| Tool runs | Tool row with elapsed; cancel enabled |
| Approval arrives | Badge on Approvals; right panel switches; composer stays focused |
| Approve | Card resolves; tool runs; event appended |
| Deny | Card resolves; error observation; model reprompt |
| Open diff | Diff tab; side-by-side; syntax highlight |
| Undo | Checkpoint restored; toast; transcript info |
| Switch panel | Panel focus; breadcrumb; status unchanged |
| Disconnect | Status "disconnected"; composer disabled; reattach offered |
| Open settings | Modal; model/provider, permissions, privacy ("what leaves this machine") |
| Open palette | Filterable commands; Enter runs; Esc closes |

## Leptos state model

```rust
enum Connection { Connecting, Attached, Disconnected }
enum LeftTab { Sessions, Board, Fleet, Artifacts, Files }
enum RightTab { Approvals, Diff, Preview }

struct AppState {
    connection: Connection,
    sessions: Vec<SessionInfo>,
    active: Option<String>,
    transcript: Vec<Entry>,
    pending: Option<Approval>,
    diff: Option<Diff>,
    left: LeftTab,
    right: RightTab,
    settings_open: bool,
    palette_open: bool,
    status: Status,
}
```

Signals: one `RwSignal<AppState>` plus derived memos per panel. The daemon
client lives on the Tauri Rust side, not in the webview:

- **Commands:** `attach`, `prompt`, `approve`, `undo`, `interrupt`, `list_sessions`.
- **Events:** the bridge reads `ServerMsg` from the Unix socket and emits
  `orexis://event`; Leptos applies it with the same `apply` semantics as the TUI.
- **No HTTP.** The webview never talks to the network; only the Rust side talks
  to providers.

## Components

`App`, `StatusBar`, `LeftRail`, `Transcript`, `Composer`, `RightPanel`,
`ApprovalCard`, `DiffViewer`, `FleetTable`, `Board`, `Artifacts`, `FileTree`,
`SettingsModal`, `Palette`.

## Accessibility

- Keyboard-first: every action has a key path; focus rings are visible.
- ARIA labels on panels, approval buttons, and the status bar.
- Approvals are announced (live region) and never color-only.
- Reduced motion and high contrast are respected.
- The composer keeps the IME cursor anchor.

## Evaluation

Two cycles, per `desktop-usability.md`: wireframes with 3 developers, then a
clickable Tauri shell with 5. Findings feed the layout decisions before the
M2 gate.
