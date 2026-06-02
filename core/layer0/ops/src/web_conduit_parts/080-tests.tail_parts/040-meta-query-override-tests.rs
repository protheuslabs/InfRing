
#[test]
fn search_early_validation_allows_meta_query_when_override_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"allow_meta_query_search": true}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_force_web_search_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"force_web_search": true}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_force_web_search_string_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"forceWebSearch": "true"}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_force_web_lookup_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"force_web_lookup": 1}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_early_validation_allows_meta_query_when_nested_force_lookup_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = search_early_validation_response(
        tmp.path(),
        &json!({"searchPolicy": {"forceWebLookup": "yes"}}),
        "did you do the web request or was that a hallucination",
    );
    assert!(out.is_none());
}

#[test]
fn search_uses_cached_response_when_available() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let request = json!({
        "query": "agent reliability benchmark",
        "summary_only": true
    });
    let query = clean_text(
        request
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        600,
    );
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let top_k = 8usize;
    let summary_only = true;
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let (policy, _) = load_policy(tmp.path());
    let provider_chain = crate::web_conduit_provider_runtime::provider_chain_from_request(
        "auto", &request, &policy,
    );
    let key = crate::web_conduit_provider_runtime::search_cache_key(
        &query,
        &scoped_query,
        &allowed_domains,
        exclude_subdomains,
        top_k,
        summary_only,
        &provider_chain,
        &normalized_search_filters(&request),
    );
    crate::web_conduit_provider_runtime::store_search_cache(
        tmp.path(),
        &key,
        &json!({
            "ok": true,
            "summary": "cached search summary",
            "content": "",
            "provider": "duckduckgo"
        }),
        "ok",
        None,
    );

    let out = api_search(tmp.path(), &request);
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        out.get("summary").and_then(Value::as_str),
        Some("cached search summary")
    );
    assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
}

#[test]
fn search_query_shape_error_flags_markdown_link_as_fetch_intent() {
    assert_eq!(
        search_query_shape_error_code("[official docs](https://example.com/agents)"),
        "query_prefers_fetch_url"
    );
}

#[test]
fn search_query_shape_suggested_next_action_extracts_markdown_link_url() {
    let action = search_query_shape_suggested_next_action(
        "[official docs](https://example.com/agents?x=1)",
        "query_prefers_fetch_url",
    );
    assert_eq!(
        action.pointer("/action").and_then(Value::as_str),
        Some("web_conduit_fetch")
    );
    assert_eq!(
        action
            .pointer("/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://example.com/agents?x=1")
    );
}

#[test]
fn api_search_blocks_markdown_link_query_with_fetch_route_hint() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = api_search(
        tmp.path(),
        &json!({
            "query": "[OpenAI docs](https://platform.openai.com/docs)"
        }),
    );
    assert_eq!(
        out.get("query_shape_error").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.get("query_shape_route_hint").and_then(Value::as_str),
        Some("web_fetch")
    );
    assert_eq!(
        out.pointer("/suggested_next_action/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://platform.openai.com/docs")
    );
    assert_eq!(
        out.pointer("/retry/strategy").and_then(Value::as_str),
        Some("use_web_fetch_route")
    );
    assert_eq!(
        out.pointer("/retry/reason").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.pointer("/retry/contract_version").and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        out.pointer("/retry/lane").and_then(Value::as_str),
        Some("web_fetch")
    );
}

#[test]
fn search_query_shape_extracts_wrapped_url_candidate() {
    let action = search_query_shape_suggested_next_action(
        "\"<https://example.com/research?q=agent&amp;mode=1>,\"",
        "query_prefers_fetch_url",
    );
    assert_eq!(
        action
            .pointer("/payload/requested_url")
            .and_then(Value::as_str),
        Some("https://example.com/research?q=agent&mode=1")
    );
}

#[test]
fn search_query_shape_supports_www_candidate() {
    let out = api_search(
        tempfile::tempdir().expect("tempdir").path(),
        &json!({
            "query": "www.example.com/docs"
        }),
    );
    assert_eq!(
        out.get("query_shape_error").and_then(Value::as_str),
        Some("query_prefers_fetch_url")
    );
    assert_eq!(
        out.pointer("/query_shape/fetch_url_candidate")
            .and_then(Value::as_str),
        Some("https://www.example.com/docs")
    );
}

#[test]
fn fetch_request_uses_query_url_when_url_field_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = execute_fetch_request(
        tmp.path(),
        &json!({
            "query": "[read this](https://example.com/path?from=query)",
            "provider": "definitely-not-a-fetch-provider"
        }),
    );
    assert_eq!(
        out.get("error").and_then(Value::as_str),
        Some("unknown_fetch_provider")
    );
    assert_eq!(
        out.get("requested_url").and_then(Value::as_str),
        Some("https://example.com/path?from=query")
    );
    assert_eq!(
        out.get("requested_url_source").and_then(Value::as_str),
        Some("query")
    );
}
