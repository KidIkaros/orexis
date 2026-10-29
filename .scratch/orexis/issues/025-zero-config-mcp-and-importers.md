# 025 — Zero-config MCP auto-discovery and rival state importers (v0.4.0, M7)

- **Type:** task
- **Blocks:** v0.4.0
- **Status:** open
- **Claimed by:** —

## Question

How can Orexis provide seamless onboarding and ecosystem interoperability by automatically discovering existing MCP servers and importing session trees from rival coding agents without data loss?

## Evidence

- `crates/adapters-protocol-mcp`: Sans-io MCP client implementation supporting stdio and streamable HTTP.
- ADR-0017: Protocol surfaces (MCP client + ACP server).
- Rival state formats:
  - Claude Desktop MCP configs (`claude_desktop_config.json`)
  - Cursor MCP configs (`~/.cursor/mcp.json`)
  - Aider chat history (`.aider.chat.history.md`)
  - Cline tasks & state (`tasks/*.json`)
  - OpenCode SQLite session database
- Data portability gate in v0.1.0 (`orexis export --all` / `import` bundle).

## Proposed Resolution

1. **Zero-Config MCP Discovery**:
   - Probe standard paths on startup: `~/.config/orexis/mcp.json`, `~/.config/claude/claude_desktop_config.json`, and `.cursor/mcp.json`.
   - Safely merge server definitions without manual JSON stitching.
2. **Universal Session Importer**:
   - Implement `orexis import --format <aider|cline|opencode>` in `crates/composition/src/importers/`.
   - Map external message histories and tool call records to Orexis `Event` types faithfully.
3. **ACP Editor Integrations**:
   - Provide reference ACP configs / extensions for VS Code, Zed, and Neovim to attach directly to `orexis serve`.

## Follow-ups

- Integration test suite running against real MCP servers (PostgreSQL, Filesystem, Memory, Fetch, GitHub).
- Release `v0.4.0` with instant migration guide for developers switching to Orexis.
