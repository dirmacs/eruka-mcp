# eruka-mcp

MCP (Model Context Protocol) server for [Eruka](https://eruka.dirmacs.com) — anti-hallucination context memory for AI agents.

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

1. Sign up at [eruka.dirmacs.com](https://eruka.dirmacs.com)
2. Create a service key (Settings > API Keys)
3. Set your key:

```bash
export ERUKA_API_KEY=eruka_sk_...
```

## Usage

### Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "eruka": {
      "command": "eruka-mcp",
      "env": {
        "ERUKA_API_KEY": "eruka_sk_..."
      }
    }
  }
}
```

### Claude Code

```bash
claude mcp add eruka eruka-mcp -e ERUKA_API_KEY=eruka_sk_...
```

### SSE Transport (web clients)

```bash
eruka-mcp --transport sse --port 8080
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
| `eruka_query_temporal` | Query context at a point in time | Pro |
| `eruka_research_gap` | Auto-research and fill knowledge gaps | Pro |

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ERUKA_API_KEY` | Service key (required) | — |
| `ERUKA_API_URL` | Eruka API URL | `https://eruka.dirmacs.com` |

## CLI Options

```
eruka-mcp [OPTIONS]

Options:
      --api-url <URL>        Eruka API URL [env: ERUKA_API_URL] [default: https://eruka.dirmacs.com]
      --api-key <KEY>        Service key [env: ERUKA_API_KEY]
      --tier <TIER>          Tier override [default: free]
      --transport <MODE>     stdio or sse [default: stdio]
      --port <PORT>          Port for SSE transport [default: 8080]
      --debug                Enable debug logging
  -h, --help                 Print help
  -V, --version              Print version
```

## License

MIT
