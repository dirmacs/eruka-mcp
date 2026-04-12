# eruka-mcp — Agent Guidelines

## What This Is

eruka-mcp bridges Eruka's knowledge engine to any MCP-compatible AI tool. It provides tools for reading, writing, searching, and validating context with anti-hallucination guarantees.

## Connecting to openeruka vs eruka.dirmacs.com

eruka-mcp supports two backends, selected via environment variables:

### Local mode — openeruka (default)

No account or API key needed. Run `openeruka serve` first:

```bash
# Install and start the local server
cargo install openeruka
openeruka serve  # starts at http://localhost:8080

# Run eruka-mcp — connects to localhost:8080 by default
eruka-mcp
```

Claude Desktop (local mode, no env vars needed):
```json
{ "mcpServers": { "eruka": { "command": "eruka-mcp" } } }
```

Claude Code (local mode):
```bash
claude mcp add eruka eruka-mcp
```

### Managed mode — eruka.dirmacs.com

Requires an API key from https://eruka.dirmacs.com:

```bash
export ERUKA_API_URL=https://eruka.dirmacs.com
export ERUKA_API_KEY=eruka_sk_...
eruka-mcp
```

Claude Desktop (managed mode):
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

### Feature comparison

| Feature | openeruka (local) | eruka.dirmacs.com (managed) |
|---------|-------------------|----------------------------|
| Knowledge states | All 4 | All 4 |
| REST API | Yes | Yes |
| Quality scoring (B6) | No | Yes |
| Knowledge decay | No | Yes |
| Graph traversal | Read only | Full |
| Multi-tenant | No | Yes |
| Auth required | No | Yes (API key) |
| Backend | SQLite / redb | PostgreSQL |

## For Agents

- Run `cargo test` before changes
- MCP tools are the public API — don't break tool schemas
- Knowledge states (CONFIRMED/INFERRED/UNCERTAIN/UNKNOWN) are the core abstraction
- Service key auth via X-Service-Key header — never hardcode keys
- Both stdio and SSE transports must work — test both
- Default URL is `http://localhost:8080` (openeruka); managed users must set `ERUKA_API_URL`
