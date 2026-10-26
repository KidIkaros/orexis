# 004 — TUI layout and keymap

- **Type:** prototype
- **Blocks:** M1
- **Status:** resolved
- **Claimed by:** devin, 2026-10-25

## Question

What is the TUI's region layout, keymap, and full state set? Which overlays exist, and how does the transcript avoid reflow?

## Evidence

- Plan §8.2 and §12: status, transcript, composer, keyhints; overlays for fleet, board, room, self, todo, interop.
- Kindred `src/tui.rs`: companion surface (relational status bar, conversation, input) — framework reusable, layout not.
- Mercury Ink fixes: `<Static itemKey>`, freeze gate, live-region guard, IME anchor.

## Resolution

**Decided 2026-10-25.** Prototype: `docs/orexis/prototypes/tui-layout.md`. Contract: `docs/orexis/adr/0004-tui-surface.md`.

- **Regions:** fixed status (1), transcript (flex, virtualized), composer (3–5), keyhints (1). Overlays draw over, never resize.
- **Initial focus:** composer; `Tab` to transcript, `Esc` back.
- **Tool rows:** collapsed one line; inline expansion for normal output; bottom sheet for diffs larger than the viewport.
- **Overlays:** full-screen at 80×24; right-side panel at ≥120 columns.
- **Status bar:** adaptive — full at ≥100 columns, compact below.
- **States:** empty, loading, streaming, tool running, approval, plan gate, error, budget exhausted, disconnected, queued, compacting.
- **Rendering:** virtualized transcript; frozen completed blocks; no reflow on overlay; 80×24 floor; colour never the only signal; `NO_COLOR`; IME anchor; mouse optional.
- **Keymap:** Enter send, ⇧Enter newline, Esc cancel/close, Ctrl-C interrupt, Ctrl-P palette, Ctrl-R search, Ctrl-F fleet, Ctrl-B board, Ctrl-D detach, `?` help; `j/k` scroll, `Enter` expand, `o` open file, `y` copy; approval `y/n/d/a`; plan `a/e/c`.

## Follow-ups

- ADR-0004 written.
- M1 implements the four regions, the keymap, and the state set.
- Overlays beyond fleet/board/self (room, todo, interop) reuse the same shell.
