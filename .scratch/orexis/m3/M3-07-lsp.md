# M3-07 — LSP diagnostics

- **Blocked by:** M3-01
- **Blocks:** M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Surface post-edit diagnostics from a language server to the model, with a no-op fallback when no server is available.

## Acceptance

- [x] An `Lsp` port and a generic stdio LSP client (`StdioLsp`), configured with `rust-analyzer` by default.
- [x] After an edit, diagnostics for the file are attached to the observation.
- [x] A missing server is a silent no-op, not an error.
- [x] A slow server times out (5 s default) without blocking the turn.
- [x] Tests with a mock LSP server (initialize, didOpen, publishDiagnostics).

## Notes

- `StdioLsp` frames JSON-RPC with `Content-Length`, performs initialize/initialized/didOpen, and waits for `publishDiagnostics`.
- Diagnostics are advisory: any failure returns an empty list, so an edit never fails because the LSP did.
- A rust-analyzer process is spawned per edit; a persistent server is a later optimization.
- `LocalTools::with_lsp` appends a diagnostics section to the edit observation.
