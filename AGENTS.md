# eruka-mcp — Agent Guidelines

## What This Is

eruka-mcp bridges Eruka's knowledge engine to any MCP-compatible AI tool. It provides tools for reading, writing, searching, and validating context with anti-hallucination guarantees.

## For Agents

- Run `cargo test` before changes
- MCP tools are the public API — don't break tool schemas
- Knowledge states (CONFIRMED/INFERRED/UNCERTAIN/UNKNOWN) are the core abstraction
- Service key auth via X-Service-Key header — never hardcode keys
- Both stdio and SSE transports must work — test both
