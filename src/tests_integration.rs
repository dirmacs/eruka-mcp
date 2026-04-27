//! Integration tests using httpmock to test client, tools dispatch, and server logic
//! without requiring a live Eruka API.

#[cfg(test)]
mod auth_tests {
    use crate::auth::Tier;

    #[test]
    fn test_tier_as_str_free() {
        assert_eq!(Tier::Free.as_str(), "free");
    }

    #[test]
    fn test_tier_as_str_pro() {
        assert_eq!(Tier::Pro.as_str(), "pro");
    }

    #[test]
    fn test_tier_as_str_enterprise() {
        assert_eq!(Tier::Enterprise.as_str(), "enterprise");
    }

    #[test]
    fn test_tier_equality() {
        assert_eq!(Tier::Free, Tier::Free);
        assert_ne!(Tier::Free, Tier::Pro);
        assert_ne!(Tier::Pro, Tier::Enterprise);
    }

    #[test]
    fn test_tier_clone() {
        let t = Tier::Pro;
        let t2 = t;
        assert_eq!(t2.as_str(), "pro");
    }
}

#[cfg(test)]
mod client_tests {
    use crate::client::ErukaClient;
    use httpmock::prelude::*;
    use serde_json::json;

    fn mock_client(server: &MockServer) -> ErukaClient {
        ErukaClient::new(&server.base_url(), "test-key")
    }

    #[tokio::test]
    async fn test_client_new_trims_trailing_slash() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/health");
            then.status(200).body("ok");
        });
        let client = ErukaClient::new(&format!("{}/", server.base_url()), "key");
        let ok = client.health().await.unwrap();
        assert!(ok);
    }

    #[tokio::test]
    async fn test_health_ok() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/health");
            then.status(200).body("ok");
        });
        let client = mock_client(&server);
        assert!(client.health().await.unwrap());
    }

    #[tokio::test]
    async fn test_health_unhealthy() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/health");
            then.status(503).body("down");
        });
        let client = mock_client(&server);
        // 503 → is_success() is false → health returns Ok(false)
        let result = client.health().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_get_context_basic() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/context");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"fields": []}));
        });
        let client = mock_client(&server);
        let r = client.get_context("identity/company_name", true).await.unwrap();
        assert!(r.get("fields").is_some());
    }

    #[tokio::test]
    async fn test_get_context_api_error() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/context");
            then.status(401).body("Unauthorized");
        });
        let client = mock_client(&server);
        let r = client.get_context("*", false).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("401"));
    }

    #[tokio::test]
    async fn test_get_context_ex_compact() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET)
                .path("/api/v1/context")
                .query_param("compact", "true");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"text": "compact output"}));
        });
        let client = mock_client(&server);
        let r = client.get_context_ex("*", true, true, 500).await.unwrap();
        assert!(r.get("text").is_some());
    }

    #[tokio::test]
    async fn test_search_context_basic() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/context/search");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"results": [{"path": "identity/name", "content": "DIRMACS"}]}));
        });
        let client = mock_client(&server);
        let r = client.search_context("what is DIRMACS", "*", 5).await.unwrap();
        assert!(r["results"].as_array().unwrap().len() == 1);
    }

    #[tokio::test]
    async fn test_search_context_ex_compact() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/context/search");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"results": []}));
        });
        let client = mock_client(&server);
        let r = client.search_context_ex("query", "Company", 3, true, 1000).await.unwrap();
        assert!(r["results"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_get_completeness_global() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/completeness");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"score": 42.5}));
        });
        let client = mock_client(&server);
        let r = client.get_completeness("*").await.unwrap();
        assert_eq!(r["score"].as_f64().unwrap(), 42.5);
    }

    #[tokio::test]
    async fn test_get_completeness_scoped() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/completeness/Company");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"score": 80.0}));
        });
        let client = mock_client(&server);
        let r = client.get_completeness("Company").await.unwrap();
        assert_eq!(r["score"].as_f64().unwrap(), 80.0);
    }

    #[tokio::test]
    async fn test_get_gaps_no_filters() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/gaps");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"gaps": []}));
        });
        let client = mock_client(&server);
        let r = client.get_gaps(None, None, "impact_score", 20).await.unwrap();
        assert!(r["gaps"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_get_gaps_with_filters() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/gaps");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"gaps": [{"field_path": "identity/name", "status": "OPEN"}]}));
        });
        let client = mock_client(&server);
        let r = client.get_gaps(Some("OPEN"), Some("Company"), "created_at", 5).await.unwrap();
        assert_eq!(r["gaps"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_write_context() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/context");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"field": {"knowledge_state": "CONFIRMED"}}));
        });
        let client = mock_client(&server);
        let r = client.write_context("identity/name", "DIRMACS", "user_input", 1.0).await.unwrap();
        assert_eq!(r["field"]["knowledge_state"].as_str().unwrap(), "CONFIRMED");
    }

    #[tokio::test]
    async fn test_get_voice() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/context");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"fields": [{"path": "content/voice/tone", "content": "professional"}]}));
        });
        let client = mock_client(&server);
        let r = client.get_voice().await.unwrap();
        assert!(r.get("fields").is_some());
    }

    #[tokio::test]
    async fn test_detect_gaps() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/gaps/detect");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"missing": [], "constraint_text": "You know nothing"}));
        });
        let client = mock_client(&server);
        let r = client.detect_gaps("linkedin_thought_leadership").await.unwrap();
        assert!(r["constraint_text"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_get_constraint() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/gaps/detect");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"constraint_text": "Do not invent facts."}));
        });
        let client = mock_client(&server);
        let r = client.get_constraint("investor_pitch_deck").await.unwrap();
        assert_eq!(r["constraint_text"].as_str().unwrap(), "Do not invent facts.");
    }

    #[tokio::test]
    async fn test_get_related_no_filter() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path_matches(regex::Regex::new("/api/v1/entities/.*/related").unwrap());
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"entities": []}));
        });
        let client = mock_client(&server);
        let r = client.get_related("DIRMACS", None, 1).await.unwrap();
        assert!(r["entities"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_get_related_with_filter() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path_matches(regex::Regex::new("/api/v1/entities/.*/related").unwrap());
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"entities": [{"name": "Acme"}]}));
        });
        let client = mock_client(&server);
        let r = client.get_related("DIRMACS", Some("COMPETES_WITH"), 2).await.unwrap();
        assert_eq!(r["entities"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_add_relationship_without_props() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/relationships");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"created": true}));
        });
        let client = mock_client(&server);
        let r = client.add_relationship("DIRMACS", "Acme", "COMPETES_WITH", None, 0.9).await.unwrap();
        assert_eq!(r["created"].as_bool().unwrap(), true);
    }

    #[tokio::test]
    async fn test_add_relationship_with_props() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/relationships");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"created": true}));
        });
        let client = mock_client(&server);
        let props = json!({"since": "2024"});
        let r = client.add_relationship("A", "B", "PARTNERS_WITH", Some(&props), 0.8).await.unwrap();
        assert!(r["created"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_get_compressed() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/compress");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"context": "Company: DIRMACS"}));
        });
        let client = mock_client(&server);
        let r = client.get_compressed("general", 1000).await.unwrap();
        assert!(r["context"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_query_temporal() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path_matches(regex::Regex::new("/api/v1/versions/.*").unwrap());
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"value": "old value"}));
        });
        let client = mock_client(&server);
        let r = client.query_temporal("identity/name", "2025-01-01T00:00:00Z").await.unwrap();
        assert!(r["value"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_research_gap() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET).path("/api/v1/tree/gaps");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({"research": "found some data"}));
        });
        let client = mock_client(&server);
        let r = client.research_gap("identity/funding").await.unwrap();
        assert!(r["research"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST).path("/api/v1/context");
            then.status(403).body("Forbidden");
        });
        let client = mock_client(&server);
        let r = client.write_context("x", "y", "user_input", 1.0).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("403"));
    }
}

#[cfg(test)]
mod execute_tool_tests {
    use crate::auth::Tier;
    use crate::client::ErukaClient;
    use crate::tools::execute_tool;
    use httpmock::prelude::*;
    use serde_json::json;

    fn setup(server: &MockServer) -> ErukaClient {
        ErukaClient::new(&server.base_url(), "test-key")
    }

    // Helper: mock any GET
    fn mock_get<'a>(server: &'a MockServer, path: &str, body: serde_json::Value) -> httpmock::Mock<'a> {
        server.mock(|when, then| {
            when.method(GET).path(path);
            then.status(200).header("Content-Type", "application/json").json_body(body);
        })
    }

    // Helper: mock any POST
    fn mock_post<'a>(server: &'a MockServer, path: &str, body: serde_json::Value) -> httpmock::Mock<'a> {
        server.mock(|when, then| {
            when.method(POST).path(path);
            then.status(200).header("Content-Type", "application/json").json_body(body);
        })
    }

    #[tokio::test]
    async fn test_execute_pro_tool_on_free_tier_fails() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_query_temporal",
            json!({"query": "identity/name", "as_of": "2025-01-01T00:00:00Z"})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("Pro tier"));
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_nonexistent", json!({})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_execute_tool_arg_too_long() {
        let server = MockServer::start();
        let client = setup(&server);
        let huge = "x".repeat(10_001);
        let r = execute_tool(&client, Tier::Free, "eruka_search_context",
            json!({"query": huge})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_execute_get_context() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/context", json!({"fields": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_context",
            json!({"path": "identity/name"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_context_compact() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/context", json!({"text": "compact"}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_context",
            json!({"path": "identity/name", "compact": true, "max_tokens": 500, "include_metadata": false})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_search_context() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context/search", json!({"results": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_search_context",
            json!({"query": "what is DIRMACS", "scope": "Company", "max_results": 3})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_search_context_compact() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context/search", json!({"results": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_search_context",
            json!({"query": "test", "compact": true, "max_tokens": 100})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_completeness() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/completeness", json!({"score": 50.0}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_completeness",
            json!({"scope": "*"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_completeness_scoped() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/completeness/Company", json!({"score": 70.0}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_completeness",
            json!({"scope": "Company"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_gaps() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/gaps", json!({"gaps": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_gaps",
            json!({"status": "OPEN", "sort_by": "impact_score", "limit": 10})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_gaps_with_category() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/gaps", json!({"gaps": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_gaps",
            json!({"category": "Company"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_write_context() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {"knowledge_state": "CONFIRMED"}}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_write_context",
            json!({"path": "identity/name", "value": "DIRMACS", "source": "user_input", "confidence": 1.0})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_write_context_missing_path_fails() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_write_context",
            json!({"value": "test"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_execute_get_voice() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/context", json!({"fields": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_voice", json!({})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_detect_gaps() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/gaps/detect", json!({"missing": []}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_detect_gaps",
            json!({"task_type": "linkedin_thought_leadership"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_constraint_pro() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/gaps/detect", json!({"constraint_text": "..."}));
        let client = setup(&server);
        // get_constraint is Pro-only
        let r = execute_tool(&client, Tier::Pro, "eruka_get_constraint",
            json!({"task_type": "pitch_deck"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_constraint_free_fails() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_constraint",
            json!({"task_type": "pitch_deck"})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("Pro tier"));
    }

    #[tokio::test]
    async fn test_execute_get_related() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path_matches(regex::Regex::new("/api/v1/entities/.*/related").unwrap());
            then.status(200).header("Content-Type", "application/json").json_body(json!({"entities": []}));
        });
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_related",
            json!({"entity": "DIRMACS", "depth": 2})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_related_with_relation_type() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path_matches(regex::Regex::new("/api/v1/entities/.*/related").unwrap());
            then.status(200).header("Content-Type", "application/json").json_body(json!({"entities": []}));
        });
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_related",
            json!({"entity": "DIRMACS", "relation_type": "COMPETES_WITH"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_add_relationship() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/relationships", json!({"created": true}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_add_relationship",
            json!({"source": "A", "target": "B", "relation_type": "USES"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_add_relationship_with_props() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/relationships", json!({"created": true}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_add_relationship",
            json!({"source": "A", "target": "B", "relation_type": "USES",
                   "properties": {"since": "2025"}, "confidence": 0.7})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_context_cached() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/context", json!({"fields": [{"path": "x", "content": "y"}]}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_context_cached",
            json!({"path": "identity/name", "session_id": "sess-abc123"})).await;
        assert!(r.is_ok());
        let v = r.unwrap();
        assert!(v["hash"].as_str().is_some());
        assert_eq!(v["session_id"].as_str().unwrap(), "sess-abc123");
        assert_eq!(v["path"].as_str().unwrap(), "identity/name");
        assert!(v["tokens_estimated"].as_u64().is_some());
        assert!(v["cache_hint"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_execute_get_context_compressed() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/compress", json!({"context": "compressed"}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_context_compressed",
            json!({"task_type": "general", "max_tokens": 500})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_temporal_pro() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path_matches(regex::Regex::new("/api/v1/versions/.*").unwrap());
            then.status(200).header("Content-Type", "application/json").json_body(json!({"value": "old"}));
        });
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Pro, "eruka_query_temporal",
            json!({"query": "identity/name", "as_of": "2025-01-01T00:00:00Z"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_research_gap_by_field_path() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/tree/gaps", json!({"research": "ok"}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Pro, "eruka_research_gap",
            json!({"field_path": "identity/funding"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_research_gap_by_gap_id() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/tree/gaps", json!({"research": "ok"}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Enterprise, "eruka_research_gap",
            json!({"gap_id": "gap-uuid-123"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_research_gap_missing_args_fails() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Pro, "eruka_research_gap", json!({})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_execute_export_context() {
        let server = MockServer::start();
        let _m = mock_get(&server, "/api/v1/context", json!({"fields": [{"path": "x", "content": "y"}]}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_export_context",
            json!({"category": "*", "include_metadata": true})).await;
        assert!(r.is_ok());
        let v = r.unwrap();
        assert_eq!(v["export_format"].as_str().unwrap(), "eruka_context_core_v1");
        assert!(v["exported_at"].as_str().is_some());
        assert!(v["instructions"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_execute_prefetch() {
        let server = MockServer::start();
        // prefetch calls search + compressed; both might fail gracefully
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/context/search");
            then.status(200).header("Content-Type", "application/json").json_body(json!({"results": []}));
        });
        server.mock(|when, then| {
            when.method(POST).path("/api/v1/compress");
            then.status(200).header("Content-Type", "application/json").json_body(json!({"context": ""}));
        });
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_prefetch",
            json!({"query": "what is DIRMACS", "max_tokens": 1000})).await;
        assert!(r.is_ok());
        let v = r.unwrap();
        assert_eq!(v["prefetch_query"].as_str().unwrap(), "what is DIRMACS");
        assert!(v["search_results"].is_object());
        assert!(v["compressed_context"].is_object());
    }

    #[tokio::test]
    async fn test_execute_prefetch_graceful_on_api_errors() {
        // prefetch unwraps errors with unwrap_or — should still succeed
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST);
            then.status(500).body("error");
        });
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_prefetch",
            json!({"query": "test"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_sync_turn() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {"knowledge_state": "CONFIRMED"}}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_sync_turn",
            json!({"user_message": "Hello", "assistant_message": "Hi there", "session_id": "sess-1"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_sync_turn_default_session() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {"knowledge_state": "INFERRED"}}));
        let client = setup(&server);
        // No session_id — should use "default"
        let r = execute_tool(&client, Tier::Free, "eruka_sync_turn",
            json!({"user_message": "Hi", "assistant_message": "Hello"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_sync_turn_long_messages_truncated() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {}}));
        let client = setup(&server);
        let long_msg = "x".repeat(2000);
        let r = execute_tool(&client, Tier::Free, "eruka_sync_turn",
            json!({"user_message": long_msg, "assistant_message": "ok"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_on_pre_compress_short() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {}}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_on_pre_compress",
            json!({"messages": "[{\"role\":\"user\",\"content\":\"hello\"}]", "session_id": "s1"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_on_pre_compress_long_truncated() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {}}));
        let client = setup(&server);
        let long_msgs = "x".repeat(3000);
        let r = execute_tool(&client, Tier::Free, "eruka_on_pre_compress",
            json!({"messages": long_msgs})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_on_pre_compress_default_session() {
        let server = MockServer::start();
        let _m = mock_post(&server, "/api/v1/context", json!({"field": {}}));
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_on_pre_compress",
            json!({"messages": "[]"})).await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_execute_get_context_missing_path_fails() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_get_context", json!({})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("Missing required argument"));
    }

    #[tokio::test]
    async fn test_execute_detect_gaps_missing_task_type_fails() {
        let server = MockServer::start();
        let client = setup(&server);
        let r = execute_tool(&client, Tier::Free, "eruka_detect_gaps", json!({})).await;
        assert!(r.is_err());
    }
}

#[cfg(test)]
mod server_tests {
    use crate::auth::Tier;
    use crate::client::ErukaClient;
    use crate::server::{McpServer, handle_request_pub};
    use httpmock::prelude::*;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn make_server(base_url: &str) -> Arc<Mutex<McpServer>> {
        let client = ErukaClient::new(base_url, "test-key");
        Arc::new(Mutex::new(McpServer::new(client, Tier::Free)))
    }

    fn rpc(method: &str, id: serde_json::Value, params: Option<serde_json::Value>) -> String {
        let mut obj = json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": id
        });
        if let Some(p) = params {
            obj["params"] = p;
        }
        obj.to_string()
    }

    #[tokio::test]
    async fn test_mcpserver_new() {
        let server = MockServer::start();
        let mcp = McpServer::new(ErukaClient::new(&server.base_url(), "key"), Tier::Pro);
        assert!(!mcp.initialized);
        assert_eq!(mcp.tier, Tier::Pro);
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2025-03-26"}
        });
        let resp = handle_request_pub(&mcp, req).await;
        assert!(resp.is_some());
        let r = resp.unwrap();
        assert_eq!(r["result"]["protocolVersion"].as_str().unwrap(), "2025-03-26");
        assert_eq!(r["result"]["serverInfo"]["name"].as_str().unwrap(), "eruka-mcp");
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert!(!tools.is_empty());
        // Verify no pro tools present on Free tier
        let names: Vec<&str> = tools.iter()
            .filter_map(|t| t["name"].as_str()).collect();
        assert!(!names.contains(&"eruka_query_temporal"));
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "id": 3, "method": "ping"});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert_eq!(resp["result"], json!({}));
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "id": 4, "method": "nonexistent/method"});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert!(resp["error"].is_object());
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32601);
    }

    #[tokio::test]
    async fn test_handle_notification_initialized_returns_none() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        // notifications have no id
        let req = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        let resp = handle_request_pub(&mcp, req).await;
        assert!(resp.is_none());
        // initialized flag should be set
        let s = mcp.lock().await;
        assert!(s.initialized);
    }

    #[tokio::test]
    async fn test_handle_other_notification_returns_none() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "method": "notifications/something_else"});
        let resp = handle_request_pub(&mcp, req).await;
        assert!(resp.is_none());
    }

    #[tokio::test]
    async fn test_handle_tools_call_missing_params() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call"});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert!(resp["error"].is_object());
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32602);
    }

    #[tokio::test]
    async fn test_handle_tools_call_missing_name() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call",
                         "params": {"arguments": {}}});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert!(resp["error"].is_object());
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32602);
    }

    #[tokio::test]
    async fn test_handle_tools_call_success() {
        let mock_srv = MockServer::start();
        mock_srv.mock(|when, then| {
            when.method(GET).path("/api/v1/completeness");
            then.status(200).header("Content-Type", "application/json")
                .json_body(json!({"score": 55.0}));
        });
        let mcp = make_server(&mock_srv.base_url());
        let req = json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": "eruka_get_completeness", "arguments": {"scope": "*"}}
        });
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert!(resp["result"]["content"].is_array());
        let content = &resp["result"]["content"][0];
        assert_eq!(content["type"].as_str().unwrap(), "text");
        assert!(content["text"].as_str().unwrap().contains("55"));
    }

    #[tokio::test]
    async fn test_handle_tools_call_tool_error() {
        let mock_srv = MockServer::start();
        mock_srv.mock(|when, then| {
            when.method(GET);
            then.status(500).body("internal error");
        });
        let mcp = make_server(&mock_srv.base_url());
        let req = json!({
            "jsonrpc": "2.0", "id": 8, "method": "tools/call",
            "params": {"name": "eruka_get_completeness", "arguments": {}}
        });
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert!(resp["error"].is_object());
        assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32000);
    }

    #[tokio::test]
    async fn test_handle_id_null_when_missing() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        // Request with id: null explicitly
        let req = json!({"jsonrpc": "2.0", "id": null, "method": "ping"});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert_eq!(resp["id"], json!(null));
    }

    #[tokio::test]
    async fn test_handle_string_id() {
        let server = MockServer::start();
        let mcp = make_server(&server.base_url());
        let req = json!({"jsonrpc": "2.0", "id": "req-abc", "method": "ping"});
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        assert_eq!(resp["id"].as_str().unwrap(), "req-abc");
    }

    #[tokio::test]
    async fn test_tools_call_no_arguments_field_defaults_to_empty() {
        let mock_srv = MockServer::start();
        mock_srv.mock(|when, then| {
            when.method(GET).path("/api/v1/completeness");
            then.status(200).header("Content-Type", "application/json")
                .json_body(json!({"score": 0.0}));
        });
        let mcp = make_server(&mock_srv.base_url());
        // No "arguments" key in params — should default to {}
        let req = json!({
            "jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": {"name": "eruka_get_completeness"}
        });
        let resp = handle_request_pub(&mcp, req).await.unwrap();
        // completeness with no scope arg defaults to "*" — should succeed
        assert!(resp["result"]["content"].is_array());
    }
}
