//! MCP protocol handler for stdio and SSE transports.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::auth::Tier;
use crate::client::ErukaClient;
use crate::tools;

/// MCP Server state
pub struct McpServer {
    pub client: ErukaClient,
    pub tier: Tier,
    pub initialized: bool,
}

impl McpServer {
    pub fn new(client: ErukaClient, tier: Tier) -> Self {
        Self {
            client,
            tier,
            initialized: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Run MCP server over stdio
pub async fn run_stdio(server: McpServer) -> Result<()> {
    let server = Arc::new(Mutex::new(server));
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = handle_request(&server, request).await;

        if let Some(resp) = response {
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

/// Run MCP server over SSE
pub async fn run_sse(server: McpServer, port: u16) -> Result<()> {
    use axum::{
        routing::{get, post},
        Router,
    };
    use tower_http::cors::CorsLayer;

    let server = Arc::new(Mutex::new(server));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/mcp", post(handle_mcp_post).get(handle_mcp_sse_stream))
        .layer(CorsLayer::permissive())
        .with_state(server);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("SSE server listening on port {}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_mcp_post(
    axum::extract::State(server): axum::extract::State<Arc<Mutex<McpServer>>>,
    headers: axum::http::HeaderMap,
    axum::Json(request): axum::Json<JsonRpcRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let response = handle_request(&server, request).await;
    let json_resp = response.unwrap_or(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Value::Null,
        result: Some(json!({})),
        error: None,
    });

    // Add Mcp-Session-Id header
    let session_id = headers.get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let new_session = if session_id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        session_id
    };

    let mut resp = axum::Json(json_resp).into_response();
    if let Ok(val) = new_session.parse() {
        resp.headers_mut().insert("Mcp-Session-Id", val);
    }
    resp
}

/// GET /mcp — SSE notification stream (Streamable HTTP spec)
async fn handle_mcp_sse_stream(
    axum::extract::State(_server): axum::extract::State<Arc<Mutex<McpServer>>>,
) -> axum::response::sse::Sse<impl futures_util::stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let stream = async_stream::stream! {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            yield Ok(axum::response::sse::Event::default().data("ping"));
        }
    };
    axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn handle_request(
    server: &Arc<Mutex<McpServer>>,
    request: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = request.id.clone().unwrap_or(Value::Null);

    if request.id.is_none() && request.method.starts_with("notifications/") {
        if request.method == "notifications/initialized" {
            let mut s = server.lock().await;
            s.initialized = true;
            tracing::info!("Client initialized");
        }
        return None;
    }

    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "eruka-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => {
            let s = server.lock().await;
            Ok(json!({ "tools": tools::get_tool_definitions(s.tier) }))
        }
        "tools/call" => handle_tools_call(server, request.params).await,
        "ping" => Ok(json!({})),
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    };

    Some(match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(value),
            error: None,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        },
    })
}

async fn handle_tools_call(
    server: &Arc<Mutex<McpServer>>,
    params: Option<Value>,
) -> Result<Value, JsonRpcError> {
    let params = params.ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing tool name".to_string(),
            data: None,
        })?;

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    let s = server.lock().await;

    let result = tools::execute_tool(&s.client, s.tier, name, arguments)
        .await
        .map_err(|e| JsonRpcError {
            code: -32000,
            message: e.to_string(),
            data: None,
        })?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&result).unwrap_or_default()
        }]
    }))
}
