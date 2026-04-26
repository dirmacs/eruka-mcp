# Changelog

All notable changes to eruka-mcp are documented here.

## [0.3.2] — 2026-04-27

### Added
- **Compact flag** for `eruka_get_context` and `eruka_search_context` — pass `compact=true` to get
  plain-text output instead of verbose JSON, saving 60-80% tokens. Accepts `max_tokens` budget.
- **Dual-target support** — eruka-mcp now defaults to local openeruka (`http://localhost:8080`)
  with no API key required. Set `ERUKA_API_URL=https://eruka.dirmacs.com` for the managed service.
- Managed mode validation: fails fast with a clear message when `eruka.dirmacs.com` is used
  without a real API key.

### Changed
- Default `ERUKA_API_URL` changed from `http://localhost:8081` to `http://localhost:8080`
  (aligns with openeruka default port).
- README and AGENTS.md updated with local/managed mode setup instructions and feature comparison table.

## [0.3.1] — 2026-04-10

### Added
- README update: document 17 tools (now 18), lifecycle/caching/export tools.

## [0.3.0] — 2026-04-02

### Added
- **Streamable HTTP transport** (MCP spec 2025-03-26): `--transport sse --port <N>` starts an
  HTTP server at `/mcp` supporting `POST` (JSON-RPC) and `GET` (SSE notifications).
- Session management via `Mcp-Session-Id` header.
- `eruka_export_context` — export all context as a portable JSON bundle (context core).
- `eruka_get_context_cached` — diff-based caching with per-session hash tracking (20-30% token savings).
- `eruka_prefetch` — agent lifecycle: pre-fetch semantically relevant context for current turn.
- `eruka_sync_turn` — persist completed conversation turns to context store.
- `eruka_on_pre_compress` — save key insights before context window compression.

## [0.2.1] — 2026-03-30

### Added
- CLI duality: `eruka-mcp` now works as both an MCP server and a standalone CLI tool.
- Subcommands: `get`, `write`, `search`, `completeness`, `gaps`, `health`.
- `--format json` flag for machine-readable output.
- CLI alias suggestion (`alias eruka='eruka-mcp'`).

## [0.2.0] — 2026-03-28

### Added
- Syntax highlighting fix: replaced regex-based with tokenizer (no more mojibake in code blocks).
- SVG architecture diagram in README.
- Prominent Eruka dashboard link.

## [0.1.1] — 2026-03-26

### Fixed
- Dead code warning suppression for `requires_write` (public API marker).

## [0.1.0] — 2026-03-26

Initial release.

- stdio MCP transport.
- 13 tools: get/write/search context, completeness, gaps, voice, detect_gaps, constraint,
  get_related, add_relationship, get_context_compressed, query_temporal, research_gap.
- Free and Pro tiers.
- Argument length validation.
