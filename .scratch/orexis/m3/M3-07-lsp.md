# M3-07 — LSP diagnostics

- **Blocked by:** M3-01
- **Blocks:** M3-08
- **Status:** open

## Goal

Surface post-edit diagnostics from a language server to the model, with a no-op fallback when no server is available.

## Acceptance

- [ ] An `Lsp` port and a rust-analyzer adapter (or a generic stdio LSP client).
- [ ] After an edit, diagnostics for the file are attached to the observation.
- [ ] A missing server is a silent no-op, not an error.
- [ ] A slow server times out without blocking the turn.
- [ ] Tests with a mock LSP server.
