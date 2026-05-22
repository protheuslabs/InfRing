
    #[test]
    fn search_requires_query() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(tmp.path(), &json!({"query": ""}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("query_required")
        );
        assert_eq!(
            out.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/retry/strategy").and_then(Value::as_str),
            Some("provide_query_text")
        );
        assert_eq!(
            out.pointer("/retry/reason").and_then(Value::as_str),
            Some("query_required")
        );
        assert_eq!(
            out.pointer("/retry/contract_version").and_then(Value::as_str),
            Some("v1")
        );
        assert_eq!(
            out.pointer("/retry/lane").and_then(Value::as_str),
            Some("web_search")
        );
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn search_smoke_records_receipt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({"query": "example domain", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_search")
        );
        assert!(
            matches!(
                out.get("provider").and_then(Value::as_str),
                Some("duckduckgo")
                    | Some("duckduckgo_lite")
                    | Some("bing_rss")
                    | Some("tavily")
                    | Some("exa")
                    | Some("brave")
                    | Some("serperdev")
                    | Some("none")
            ),
            "unexpected provider: {:?}",
            out.get("provider")
        );
        assert!(out.get("provider_chain").is_some());
    }

    #[test]
    fn structured_search_provider_payload_preserves_result_rows() {
        let out = render_tavily_payload(
            r#"{
                "results": [{
                    "title": "Example current research update",
                    "url": "https://news.example.com/current-research-update",
                    "content": "Current research update with concrete source-backed details."
                }],
                "request_id": "req-test"
            }"#,
            &[],
            false,
            5,
            20_000,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/results/0/url").and_then(Value::as_str),
            Some("https://news.example.com/current-research-update")
        );
        assert_eq!(
            out.pointer("/links/0").and_then(Value::as_str),
            Some("https://news.example.com/current-research-update")
        );
    }

    #[test]
    fn search_summary_only_aliases_are_explicit_opt_in() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({"query": "example domain", "summaryOnly": "true"}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_search")
        );
        assert_eq!(
            out.get("ok").and_then(Value::as_bool),
            Some(true),
            "summaryOnly alias should be accepted as explicit opt-in, not treated as unknown/no-op"
        );
    }

    #[test]
    fn api_search_rejects_unknown_explicit_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({
                "query": "agent reliability benchmarks",
                "provider": "perplexity"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unknown_search_provider")
        );
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("perplexity")
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
            Some("unknown_search_provider")
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/reason")
                .and_then(Value::as_str),
            Some("unknown_search_provider")
        );
        assert_eq!(
            out.get("meta_query_blocked").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/retry/strategy").and_then(Value::as_str),
            Some("use_supported_provider_or_auto")
        );
        assert_eq!(
            out.pointer("/retry/reason").and_then(Value::as_str),
            Some("unknown_search_provider")
        );
        assert_eq!(
            out.pointer("/retry/contract_version").and_then(Value::as_str),
            Some("v1")
        );
        assert!(out
            .get("provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("provider").and_then(Value::as_str) == Some("duckduckgo")))
            .unwrap_or(false));
    }

    #[test]
    fn challenge_detector_flags_anomaly_copy() {
        assert!(looks_like_search_challenge_payload(
            "Unfortunately, bots use DuckDuckGo too.",
            "Please complete the following challenge and select all squares containing a duck."
        ));
    }

    #[test]
    fn challenge_detector_ignores_normal_results() {
        assert!(!looks_like_search_challenge_payload(
            "Tech News | Today's Latest Technology News | Reuters",
            "www.reuters.com/technology/ Find latest technology news from every corner of the globe."
        ));
    }

    #[test]
    fn scoped_search_query_applies_domain_filters() {
        let scoped = scoped_search_query(
            "agent reliability",
            &vec!["github.com".to_string(), "docs.rs".to_string()],
            false,
        );
        assert!(scoped.contains("site:github.com"));
        assert!(scoped.contains("site:docs.rs"));
        assert!(scoped.contains("agent reliability"));
    }

    #[test]
    fn scoped_search_query_leaves_plain_query_when_domains_empty() {
        let scoped = scoped_search_query("agent reliability", &[], false);
        assert_eq!(scoped, "agent reliability");
    }

    #[test]
    fn normalize_allowed_domains_sanitizes_urls_and_duplicates() {
        let domains = normalize_allowed_domains(&json!([
            "https://www.github.com/openai",
            "docs.rs",
            "github.com",
            "not a domain"
        ]));
        assert_eq!(
            domains,
            vec!["github.com".to_string(), "docs.rs".to_string()]
        );
    }

    #[test]
    fn scoped_search_query_supports_exact_domain_mode() {
        let scoped =
            scoped_search_query("agent reliability", &vec!["example.com".to_string()], true);
        assert!(scoped.contains("site:example.com"));
        assert!(scoped.contains("-site:*.example.com"));
    }
