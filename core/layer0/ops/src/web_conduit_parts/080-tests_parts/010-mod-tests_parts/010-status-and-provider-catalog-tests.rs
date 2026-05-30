
    #[test]
    fn status_bootstraps_default_policy_and_receipts_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_status(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out.get("policy").is_some());
        assert!(out
            .get("fetch_provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("default_provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("default_fetch_provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/enabled")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/tool_surface_health/blocking_reason")
                .and_then(Value::as_str),
            Some("capability_not_enabled")
        );
        assert!(out
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("tool").and_then(Value::as_str)
                    == Some("web_browser_materialize_page")
                    && row.get("optional_capability").and_then(Value::as_bool) == Some(true)
            }))
            .unwrap_or(false));
        let browser_catalog = out
            .pointer("/tool_catalog")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("tool").and_then(Value::as_str)
                        == Some("web_browser_materialize_page")
                })
            })
            .expect("browser materialization catalog row");
        assert_eq!(
            browser_catalog
                .pointer("/request_contract/input_contract/required_fields/1")
                .and_then(Value::as_str),
            Some("admission_ref")
        );
        assert!(browser_catalog
            .pointer("/request_contract/input_contract/denied_fields")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("browser_args")))
            .unwrap_or(false));
        assert_eq!(
            browser_catalog
                .pointer("/request_contract/profile_contract/state_scope")
                .and_then(Value::as_str),
            Some("stateless")
        );
        assert_eq!(
            browser_catalog
                .pointer("/request_contract/security_contract/reject_url_credentials")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            browser_catalog
                .pointer("/request_contract/evidence_handoff/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            browser_catalog
                .pointer("/request_contract/blocker_taxonomy/classes/0")
                .and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
        assert!(out
            .pointer("/tool_effective_inventory/rows")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("tool_id").and_then(Value::as_str)
                    == Some("web.browser_materialize_page")
                    && row.get("optional_capability").and_then(Value::as_bool) == Some(true)
                    && row.get("blocking_reason").and_then(Value::as_str)
                        == Some("capability_not_enabled")
            }))
            .unwrap_or(false));
        let browser_inventory = out
            .pointer("/tool_effective_inventory/rows")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("tool_id").and_then(Value::as_str)
                        == Some("web.browser_materialize_page")
                })
            })
            .expect("browser materialization inventory row");
        assert_eq!(
            browser_inventory
                .get("profile_compilation_status")
                .and_then(Value::as_str),
            Some("prepared_capability_disabled")
        );
        assert_eq!(
            browser_inventory
                .get("readiness_lifecycle_state")
                .and_then(Value::as_str),
            Some("not_configured")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/capability_contract/readiness_lifecycle/state")
                .and_then(Value::as_str),
            Some("not_configured")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/profile_compilation/version")
                .and_then(Value::as_str),
            Some("browser_profile_compilation_v1")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/profile_compilation/status")
                .and_then(Value::as_str),
            Some("prepared_capability_disabled")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/profile_compilation/state_scope")
                .and_then(Value::as_str),
            Some("stateless")
        );
        assert!(out
            .pointer("/runtime_web_tools_metadata/browser_materialization/profile_compilation/denied_caller_fields")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("browser_args")))
            .unwrap_or(false));
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/browser_materialization/profile_compilation/raw_browser_trace_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn providers_surface_returns_ranked_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_providers")
        );
        let providers = out
            .get("providers")
            .and_then(Value::as_array)
            .expect("provider catalog");
        assert!(providers
            .iter()
            .any(|row| row.get("provider").and_then(Value::as_str) == Some("duckduckgo")));
        assert!(providers.iter().all(|row| row.get("auto_detect_rank").is_some()));
        let fetch_providers = out
            .get("fetch_providers")
            .and_then(Value::as_array)
            .expect("fetch provider catalog");
        assert!(fetch_providers.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("direct_http")
        }));
    }

    #[test]
    fn requests_last_minute_counts_network_attempts_not_internal_denials() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let recent = crate::now_iso();
        let old = (Utc::now() - chrono::Duration::seconds(90)).to_rfc3339();
        let rows = vec![
            json!({
                "type": "web_conduit_receipt",
                "timestamp": recent,
                "requested_url": "https://www.bing.com/search?q=research&format=rss",
                "policy_decision": "allow",
                "policy_reason": "search_provider_chain",
                "status_code": 200,
                "response_hash": "abc123",
                "error": ""
            }),
            json!({
                "type": "web_conduit_receipt",
                "timestamp": crate::now_iso(),
                "requested_url": "https://example.com/high-value-page",
                "policy_decision": "deny",
                "policy_reason": "rate_limit_exceeded",
                "status_code": 0,
                "response_hash": "",
                "error": "policy_denied"
            }),
            json!({
                "type": "web_conduit_receipt",
                "timestamp": crate::now_iso(),
                "requested_url": "https://duckduckgo.com/html/?q=research",
                "policy_decision": "deny",
                "policy_reason": "search_provider_chain",
                "status_code": 0,
                "response_hash": "",
                "error": "search_providers_exhausted"
            }),
            json!({
                "type": "web_conduit_receipt",
                "timestamp": crate::now_iso(),
                "requested_url": "https://news.google.com/rss/search?q=research",
                "policy_decision": "deny",
                "policy_reason": "search_provider_chain",
                "status_code": 200,
                "response_hash": "",
                "error": "no_relevant_results"
            }),
            json!({
                "type": "web_conduit_receipt",
                "timestamp": old,
                "requested_url": "https://example.com/old",
                "policy_decision": "allow",
                "policy_reason": "fetch",
                "status_code": 200,
                "response_hash": "old123",
                "error": ""
            }),
        ];
        for row in rows {
            append_jsonl(&receipts_path(tmp.path()), &row).expect("append receipt");
        }

        assert_eq!(requests_last_minute(tmp.path()), 2);
        assert_eq!(requests_last_minute_for_lane(tmp.path(), "search"), 2);
        assert_eq!(requests_last_minute_for_lane(tmp.path(), "fetch"), 0);
    }

    #[test]
    fn structured_search_http_errors_preserve_provider_account_boundary() {
        assert_eq!(
            structured_search_http_error("exa", 402).as_deref(),
            Some("exa_provider_quota_exceeded_or_billing_required_http_402")
        );
        assert_eq!(
            structured_search_http_error("tavily", 432).as_deref(),
            Some("tavily_provider_quota_exceeded_or_billing_required_http_432")
        );
        assert_eq!(
            structured_search_http_error("tavily", 429).as_deref(),
            Some("tavily_rate_limited_http_429")
        );
        assert_eq!(structured_search_http_error("tavily", 200), None);
    }

    #[test]
    fn sensitive_domain_requires_explicit_human_approval() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/policy_decision/reason")
                .and_then(Value::as_str),
            Some("human_approval_required_for_sensitive_domain")
        );
        assert_eq!(
            out.get("approval_required").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("cache_status").and_then(Value::as_str),
            Some("skipped_validation")
        );
        assert_eq!(
            out.get("cache_skip_reason").and_then(Value::as_str),
            Some("approval_required")
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/reason")
                .and_then(Value::as_str),
            Some("policy_denied")
        );
        assert!(out.pointer("/approval/id").is_some());
    }

    #[test]
    fn approved_token_allows_sensitive_domain_policy_gate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let first = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        let approval_id = first
            .pointer("/approval/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!approval_id.is_empty());

        let mut approvals = load_approvals(tmp.path());
        if let Some(row) = approvals.iter_mut().find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
        }) {
            row["status"] = json!("approved");
            row["updated_at"] = json!(crate::now_iso());
        }
        save_approvals(tmp.path(), &approvals).expect("save approvals");

        let second = api_fetch(
            tmp.path(),
            &json!({
                "url": "https://accounts.google.com/login",
                "approval_id": approval_id,
                "summary_only": true
            }),
        );
        assert_eq!(
            second
                .pointer("/policy_decision/allow")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn fetch_example_com_and_summarize_smoke() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://example.com", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("direct_http")
        );
        assert!(out
            .get("provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            assert!(out
                .get("summary")
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false));
        } else {
            assert!(out.get("error").is_some());
        }
    }
