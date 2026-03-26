//! MCP tool definitions and HTTP-backed execution.

use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::Tier;
use crate::client::ErukaClient;

/// Check if a tool requires Pro tier
pub fn requires_pro(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "eruka_query_temporal" | "eruka_research_gap" | "eruka_get_constraint"
    )
}

/// Get tool definitions for a given tier
pub fn get_tool_definitions(tier: Tier) -> Vec<Value> {
    let mut tools = vec![
        json!({
            "name": "eruka_get_context",
            "description": "Retrieve a specific field or subtree from the business context store",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Schema path, e.g., 'identity/company_name' or 'products/*'" },
                    "depth": { "type": "integer", "description": "How deep to traverse (default 1)", "default": 1 },
                    "include_metadata": { "type": "boolean", "description": "Include knowledge state, confidence, timestamps", "default": true }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "eruka_search_context",
            "description": "Semantic search across all stored context",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "scope": { "type": "string", "description": "Limit search to a category", "default": "*" },
                    "max_results": { "type": "integer", "description": "Maximum results to return", "default": 5 }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "eruka_get_completeness",
            "description": "Get the current completeness score with per-category breakdown",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": { "type": "string", "description": "Category to check, or '*' for global", "default": "*" }
                }
            }
        }),
        json!({
            "name": "eruka_get_gaps",
            "description": "List current knowledge gaps sorted by impact",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": { "type": "string", "enum": ["OPEN", "DEFERRED", "RESOLVED", "ALL"], "default": "OPEN" },
                    "category": { "type": "string", "description": "Filter by category" },
                    "sort_by": { "type": "string", "enum": ["impact_score", "created_at"], "default": "impact_score" },
                    "limit": { "type": "integer", "default": 20 }
                }
            }
        }),
        json!({
            "name": "eruka_write_context",
            "description": "Write or update a field in the context store",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Schema path to write to" },
                    "value": { "type": "string", "description": "The value (JSON-encoded if complex)" },
                    "source": { "type": "string", "enum": ["user_input", "agent_inference", "document_extraction", "web_search"], "default": "user_input" },
                    "confidence": { "type": "number", "description": "Confidence score 0.0-1.0", "default": 1.0 },
                    "valid_from": { "type": "string", "description": "When this fact became true (ISO 8601)" }
                },
                "required": ["path", "value"]
            }
        }),
        json!({
            "name": "eruka_get_voice",
            "description": "Retrieve brand voice guidelines from content/voice/*",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "eruka_detect_gaps",
            "description": "Run gap detection for a specific task type. Returns what's missing, what's stale, and what to ask the user.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_type": { "type": "string", "description": "e.g., 'linkedin_thought_leadership', 'investor_pitch_deck'" }
                },
                "required": ["task_type"]
            }
        }),
        json!({
            "name": "eruka_get_constraint",
            "description": "Generate hallucination prevention constraint text for LLM system prompt injection",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_type": { "type": "string", "description": "Task type to generate constraints for" }
                },
                "required": ["task_type"]
            }
        }),
        json!({
            "name": "eruka_get_related",
            "description": "Traverse the knowledge graph from an entity to find related entities",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "entity": { "type": "string", "description": "Entity name or ID" },
                    "relation_type": { "type": "string", "description": "Filter by relationship type" },
                    "depth": { "type": "integer", "description": "Hops to traverse (1-3)", "default": 1 }
                },
                "required": ["entity"]
            }
        }),
        json!({
            "name": "eruka_add_relationship",
            "description": "Create a typed edge between two entities in the knowledge graph",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Source entity name" },
                    "target": { "type": "string", "description": "Target entity name" },
                    "relation_type": { "type": "string", "description": "Relationship type (e.g., COMPETES_WITH, OFFERS, TARGETS)" },
                    "properties": { "type": "object", "description": "Additional edge properties" },
                    "confidence": { "type": "number", "description": "Confidence score 0.0-1.0", "default": 0.8 }
                },
                "required": ["source", "target", "relation_type"]
            }
        }),
        json!({
            "name": "eruka_get_context_compressed",
            "description": "Get task-relevant context in a token-efficient compressed format",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_type": { "type": "string", "description": "Task type for relevance ranking" },
                    "max_tokens": { "type": "integer", "description": "Maximum tokens in output", "default": 1000 }
                },
                "required": ["task_type"]
            }
        }),
    ];

    // Pro-only tools
    if tier != Tier::Free {
        tools.push(json!({
            "name": "eruka_query_temporal",
            "description": "Query context as it existed at a specific point in time",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Field path or search query" },
                    "as_of": { "type": "string", "description": "ISO 8601 datetime to query as of" }
                },
                "required": ["query", "as_of"]
            }
        }));

        tools.push(json!({
            "name": "eruka_research_gap",
            "description": "Autonomously research and fill a knowledge gap using web search",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "gap_id": { "type": "string", "description": "Gap ID from eruka_get_gaps" },
                    "field_path": { "type": "string", "description": "Or directly specify the field path" }
                }
            }
        }));
    }

    tools
}

/// Execute a tool via Eruka HTTP API
pub async fn execute_tool(
    client: &ErukaClient,
    tier: Tier,
    tool_name: &str,
    args: Value,
) -> Result<Value> {
    if requires_pro(tool_name) && tier == Tier::Free {
        anyhow::bail!("This tool requires Pro tier");
    }

    match tool_name {
        "eruka_get_context" => {
            let path = arg_str(&args, "path")?;
            let include_metadata = args.get("include_metadata").and_then(|v| v.as_bool()).unwrap_or(true);
            client.get_context(path, include_metadata).await
        }
        "eruka_search_context" => {
            let query = arg_str(&args, "query")?;
            let scope = args.get("scope").and_then(|v| v.as_str()).unwrap_or("*");
            let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            client.search_context(query, scope, max_results).await
        }
        "eruka_get_completeness" => {
            let scope = args.get("scope").and_then(|v| v.as_str()).unwrap_or("*");
            client.get_completeness(scope).await
        }
        "eruka_get_gaps" => {
            let status = args.get("status").and_then(|v| v.as_str());
            let category = args.get("category").and_then(|v| v.as_str());
            let sort_by = args.get("sort_by").and_then(|v| v.as_str()).unwrap_or("impact_score");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            client.get_gaps(status, category, sort_by, limit).await
        }
        "eruka_write_context" => {
            let path = arg_str(&args, "path")?;
            let value = arg_str(&args, "value")?;
            let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("user_input");
            let confidence = args.get("confidence").and_then(|v| v.as_f64()).unwrap_or(1.0);
            client.write_context(path, value, source, confidence).await
        }
        "eruka_get_voice" => client.get_voice().await,
        "eruka_detect_gaps" => {
            let task_type = arg_str(&args, "task_type")?;
            client.detect_gaps(task_type).await
        }
        "eruka_get_constraint" => {
            let task_type = arg_str(&args, "task_type")?;
            client.get_constraint(task_type).await
        }
        "eruka_get_related" => {
            let entity = arg_str(&args, "entity")?;
            let relation_type = args.get("relation_type").and_then(|v| v.as_str());
            let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1).min(3) as usize;
            client.get_related(entity, relation_type, depth).await
        }
        "eruka_add_relationship" => {
            let source = arg_str(&args, "source")?;
            let target = arg_str(&args, "target")?;
            let relation_type = arg_str(&args, "relation_type")?;
            let properties = args.get("properties");
            let confidence = args.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.8);
            client.add_relationship(source, target, relation_type, properties, confidence).await
        }
        "eruka_get_context_compressed" => {
            let task_type = arg_str(&args, "task_type")?;
            let max_tokens = args.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
            client.get_compressed(task_type, max_tokens).await
        }
        "eruka_query_temporal" => {
            let query = arg_str(&args, "query")?;
            let as_of = arg_str(&args, "as_of")?;
            client.query_temporal(query, as_of).await
        }
        "eruka_research_gap" => {
            let field_path = args.get("field_path").and_then(|v| v.as_str())
                .or_else(|| args.get("gap_id").and_then(|v| v.as_str()))
                .ok_or_else(|| anyhow::anyhow!("Missing field_path or gap_id"))?;
            client.research_gap(field_path).await
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: {}", key))
}
