//! HTTP client for connecting to Eruka API.
//!
//! Used in standalone mode when eruka-mcp doesn't have direct DB access.
//! Authenticates via `Authorization: ApiKey <eruka_sk_*>` header.

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

/// Eruka HTTP API client
#[derive(Clone)]
pub struct ErukaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ErukaClient {
    /// Create a new client pointing at an Eruka API instance.
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// GET request with auth
    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("ApiKey {}", self.api_key))
            .send()
            .await
            .with_context(|| format!("GET {}", url))?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, body);
        }

        serde_json::from_str(&body).with_context(|| "Failed to parse API response")
    }

    /// POST request with auth and JSON body
    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("ApiKey {}", self.api_key))
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {}", url))?;

        let status = resp.status();
        let resp_body = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("API error ({}): {}", status, resp_body);
        }

        serde_json::from_str(&resp_body).with_context(|| "Failed to parse API response")
    }

    // ── Tool implementations via HTTP ────────────────────────────────────────

    /// eruka_get_context: GET /api/v1/context?path=X&include_metadata=true
    pub async fn get_context(&self, path: &str, include_metadata: bool) -> Result<Value> {
        let url = format!(
            "/api/v1/context?path={}&include_metadata={}",
            urlencoding::encode(path),
            include_metadata
        );
        self.get(&url).await
    }

    /// eruka_search_context: POST /api/v1/context/search
    pub async fn search_context(
        &self,
        query: &str,
        scope: &str,
        max_results: usize,
    ) -> Result<Value> {
        self.post(
            "/api/v1/context/search",
            &serde_json::json!({
                "query": query,
                "scope": scope,
                "max_results": max_results
            }),
        )
        .await
    }

    /// eruka_get_completeness: GET /api/v1/completeness or /api/v1/completeness/:scope
    pub async fn get_completeness(&self, scope: &str) -> Result<Value> {
        if scope == "*" {
            self.get("/api/v1/completeness").await
        } else {
            self.get(&format!(
                "/api/v1/completeness/{}",
                urlencoding::encode(scope)
            ))
            .await
        }
    }

    /// eruka_get_gaps: GET /api/v1/gaps?status=X&category=Y&sort_by=Z&limit=N
    pub async fn get_gaps(
        &self,
        status: Option<&str>,
        category: Option<&str>,
        sort_by: &str,
        limit: usize,
    ) -> Result<Value> {
        let mut params = vec![
            format!("sort_by={}", sort_by),
            format!("limit={}", limit),
        ];
        if let Some(s) = status {
            params.push(format!("status={}", s));
        }
        if let Some(c) = category {
            params.push(format!("category={}", urlencoding::encode(c)));
        }
        self.get(&format!("/api/v1/gaps?{}", params.join("&"))).await
    }

    /// eruka_write_context: POST /api/v1/context
    pub async fn write_context(
        &self,
        path: &str,
        value: &str,
        source: &str,
        confidence: f64,
    ) -> Result<Value> {
        self.post(
            "/api/v1/context",
            &serde_json::json!({
                "path": path,
                "value": value,
                "source": source,
                "confidence": confidence
            }),
        )
        .await
    }

    /// eruka_get_voice: GET /api/v1/context?path=content/voice/*
    pub async fn get_voice(&self) -> Result<Value> {
        self.get_context("content/voice/*", false).await
    }

    /// eruka_detect_gaps: POST /api/v1/gaps/detect
    pub async fn detect_gaps(&self, task_type: &str) -> Result<Value> {
        self.post(
            "/api/v1/gaps/detect",
            &serde_json::json!({ "task_type": task_type }),
        )
        .await
    }

    /// eruka_get_constraint: POST /api/v1/gaps/detect (extracts constraint_text)
    pub async fn get_constraint(&self, task_type: &str) -> Result<Value> {
        // Uses the same endpoint as detect_gaps — constraint_text is part of the response
        self.detect_gaps(task_type).await
    }

    /// eruka_get_related: GET /api/v1/entities/:name/related?depth=N
    pub async fn get_related(
        &self,
        entity: &str,
        relation_type: Option<&str>,
        depth: usize,
    ) -> Result<Value> {
        let mut params = vec![format!("depth={}", depth)];
        if let Some(rt) = relation_type {
            params.push(format!("relation_type={}", urlencoding::encode(rt)));
        }
        self.get(&format!(
            "/api/v1/entities/{}/related?{}",
            urlencoding::encode(entity),
            params.join("&")
        ))
        .await
    }

    /// eruka_add_relationship: POST /api/v1/relationships
    pub async fn add_relationship(
        &self,
        source: &str,
        target: &str,
        relation_type: &str,
        properties: Option<&Value>,
        confidence: f64,
    ) -> Result<Value> {
        let mut body = serde_json::json!({
            "source": source,
            "target": target,
            "relation_type": relation_type,
            "confidence": confidence
        });
        if let Some(props) = properties {
            body["properties"] = props.clone();
        }
        self.post("/api/v1/relationships", &body).await
    }

    /// eruka_get_context_compressed: POST /api/v1/compress
    pub async fn get_compressed(&self, task_type: &str, max_tokens: usize) -> Result<Value> {
        self.post(
            "/api/v1/compress",
            &serde_json::json!({
                "task_type": task_type,
                "max_tokens": max_tokens
            }),
        )
        .await
    }

    /// eruka_query_temporal: GET /api/v1/versions/:path?as_of=X
    pub async fn query_temporal(&self, path: &str, as_of: &str) -> Result<Value> {
        self.get(&format!(
            "/api/v1/versions/{}?as_of={}",
            urlencoding::encode(path),
            urlencoding::encode(as_of)
        ))
        .await
    }

    /// eruka_research_gap: GET /api/v1/tree/gaps?field_path=X
    pub async fn research_gap(&self, field_path: &str) -> Result<Value> {
        self.get(&format!(
            "/api/v1/tree/gaps?field_path={}",
            urlencoding::encode(field_path)
        ))
        .await
    }

    /// Health check
    pub async fn health(&self) -> Result<bool> {
        let resp = self
            .client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}
