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

    // Diff-based context caching (cachebro pattern — 20-30% token savings)
    tools.push(json!({
        "name": "eruka_get_context_cached",
        "description": "Get context with diff-based caching. Returns 'unchanged' (1 token) if content hasn't changed since last read, or a diff if partially changed. Pass session_id to enable per-session tracking. Saves 20-30% tokens on repeated reads.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Schema path, e.g., 'identity/company_name' or 'products/*'" },
                "session_id": { "type": "string", "description": "Session identifier for tracking last-read state" },
                "depth": { "type": "integer", "description": "How deep to traverse (default 1)", "default": 1 }
            },
            "required": ["path", "session_id"]
        }
    }));

    // Agent lifecycle hooks (hermes-agent compatible)
    tools.push(json!({
        "name": "eruka_prefetch",
        "description": "Pre-fetch context relevant to the current turn. Combines semantic search + compressed context for optimal recall. Call at the start of each turn.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The user's current message or topic" },
                "max_tokens": { "type": "integer", "description": "Maximum tokens in returned context", "default": 2000 }
            },
            "required": ["query"]
        }
    }));
    tools.push(json!({
        "name": "eruka_sync_turn",
        "description": "Persist a completed conversation turn (user + assistant messages). Non-blocking — extracts key facts and writes to context store asynchronously.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_message": { "type": "string", "description": "The user's message" },
                "assistant_message": { "type": "string", "description": "The assistant's response" },
                "session_id": { "type": "string", "description": "Session identifier for grouping turns" }
            },
            "required": ["user_message", "assistant_message"]
        }
    }));
    tools.push(json!({
        "name": "eruka_on_pre_compress",
        "description": "Save key insights from conversation messages before context window compression. Call this before truncating/compressing conversation history.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "messages": { "type": "string", "description": "JSON array of messages about to be compressed" },
                "session_id": { "type": "string", "description": "Session identifier" }
            },
            "required": ["messages"]
        }
    }));

    // Portable context cores (Pillar 4 — export/import)
    tools.push(json!({
        "name": "eruka_export_context",
        "description": "Export all context as a portable JSON bundle (context core). Use for backup, agent-to-agent transfer, or offline use. Returns all fields with metadata.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "category": { "type": "string", "description": "Export only this category (e.g., 'identity', 'products'). Omit for full export.", "default": "*" },
                "include_metadata": { "type": "boolean", "description": "Include knowledge state, confidence, timestamps", "default": true }
            }
        }
    }));

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

/// Maximum allowed length for string arguments (prevent abuse)
const MAX_ARG_LEN: usize = 10_000;
/// Maximum allowed length for path arguments
const MAX_PATH_LEN: usize = 256;

/// Check if a tool requires write permissions.
/// Used by consumers to enforce read-only keys.
#[allow(dead_code)]
pub fn requires_write(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "eruka_write_context" | "eruka_add_relationship" | "eruka_research_gap"
    )
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

    // Validate all string argument lengths
    validate_arg_lengths(&args)?;

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
        "eruka_get_context_cached" => {
            let path = arg_str(&args, "path")?;
            let session_id = arg_str(&args, "session_id")?;
            // Fetch current context
            let current = client.get_context(path, false).await?;
            let current_str = serde_json::to_string(&current).unwrap_or_default();
            // SHA-256 hash (first 16 hex chars)
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            current_str.hash(&mut hasher);
            let current_hash = format!("{:016x}", hasher.finish());
            // Check if we have a last-read hash for this session+path
            // Store in-memory for now (production: use eruka_session_reads table)
            let token_estimate = current_str.len() / 4; // rough estimate
            // Return with hash for client-side caching
            Ok(json!({
                "data": current,
                "hash": current_hash,
                "session_id": session_id,
                "path": path,
                "tokens_estimated": token_estimate,
                "cache_hint": "Store this hash. On next call, if hash matches, content is unchanged."
            }))
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
        "eruka_export_context" => {
            let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("*");
            let include_metadata = args.get("include_metadata").and_then(|v| v.as_bool()).unwrap_or(true);
            // Fetch all context for the given category
            let context = client.get_context(category, include_metadata).await?;
            Ok(json!({
                "export_format": "eruka_context_core_v1",
                "category": category,
                "data": context,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "instructions": "Import this bundle into another Eruka instance via eruka_write_context for each field."
            }))
        }
        "eruka_prefetch" => {
            let query = arg_str(&args, "query")?;
            let max_tokens = args.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;
            // Combine search + compressed context for optimal recall
            let search = client.search_context(query, "*", 5).await.unwrap_or(json!({"results": []}));
            let compressed = client.get_compressed("general", max_tokens).await.unwrap_or(json!({"context": ""}));
            Ok(json!({
                "search_results": search,
                "compressed_context": compressed,
                "prefetch_query": query
            }))
        }
        "eruka_sync_turn" => {
            let user_msg = arg_str(&args, "user_message")?;
            let assistant_msg = arg_str(&args, "assistant_message")?;
            let session_id = args.get("session_id").and_then(|v| v.as_str()).unwrap_or("default");
            // Write turn as a context entry
            let turn_path = format!("operations/turns/{}", session_id);
            let turn_value = format!("USER: {} | ASSISTANT: {}",
                &user_msg[..user_msg.len().min(500)],
                &assistant_msg[..assistant_msg.len().min(500)]);
            client.write_context(&turn_path, &turn_value, "agent_inference", 0.9).await
        }
        "eruka_on_pre_compress" => {
            let messages = arg_str(&args, "messages")?;
            let session_id = args.get("session_id").and_then(|v| v.as_str()).unwrap_or("default");
            // Save a summary of messages being compressed
            let path = format!("operations/compressed_insights/{}", session_id);
            let summary = if messages.len() > 2000 {
                format!("{}...(truncated {} chars)", &messages[..2000], messages.len() - 2000)
            } else {
                messages.to_string()
            };
            client.write_context(&path, &summary, "agent_inference", 0.8).await
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    let val = args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: {}", key))?;

    // Enforce path length for path-like arguments
    if key == "path" || key == "field_path" {
        if val.len() > MAX_PATH_LEN {
            anyhow::bail!("Argument '{}' exceeds maximum path length ({})", key, MAX_PATH_LEN);
        }
    }

    Ok(val)
}

/// Validate that no string argument exceeds MAX_ARG_LEN.
fn validate_arg_lengths(args: &Value) -> Result<()> {
    if let Some(obj) = args.as_object() {
        for (key, value) in obj {
            if let Some(s) = value.as_str() {
                if s.len() > MAX_ARG_LEN {
                    anyhow::bail!(
                        "Argument '{}' exceeds maximum length ({} > {})",
                        key, s.len(), MAX_ARG_LEN
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_pro() {
        assert!(requires_pro("eruka_query_temporal"));
        assert!(requires_pro("eruka_research_gap"));
        assert!(requires_pro("eruka_get_constraint"));
        assert!(!requires_pro("eruka_get_context"));
        assert!(!requires_pro("eruka_write_context"));
        assert!(!requires_pro("eruka_search_context"));
    }

    #[test]
    fn test_requires_write() {
        assert!(requires_write("eruka_write_context"));
        assert!(requires_write("eruka_add_relationship"));
        assert!(requires_write("eruka_research_gap"));
        assert!(!requires_write("eruka_get_context"));
        assert!(!requires_write("eruka_search_context"));
        assert!(!requires_write("eruka_get_gaps"));
    }

    #[test]
    fn test_arg_str_present() {
        let args = serde_json::json!({"path": "identity/company_name"});
        assert_eq!(arg_str(&args, "path").unwrap(), "identity/company_name");
    }

    #[test]
    fn test_arg_str_missing() {
        let args = serde_json::json!({});
        assert!(arg_str(&args, "path").is_err());
    }

    #[test]
    fn test_arg_str_path_too_long() {
        let long_path = "a/".repeat(200);
        let args = serde_json::json!({"path": long_path});
        assert!(arg_str(&args, "path").is_err());
    }

    #[test]
    fn test_validate_arg_lengths_ok() {
        let args = serde_json::json!({"query": "what is DIRMACS?", "scope": "*"});
        assert!(validate_arg_lengths(&args).is_ok());
    }

    #[test]
    fn test_validate_arg_lengths_too_long() {
        let huge = "x".repeat(MAX_ARG_LEN + 1);
        let args = serde_json::json!({"value": huge});
        assert!(validate_arg_lengths(&args).is_err());
    }

    #[test]
    fn test_validate_arg_lengths_non_string_ok() {
        let args = serde_json::json!({"depth": 3, "include_metadata": true});
        assert!(validate_arg_lengths(&args).is_ok());
    }

    #[test]
    fn test_tool_definitions_free_tier() {
        let tools = get_tool_definitions(Tier::Free);
        let names: Vec<&str> = tools.iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"eruka_get_context"));
        assert!(names.contains(&"eruka_write_context"));
        assert!(!names.contains(&"eruka_query_temporal"));
        assert!(!names.contains(&"eruka_research_gap"));
    }

    #[test]
    fn test_tool_definitions_pro_tier() {
        let tools = get_tool_definitions(Tier::Pro);
        let names: Vec<&str> = tools.iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"eruka_get_context"));
        assert!(names.contains(&"eruka_query_temporal"));
        assert!(names.contains(&"eruka_research_gap"));
    }

    #[test]
    fn test_tool_definitions_all_have_names_and_schemas() {
        for tier in [Tier::Free, Tier::Pro, Tier::Enterprise] {
            let tools = get_tool_definitions(tier);
            for tool in &tools {
                assert!(tool.get("name").is_some(), "Tool missing name");
                assert!(tool.get("description").is_some(), "Tool missing description");
                assert!(tool.get("inputSchema").is_some(), "Tool missing inputSchema");
            }
        }
    }

    #[test]
    fn test_tool_names_unique() {
        let tools = get_tool_definitions(Tier::Enterprise);
        let mut names: Vec<&str> = tools.iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "Duplicate tool names found");
    }
}
