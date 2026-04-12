<p align="center">
  <img src="docs/img/eruka-mcp-logo.svg" width="128" alt="eruka-mcp">
</p>

<h1 align="center">eruka-mcp</h1>

<p align="center">MCP (Model Context Protocol) server for <a href="https://eruka.dirmacs.com">Eruka</a> — anti-hallucination context memory for AI agents.</p>

## What is Eruka?

Eruka is a knowledge engine that tracks what AI agents know, what they don't know, and what they should never fabricate. It provides:

- **4 Knowledge States**: CONFIRMED, INFERRED, UNCERTAIN, UNKNOWN
- **Gap Detection**: identifies missing information before generation
- **Constraint Injection**: "DO NOT FABRICATE" directives for LLM prompts
- **Knowledge Decay**: confidence degrades over time, facts become UNCERTAIN
- **Quality Scoring**: 6-layer hallucination detection pipeline

## Install

```bash
cargo install eruka-mcp
```

Or build from source:

```bash
git clone https://github.com/dirmacs/eruka-mcp
cd eruka-mcp
cargo build --release
```

## Setup

eruka-mcp works with two backends:

### Local mode — openeruka (no account needed)

```bash
# 1. Install the local server
cargo install openeruka

# 2. Start it
openeruka serve  # runs at http://localhost:8080

# 3. Run eruka-mcp — connects to localhost:8080 by default
eruka-mcp
```

### Managed mode — eruka.dirmacs.com

1. Sign up at [eruka.dirmacs.com](https://eruka.dirmacs.com)
2. Create a service key (Settings > API Keys)
3. Set your key:

```bash
export ERUKA_API_URL=https://eruka.dirmacs.com
export ERUKA_API_KEY=eruka_sk_...
```

## Usage

### Claude Desktop

**Local mode** (openeruka — no account needed):
```json
{
  "mcpServers": {
    "eruka": {
      "command": "eruka-mcp"
    }
  }
}
```

**Managed mode** (eruka.dirmacs.com):
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

### Claude Code

**Local mode:**
```bash
claude mcp add eruka eruka-mcp
```

**Managed mode:**
```bash
claude mcp add eruka eruka-mcp \
  -e ERUKA_API_URL=https://eruka.dirmacs.com \
  -e ERUKA_API_KEY=eruka_sk_...
```

### Streamable HTTP Transport (remote MCP)

Run as a remote MCP server for claude.ai web, Cursor, or any HTTP-capable MCP client:

```bash
eruka-mcp --transport sse --port 8080
```

This starts a Streamable HTTP server (MCP spec 2025-03-26) at `http://localhost:8080/mcp`:
- `POST /mcp` — JSON-RPC 2.0 requests (initialize, tools/list, tools/call)
- `GET /mcp` — SSE notification stream
- Session management via `Mcp-Session-Id` header

#### Claude.ai Web

Use the hosted endpoint at `https://eruka.dirmacs.com/mcp`:
1. Go to claude.ai → Settings → Connectors → Add custom connector
2. Enter URL: `https://eruka.dirmacs.com/mcp`
3. Authenticate with your Eruka credentials (OAuth)
4. All 17 tools are automatically available

#### Direct HTTP Test

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

## Tools

| Tool | Description | Tier |
|------|-------------|------|
| `eruka_get_context` | Retrieve fields by schema path | Free |
| `eruka_search_context` | Semantic search across all context | Free |
| `eruka_get_completeness` | Completeness score with per-category breakdown | Free |
| `eruka_get_gaps` | List knowledge gaps sorted by impact | Free |
| `eruka_write_context` | Write or update a field | Free |
| `eruka_get_voice` | Retrieve brand voice guidelines | Free |
| `eruka_detect_gaps` | Run gap detection for a task type | Free |
| `eruka_get_constraint` | Generate anti-hallucination constraint text | Pro |
| `eruka_get_related` | Traverse the knowledge graph | Free |
| `eruka_add_relationship` | Create typed edges in the knowledge graph | Free |
| `eruka_get_context_compressed` | Token-efficient compressed context | Free |
| `eruka_get_context_cached` | Diff-based caching with session tracking (20-30% token savings) | Free |
| `eruka_prefetch` | Agent lifecycle: pre-fetch context for current turn | Free |
| `eruka_sync_turn` | Persist conversation turns to context store | Free |
| `eruka_on_pre_compress` | Save insights before context window compression | Free |
| `eruka_export_context` | Export all context as portable JSON bundle | Free |
| `eruka_query_temporal` | Query context at a point in time | Pro |
| `eruka_research_gap` | Auto-research and fill knowledge gaps | Pro |

## CLI Alias

For shorter commands, add an alias:

```bash
alias eruka='eruka-mcp'
# Then: eruka get "*", eruka write ..., eruka health
```

## CLI Commands

eruka-mcp works as both an MCP server AND a standalone CLI tool. Without a subcommand, it runs as an MCP server (backward compatible).

```bash
# Read all context
eruka-mcp get "*"

# Read a specific field
eruka-mcp get "identity/company_name"

# Write a field
eruka-mcp write "identity/mission" "Build anti-hallucination infrastructure"

# Write with custom confidence and source
eruka-mcp write "market/tam" '$4.2B' --confidence 0.7 --source agent_inference

# Search
eruka-mcp search "revenue"

# Completeness report
eruka-mcp completeness

# Knowledge gaps
eruka-mcp gaps

# Health check
eruka-mcp health

# JSON output (for scripting)
eruka-mcp get "*" --format json
eruka-mcp completeness --format json
```

All CLI commands use the same `ERUKA_API_KEY` and `ERUKA_API_URL` environment variables as the MCP server.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ERUKA_API_URL` | Backend URL — local openeruka or managed service | `http://localhost:8080` |
| `ERUKA_API_KEY` | Service key — `"local"` for openeruka, `eruka_sk_...` for managed | `local` |

## CLI Reference

```
eruka-mcp [OPTIONS] [COMMAND]

Commands:
  get             Read context fields
  write           Write a context field
  search          Search context
  completeness    Show completeness report
  gaps            List knowledge gaps
  health          Check API health

Options:
      --api-url <URL>        Eruka API URL [env: ERUKA_API_URL]
      --api-key <KEY>        Service key [env: ERUKA_API_KEY]
      --tier <TIER>          Tier override [default: free]
      --transport <MODE>     stdio or sse [default: stdio]
      --port <PORT>          SSE port [default: 8080]
      --format <FMT>         Output: text or json [default: text]
      --debug                Debug logging
```

## License

MIT
