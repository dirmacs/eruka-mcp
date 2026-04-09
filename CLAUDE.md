# eruka-mcp

MCP server for Eruka — anti-hallucination context memory for AI agents. Knowledge state tracking, gap detection, constraint injection, quality scoring.

## Build & Test

```bash
cargo build --release
cargo test
cargo install eruka-mcp       # from crates.io
```

## Usage

```bash
eruka-mcp                                          # MCP server (stdio)
eruka-mcp --transport sse --port 8080              # HTTP/SSE transport
eruka-mcp get "*"                                  # CLI: get all context
eruka-mcp search "query"                           # CLI: search context
eruka-mcp completeness                             # CLI: check coverage
eruka-mcp gaps                                     # CLI: find missing fields
```

## Environment

- `ERUKA_API_KEY` — required, service key for Eruka API
- `ERUKA_API_URL` — optional, defaults to http://localhost:8081

## Conventions

- Git author: `bkataru <baalateja.k@gmail.com>`
- Dual mode: MCP server + standalone CLI
- Axum for SSE transport, tokio async runtime
- Knowledge states: CONFIRMED, INFERRED, UNCERTAIN, UNKNOWN
