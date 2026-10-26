# TUI layout and keymap — prototype

**Ticket:** `.scratch/orexis/issues/004-tui-layout-keymap.md` · **Status:** proposed · **Date:** 2026-10-25

The TUI is the fast path: keyboard-first, SSH-friendly, 80×24 minimum. It answers **"what is the system doing?"** — unlike Kindred's companion TUI, which answers "how is the entity?"

## Regions (fixed, never reflow)

```
┌ status ──────────────────────────────────────────────────────────────┐
│ transcript (primary, virtualized, scrollable)                        │
├ composer (3–5 lines, multiline, paste, file drop) ───────────────────┤
│ keyhints (context-sensitive, one line)                               │
└──────────────────────────────────────────────────────────────────────┘
```

Overlays draw **over** the transcript; they never resize it, so scroll position and spatial memory survive.

## 80×24

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ s12 · worker 2/4 · claude-sonnet-4-5 · anthropic · ctx 42% · $0.03 · plan:off │
├──────────────────────────────────────────────────────────────────────────────┤
│ you › refactor the session store to use the new event schema                  │
│                                                                              │
│ orexis › I'll read the store and the schema first.                           │
│                                                                              │
│ ▸ read   src/session/store.rs                    1.2s ✓                      │
│ ▸ read   docs/adr/0001-session-event-model.md    0.8s ✓                      │
│ ▸ edit   src/session/store.rs                    3 hunks ✓                   │
│ ▸ shell  cargo test -p orexis-session            4.1s ✓                      │
│                                                                              │
│ orexis › Tests pass. The store now appends Compaction events instead of      │
│          rewriting rows.                                                     │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ › _                                                                          │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ Enter send · ⇧Enter newline · Esc cancel · ^P palette · ^D detach · ? help    │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 120×40

Same four regions. The status bar gains `queue 2` and `tier skills`; tool rows gain a right-aligned duration and cost. No new panes.

## Tool row

Collapsed (default):

```
▸ shell  cargo test -p orexis-session            4.1s ✓
▸ edit   src/session/store.rs                    3 hunks ✓
```

Expanded (`Enter` on the row):

```
▾ edit   src/session/store.rs                    3 hunks ✓
  @@ -41,7 +41,9 @@
  -    store.rewrite(events)
  +    store.append(Compaction { range, summary })
  observation: 3 hunks applied
```

## Approval card

```
⚠ approval  shell  rm -rf target
  cwd: /home/ikaaros/Coding/Gold/orexis   risk: high   reversible: no
  [y] approve   [n] deny   [d] show diff   [a] always allow this pattern
```

## Plan gate

```
┌ plan ─ 4 tasks ──────────────────────────────────────────────┐
│ 1. read store + schema                            ✓          │
│ 2. rewrite append path                            ✓          │
│ 3. migrate tests                                  ▶          │
│ 4. run full suite                                 ○          │
│ [a]pprove   [e]dit   [c]ancel                                │
└──────────────────────────────────────────────────────────────┘
```

## Overlays

Fleet (`Ctrl-F` / `/fleet`):

```
┌ fleet ─ 4 workers ───────────────────────────────────────────┐
│ ● 1  builder   claude-sonnet   running  task 12   $0.04      │
│ ● 2  reviewer  gpt-5           waiting  task 12   $0.01      │
│ ○ 3  planner   local 8b        idle               $0.00      │
│ ○ 4  miner     deepseek        queued   task 14   $0.00      │
└──────────────────────────────────────────────────────────────┘
```

Board (`Ctrl-B` / `/board`):

```
┌ board ─ 6 tasks ─────────────────────────────────────────────┐
│ #12  rewrite store     build    worker 1   running           │
│ #13  review store      review   worker 2   blocked by #12    │
│ #14  migrate tests     build    —          queued            │
└──────────────────────────────────────────────────────────────┘
```

Self (`/self`):

```
┌ self ────────────────────────────────────────────────────────┐
│ drives   curiosity 0.7 · mastery 0.6 · coherence 0.8 · rest 0.3 │
│ affect   valence +0.2 · arousal 0.4 · dominance 0.5          │
│ tier     skills        budget 42k/100k    provider anthropic │
│ leaves   prompts + tool args + file reads → api.anthropic.com │
└──────────────────────────────────────────────────────────────┘
```

Room (`/room`), Todo (`/todo`), Interop (`/interop`), Help (`?`) follow the same overlay shell.

## Keymap

| Context | Key | Action |
|---|---|---|
| Global | `Enter` | Send |
| Global | `Shift+Enter` | Newline |
| Global | `Esc` | Cancel turn; close overlay |
| Global | `Ctrl+C` | Interrupt (hard stop) |
| Global | `Ctrl+P` / `/` | Command palette |
| Global | `Ctrl+R` | Search transcript |
| Global | `Ctrl+F` | Fleet |
| Global | `Ctrl+B` | Board |
| Global | `Ctrl+D` | Detach |
| Global | `?` | Help |
| Transcript | `j`/`k`, `↑`/`↓` | Scroll |
| Transcript | `g`/`G` | Top / bottom |
| Transcript | `Enter` | Expand/collapse tool row |
| Transcript | `o` | Open referenced file at line |
| Transcript | `y` | Copy row |
| Composer | `Ctrl+E` | External editor |
| Composer | `Ctrl+V` | Paste |
| Composer | `Tab` | Complete |
| Composer | `↑` | History |
| Approval | `y`/`n`/`d`/`a` | Approve / deny / diff / always |
| Plan gate | `a`/`e`/`c` | Approve / edit / cancel |
| Overlay | `j`/`k`, `Enter`, `/` | Navigate, select, filter |

Mouse is optional. Every action has a keyboard path.

## States

| State | Rendering |
|---|---|
| Empty | "no model configured — run `orexis init`" + the command |
| Loading | provider start, elapsed, cancel |
| Streaming | token deltas, stop affordance |
| Tool running | tool, target, elapsed, cancel |
| Approval | inline card, risk, y/n/d/a |
| Plan gate | plan card, a/e/c |
| Error | cause, retry, fallback hint |
| Budget exhausted | spent/limit, how to raise |
| Disconnected | detached, reattach hint |
| Queued | position, why it is waiting |
| Compacting | "compacting context…", progress |

## Rendering rules

- **Virtualized transcript.** Render only the visible window; completed blocks are frozen and appended, never re-laid-out. The live tail is the only thing that re-renders.
- **No reflow on overlay.** Overlays draw over the transcript; the underlying scroll position is preserved.
- **80×24 floor.** Narrow mode collapses the status bar to `s12 · 2/4 · 42%` and the keyhints to the two most relevant keys.
- **Colour is never the only signal.** Every state has a glyph and a word.
- **`NO_COLOR` respected.** Piped output stays plain.
- **IME anchor.** The composer tracks the cursor cell for CJK input.
- **Mouse optional.** Click selects a row; `o` is the keyboard equivalent.

## Decisions

- **Initial focus:** the composer, because you are usually typing. `Tab` moves to the transcript; `Esc` returns.
- **Tool row expansion:** inline for normal output; a diff larger than the viewport opens in a bottom sheet.
- **Overlays:** full-screen at 80×24; a right-side panel at ≥120 columns so the transcript stays visible.
- **Status bar:** adaptive — full at ≥100 columns, compact below (`s12 · 2/4 · 42%`).
- **Rendering:** virtualized transcript; frozen completed blocks; no reflow on overlay; 80×24 floor; colour never the only signal; `NO_COLOR` respected; IME anchor; mouse optional.
