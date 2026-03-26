//! # Eruka MCP Server
//!
//! Model Context Protocol server for [Eruka](https://eruka.dirmacs.com) —
//! anti-hallucination context memory for AI agents.
//!
//! ## Quick Start
//!
//! ```bash
//! # Install
//! cargo install eruka-mcp
//!
//! # Get your API key at https://eruka.dirmacs.com
//! export ERUKA_API_KEY=eruka_sk_...
//!
//! # Run (stdio transport for Claude Desktop / Claude Code)
//! eruka-mcp
//!
//! # Or with SSE transport for web clients
//! eruka-mcp --transport sse --port 8080
//! ```
//!
//! ## Claude Desktop Configuration
//!
//! Add to `claude_desktop_config.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "eruka": {
//!       "command": "eruka-mcp",
//!       "env": {
//!         "ERUKA_API_KEY": "eruka_sk_..."
//!       }
//!     }
//!   }
//! }
//! ```

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod auth;
mod client;
mod server;
mod tools;

/// Eruka MCP Server — Context Memory for AI Agents
#[derive(Parser, Debug)]
#[command(name = "eruka-mcp")]
#[command(about = "Eruka MCP Server — Anti-hallucination context memory for AI agents")]
#[command(version)]
struct Args {
    /// Eruka API URL
    #[arg(long, env = "ERUKA_API_URL", default_value = "https://eruka.dirmacs.com")]
    api_url: String,

    /// Service key (get yours at https://eruka.dirmacs.com)
    #[arg(long, env = "ERUKA_API_KEY")]
    api_key: Option<String>,

    /// Tier override (auto-detected from key prefix)
    #[arg(long, default_value = "free")]
    tier: String,

    /// Transport mode: stdio or sse
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// Port for SSE transport
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Logging to stderr (stdio transport uses stdout for JSON-RPC)
    let level = if args.debug { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Eruka MCP Server v{}", env!("CARGO_PKG_VERSION"));

    let api_key = args.api_key.ok_or_else(|| {
        anyhow::anyhow!(
            "Missing API key. Set ERUKA_API_KEY or pass --api-key.\n\
             Get your key at https://eruka.dirmacs.com"
        )
    })?;

    info!("Connecting to {}", args.api_url);
    let eruka = client::ErukaClient::new(&args.api_url, &api_key);

    // Verify connectivity
    match eruka.health().await {
        Ok(true) => info!("Connected to Eruka API"),
        Ok(false) => anyhow::bail!("Eruka API returned unhealthy status"),
        Err(e) => anyhow::bail!("Cannot connect to Eruka API at {}: {}", args.api_url, e),
    }

    // Auto-detect tier from key prefix
    let tier = if api_key.starts_with("eruka_sk_ent_") || args.tier == "enterprise" {
        auth::Tier::Enterprise
    } else if api_key.starts_with("eruka_sk_pro_") || args.tier == "pro" {
        auth::Tier::Pro
    } else {
        auth::Tier::Free
    };

    info!("Tier: {}", tier.as_str());

    let mcp_server = server::McpServer::new(eruka, tier);

    match args.transport.as_str() {
        "stdio" => {
            info!("Starting stdio transport");
            server::run_stdio(mcp_server).await?;
        }
        "sse" => {
            info!("Starting SSE transport on port {}", args.port);
            server::run_sse(mcp_server, args.port).await?;
        }
        other => anyhow::bail!("Unknown transport: {}. Use 'stdio' or 'sse'.", other),
    }

    Ok(())
}
