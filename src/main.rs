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
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod auth;
mod client;
mod server;
mod tools;

/// Eruka MCP Server & CLI — Context Memory for AI Agents
#[derive(Parser, Debug)]
#[command(name = "eruka-mcp")]
#[command(about = "Eruka — Anti-hallucination context memory for AI agents.\nRun as MCP server (default) or use CLI subcommands.")]
#[command(version)]
#[command(args_conflicts_with_subcommands = true)]
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

    /// Transport mode: stdio or sse (only for MCP server mode)
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// Port for SSE transport
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Output format for CLI commands
    #[arg(long, default_value = "text")]
    format: String,

    /// CLI subcommand (omit to run as MCP server)
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Read context fields
    Get {
        /// Field path (e.g., "identity/company_name" or "*" for all)
        path: String,
    },
    /// Write a context field
    Write {
        /// Field path
        path: String,
        /// Value (string or JSON)
        value: String,
        /// Confidence (0.0-1.0)
        #[arg(short, long, default_value = "1.0")]
        confidence: f64,
        /// Source type
        #[arg(short, long, default_value = "user_input")]
        source: String,
    },
    /// Search context
    Search {
        /// Search query
        query: String,
        /// Max results
        #[arg(short, long, default_value = "10")]
        max_results: usize,
    },
    /// Show completeness report
    Completeness,
    /// List knowledge gaps
    Gaps,
    /// Check API health
    Health,
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

    let eruka = client::ErukaClient::new(&args.api_url, &api_key);
    let is_json = args.format == "json";

    // CLI subcommand mode — run command and exit
    if let Some(cmd) = args.command {
        return run_cli(cmd, &eruka, is_json).await;
    }

    // MCP server mode
    info!("Connecting to {}", args.api_url);

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

async fn run_cli(cmd: CliCommand, eruka: &client::ErukaClient, json: bool) -> Result<()> {
    match cmd {
        CliCommand::Health => {
            match eruka.health().await {
                Ok(true) => {
                    if json { println!(r#"{{"status":"ok"}}"#); }
                    else { println!("Eruka API: healthy"); }
                }
                Ok(false) => {
                    if json { println!(r#"{{"status":"unhealthy"}}"#); }
                    else { println!("Eruka API: unhealthy"); }
                    std::process::exit(1);
                }
                Err(e) => {
                    if json { println!(r#"{{"status":"error","message":"{}"}}"#, e); }
                    else { eprintln!("Error: {}", e); }
                    std::process::exit(1);
                }
            }
        }
        CliCommand::Get { path } => {
            let result = eruka.get_context(&path, true).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let fields = result["fields"].as_array().cloned().unwrap_or_default();
                if fields.is_empty() {
                    println!("No fields found for path: {}", path);
                } else {
                    for f in &fields {
                        let fp = f["field_path"].as_str().unwrap_or("?");
                        let val_str = f["value"].to_string();
                        let val = f["value"].as_str().unwrap_or(&val_str);
                        let state = f["knowledge_state"].as_str().unwrap_or("?");
                        println!("  {} = {} [{}]", fp, val, state);
                    }
                }
            }
        }
        CliCommand::Write { path, value, confidence, source } => {
            let result = eruka.write_context(&path, &value, &source, confidence).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let state = result["field"]["knowledge_state"].as_str().unwrap_or("?");
                println!("Wrote {} [{}]", path, state);
            }
        }
        CliCommand::Search { query, max_results } => {
            let result = eruka.search_context(&query, "*", max_results).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let results = result["results"].as_array().cloned().unwrap_or_default();
                println!("{} results for \"{}\":", results.len(), query);
                for r in &results {
                    let path = r["field_path"].as_str().unwrap_or("?");
                    let val = r["value"].as_str().unwrap_or("?");
                    println!("  {} = {}", path, val);
                }
            }
        }
        CliCommand::Completeness => {
            let result = eruka.get_completeness("*").await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let score = result["score"].as_f64().unwrap_or(0.0);
                println!("Completeness: {:.1}%", score);
                if let Some(cats) = result["per_category"].as_array() {
                    for c in cats {
                        let name = c["category"].as_str().unwrap_or("?");
                        let s = c["score"].as_f64().unwrap_or(0.0);
                        println!("  {}: {:.1}%", name, s);
                    }
                }
            }
        }
        CliCommand::Gaps => {
            let result = eruka.get_gaps(None, None, "impact_score", 20).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let gaps = result["gaps"].as_array().cloned().unwrap_or_default();
                if gaps.is_empty() {
                    println!("No knowledge gaps detected.");
                } else {
                    println!("{} gaps:", gaps.len());
                    for g in &gaps {
                        let path = g["field_path"].as_str().unwrap_or("?");
                        let status = g["status"].as_str().unwrap_or("?");
                        println!("  {} [{}]", path, status);
                    }
                }
            }
        }
    }
    Ok(())
}
