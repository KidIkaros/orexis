# ADR-0004 — TUI surface contract

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/issues/004-tui-layout-keymap.md`
- **Prototype:** `docs/orexis/prototypes/tui-layout.md`

## Context

The TUI is Orexis's fast path: keyboard-first, SSH-friendly, 80×24 minimum. It must answer "what is the system doing?" — unlike Kindred's companion TUI, which answers "how is the entity?" The framework (ratatui + crossterm) is reusable from Kindred; the layout is not. Mercury's Ink fixes (`<Static itemKey>`, freeze gate, live-region guard, IME anchor) describe the exact rendering problems a long transcript creates.

## Decision

**Regions.** Four fixed regions: status (1 line), transcript (flex, virtualized), composer (3–5 lines), keyhints (1 line). Overlays draw over the transcript; they never resize it.

**Focus.** The composer starts focused. `Tab` moves to the transcript; `Esc` returns.

**Tool rows.** Collapsed to one line (`▸ shell cargo test 4.1s ✓`). `Enter` expands inline for normal output; a diff larger than the viewport opens in a bottom sheet.

**Overlays.** Full-screen at 80×24; a right-side panel at ≥120 columns so the transcript stays visible.

**Status bar.** Adaptive: full at ≥100 columns (session, worker, model, provider, ctx, cost, plan, queue); compact below (`s12 · 2/4 · 42%`).

**States.** Empty, loading, streaming, tool running, approval, plan gate, error, budget exhausted, disconnected, queued, compacting. Every state has a glyph and a word, not colour alone.

**Rendering.** Virtualized transcript; completed blocks are frozen and appended, never re-laid-out; only the live tail re-renders. No reflow on overlay. 80×24 floor. `NO_COLOR` respected. IME cursor anchor for CJK. Mouse optional; every action has a keyboard path.

**Keymap.**

| Context | Keys |
|---|---|
| Global | Enter send · ⇧Enter newline · Esc cancel/close · Ctrl-C interrupt · Ctrl-P palette · Ctrl-R search · Ctrl-F fleet · Ctrl-B board · Ctrl-D detach · `?` help |
| Transcript | `j/k` or arrows scroll · `g/G` top/bottom · Enter expand · `o` open file · `y` copy |
| Composer | Ctrl-E external editor · Ctrl-V paste · Tab complete · ↑ history |
| Approval | `y` approve · `n` deny · `d` diff · `a` always allow |
| Plan gate | `a` approve · `e` edit · `c` cancel |
| Overlay | `j/k` navigate · Enter select · `/` filter |

## Consequences

- Long sessions stay responsive; the transcript does not re-render history.
- Overlays preserve scroll position and spatial memory.
- The 80×24 floor forces a compact status and keyhints, which keeps SSH usable.
- Every action is keyboard-reachable, so accessibility does not depend on mouse support.
- Inline expansion can push content; the bottom-sheet threshold is a judgement call to tune in M1.
- The keymap is a contract; changing it is a breaking change for muscle memory.

## Alternatives considered

- **Transcript-focused start** — better for reading, worse for the common case of typing.
- **Always-full-screen overlays** — simpler, but loses the transcript at wide sizes.
- **Always side panels** — unusable at 80 columns.
- **Non-virtualized transcript** — simpler, but long sessions degrade.
- **Colour-only states** — inaccessible and unreadable when piped.
