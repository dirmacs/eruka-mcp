+++
title = "eruka-mcp"
+++

# eruka-mcp — MCP Server for Context Memory

MCP (Model Context Protocol) server for [Eruka](https://eruka.dirmacs.com) — anti-hallucination context memory for AI agents.

## Install

```bash
cargo install eruka-mcp
```

## Quick Start

### Local Mode (openeruka — no account needed)

```bash
cargo install openeruka-server
openeruka serve        # runs at http://localhost:8080
eruka-mcp              # connects to localhost:8080 by default
```

### Managed Mode (eruka.dirmacs.com)

```bash
export ERUKA_API_URL=https://eruka.dirmacs.com
export ERUKA_API_KEY=eruka_sk_...
eruka-mcp
```

## Claude Desktop Config

**Local mode:**
```json
{ "mcpServers": { "eruka": { "command": "eruka-mcp" } } }
```

**Managed mode:**
```json
{
  "mcpServers": {
    "eruka": {
      "command": "eruka-mcp",
      "env": {
        "ERUKA_API_URL": "https://eruka.dirmacs.com",
        "ERUKA_API_KEY": "eruka_sk_..."
      }
    }
  }
}
```

## Claude Code Config

```bash
claude mcp add eruka eruka-mcp
```

## Tools (18 total)

| Tool | Description | Tier |
|------|-------------|------|
| `eruka_get_context` | Retrieve fields by schema path (supports `compact` mode) | Free |
| `eruka_search_context` | Semantic search across all context (supports `compact` mode) | Free |
| `eruka_get_completeness` | Completeness score with per-category breakdown | Free |
| `eruka_get_gaps` | List knowledge gaps sorted by impact | Free |
| `eruka_write_context` | Write or update a field | Free |
| `eruka_get_voice` | Retrieve brand voice guidelines | Free |
| `eruka_detect_gaps` | Run gap detection for a task type | Free |
| `eruka_get_constraint` | Generate anti-hallucination constraint text | Pro |
| `eruka_get_related` | Traverse the knowledge graph | Free |
| `eruka_add_relationship` | Create typed edges in the knowledge graph | Free |
| `eruka_get_context_compressed` | Token-efficient compressed context | Free |
| `eruka_get_context_cached` | Diff-based caching, 20-30% token savings | Free |
| `eruka_prefetch` | Pre-fetch context for current turn | Free |
| `eruka_sync_turn` | Persist conversation turns to context store | Free |
| `eruka_on_pre_compress` | Save insights before context window compression | Free |
| `eruka_export_context` | Export all context as portable JSON bundle | Free |
| `eruka_query_temporal` | Query context at a point in time | Pro |
| `eruka_research_gap` | Auto-research and fill knowledge gaps | Pro |

## Changelog

See [CHANGELOG.md](https://github.com/dirmacs/eruka-mcp/blob/main/CHANGELOG.md) for full history.

## License

MIT
