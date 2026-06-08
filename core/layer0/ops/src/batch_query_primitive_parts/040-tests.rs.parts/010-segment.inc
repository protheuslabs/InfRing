mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct ScopedEnvVar {
        key: &'static str,
        previous: Option<String>,
    }

    impl ScopedEnvVar {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn with_fixture<T>(fixture: Value, run: impl FnOnce() -> T) -> T {
        let _guard = TEST_ENV_MUTEX.lock().expect("lock");
        let _fixture = ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&fixture).expect("encode fixture"),
        );
        run()
    }

    fn run_query(root: &Path, query: &str, aperture: &str) -> Value {
        api_batch_query(
            root,
            &json!({
                "source":"web",
                "query": query,
                "aperture": aperture
            }),
        )
    }

    fn run_query_with_fixture(fixture: Value, query: &str, aperture: &str) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || run_query(tmp.path(), query, aperture))
    }

    fn summary_lowered(out: &Value) -> String {
        out.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
    }

    #[test]
    fn summary_insights_prefer_display_claim_over_raw_ui_chrome_claim() {
        let evidence_claims = json!([
            {
                "claim": "Talk to an expertTry it out search extract crawl research Trusted by 2M+ developers around the world",
                "display_claim": "Connect your AI agents to the web with real-time search, extraction, research, and web crawling through a single API",
                "source_domain": "example-web-layer.test"
            }
        ]);

        let insights = summary_insights_from_evidence_claims(&evidence_claims, 2);

        assert_eq!(insights.len(), 1, "{insights:#?}");
        assert!(
            insights[0].contains("Connect your AI agents to the web"),
            "{insights:#?}"
        );
        assert!(!insights[0].contains("Talk to an expert"), "{insights:#?}");
        assert!(!insights[0].contains("Try it out"), "{insights:#?}");
    }

    #[test]
    fn large_aperture_is_blocked_without_policy_opt_in() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_batch_query(
            tmp.path(),
            &json!({"source": "web", "query": "agent systems", "aperture": "large"}),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn web_query_with_results_returns_evidence_and_clean_summary() {
        let out = run_query_with_fixture(
            json!({"agent systems":{"ok":true,"summary":"Agent systems coordinate tools with deterministic receipts.","requested_url":"https://example.com/agents","status_code":200}}),
            "agent systems",
            "small",
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        assert!(out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        let summary = summary_lowered(&out);
        assert!(!summary.contains("web search completed"));
        assert!(!summary.contains("key findings for"));
        assert!(!summary.contains("potential sources:"));
    }

    #[test]
    fn no_results_path_returns_clean_no_results_status() {
        let out = run_query_with_fixture(
            json!({
                "batch query no results":{"ok":false,"error":"provider_network_policy_blocked"},
                "bing_rss::batch query no results":{"ok":false,"error":"bing_rss_search_failed"}
            }),
            "batch query no results",
            "small",
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert_eq!(
            out.get("evidence_refs")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(9),
            0
        );
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("low-signal") || lowered.contains("no usable findings were extracted"));
        assert!(!lowered.contains("search returned no useful information"));
    }

    #[test]
    fn source_only_scaffold_is_filtered_and_returns_no_results() {
        let out = run_query_with_fixture(
            json!({
                "infring competitors": {
                    "ok": true,
                    "summary": "Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai, iot-analytics.com, linkedin.com.",
                    "requested_url": "https://duckduckgo.com/html/?q=infring+competitors",
                    "status_code": 200
                },
                "bing_rss::infring competitors": {
                    "ok": false,
                    "error": "bing_rss_search_failed"
                },
                "duckduckgo_instant::infring competitors": {
                    "ok": false,
                    "error": "duckduckgo_instant_no_usable_summary"
                }
            }),
            "infring competitors",
            "small",
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let lowered = summary_lowered(&out);
        assert!(!lowered.contains("key findings for"));
        assert!(!lowered.contains("potential sources:"));
        assert!(lowered.contains("usable tool findings") || lowered.contains("source-backed findings"));
    }

    #[test]
    fn medium_aperture_enables_parallel_retrieval_for_rewrites() {
        let fixture = json!({
            "agent runtime reliability":{"ok":true,"summary":"Primary finding for runtime reliability.","requested_url":"https://example.com/one","status_code":200},
            "agent runtime reliability overview":{"ok":true,"summary":"Secondary finding for runtime reliability.","requested_url":"https://example.com/two","status_code":200}
        });
        let out = run_query_with_fixture(fixture, "agent runtime reliability", "medium");
        assert_eq!(
            out.get("parallel_retrieval_used").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("rewrite_set")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0),
            1
        );
        assert_eq!(
            out.get("query_timeout_ms").and_then(Value::as_u64),
            Some(5000)
        );
        assert_eq!(out.get("parallel_window").and_then(Value::as_u64), Some(4));
    }

    #[test]
    fn query_timeout_policy_defaults_and_clamps() {
        assert_eq!(query_timeout(&json!({})).as_millis() as u64, 5000);
        assert_eq!(
            query_timeout(&json!({"batch_query":{"query_timeout_ms":100}})).as_millis() as u64,
            500
        );
        assert_eq!(
            query_timeout(&json!({"batch_query":{"query_timeout_ms":999999}})).as_millis() as u64,
            60_000
        );
    }

    #[test]
    fn instructional_prompt_rewrite_focuses_query_topic() {
        let query = "Verify web search and fetch capabilities by researching current AI agent framework benchmarks. Report top 3 performance metrics found.";
        let rewrite = normalize_instructional_query(query).expect("rewrite");
        assert!(rewrite.contains("agent"));
        assert!(rewrite.contains("benchmarks"));
        assert!(rewrite.contains("metrics"));
        assert!(!rewrite.contains("verify"));
        assert!(!rewrite.contains("report"));
        let budget = aperture_budget("medium").expect("budget");
        let (plan, rewrite_set, rewrite_applied) = build_query_plan(query, budget);
        assert!(rewrite_applied);
        assert_eq!(plan.len(), 2);
        assert_eq!(rewrite_set.len(), 1);
        assert_eq!(plan[1], rewrite);
    }

    #[test]
    fn canonical_batch_query_strips_answer_presentation_instructions() {
        let news = "Give me a concise briefing on major news from this week. Prioritize broadly important stories, group by theme, and cite sources.";
        let focused = canonical_batch_web_search_query(news);
        assert!(focused.contains("major"));
        assert!(focused.contains("news"));
        assert!(focused.contains("week"));
        assert!(!focused.contains("concise"));
        assert!(!focused.contains("briefing"));
        assert!(!focused.contains("cite"));

        let science = "What are some scientific breakthroughs reported in 2026? Give examples across different fields and cite sources.";
        let focused = canonical_batch_web_search_query(science);
        assert!(focused.contains("scientific"));
        assert!(focused.contains("breakthroughs"));
        assert!(focused.contains("2026"));
        assert!(!focused.contains("give"));
        assert!(!focused.contains("cite"));
    }

    #[test]
    fn benchmark_intent_skips_definition_noise_and_synthesizes_metrics() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "Verify web search and fetch capabilities by researching current AI agent framework benchmarks. Report top 3 performance metrics found.";
        let rewrite = normalize_instructional_query(query).expect("rewrite");
        let mut fixture = Map::new();
        fixture.insert(
            query.to_string(),
            json!({
                "ok": true,
                "summary": "VERIFY Definition & Meaning - Merriam-Webster",
                "requested_url": "https://www.merriam-webster.com/dictionary/verify",
                "status_code": 200
            }),
        );
        fixture.insert(
            rewrite,
            json!({
                "ok": true,
                "summary": "Latest benchmark run reports median latency 820ms, throughput 48 tokens/s, and task success rate 86% across top agent frameworks.",
                "requested_url": "https://artificialanalysis.ai/benchmarks/agent-frameworks",
                "status_code": 200
            }),
        );
        let out = with_fixture(Value::Object(fixture), || {
            api_batch_query(
                tmp.path(),
                &json!({"source":"web","query":query,"aperture":"medium"}),
            )
        });
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("web benchmark synthesis"));
        assert!(lowered.contains("latency") || lowered.contains("tokens/s"));
        assert!(!lowered.contains("merriam-webster"));
        let evidence = out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!evidence.is_empty());
        let has_metric_locator = evidence.iter().any(|row| {
            row.get("locator")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("artificialanalysis.ai")
        });
        assert!(has_metric_locator);
    }

    #[test]
    fn benchmark_intent_fails_closed_on_non_metric_compare_noise_snippet() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "Verify web search and fetch capabilities by researching current AI agent framework benchmarks. Report top 3 performance metrics found.";
        let rewrite = normalize_instructional_query(query).expect("rewrite");
        let mut fixture = Map::new();
        fixture.insert(
            query.to_string(),
            json!({
                "ok": true,
                "summary": "VERIFY Definition & Meaning - Merriam-Webster",
                "requested_url": "https://www.merriam-webster.com/dictionary/verify",
                "status_code": 200
            }),
        );
        fixture.insert(
            rewrite,
            json!({
                "ok": true,
                "summary": "bing.com: compare [A with B] vs compare A [with B].",
                "requested_url": "https://www.bing.com/search?q=agent+benchmark+metrics",
                "status_code": 200
            }),
        );
        let out = with_fixture(Value::Object(fixture), || {
            api_batch_query(
                tmp.path(),
                &json!({"source":"web","query":query,"aperture":"medium"}),
            )
        });
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let lowered = summary_lowered(&out);
        assert!(!lowered.contains("web benchmark synthesis"));
        assert!(lowered.contains("source-backed") || lowered.contains("low-signal"));
    }

    #[test]
    fn search_payload_prefers_result_link_locator_over_search_engine_request_url() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "current ai agent framework benchmarks latency throughput";
        let source_url = "https://artificialanalysis.ai/benchmarks/agent-frameworks";
        let out = with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Independent benchmark run reports median latency 820ms and throughput 48 tokens/s.",
                    "requested_url": "https://www.bing.com/search?q=current+ai+agent+framework+benchmarks+latency+throughput&format=rss&setlang=en-US",
                    "links": [source_url],
                    "status_code": 200
                },
                format!("primary::{source_url}"): {
                    "ok": true,
                    "summary": "Artificial Analysis published a current AI agent framework benchmark page in May 2026. The benchmark reports median latency of 820ms and throughput of 48 tokens/s, and explains that the comparison uses measured framework runs rather than generic vendor claims.",
                    "requested_url": source_url,
                    "resolved_url": source_url,
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"medium"}),
                )
            },
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("ok"),
            "{out:#}"
        );
        let locator = out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("locator"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            locator.contains("artificialanalysis.ai"),
            "locator={locator}"
        );
        assert!(!locator.contains("bing.com/search"), "locator={locator}");
    }

    #[test]
    fn provider_side_links_are_fetched_into_packaged_evidence_when_search_summary_is_thin() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "scientific breakthroughs april 2026";
        let source_url =
            "https://www.cas.org/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch";
        let out = with_fixture(
            json!({
                query: {
                    "ok": true,
                    "provider": "exa",
                    "source_kind": "exa_api_search_result",
                    "summary": "Search returned mixed provider previews and needs direct page extraction.",
                    "requested_url": "https://api.example.test/search",
                    "links": [
                        source_url,
                        "https://example.com/unrelated-directory"
                    ],
                    "status_code": 200
                },
                format!("primary::{source_url}"): {
                    "ok": true,
                    "summary": "CAS highlights scientific breakthroughs and emerging research trends to watch in 2026, including advances in AI-enabled discovery, materials research, biotechnology, and climate-focused chemistry.",
                    "requested_url": source_url,
                    "resolved_url": source_url,
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"small"}),
                )
            },
        );
        assert_ne!(
            out.get("status").and_then(Value::as_str),
            Some("no_results"),
            "{out:#}"
        );
        let evidence = out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            evidence.iter().any(|row| {
                row.get("locator")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("cas.org")
            }),
            "{out:#}"
        );
        assert!(
            out.get("evidence_pack")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{out:#}"
        );
    }

    #[test]
    fn page_extraction_prefetch_ignores_evidence_instruction_scaffold_terms() {
        let query = "source-backed scientific breakthroughs or notable research milestones from April 2026? Separate confirmed findings from early or preliminary claims";
        let source_url =
            "https://www.cas.org/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch";
        let context = "Scientific breakthroughs: 2026 emerging trends to watch - https://www.cas.org/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch - CAS summarizes emerging research trends to watch in 2026 with source-backed scientific breakthrough evidence.";

        assert_eq!(
            page_extraction_link_preflight_rejection_reason_with_context(
                query, source_url, context
            ),
            None
        );
    }

    #[test]
    fn benchmark_small_aperture_falls_through_to_bing_when_primary_is_definition_noise() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "ai agent framework benchmark metrics 2024";
        let out = with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "VERIFY Definition & Meaning - Merriam-Webster",
                    "requested_url": "https://www.merriam-webster.com/dictionary/verify",
                    "status_code": 200
                },
                format!("bing_rss::{query}"): {
                    "ok": true,
                    "summary": "Independent benchmark reports median latency 780ms, throughput 51 tokens/s, and completion success rate 88%.",
                    "requested_url": "https://artificialanalysis.ai/benchmarks/agent-frameworks",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("latency") || lowered.contains("tokens/s"));
        assert!(!lowered.contains("merriam"));
    }

    #[test]
    fn low_signal_search_payload_uses_link_fetch_fallback_for_synthesis() {
        let query = "latest technology news today";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "latest technology news today at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "",
                    "links": ["https://www.reuters.com/technology/ai-chip-demand-2026-04-01"],
                    "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
                    "status_code": 200
                },
                "fetch::https://www.reuters.com/technology/ai-chip-demand-2026-04-01": {
                    "ok": true,
                    "summary": "Reuters reports AI chip demand climbed 28% year-over-year while inference latency dropped to 640ms.",
                    "requested_url": "https://www.reuters.com/technology/ai-chip-demand-2026-04-01",
                    "status_code": 200
                }
            }),
            query,
            "small",
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("reuters") || lowered.contains("latency"));
        assert!(!lowered.contains("all regions"));
        assert!(!lowered.contains("safe search"));
    }

    #[test]
    fn search_engine_domain_only_candidates_fail_closed() {
        let query = "framework compare snapshots";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Search results portal and links",
                    "requested_url": "https://www.bing.com/search?q=framework+compare+snapshots",
                    "status_code": 200
                },
                format!("bing_rss::{query}"): {
                    "ok": true,
                    "summary": "Search result page overview",
                    "requested_url": "https://www.bing.com/search?q=framework+compare+snapshots&format=rss&setlang=en-US",
                    "status_code": 200
                },
                format!("duckduckgo_instant::{query}"): {
                    "ok": false,
                    "error": "duckduckgo_instant_no_usable_summary"
                }
            }),
            query,
            "small",
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert!(!summary_lowered(&out).contains("bing.com"));
    }

    #[test]
    fn compare_query_resolves_deictic_framework_and_blocks_grammar_noise() {
        let query = "compare this framework to openclaw";
        let out = run_query_with_fixture(
            json!({
                "compare infring to openclaw": {
                    "ok": true,
                    "summary": "bing.com: compare [A with B] vs compare A [with B] | WordReference Forums",
                    "requested_url": "https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
                    "status_code": 200
                }
            }),
            query,
            "medium",
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let lowered = summary_lowered(&out);
        assert!(
            lowered.contains("comparison findings")
                || lowered.contains("retrieval-quality miss")
                || lowered.contains("source coverage")
        );
        assert!(lowered.contains("infring"));
        assert!(lowered.contains("openclaw"));
        assert!(!lowered.contains("wordreference"));
    }

    #[test]
    fn compare_query_prefers_entities_coverage_for_synthesis() {
        let query = "compare this framework to openclaw";
        let out = run_query_with_fixture(
            json!({
                "compare infring to openclaw": {
                    "ok": true,
                    "summary": "Independent benchmark: Infring achieved 52 ops/sec while OpenClaw achieved 47 ops/sec with median latency 840ms vs 910ms.",
                    "requested_url": "https://example.com/agent-framework-benchmark",
                    "status_code": 200
                },
                "compare infring to openclaw overview": {
                    "ok": true,
                    "summary": "bing.com: compare [A with B] vs compare A [with B] | WordReference Forums",
                    "requested_url": "https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
                    "status_code": 200
                }
            }),
            query,
            "medium",
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("infring"));
        assert!(lowered.contains("openclaw"));
        assert!(lowered.contains("ops/sec") || lowered.contains("latency"));
        assert!(!lowered.contains("wordreference"));
    }

    #[test]
    fn compare_query_filters_unrelated_news_and_portal_noise() {
        let query = "compare this framework to openclaw";
        let out = run_query_with_fixture(
            json!({
                "compare infring to openclaw": {
                    "ok": true,
                    "summary": "Fox News Cut Trump Off For Gutfeld! So Trump Called Gutfeld Live On Air.",
                    "requested_url": "https://www.dailywire.com/news/fox-news-cut-trump-off-for-gutfeld-so-trump-called-gutfeld-live-on-air",
                    "status_code": 200
                },
                "compare infring to openclaw overview": {
                    "ok": true,
                    "summary": "UCSF MyChart - Login Page - Username or Forgot login information?",
                    "requested_url": "https://ucsfmychart.ucsfmedicalcenter.org/",
                    "status_code": 200
                }
            }),
            query,
            "medium",
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let lowered = summary_lowered(&out);
        assert!(
            lowered.contains("comparison findings")
                || lowered.contains("retrieval-quality miss")
                || lowered.contains("source coverage")
        );
        assert!(!lowered.contains("fox news"));
        assert!(!lowered.contains("mychart"));
    }

    #[test]
    fn vague_retry_prompt_fails_closed_instead_of_synthesizing_noise() {
        let query = "we tired to patch it, try again";
        let out = run_query_with_fixture(
            json!({
                "we tired to patch it, try again": {
                    "ok": true,
                    "summary": "Fox News Cut Trump Off For Gutfeld! So Trump Called Gutfeld Live On Air.",
                    "requested_url": "https://www.dailywire.com/news/fox-news-cut-trump-off-for-gutfeld-so-trump-called-gutfeld-live-on-air",
                    "status_code": 200
                },
                "we tired to patch it, try again overview": {
                    "ok": true,
                    "summary": "UCSF MyChart - Login Page - Username or Forgot login information?",
                    "requested_url": "https://ucsfmychart.ucsfmedicalcenter.org/",
                    "status_code": 200
                }
            }),
            query,
            "medium",
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("usable tool findings") || lowered.contains("source-backed findings"));
        assert!(!lowered.contains("fox news"));
        assert!(!lowered.contains("mychart"));
    }

    #[test]
    fn exact_match_query_disables_rewrite_and_parallel() {
        let out = run_query_with_fixture(
            json!({"\"agent::run\"":{"ok":true,"summary":"Exact symbol lookup result.","requested_url":"https://example.com/symbol","status_code":200}}),
            "\"agent::run\"",
            "medium",
        );
        assert_eq!(
            out.get("parallel_retrieval_used").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("rewrite_set")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(99),
            0
        );
    }

    #[test]
    fn ack_only_summary_is_never_returned_to_user() {
        let out = run_query_with_fixture(
            json!({"ack leak":{"ok":true,"summary":"Web search completed.","requested_url":"https://example.com/ack","status_code":200}}),
            "ack leak",
            "small",
        );
        assert!(!summary_lowered(&out).contains("web search completed"));
    }

    #[test]
    fn low_signal_search_summary_falls_back_to_content_source_domains() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "latest technology news today": {
                    "ok": true,
                    "summary": "latest technology news today at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "Tech News | Today's Latest Technology News | Reuters www.reuters.com/technology/ Find latest technology news from every corner of the globe. Technology News - CNBC www.cnbc.com/technology/ Business news related to the technology industry.",
                    "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"latest technology news today","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("reuters.com") || lowered.contains("cnbc.com"));
        assert!(!lowered.contains("all regions"));
        assert!(!lowered.contains("safe search"));
    }

    #[test]
    fn low_signal_source_only_domains_are_promoted_into_actionable_summary() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "runtime dashboard reliability": {
                    "ok": true,
                    "summary": "runtime dashboard reliability at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "Potential sources: reuters.com, cnbc.com, bloomberg.com, wsj.com.",
                    "requested_url": "https://duckduckgo.com/html/?q=runtime+dashboard+reliability",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"runtime dashboard reliability","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("reuters.com") || lowered.contains("cnbc.com"));
        assert!(lowered.contains("candidate domains") || lowered.contains("source domains"));
        assert!(!lowered.contains("potential sources:"));
        assert!(!lowered.contains("all regions"));
        assert!(!lowered.contains("safe search"));
        assert!(!lowered.contains("no usable findings"));
    }

    #[test]
    fn duckduckgo_instant_fallback_recovers_when_primary_search_has_no_findings() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "current technology news": {
                    "ok": true,
                    "summary": "current technology news at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "",
                    "requested_url": "https://duckduckgo.com/html/?q=current+technology+news",
                    "status_code": 200
                },
                "bing_rss::current technology news": {
                    "ok": false,
                    "error": "bing_rss_search_failed"
                },
                "duckduckgo_instant::current technology news": {
                    "ok": true,
                    "status_code": 200,
                    "requested_url": "https://api.duckduckgo.com/?q=current+technology+news&format=json&no_html=1&skip_disambig=1",
                    "summary": "Instant answer payload",
                    "content": "{\"Heading\":\"Technology news\",\"AbstractText\":\"Technology coverage tracks AI launches, developer tools, and product release cadence.\",\"AbstractURL\":\"https://example.com/technology-news\"}"
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"current technology news","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("technology coverage tracks ai launches"));
        let evidence = out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!evidence.is_empty());
        let locator = evidence
            .first()
            .and_then(|row| row.get("locator"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(locator.contains("example.com/technology-news"));
        let source_kind = evidence
            .first()
            .and_then(|row| row.get("source_kind"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert_eq!(source_kind, "web_duckduckgo_instant");
    }

    #[test]
    fn html_noise_content_is_slimmed_before_snippet_generation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "runtime dashboard reliability": {
                    "ok": true,
                    "summary": "",
                    "content": "<html><body><!-- tracking chrome --><script>alert('x')</script><style>.x{display:none}</style><svg><circle/></svg><noscript>Enable JavaScript to continue</noscript><template>Hydration placeholder</template><iframe src='https://tracker.example/frame'>iframe noise</iframe><picture><source srcset='data:image/png;base64,abc123'></picture><img src='data:image/png;base64,abc123' /><div class='hero' data-x='1'>Runtime contract verification is healthy and dashboard startup is deterministic.</div></body></html>",
                    "requested_url": "https://example.com/runtime-health",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"runtime dashboard reliability","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("runtime contract verification is healthy"));
        assert!(!lowered.contains("<script"));
        assert!(!lowered.contains("alert('x')"));
        assert!(!lowered.contains("<svg"));
        assert!(!lowered.contains("data:image"));
        assert!(!lowered.contains("tracking chrome"));
        assert!(!lowered.contains("enable javascript"));
        assert!(!lowered.contains("hydration placeholder"));
        assert!(!lowered.contains("iframe noise"));
    }

    #[test]
    fn batch_query_emits_nexus_connection_metadata() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"nexus route test":{"ok":true,"summary":"Route metadata fixture.","requested_url":"https://example.com/nexus","status_code":200}}),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"nexus route test","aperture":"small"}),
                )
            },
        );
        assert_eq!(
            out.pointer("/nexus_connection/source")
                .and_then(Value::as_str),
            Some("client_ingress")
        );
        assert_eq!(
            out.pointer("/nexus_connection/target")
                .and_then(Value::as_str),
            Some("context_stacks")
        );
    }

    #[test]
    fn batch_query_fails_closed_when_ingress_lifecycle_blocks_new_leases() {
        let _guard = TEST_ENV_MUTEX.lock().expect("lock");
        std::env::set_var(
            "INFRING_HIERARCHICAL_NEXUS_CLIENT_INGRESS_LIFECYCLE",
            "quiesced",
        );
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_batch_query(
            tmp.path(),
            &json!({"source":"web","query":"lifecycle deny path","aperture":"small"}),
        );
        std::env::remove_var("INFRING_HIERARCHICAL_NEXUS_CLIENT_INGRESS_LIFECYCLE");
        assert_eq!(out.get("status").and_then(Value::as_str), Some("blocked"));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("batch_query_nexus_delivery_denied")
        );
    }

    #[test]
    fn repeated_query_uses_cache_when_follow_up_source_payload_degrades() {
        let _guard = TEST_ENV_MUTEX.lock().expect("lock");
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "agent cache replay reliability";
        std::env::set_var(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            serde_json::to_string(&json!({
                query: {
                    "ok": true,
                    "summary": "Agent systems coordinate tools with deterministic receipts.",
                    "requested_url": "https://example.com/agent-cache",
                    "status_code": 200
                }
            }))
            .expect("fixture encode"),
        );
        let first = api_batch_query(
            tmp.path(),
            &json!({"source":"web","query":query,"aperture":"small"}),
        );
        std::env::set_var(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            serde_json::to_string(&json!({
                query: {
                    "ok": false,
                    "error": "provider_network_policy_blocked"
                },
                format!("bing_rss::{query}"): {
                    "ok": false,
                    "error": "bing_rss_search_failed"
                }
            }))
            .expect("fixture encode"),
        );
        let second = api_batch_query(
            tmp.path(),
            &json!({"source":"web","query":query,"aperture":"small"}),
        );
        std::env::remove_var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON");

        assert_eq!(
            first.get("cache_status").and_then(Value::as_str),
            Some("miss")
        );
        assert_eq!(
            second.get("cache_status").and_then(Value::as_str),
            Some("hit")
        );
        assert_eq!(second.get("status").and_then(Value::as_str), Some("ok"));
        let summary = second.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("deterministic receipts"));
        assert!(second
            .get("evidence_refs")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn internal_route_query_returns_local_diagnostics_hint_without_web_noise() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_batch_query(
            tmp.path(),
            &json!({
                "source":"web",
                "query":"Map `tool::web_search` into a supported route",
                "aperture":"small"
            }),
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("internal command mapping request"));
        assert!(!lowered.contains("bing.com"));
        assert!(!lowered.contains("duckduckgo"));
    }

    #[test]
    fn anti_bot_challenge_returns_actionable_no_results_summary() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "latest technology news today";
        let out = with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "DuckDuckGo DuckDuckGo Unfortunately, bots use DuckDuckGo too. Please complete the following challenge to confirm this search was made by a human. Select all squares containing a duck.",
                    "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
                    "status_code": 200
                },
                format!("bing_rss::{query}"): {
                    "ok": false,
                    "error": "bing_rss_search_failed"
                },
                format!("duckduckgo_instant::{query}"): {
                    "ok": false,
                    "error": "duckduckgo_instant_no_usable_summary"
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"small"}),
                )
            },
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("anti-bot challenge"));
        assert!(!lowered.contains("select all squares containing"));
        assert!(!lowered.contains("duckduckgo duckduckgo"));
    }

    #[test]
    fn quota_or_circuit_block_skips_generated_second_pass_recovery() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "compare remote meeting overload reduction strategies";
        let out = with_fixture(
            json!({
                query: {
                    "ok": false,
                    "provider": "exa",
                    "error": "provider_circuit_open:exa_provider_quota_exceeded_or_billing_required_http_402",
                    "summary": "Search provider is temporarily unavailable because its circuit breaker is open."
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"small"}),
                )
            },
        );

        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert_eq!(
            out.pointer("/second_pass_recovery/reason")
                .and_then(Value::as_str),
            Some("provider_access_or_budget_blocked")
        );
        assert_eq!(
            out.pointer("/second_pass_recovery/queries")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(0)
        );
        assert!(out
            .get("partial_failure_details")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.as_str()
                        .unwrap_or("")
                        .contains("provider_access_or_budget_blocked")
                })
                })
                .unwrap_or(false));
    }

    #[test]
    fn quota_or_circuit_block_does_not_skip_deferred_recovery_after_partial_usable_evidence() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "AlphaSearch BetaSearch source-backed evidence";
        let blocked = "quota blocked source lane";
        let weak = "thin unrelated source lane";
        let deferred_good = "BetaSearch source-backed evidence";
        let out = with_fixture(
            json!({
                query: {
                    "ok": true,
                    "provider": "duckduckgo",
                    "source_kind": "document_page_artifact",
                    "summary": "AlphaSearch evidence report says AlphaSearch supports web research agents with source-backed search and citation extraction.",
                    "requested_url": "https://alpha.example.test/evidence",
                    "status_code": 200
                },
                blocked: {
                    "ok": false,
                    "provider": "exa",
                    "error": "provider_circuit_open:exa_provider_quota_exceeded_or_billing_required_http_402",
                    "summary": "Search provider is temporarily unavailable because its circuit breaker is open."
                },
                weak: {
                    "ok": true,
                    "provider": "duckduckgo",
                    "summary": "A generic directory page with no source-backed product evidence.",
                    "requested_url": "https://directory.example.test/generic",
                    "status_code": 200
                },
                deferred_good: {
                    "ok": true,
                    "provider": "duckduckgo",
                    "source_kind": "document_page_artifact",
                    "summary": "BetaSearch evidence report says BetaSearch supports research agents with crawl-ready source extraction and grounded summaries.",
                    "requested_url": "https://beta.example.test/evidence",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({
                        "source": "web",
                        "query": query,
                        "aperture": "medium",
                        "queries": [blocked, weak, deferred_good]
                    }),
                )
            },
        );

        assert_eq!(
            out.pointer("/query_execution_limiter/applied")
                .and_then(Value::as_bool),
            Some(true),
            "{out:#?}"
        );
        assert_eq!(
            out.pointer("/second_pass_recovery/used")
                .and_then(Value::as_bool),
            Some(true),
            "{out:#?}"
        );
        assert_eq!(
            out.pointer("/second_pass_recovery/reason")
                .and_then(Value::as_str),
            Some("deferred_query_source_quality_recovery"),
            "{out:#?}"
        );
        assert!(
            out.pointer("/second_pass_recovery/queries")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some(deferred_good)))
                .unwrap_or(false),
            "{out:#?}"
        );
        assert!(
            out.pointer("/evidence_pack_quality/source_domain_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 2,
            "{out:#?}"
        );
    }

    #[test]
    fn quota_or_circuit_block_allows_submitted_deferred_recovery_after_zero_usable_evidence() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "AlphaSearch source-backed evidence";
        let blocked = "quota blocked AlphaSearch lane";
        let weak = "thin unrelated AlphaSearch lane";
        let deferred_good = "AlphaSearch official source";
        let out = with_fixture(
            json!({
                query: {
                    "ok": false,
                    "provider": "exa",
                    "error": "provider_circuit_open:exa_provider_quota_exceeded_or_billing_required_http_402",
                    "summary": "Search provider is temporarily unavailable because its circuit breaker is open."
                },
                blocked: {
                    "ok": false,
                    "provider": "exa",
                    "error": "provider_circuit_open:exa_provider_quota_exceeded_or_billing_required_http_402",
                    "summary": "Search provider is temporarily unavailable because its circuit breaker is open."
                },
                weak: {
                    "ok": true,
                    "provider": "duckduckgo",
                    "summary": "A generic directory page with no source-backed product evidence.",
                    "requested_url": "https://directory.example.test/generic",
                    "status_code": 200
                },
                deferred_good: {
                    "ok": true,
                    "provider": "duckduckgo",
                    "source_kind": "document_page_artifact",
                    "summary": "AlphaSearch official source says AlphaSearch supports research agents with crawl-ready source extraction and grounded summaries.",
                    "requested_url": "https://alpha.example.test/evidence",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({
                        "source": "web",
                        "query": query,
                        "aperture": "medium",
                        "queries": [blocked, weak, deferred_good]
                    }),
                )
            },
        );

        assert_eq!(
            out.pointer("/query_execution_limiter/applied")
                .and_then(Value::as_bool),
            Some(true),
            "{out:#?}"
        );
        assert_eq!(
            out.pointer("/second_pass_recovery/used")
                .and_then(Value::as_bool),
            Some(true),
            "{out:#?}"
        );
        assert_eq!(
            out.pointer("/second_pass_recovery/reason")
                .and_then(Value::as_str),
            Some("deferred_query_access_or_budget_recovery"),
            "{out:#?}"
        );
        assert!(
            out.pointer("/second_pass_recovery/queries")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some(deferred_good)))
                .unwrap_or(false),
            "{out:#?}"
        );
        assert!(
            out.get("evidence_refs")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{out:#?}"
        );
    }
}
