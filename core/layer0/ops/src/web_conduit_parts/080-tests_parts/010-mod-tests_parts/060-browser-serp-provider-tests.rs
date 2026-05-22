// Layer ownership: core/layer0/ops::web-search-orchestration-tests (authoritative)

#[test]
fn browser_serp_materialization_extracts_decoded_result_links() {
    use base64::Engine as _;

    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode("https://example.com/research-report");
    let materialized = json!({
        "ok": true,
        "materialized_page": {
            "status_code": 200,
            "title": "Search",
            "main_text_or_markdown": "Example research report\nThis page has a useful source-backed summary.",
            "links_summary": [
                {
                    "href": format!("https://www.bing.com/ck/a?u=a1{encoded}"),
                    "text": "Example research report"
                },
                {
                    "href": "https://www.bing.com/search?q=example",
                    "text": "Images"
                }
            ],
            "blocker_classification": {"status": "clear"}
        }
    });

    let out = render_browser_serp_materialization(
        "bing_html",
        "https://www.bing.com/search?q=example",
        &materialized,
        &[],
        false,
        8,
        12_000,
    );

    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        out.pointer("/links/0").and_then(Value::as_str),
        Some("https://example.com/research-report")
    );
    assert_eq!(
        out.get("provider_filtered_count").and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn browser_serp_materialization_reports_challenge_as_provider_blocker() {
    let materialized = json!({
        "ok": true,
        "materialized_page": {
            "status_code": 200,
            "title": "DuckDuckGo",
            "main_text_or_markdown": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge.",
            "links_summary": [
                {
                    "href": "https://html.duckduckgo.com/html/",
                    "text": "DuckDuckGo"
                }
            ],
            "blocker_classification": {"status": "challenge"}
        }
    });

    let out = render_browser_serp_materialization(
        "duckduckgo_html",
        "https://duckduckgo.com/html/?q=example",
        &materialized,
        &[],
        false,
        8,
        12_000,
    );

    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(out.get("error").and_then(Value::as_str), Some("anti_bot_challenge"));
    assert_eq!(
        out.get("provider_filtered_count").and_then(Value::as_u64),
        Some(0)
    );
    assert!(payload_looks_like_search_challenge(&out));
}

#[test]
fn browser_serp_default_engine_list_avoids_known_challenge_lane() {
    let engines = browser_serp_engine_urls("example query", 8)
        .into_iter()
        .map(|(engine, _)| engine)
        .collect::<Vec<_>>();
    assert_eq!(engines, vec!["bing_html"]);
}
