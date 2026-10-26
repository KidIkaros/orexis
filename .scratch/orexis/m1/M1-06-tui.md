# M1-06 — ratatui TUI (ADR-0004)

- **Blocked by:** M1-01, M1-02, M1-05
- **Blocks:** M1-07, M1-08
- **Status:** open

## Goal

Implement the ADR-0004 surface: four fixed regions, composer-first focus, inline tool expansion with a bottom sheet for large diffs, adaptive overlays and status bar, virtualized transcript, and the eleven states.

## Acceptance

- [ ] Snapshot tests for all eleven states at 80×24 and 120×40.
- [ ] Completed transcript blocks are frozen; only the live tail re-renders.
- [ ] Overlays draw over the transcript without reflow or scroll loss.
- [ ] Composer: multiline, paste, external editor, IME anchor.
- [ ] Keymap matches ADR-0004; every action has a keyboard path.
- [ ] `NO_COLOR` respected; state is never colour-only.
- [ ] Streaming renders token deltas without blocking input.

## Notes

- Reuse Kindred's ratatui/crossterm foundation; the layout is new.
- Fleet and board overlays are read-only until M8.
