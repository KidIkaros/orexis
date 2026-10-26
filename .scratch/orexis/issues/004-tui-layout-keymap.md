# 004 — TUI layout and keymap

- **Type:** prototype
- **Blocks:** M1
- **Status:** open
- **Claimed by:** —

## Question

What is the TUI's region layout, keymap, and full state set? Which overlays exist, and how does the transcript avoid reflow?

## Evidence

- Plan §8.2 and §12: status, transcript, composer, keyhints; overlays for fleet, board, room, self, todo, interop.
- Kindred `src/tui.rs`: companion surface (relational status bar, conversation, input) — framework reusable, layout not.
- Mercury Ink fixes: `<Static itemKey>`, freeze gate, live-region guard, IME anchor.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
