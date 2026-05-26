// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy authority tests)

mod web_quality_diagnostics_tests {
    use super::*;
    use std::sync::Mutex;

    static WEB_QUALITY_TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

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
        let _guard = WEB_QUALITY_TEST_ENV_MUTEX.lock().expect("lock");
        let _fixture = ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&fixture).expect("encode fixture"),
        );
        run()
    }

    fn run_query_with_fixture(fixture: Value, query: &str) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || {
            api_batch_query(
                tmp.path(),
                &json!({"source":"web","query":query,"aperture":"small"}),
            )
        })
    }

    fn run_request_with_fixture(fixture: Value, request: &Value) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || api_batch_query(tmp.path(), request))
    }

    fn quality_flags(out: &Value) -> Vec<String> {
        out.pointer("/tool_result_quality/flags")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn summary_lowered(out: &Value) -> String {
        out.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
    }

    fn candidate(locator: &str, snippet: &str) -> Candidate {
        Candidate {
            source_kind: "web".to_string(),
            title: format!("Web result from {locator}"),
            locator: locator.to_string(),
            snippet: snippet.to_string(),
            excerpt_hash: sha256_hex(snippet),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        }
    }

    fn materialized_candidate(locator: &str, snippet: &str) -> Candidate {
        let mut candidate = candidate(locator, snippet);
        candidate.source_kind = "browser_materialized_page".to_string();
        candidate.permissions = Some("public_web;browser_materialized".to_string());
        candidate
    }

    fn structured_feed_candidate(locator: &str, snippet: &str) -> Candidate {
        let mut candidate = candidate(locator, snippet);
        candidate.source_kind = "google_news_rss".to_string();
        candidate.permissions = Some("public_web;headline_feed".to_string());
        candidate
    }

    #[test]
    fn source_class_path_rules_do_not_match_url_host_text() {
        let policy = json!({
            "batch_query": {
                "evidence_pack": {
                    "source_class_rules": [
                        {"class": "announcement_or_news", "path_contains": ["/news"]}
                    ]
                }
            }
        });
        let rss_wrapper = candidate(
            "https://news.google.com/rss/articles/example",
            "A search result surfaced through a news aggregator wrapper.",
        );
        let news_article = candidate(
            "https://example.org/news/article",
            "A direct article URL whose path really contains news.",
        );

        assert_eq!(
            evidence_pack_source_class(&policy, &rss_wrapper),
            "general_web"
        );
        assert_eq!(
            evidence_pack_source_class(&policy, &news_article),
            "announcement_or_news"
        );
    }

    #[test]
    fn source_class_rules_match_title_and_snippet_hints() {
        let policy = json!({
            "batch_query": {
                "evidence_pack": {
                    "source_class_rules": [
                        {"class": "documentation_or_reference", "title_contains": ["how to", "tutorial"]},
                        {"class": "independent_analysis", "title_contains": ["best ", " vs "]},
                        {"class": "news_or_current", "snippet_contains": ["announced", "release"]}
                    ]
                }
            }
        });
        let guide = Candidate {
            source_kind: "web".to_string(),
            title: "How to build a retrieval agent".to_string(),
            locator: "https://news.google.com/rss/articles/example".to_string(),
            snippet: "Search result surfaced through an aggregator wrapper.".to_string(),
            excerpt_hash: "guide".to_string(),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        };
        let analysis = Candidate {
            source_kind: "web".to_string(),
            title: "Best retrieval tools for AI agents".to_string(),
            locator: "https://example.org/articles/result".to_string(),
            snippet: "A comparison-style article.".to_string(),
            excerpt_hash: "analysis".to_string(),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        };
        let announcement = Candidate {
            source_kind: "web".to_string(),
            title: "Provider update".to_string(),
            locator: "https://example.org/articles/result".to_string(),
            snippet: "The provider announced a new release today.".to_string(),
            excerpt_hash: "announcement".to_string(),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        };

        assert_eq!(
            evidence_pack_source_class(&policy, &guide),
            "documentation_or_reference"
        );
        assert_eq!(
            evidence_pack_source_class(&policy, &analysis),
            "independent_analysis"
        );
        assert_eq!(
            evidence_pack_source_class(&policy, &announcement),
            "news_or_current"
        );
    }

    #[test]
    fn provider_source_hint_domain_allows_source_name_parentheses() {
        let row = candidate(
            "https://news.google.com/rss/articles/example",
            "Result text. Source: Amazon Web Services (AWS) (aws.amazon.com).",
        );
        assert_eq!(candidate_domain_hint(&row), "aws.amazon.com");
    }

    #[test]
    fn social_share_wrapper_locator_decodes_target_url() {
        let wrapper = "https://www.facebook.com/share.php?u=https%3A%2F%2Faje.news%2Fswm9ax";

        assert!(citation_wrapper_link(wrapper));
        assert_eq!(
            canonical_search_result_locator(wrapper, &[]),
            "https://aje.news/swm9ax"
        );
        assert_eq!(
            candidate_domain_hint(&candidate(
                wrapper,
                "Result text shared through a social redirect wrapper."
            )),
            "aje.news"
        );
    }

    #[test]
    fn social_share_wrapper_locator_trims_concatenated_share_targets() {
        let wrapper = "https://twitter.com/intent/tweet?text=Firecrawl%20vs%20Exa%20vs%20Tavily&url=https%3A%2F%2Fapiscout.dev%2Fguides%2Ffirecrawl-vs-exa-vs-tavily-web-data-apis-2026https://www.linkedin.com/sharing/share-offsite/?url=https%3A%2F%2Fapiscout.dev%2Fguides%2Ffirecrawl-vs-exa-vs-tavily-web-data-apis-2026";

        assert!(citation_wrapper_link(wrapper));
        assert_eq!(
            canonical_search_result_locator(wrapper, &[]),
            "https://apiscout.dev/guides/firecrawl-vs-exa-vs-tavily-web-data-apis-2026"
        );
        assert_eq!(
            candidate_domain_hint(&candidate(
                wrapper,
                "Result text shared through a social redirect wrapper."
            )),
            "apiscout.dev"
        );
    }

    #[test]
    fn anti_bot_failures_emit_structured_quality_retry() {
        let query = "latest technology news today";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge.",
                    "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
                    "status_code": 200
                },
                format!("bing_rss::{query}"): {"ok": false, "error": "bing_rss_search_failed"},
                format!("duckduckgo_instant::{query}"): {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            query,
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let flags = quality_flags(&out);
        assert!(flags.iter().any(|flag| flag == "anti_bot_filtered"));
        assert!(flags.iter().any(|flag| flag == "insufficient_evidence"));
        assert_eq!(
            out.pointer("/tool_result_quality/retry/recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer(
                "/tool_result_quality/browser_materialization/recommended_when_policy_allows"
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/capability")
                .and_then(Value::as_str),
            Some("browser_materialize_page")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/decision_authority")
                .and_then(Value::as_str),
            Some("tool_cd_and_gateway_policy")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/blocker_taxonomy/primary_class")
                .and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/blocker_class")
                .and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/evidence_handoff/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/profile_compilation/status")
                .and_then(Value::as_str),
            Some("contract_ready_default_off")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/profile_compilation/raw_launch_args_accepted_from_caller")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/readiness_lifecycle/status")
                .and_then(Value::as_str),
            Some("not_configured_default_off")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/readiness_lifecycle/ordinary_research_may_install_dependency")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/retry_stop_conditions/status")
                .and_then(Value::as_str),
            Some("continue_with_alternate_provider_if_admitted")
        );
        assert_eq!(
            out.pointer(
                "/retrieval_broker/retry_stop_conditions/stop_conditions/capability_required"
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/action_status")
                .and_then(Value::as_str),
            Some("requires_admitted_alternate_provider_or_browser_retrieval_capability")
        );
    }

    #[test]
    fn blocker_taxonomy_splits_js_rate_limit_and_access_denied_failures() {
        let report = web_tool_quality_report(
            "current public research evidence",
            "no_results",
            0,
            0,
            &[
                "needs_js: please enable javascript before content renders".to_string(),
                "http_429 provider rate limit".to_string(),
                "access denied 403 forbidden".to_string(),
            ],
            &[],
            &[],
        );
        assert_eq!(
            report
                .pointer("/blocker_taxonomy/primary_class")
                .and_then(Value::as_str),
            Some("needs_js")
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("needs_js")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/action_status")
                .and_then(Value::as_str),
            Some("requires_admitted_alternate_provider_or_browser_retrieval_capability")
        );
        let classes = report
            .pointer("/blocker_taxonomy/classes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for expected in ["needs_js", "rate_limited", "access_denied"] {
            assert!(
                classes.iter().any(|row| {
                    row.get("class").and_then(Value::as_str) == Some(expected)
                        && row.get("present").and_then(Value::as_bool) == Some(true)
                }),
                "{report:#?}"
            );
        }
        assert_eq!(
            report
                .pointer("/browser_materialization/recommended_when_policy_allows")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn junk_pages_are_filtered_before_synthesis_and_diagnosed() {
        let query = "current agent framework releases";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Please enable JavaScript and cookies to continue. Access denied.",
                    "requested_url": "https://example.com/blocked",
                    "status_code": 403
                },
                format!("bing_rss::{query}"): {"ok": false, "error": "bing_rss_search_failed"},
                format!("duckduckgo_instant::{query}"): {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            query,
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let flags = quality_flags(&out);
        assert!(
            flags.iter().any(|flag| flag == "access_denied"),
            "{flags:?}"
        );
        assert!(!out
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Access denied"));
    }

    #[test]
    fn diverse_ranked_selection_avoids_same_domain_monoculture_first() {
        let ranked = vec![
            (
                candidate(
                    "https://news.example.com/one",
                    "Agent framework release notes include current benchmark data for May 2026.",
                ),
                0.94,
            ),
            (
                candidate(
                    "https://news.example.com/two",
                    "Agent framework release notes include current benchmark data for May 2026.",
                ),
                0.93,
            ),
            (
                candidate(
                    "https://docs.example.org/agent-frameworks",
                    "Agent framework documentation describes current tooling support in May 2026.",
                ),
                0.82,
            ),
        ];
        let selected = select_diverse_ranked_candidates(ranked, 2);
        let domains = selected
            .iter()
            .map(|(row, _)| candidate_domain_hint(row))
            .collect::<Vec<_>>();
        assert_eq!(domains.len(), 2);
        assert_ne!(domains[0], domains[1]);
    }

    #[test]
    fn source_trust_scoring_prefers_primary_sources_over_forums() {
        let query = "current AI agent framework release notes";
        let official = candidate(
            "https://docs.example.com/agent-framework/releases",
            "Agent framework release notes list May 2026 tool support and current APIs.",
        );
        let forum = candidate(
            "https://reddit.com/r/LocalLLaMA/comments/example",
            "A forum thread discusses agent frameworks with anecdotes and no source links.",
        );
        assert!(
            rerank_score(query, &official) > rerank_score(query, &forum),
            "official={} forum={}",
            rerank_score(query, &official),
            rerank_score(query, &forum)
        );
    }

    #[test]
    fn fallback_links_are_ranked_before_followup_fetch() {
        let payload = json!({
            "links": [
                "https://reddit.com/r/agents/comments/example",
                "https://docs.example.com/agent-framework/releases",
                "https://www.bing.com/search?q=agent+frameworks",
                "https://news.example.com/ai-agent-frameworks-2026"
            ]
        });
        let links =
            payload_links_for_fallback("current AI agent framework release notes", &payload, 2);
        assert_eq!(
            links.first().map(String::as_str),
            Some("https://docs.example.com/agent-framework/releases")
        );
        assert!(!links.iter().any(|link| link.contains("bing.com")));
    }

    #[test]
    fn quality_report_keeps_retry_query_authority_with_agent() {
        let out = run_query_with_fixture(
            json!({
                "current agent frameworks": {
                    "ok": false,
                    "error": "provider_timeout"
                },
                "bing_rss::current agent frameworks": {"ok": false, "error": "bing_rss_search_failed"},
                "duckduckgo_instant::current agent frameworks": {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            "current agent frameworks",
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retry/input_contract/authority")
                .and_then(Value::as_str),
            Some("agent_submitted")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retry/input_contract/hidden_query_expansion")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/freshness/current_intent")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn degraded_provider_issue_survives_when_one_candidate_is_retained() {
        let query = "CrewAI automation workforce training";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": false,
                    "error": "web_search_tool_surface_degraded",
                    "summary": "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                },
                format!("bing_rss::{query}"): {
                    "ok": true,
                    "summary": "AI and Automation Impact on Workforce Training | .Training - crewai.io",
                    "content": "AI and Automation Impact on Workforce Training | .Training - crewai.io — https://www.crewai.io/lander — AI and automation are revolutionizing workforce training by reshaping job roles, necessitating reskilling, and enhancing learning experiences.",
                    "requested_url": "https://www.crewai.io/lander",
                    "status_code": 200
                }
            }),
            query,
        );
        assert!(
            quality_flags(&out)
                .iter()
                .any(|flag| flag == "provider_degraded"),
            "{out:#}"
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retry/reason")
                .and_then(Value::as_str),
            Some("provider_degraded")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert!(out
            .pointer("/provider_results/0/summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("provider readiness mismatch"));
        assert_eq!(
            out.pointer("/retrieval_broker/provider_normalization/version")
                .and_then(Value::as_str),
            Some("provider_normalization_v1")
        );
        assert!(
            out.pointer("/retrieval_broker/provider_normalization/failure_classes")
                .and_then(Value::as_array)
                .map(|rows| rows
                    .iter()
                    .any(|row| row.as_str() == Some("provider_degraded")))
                .unwrap_or(false),
            "{out:#?}"
        );
    }

    #[test]
    fn successful_web_result_exports_synthesis_quality_bundle() {
        let query = "current AI agent frameworks May 2026";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "LangGraph, OpenAI Agents SDK, CrewAI, and AutoGen publish official 2026 documentation for agent framework tool use and orchestration patterns.",
                    "requested_url": "https://docs.example.com/agent-frameworks-2026",
                    "status_code": 200
                }
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        assert_eq!(
            out.pointer("/tool_result_quality/synthesis_contract/authority")
                .and_then(Value::as_str),
            Some("agent_authored")
        );
        assert!(out
            .pointer("/tool_result_quality/candidate_quality/0/snippet_preview")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("LangGraph"));
        assert_eq!(
            out.pointer("/tool_result_quality/retry/recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("synthesize_from_evidence")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/retry_stop_conditions/status")
                .and_then(Value::as_str),
            Some("stop_ready_for_synthesis")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/artifact_quarantine/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/retrieval_broker/page_readiness_extraction/status")
                .and_then(Value::as_str),
            Some("evidence_packaged")
        );
    }

    #[test]
    fn evidence_pack_exports_processible_research_context_without_answer_format() {
        let query = "scientific breakthroughs 2026";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Researchers reported a scientific breakthroughs 2026 update: an April 2026 quantum sensing result improved measurement precision and documented methods, limits, and institutional context.",
                    "requested_url": "https://science.example.edu/research/publications/scientific-breakthroughs-2026",
                    "status_code": 200
                }
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let pack = out
            .get("evidence_pack")
            .and_then(Value::as_array)
            .expect("evidence pack");
        let first = pack.first().expect("first evidence item");
        assert_eq!(
            first.get("pack_version").and_then(Value::as_str),
            Some("evidence_pack_v1")
        );
        assert_eq!(
            first.get("source_class").and_then(Value::as_str),
            Some("scholarly_or_research")
        );
        assert_eq!(
            first.get("confidence").and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            first
                .pointer("/freshness/current_intent")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(first
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(first
            .get("term_hints")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(first.pointer("/score_components/relevance").is_some());
        assert_eq!(
            first.pointer("/promotion/version").and_then(Value::as_str),
            Some("evidence_promotion_v1")
        );
        assert_eq!(
            first.pointer("/promotion/decision").and_then(Value::as_str),
            Some("promoted")
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/url_safety/status")
                .and_then(Value::as_str),
            Some("allowed_public_http_https")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/artifact_quarantine/version")
                .and_then(Value::as_str),
            Some("artifact_quarantine_v1")
        );
        assert_eq!(
            out.pointer(
                "/retrieval_broker/artifact_quarantine/evidence_promotions/0/promotion_decision"
            )
            .and_then(Value::as_str),
            Some("promoted")
        );
        assert_eq!(
            out.pointer("/evidence_pack_quality/status")
                .and_then(Value::as_str),
            Some("thin")
        );
        assert_eq!(
            out.pointer("/source_class_coverage/status")
                .and_then(Value::as_str),
            Some("limited")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/primitive")
                .and_then(Value::as_str),
            Some("web_research")
        );
        assert!(
            out.pointer("/retrieval_broker/lanes")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| {
                    row.get("lane").and_then(Value::as_str) == Some("candidate_enrichment")
                }))
                .unwrap_or(false),
            "{out:#?}"
        );
        assert!(
            out.pointer("/retrieval_broker/provider_attempts")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{out:#?}"
        );
        assert!(!out
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("claim_hints"));
    }

    #[test]
    fn evidence_promotion_marks_internal_or_credentialed_candidate_as_caveated() {
        let candidate = candidate(
            "http://user:pass@127.0.0.1/admin",
            "The public science report describes research milestones, publication dates, method limitations, institutional context, and measured outcomes for the requested investigation.",
        );
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "public science report research milestones",
            &[],
            1,
            &[(candidate, 0.91)],
            1,
        );
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first.pointer("/promotion/decision").and_then(Value::as_str),
            Some("promoted_with_caveats")
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/status")
                .and_then(Value::as_str),
            Some("unsafe_or_internal_hint")
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/credentials_in_url")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/internal_host_hint")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/url_safety/status")
                .and_then(Value::as_str),
            Some("blocked_internal_host_hint")
        );
    }

    #[test]
    fn candidate_only_rows_do_not_count_as_usable_evidence() {
        let candidate = candidate(
            "https://search.example/results/open-source-agents",
            "Open-source coding agents roundup with a long search snippet that sounds substantive enough to tempt the system, but it is still only a candidate row and not materialized source text with direct evidence support for synthesis.",
        );
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "best open source coding agents 2026",
            &[],
            1,
            &[(candidate, 0.89)],
            1,
        );
        let quality = evidence_pack_quality_report(&default_policy(), &pack, &json!([]));
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first.get("materialization_quality").and_then(Value::as_str),
            Some("candidate_only")
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(quality.get("usable_count").and_then(Value::as_u64), Some(0));
        assert_eq!(
            quality.get("candidate_only_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(quality.get("status").and_then(Value::as_str), Some("thin"));
    }

    #[test]
    fn evidence_selection_prefers_pack_ready_articles_over_directory_diversity() {
        let mut directory = candidate(
            "https://www.cnn.com/world",
            "World news directory with breaking news, video, headlines, opinion, sections, newsletters, latest updates, photos, clips, social links, topic pages, and a broad list of unrelated story links from around the world.",
        );
        directory.title =
            "World news - breaking news, video, headlines and opinion | CNN".to_string();
        directory.source_kind = "tavily_api_search_result".to_string();
        directory.permissions = Some("public_web;structured_feed".to_string());

        let first_article = materialized_candidate(
            "https://www.aljazeera.com/news/2026/5/20/multipolar-world-summit",
            "This week's world news includes a May 20, 2026 summit where leaders announced a bilateral cooperation package, and officials said the agreement includes energy, trade, and security commitments for the coming year.",
        );
        let second_article = materialized_candidate(
            "https://www.aljazeera.com/news/2026/5/21/iran-reviews-us-proposal",
            "This week's world news also includes a May 21, 2026 diplomacy report where mediators said Iran reviewed a US proposal while regional officials reported new talks, prisoner-swap discussions, and ceasefire conditions.",
        );

        let selected = select_pack_ready_ranked_candidates(
            "Give me the biggest world news from this week.",
            vec![
                (directory, 0.98),
                (first_article.clone(), 0.78),
                (second_article.clone(), 0.77),
            ],
            &[],
            2,
            1,
        );
        let locators = selected
            .iter()
            .map(|(candidate, _)| candidate.locator.as_str())
            .collect::<Vec<_>>();

        assert_eq!(locators.len(), 2, "{selected:#?}");
        assert!(locators.contains(&first_article.locator.as_str()));
        assert!(locators.contains(&second_article.locator.as_str()));
        assert!(!locators.contains(&"https://www.cnn.com/world"));
    }

    #[test]
    fn evidence_selection_dedupes_locator_and_keeps_richer_materialized_row() {
        let article_locator = "https://www.aljazeera.com/news/2026/5/20/multipolar-world-summit";
        let materialized = materialized_candidate(
            article_locator,
            "This week's world news includes a May 20, 2026 summit where leaders announced a bilateral cooperation package, and officials said the agreement includes energy, trade, and security commitments for the coming year.",
        );
        let mut structured_duplicate = structured_feed_candidate(
            article_locator,
            "This week's world news includes a summit headline and a short feed summary saying officials announced bilateral cooperation.",
        );
        structured_duplicate.title = "Summit feed summary".to_string();
        let second_article = materialized_candidate(
            "https://www.aljazeera.com/news/2026/5/21/iran-reviews-us-proposal",
            "This week's world news also includes a May 21, 2026 diplomacy report where mediators said Iran reviewed a US proposal while regional officials reported new talks, prisoner-swap discussions, and ceasefire conditions.",
        );

        let selected = select_pack_ready_ranked_candidates(
            "Give me the biggest world news from this week.",
            vec![
                (structured_duplicate, 0.99),
                (materialized.clone(), 0.78),
                (second_article.clone(), 0.77),
            ],
            &[],
            2,
            1,
        );
        let locators = selected
            .iter()
            .map(|(candidate, _)| candidate.locator.as_str())
            .collect::<Vec<_>>();
        let first = selected
            .iter()
            .find(|(candidate, _)| candidate.locator == article_locator)
            .map(|(candidate, _)| candidate)
            .expect("deduped article selected");

        assert_eq!(
            locators
                .iter()
                .filter(|locator| **locator == article_locator)
                .count(),
            1,
            "{selected:#?}"
        );
        assert_eq!(
            candidate_materialization_quality(first),
            "full_materialized"
        );
        assert!(locators.contains(&second_article.locator.as_str()));
    }

    #[test]
    fn evidence_selection_preserves_pack_ready_materialization_preference_when_covering_facets() {
        let facet = research_facet_from_metadata_text("diplomacy", 0, "facet").expect("facet");
        let stronger_materialized = materialized_candidate(
            "https://www.aljazeera.com/news/2026/5/21/diplomacy-summit-terms",
            "This week's world news includes a May 21, 2026 diplomacy report where mediators outlined ceasefire terms, trade concessions, and regional security commitments.",
        );
        let mut higher_score_feed = structured_feed_candidate(
            "https://www.reuters.com/world/diplomacy-roundup",
            "This week's world news includes a May 21, 2026 diplomacy roundup where mediators outlined ceasefire terms and regional commitments in a concise feed summary.",
        );
        higher_score_feed.title = "Diplomacy roundup".to_string();

        let selected = select_pack_ready_ranked_candidates(
            "Give me the biggest world news from this week.",
            vec![
                (higher_score_feed, 0.99),
                (stronger_materialized.clone(), 0.78),
            ],
            &[facet],
            1,
            1,
        );

        assert_eq!(selected.len(), 1, "{selected:#?}");
        assert_eq!(selected[0].0.locator, stronger_materialized.locator);
    }

    #[test]
    fn social_video_shell_rows_do_not_count_as_usable_evidence() {
        let mut candidate = materialized_candidate(
            "https://www.tiktok.com/@reviewer/video/123",
            "#robotvacuum #cleantok #pets. Keywords: AlphaVac pet hair robot vacuum, BetaBot for pets, GammaClean apartment cleanup, best pet vacuum comparison, short video.",
        );
        candidate.title = "AlphaVac vs BetaBot for dog hair | TikTok".to_string();
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "Compare AlphaVac BetaBot and GammaClean for pet hair in apartments",
            &[],
            1,
            &[(candidate, 0.91)],
            1,
        );
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .any(|row| row.as_str() == Some("social_video_shell"))
                })
                .unwrap_or(false),
            "{first:#?}"
        );
    }

    #[test]
    fn generic_media_embed_shell_rows_do_not_count_as_usable_evidence() {
        let mut candidate = materialized_candidate(
            "https://media.example.com/ad/robot-vacuum-pet-hair",
            "Sorry, our video player is not supported in this browser. Share x Social Share This Ad Link Embed Browse Home & Real Estate Appliances iRobot Roomba i7+ TV Spot, 'More Pet Hair' Get Free Access",
        );
        candidate.title = "iRobot Roomba i7+ TV Spot, 'More Pet Hair'".to_string();
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "Compare Dyson Roborock and iRobot for pet hair in apartments",
            &[],
            1,
            &[(candidate, 0.91)],
            1,
        );
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .any(|row| row.as_str() == Some("social_video_shell"))
                })
                .unwrap_or(false),
            "{first:#?}"
        );
    }

    #[test]
    fn style_shell_rows_do_not_count_as_content_rich_evidence() {
        let candidate = materialized_candidate(
            "https://support.example.com/hc/en-us/sections/downloads",
            "Downloads Support Center Copyright Zendesk, Inc. Use of this source code is governed under the Apache License, Version 2.0 found at http://www.apache.org/licenses/LICENSE-2.0. :root { --zd-color-black: #000; --zd-color-green-100: #edf8f4; --zd-color-grey-200: #ddd; } body { font-family: system-ui; }",
        );
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "Compare AlphaVac BetaBot and GammaClean for pet hair in apartments",
            &[],
            1,
            &[(candidate, 0.88)],
            1,
        );
        let quality = evidence_pack_quality_report(&default_policy(), &pack, &json!([]));
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            quality
                .get("content_rich_item_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(quality.get("status").and_then(Value::as_str), Some("thin"));
    }

    #[test]
    fn admitted_api_search_rows_keep_structured_evidence_provenance() {
        let payload = json!({
            "ok": true,
            "provider": "tavily",
            "content": "Agent research systems comparison — https://example.org/agent-research-systems — The comparison explains how agent research systems use query planning, source retrieval, evidence extraction, citation packaging, and synthesis checks to produce grounded answers for users.",
            "status_code": 200
        });
        let candidates = candidates_from_rendered_search_payload(
            "agent research systems comparison",
            &payload,
            4,
        );
        let candidate = candidates.first().expect("candidate").clone();
        assert_eq!(candidate.source_kind, "tavily_api_search_result");
        assert!(candidate
            .permissions
            .as_deref()
            .unwrap_or("")
            .contains("structured_feed"));

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "agent research systems comparison",
            &[],
            1,
            &[(candidate, 0.91)],
            1,
        );
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first.get("materialization_quality").and_then(Value::as_str),
            Some("trusted_structured_feed")
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn claim_hints_strip_markdown_heading_fragments_before_synthesis() {
        let hints = evidence_pack_claim_hints(
            "compare Roborock and iRobot for pet hair",
            "#### iRobot Roomba j9+ [...] #### Roborock Qrevo Curv [...] ## iRobot Roomba j9+ [...] With its counter-rotating brush rolls, the Roomba j9+ excels at agitating carpets and capturing pet hair. In testing, it effectively cleaned up after two heavily shedding cats without clogging.",
            4,
        );
        assert!(
            hints
                .iter()
                .any(|hint| hint.contains("counter-rotating brush rolls")),
            "{hints:#?}"
        );
        assert!(
            !hints
                .iter()
                .any(|hint| hint.contains("####") || hint.contains("[...]")),
            "{hints:#?}"
        );
    }

    #[test]
    fn claim_hints_keep_first_body_sentence_for_broad_queries() {
        let hints = evidence_pack_claim_hints(
            "give me news from this week",
            "The US Department of Justice has announced that this week's settlement blocks the IRS from reviewing tax filings connected to Trump, his family, and his businesses. Some lawmakers and legal experts say the department has violated federal law with its addendum to the settlement.",
            2,
        );

        assert!(
            hints
                .iter()
                .any(|hint| hint.contains("blocks the IRS from reviewing tax filings")),
            "{hints:#?}"
        );
    }

    #[test]
    fn claim_hints_accept_product_feature_verbs_without_forcing_domain_terms() {
        let hints = evidence_pack_claim_hints(
            "compare Dyson and iRobot for pet hair",
            "Introducing our promise to you and your pets. The new Roomba j7+ robot vacuum uses PrecisionVision Navigation to recognize and avoid pet messes. Dyson vacuums are engineered for homes with pets and automatically de-tangle long hair and pet hair.",
            4,
        );
        let joined = hints.join(" ").to_ascii_lowercase();
        assert!(joined.contains("precisionvision"), "{hints:#?}");
        assert!(
            joined.contains("de-tangle") || joined.contains("engineered"),
            "{hints:#?}"
        );
    }

    #[test]
    fn claim_hints_fallback_extracts_clean_query_overlapping_evidence_fragment() {
        let candidate = materialized_candidate(
            "https://example.test/pet-hair-vacuum",
            "Photo: Testing notes. Pet hair sticks like a magnet to carpeting and upholstery, making it hard to remove with weak suction or tangled brush rolls.",
        );
        let hints = evidence_pack_claim_hints_for_candidate(
            "compare vacuums for pet hair in apartments",
            &candidate,
            2,
        );
        assert!(
            hints
                .iter()
                .any(|hint| hint.contains("Pet hair sticks like a magnet")),
            "{hints:#?}"
        );
    }

    #[test]
    fn claim_hints_strip_markdown_action_links_without_losing_claim() {
        let hints = evidence_pack_claim_hints(
            "scientific breakthroughs reported 2026 different fields",
            "On May 20, 2026, OpenAI reported that its model has disproved a central conjecture in discrete geometry [Read the proof](https://example.test/proof.pdf) Listen to article 9:23 Share.",
            2,
        );
        let joined = hints.join(" ").to_ascii_lowercase();

        assert!(
            joined.contains("disproved a central conjecture"),
            "{hints:#?}"
        );
        assert!(!joined.contains("read the proof"), "{hints:#?}");
        assert!(!joined.contains("listen to article"), "{hints:#?}");
        assert!(!joined.contains("http"), "{hints:#?}");
    }

    #[test]
    fn claim_hints_reject_publication_workflow_and_javascript_boilerplate() {
        let workflow_hints = evidence_pack_claim_hints(
            "medical research breakthroughs 2026",
            "Before final publication, the manuscript will undergo peer review and copy editing before it appears online.",
            2,
        );
        assert!(workflow_hints.is_empty(), "{workflow_hints:#?}");

        let candidate = materialized_candidate(
            "https://example.test/research/clinical-trial",
            "Lilly reported that its triple agonist retatrutide delivered powerful weight loss in a pivotal Phase 3 obesity trial News provided by Example Research May 21, 2026, 06:45 ET Share this article javascript:;",
        );
        let hints = evidence_pack_claim_hints_for_candidate(
            "medical research breakthroughs 2026 clinical trial",
            &candidate,
            2,
        );
        let joined = hints.join(" ").to_ascii_lowercase();

        assert!(joined.contains("retatrutide"), "{hints:#?}");
        assert!(!joined.contains("share this article"), "{hints:#?}");
        assert!(!joined.contains("javascript"), "{hints:#?}");
    }

    #[test]
    fn claim_hints_require_query_overlap_even_for_first_sentence() {
        let hints = evidence_pack_claim_hints(
            "give me news from this week",
            "Tony Carruthers was granted a one-year reprieve from death after his executioners failed to find a vein for lethal injection. The US Department of Justice has announced that this week's settlement blocks the IRS from reviewing tax filings connected to Trump, his family, and his businesses.",
            4,
        );

        let joined = hints.join(" ").to_ascii_lowercase();
        assert!(!joined.contains("carruthers"), "{hints:#?}");
        assert!(
            joined.contains("this week's settlement")
                || joined.contains("blocks the irs from reviewing"),
            "{hints:#?}"
        );
    }

    #[test]
    fn current_intent_rows_without_freshness_do_not_count_as_usable_evidence() {
        let candidate = materialized_candidate(
            "https://example.org/ai-agentic-landscape",
            "AI agentic landscape analysis says agent workflows are reshaping enterprise automation, tool use, orchestration, and software delivery across several organizations.",
        );
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "AI agentic landscape May 2026 update",
            &[],
            1,
            &[(candidate, 0.91)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|flags| flags
                    .iter()
                    .any(|flag| flag.as_str() == Some("freshness_unproven")))
                .unwrap_or(false),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{first:#?}"
        );
    }

    #[test]
    fn current_intent_does_not_treat_retrieval_timestamp_as_source_freshness() {
        let mut candidate = candidate(
            "https://www.youtube.com/watch?v=iHo_YnxEiW0",
            "Top 10 Most RELIABLE NEWS Sources You Can Trust. However, with so many fake news sites out there, and so much propaganda, it can be useful to go over those news sources best known for holding their standards to something more approaching World News | Latest Top Stories - Reuters. 8 Mar 2017.",
        );
        candidate.title = "Top 10 Most RELIABLE NEWS Sources You Can Trust - YouTube".to_string();
        candidate.source_kind = "tavily_api_search_result".to_string();
        candidate.permissions = Some("public_web;structured_feed".to_string());
        candidate.timestamp = Some(crate::now_iso());

        let flags = candidate_quality_flags(
            "Give me the biggest world news from this week.",
            &candidate,
            0.93,
        );
        assert!(
            flags
                .iter()
                .any(|flag| flag.as_str() == "freshness_unproven"),
            "{flags:#?}"
        );
        assert!(
            !candidate_counts_as_query_usable_evidence(
                "Give me the biggest world news from this week.",
                &candidate,
                0.93,
            ),
            "retrieval timestamps must not make stale source rows usable"
        );
    }

    #[test]
    fn comparison_structured_feed_can_count_despite_unproven_freshness_when_content_rich() {
        let query =
            "compare Dyson V15 Detect and Shark Stratos cordless stick vacuums for pet hair 2026";
        let mut candidate = structured_feed_candidate(
            "https://provedhome.com/reviews/dyson-v15-detect-cordless-vacuum-vs-shark-stratos/",
            "Dyson V15 Detect vs Shark Stratos comparison says Dyson emphasizes laser dust detection and particle counting, while Shark emphasizes anti-hair-wrap brush design, a folding wand, and larger dustbin capacity for pet owners.",
        );
        candidate.title =
            "Dyson V15 Detect vs Shark Stratos: Which Survives Real Use? | ProvedHome".to_string();

        let flags = candidate_quality_flags(query, &candidate, 0.92);
        assert!(
            flags
                .iter()
                .any(|flag| flag.as_str() == "freshness_unproven"),
            "{flags:#?}"
        );
        assert!(
            candidate_counts_as_query_usable_evidence(query, &candidate, 0.92),
            "content-rich comparative structured-provider rows should remain usable partial evidence"
        );

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            query,
            &[],
            1,
            &[(candidate, 0.92)],
            1,
        );
        assert_eq!(
            pack.pointer("/0/counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{pack:#?}"
        );
    }

    #[test]
    fn broad_current_claim_hints_accept_event_claims_without_category_overlap() {
        let hints = evidence_pack_claim_hints(
            "Give me the biggest world news from this week.",
            "Russia holds nuclear drills on land, sea and air, joined by its ally Belarus on May 21, 2026. Associated Press updated the report with details from Russia's Defense Ministry.",
            4,
        );

        let joined = hints.join(" ").to_ascii_lowercase();
        assert!(joined.contains("nuclear drills"), "{hints:#?}");
        assert!(
            joined.contains("joined by its ally belarus") || joined.contains("associated press"),
            "{hints:#?}"
        );
    }

    #[test]
    fn broad_current_claim_hints_treat_story_words_as_non_distinctive() {
        let hints = evidence_pack_claim_hints(
            "major news from this week broadly important stories",
            "NATO allies were bewildered by Trump's about-face on US troop moves in Europe. Associated Press published the report on May 22, 2026 with details from officials and allied governments.",
            3,
        );

        let joined = hints.join(" ").to_ascii_lowercase();
        assert!(
            joined.contains("nato allies") || joined.contains("troop moves in europe"),
            "{hints:#?}"
        );
    }

    #[test]
    fn broad_current_list_briefing_clauses_become_claim_units() {
        let mut candidate = structured_feed_candidate(
            "https://www.example.org/p/briefing-21st-may-2026",
            "Example Global Briefing - 21st May 2026. Trump pressing for resolution with Iran, Groundbreaking climate law ratified at UN, Greenland back on the agenda and US pushing Cuba | Succinct public briefing.",
        );
        candidate.title = "Example Global Briefing - 21st May 2026".to_string();

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "major news from this week broadly important stories",
            &[],
            1,
            &[(candidate, 0.58)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        let hints = first
            .pointer("/claim_hints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let joined = hints
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            joined.contains("resolution with iran") || joined.contains("climate law ratified"),
            "{first:#?}"
        );
        assert_eq!(
            first
                .pointer("/counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{first:#?}"
        );
        let claims = evidence_claims_from_pack(&BatchQueryKeywordPack::default(), &pack, 4);
        assert!(
            claims
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{claims:#?}"
        );
    }

    #[test]
    fn evidence_pack_uses_retrieval_query_not_raw_instruction_shell() {
        let raw_query = "Give me a concise briefing on major news from this week. Prioritize broadly important stories, group by theme, and cite sources.";
        let retrieval_query = "major news from this week broadly important stories";
        let candidate = materialized_candidate(
            "https://www.bbc.co.uk/news/articles/cn0pk2e22jro",
            "The US Department of Justice has announced that this week's unprecedented settlement of President Donald Trump's lawsuit over the leaking of his tax returns blocks the IRS from reviewing tax filings that Trump, his family and his businesses made in 2026.",
        );

        let raw_pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            raw_query,
            &[],
            1,
            &[(candidate.clone(), 0.92)],
            1,
        );
        assert_eq!(
            raw_pack
                .pointer("/0/counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "raw instruction wording should not be the evidence relevance contract: {raw_pack:#?}"
        );

        let retrieval_pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            retrieval_query,
            &[],
            1,
            &[(candidate, 0.92)],
            1,
        );
        assert_eq!(
            retrieval_pack
                .pointer("/0/counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{retrieval_pack:#?}"
        );
        let claims =
            evidence_claims_from_pack(&BatchQueryKeywordPack::default(), &retrieval_pack, 4);
        assert!(
            claims
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{claims:#?}"
        );
    }

    #[test]
    fn broad_current_trusted_source_without_article_path_can_pass_relevance_gate() {
        let query = "scientific breakthroughs reported 2026 different fields";
        let mut candidate = structured_feed_candidate(
            "https://openai.com/index/model-disproves-discrete-geometry-conjecture/",
            "Today, we share a breakthrough on the unit distance problem. An internal OpenAI model disproved a longstanding conjecture and provided an infinite family of examples that yield a polynomial improvement.",
        );
        candidate.title =
            "An OpenAI model has disproved a central conjecture in discrete geometry | OpenAI"
                .to_string();

        assert!(
            !query_has_distinctive_relevance_terms(query),
            "broad current category wording should not force exact category overlap"
        );
        assert!(
            candidate_passes_relevance_gate(query, &candidate, false),
            "trusted current source evidence should pass for broad current research prompts"
        );
        let hints = evidence_pack_claim_hints_for_candidate(query, &candidate, 2);
        assert!(
            hints.iter().any(|hint| hint
                .to_ascii_lowercase()
                .contains("disproved a longstanding conjecture")),
            "{hints:#?}"
        );
    }

    #[test]
    fn broad_current_article_evidence_is_not_downgraded_for_generic_query_terms() {
        let query = "major news from this week broadly important stories";
        let candidate = structured_feed_candidate(
            "https://example.test/p/may-22-2026",
            "Today, we will look at several current stories spanning the globe. Senate Republicans canceled a planned vote on funding immigration enforcement yesterday amid internal disagreements, and leaders in Cuba reported new economic measures on May 22, 2026.",
        );
        let flags = candidate_quality_flags(query, &candidate, rerank_score(query, &candidate));

        assert!(
            !query_has_distinctive_relevance_terms(query),
            "generic current-news prompts should not force exact topic overlap"
        );
        assert!(
            !flags.iter().any(|flag| flag == "thin_query_overlap"),
            "{flags:#?}"
        );
        assert!(
            candidate_counts_as_query_usable_evidence(
                query,
                &candidate,
                rerank_score(query, &candidate)
            ),
            "{flags:#?}"
        );
    }

    #[test]
    fn evidence_pack_extract_strips_markdown_action_link_chrome() {
        let mut candidate = materialized_candidate(
            "https://example.test/research/milestone",
            "On May 20, 2026, Example Research reported that its model disproved a central conjecture in discrete geometry [Read the proof](https://example.test/proof.pdf)[Read the companion remarks](https://example.test/remarks.pdf) Listen to article 9:23 Share For nearly 80 years, mathematicians studied the unit distance problem.",
        );
        candidate.title = "Model disproves a central conjecture in discrete geometry".to_string();
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "scientific breakthroughs reported 2026 different fields",
            &[],
            1,
            &[(candidate, 0.95)],
            1,
        );
        let extract = pack
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("relevant_extract"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let lowered = extract.to_ascii_lowercase();

        assert!(
            lowered.contains("disproved a central conjecture"),
            "{extract}"
        );
        assert!(!lowered.contains("read the proof"), "{extract}");
        assert!(!lowered.contains("companion remarks"), "{extract}");
        assert!(!lowered.contains("listen to article"), "{extract}");
        assert!(!lowered.contains("http"), "{extract}");
    }

    #[test]
    fn anti_bot_page_copy_blocks_candidate_evidence() {
        let mut candidate = materialized_candidate(
            "https://www.bloomberg.com/news/articles/2026-05-21/ukraine-starts-major-operation",
            "Bloomberg - Are you a robot? We've detected unusual activity from your computer network. To continue, please click the box below to let us know you're not a robot.",
        );
        candidate.title = "Bloomberg - Are you a robot?".to_string();

        assert!(
            contains_antibot_marker(&candidate.snippet),
            "Bloomberg robot-check copy should be classified as anti-bot"
        );
        assert!(
            !candidate_counts_as_query_usable_evidence(
                "Give me the biggest world news from this week.",
                &candidate,
                0.92,
            ),
            "anti-bot pages must not count as usable evidence"
        );
    }

    #[test]
    fn doc_action_shell_copy_blocks_candidate_evidence() {
        let mut candidate = materialized_candidate(
            "https://developers.llamaindex.ai/python/framework/module_guides/deploying/agents/",
            "LlamaAgents Agent Workflows Introduction Copy Markdown Open in Claude Open in ChatGPT Open in Cursor View as Markdown Introduction What is a workflow.",
        );
        candidate.title = "LlamaAgents Agent Workflows".to_string();

        assert!(
            contains_web_junk_marker(&candidate.snippet),
            "doc action chrome should be classified as web junk"
        );
        assert!(
            !candidate_counts_as_query_usable_evidence(
                "Compare LlamaIndex workflows versus LangGraph for document-heavy research assistants.",
                &candidate,
                0.92,
            ),
            "doc action chrome must not count as usable evidence"
        );
    }

    #[test]
    fn title_claim_hint_rejects_non_substantive_doc_heading() {
        let mut candidate = materialized_candidate(
            "https://developers.llamaindex.ai/python/framework/module_guides/deploying/agents/",
            "LlamaAgents Agent Workflows Introduction Copy Markdown Open in Claude Open in ChatGPT Open in Cursor View as Markdown Introduction What is a workflow.",
        );
        candidate.title = "LlamaAgents Agent Workflows Introduction".to_string();

        assert!(
            evidence_pack_title_claim_hint_for_candidate("LlamaAgents workflows", &candidate)
                .is_none()
        );
    }

    #[test]
    fn evidence_selection_does_not_backfill_non_pack_ready_when_ready_exists() {
        let ready = materialized_candidate(
            "https://www.aljazeera.com/news/2026/5/20/multipolar-world-summit",
            "Politics: Multipolar world summit. Leaders announced bilateral cooperation during Putin's visit to China on May 20, 2026.",
        );
        let mut directory = structured_feed_candidate(
            "https://www.politico.com/news/primary-source",
            "Updated World News: Top & Breaking World News Today | AP News — https://apnews.com/world-news — Reuters World — https://www.reuters.com/world — headlines, sections, newsletters, photos, videos, and topic pages.",
        );
        directory.title =
            "Primary Source: Latest News, Top Stories & Analysis - POLITICO".to_string();

        let selected = select_pack_ready_ranked_candidates(
            "Give me the biggest world news from this week.",
            vec![(directory, 0.99), (ready.clone(), 0.78)],
            &[],
            2,
            1,
        );

        assert_eq!(selected.len(), 1, "{selected:#?}");
        assert_eq!(selected[0].0.locator, ready.locator);
    }

    #[test]
    fn weak_required_coverage_keeps_evidence_pack_thin() {
        let pack = json!([{
            "title": "Relevant source",
            "locator": "https://example.org/source",
            "source_domain": "example.org",
            "source_kind": "browser_materialized_page",
            "snippet": "The source says Alpha announced a concrete update this week with enough detail to support a narrow answer.",
            "claim_hints": ["Alpha announced a concrete update this week with enough detail to support a narrow answer."],
            "confidence": "usable",
            "materialization_quality": "full_materialized",
            "counts_as_usable_evidence": true,
            "quality_flags": [],
            "coverage_facets": ["facet_01"]
        }]);
        let coverage = json!([{
            "facet_id": "facet_01",
            "status": "covered"
        }, {
            "facet_id": "facet_02",
            "status": "weak"
        }]);

        let quality = evidence_pack_quality_report(&default_policy(), &pack, &coverage);
        assert_eq!(quality.get("status").and_then(Value::as_str), Some("thin"));
        assert_eq!(
            quality.get("weak_facet_count").and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn link_directory_shell_rows_do_not_count_as_usable_evidence() {
        let mut candidate = candidate(
            "https://theweek.example",
            "A weekly magazine of news, business, arts and leisure. Recent editions. Evening Review. The Nation — https://www.thenation.example — Latest. Children play on American military helicopter wreckage. A foreign policy headline says this is one assault in a global war. The Guardian — https://www.theguardian.example — France. A politics headline says officials must address reparations.",
        );
        candidate.source_kind = "tavily_api_search_result".to_string();
        candidate.permissions = Some("public_web;structured_feed".to_string());

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "give me news from this week",
            &[],
            1,
            &[(candidate, 0.97)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        let flags = first
            .get("quality_flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            flags
                .iter()
                .any(|flag| flag.as_str() == Some("link_directory_or_aggregator_shell")),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{first:#?}"
        );
        assert_eq!(
            first.get("confidence").and_then(Value::as_str),
            Some("candidate_only"),
            "{first:#?}"
        );
    }

    #[test]
    fn materialized_news_index_pages_do_not_count_as_usable_evidence() {
        let mut candidate = candidate(
            "https://apnews.example/world-news",
            "World News: Top & Breaking World News Today. AP News / Menu World SECTIONS Iran war Russia-Ukraine war China Asia Pacific Latin America Europe Africa TOP STORIES Residents burn an Ebola treatment center in Congo as anger grows over the outbreak. The Afternoon Wire Get caught up on what you may have missed throughout the day.",
        );
        candidate.source_kind = "web_conduit_fetch_page_enriched".to_string();
        candidate.permissions = Some("public_web;page_enriched".to_string());

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "give me news from this week",
            &[],
            1,
            &[(candidate, 0.97)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|flags| flags
                    .iter()
                    .any(|flag| flag.as_str() == Some("link_directory_or_aggregator_shell")))
                .unwrap_or(false),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{first:#?}"
        );
    }

    #[test]
    fn materialized_category_pages_do_not_count_as_usable_evidence_even_when_content_rich() {
        let mut candidate = candidate(
            "https://thegeochronicle.example/category/social-media-posts/",
            "Social Media Posts 2026 Red Sea Conflict: Geopolitical Risk for Global Logistics and Investors. As of May 23, 2026, shipping insurers raised premiums and carriers warned of Suez route disruptions affecting Q3 planning.",
        );
        candidate.source_kind = "web_conduit_fetch_page_enriched".to_string();
        candidate.permissions = Some("public_web;page_enriched".to_string());

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "what are the current shipping disruptions this month",
            &[],
            1,
            &[(candidate, 0.99)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|flags| flags
                    .iter()
                    .any(|flag| flag.as_str() == Some("listing_or_index_path")))
                .unwrap_or(false),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{first:#?}"
        );
        assert_eq!(
            first.get("confidence").and_then(Value::as_str),
            Some("candidate_only"),
            "{first:#?}"
        );
    }

    #[test]
    fn short_multi_story_index_pages_do_not_count_as_usable_evidence() {
        let mut candidate = candidate(
            "https://publisher.example/news/world",
            "World News More World News China Foreign Minister to Chair UN Security Council Meeting in US, Visit Canada Reuters May 22, 2026 The Latest US Sanctions Tanzanian Police Chief Over Human Rights Violations The United States has sanctioned Tanzania's police chief and barred him from entry, citing alleged human rights violations committed by the police force.",
        );
        candidate.source_kind = "web_conduit_fetch_page_enriched".to_string();
        candidate.permissions = Some("public_web;page_enriched".to_string());

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "give me news from this week",
            &[],
            1,
            &[(candidate, 0.97)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|flags| flags
                    .iter()
                    .any(|flag| flag.as_str() == Some("link_directory_or_aggregator_shell")))
                .unwrap_or(false),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{first:#?}"
        );
    }

    #[test]
    fn short_syndicated_digest_pages_do_not_count_as_usable_evidence() {
        let mut candidate = candidate(
            "https://publisher.example/posts/world-news-digest-20260322/",
            "Home Posts World News Digest 2026-03-22: Reuters, AP, BBC Headlines Daily world news summary from Reuters, AP, and BBC. US-Iran, Middle East, and global headlines.",
        );
        candidate.source_kind = "web_conduit_fetch_page_enriched".to_string();
        candidate.permissions = Some("public_web;page_enriched".to_string());

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "give me news from this week",
            &[],
            1,
            &[(candidate, 0.97)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert!(
            first
                .get("quality_flags")
                .and_then(Value::as_array)
                .map(|flags| flags
                    .iter()
                    .any(|flag| flag.as_str() == Some("link_directory_or_aggregator_shell")))
                .unwrap_or(false),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(false),
            "{first:#?}"
        );
    }

    #[test]
    fn unavailable_page_chrome_does_not_count_as_usable_evidence() {
        let candidate = materialized_candidate(
            "https://www.example.org",
            "Example News. The Iran War, Today, 23/05/2026 Homepage Accessibility links Accessibility Help Account Notifications More menu Search Close menu Main content Sorry, this episode is not currently available 23/05/2026.",
        );

        let flags = candidate_quality_flags(
            "major news from this week broadly important stories",
            &candidate,
            0.92,
        );
        assert!(
            flags
                .iter()
                .any(|flag| flag == "page_chrome_or_unavailable_shell"),
            "{flags:#?}"
        );
        assert!(!candidate_counts_as_query_usable_evidence(
            "major news from this week broadly important stories",
            &candidate,
            0.92
        ));
        let hints = evidence_pack_claim_hints_for_candidate(
            "major news from this week broadly important stories",
            &candidate,
            2,
        );
        assert!(hints.is_empty(), "{hints:#?}");
    }

    #[test]
    fn claim_text_is_synthesis_safe_rejects_teaser_shell_and_metadata_labels() {
        assert!(!claim_text_is_synthesis_safe(
            "title: \"AI Agents in Legal: Harvey AI and CoCounsel Process 10 Million Legal Documents in Q1\" description: \"Legal AI agents from Harvey AI and Thomson Reuters CoCounsel are transforming contract review.\""
        ));
        assert!(!claim_text_is_synthesis_safe(
            "This IDC Survey examines how digital sovereignty concerns are shaping cloud strategies, application placement decisions, and technology investment priorities."
        ));
        assert!(!claim_text_is_synthesis_safe(
            "Pt 2: Long term service contracts Ian Makgill Business ,Software ,Technology 27 Apr, 2026 09 Mins read If you sell long-term services into European public sector buyers, the ground is moving under your feet."
        ));
    }

    #[test]
    fn broad_current_claim_hints_use_neighboring_date_signal() {
        let claims = evidence_pack_claim_hints(
            "Give me the biggest world news from this week.",
            "Xi and Putin condemn strikes, urge US to end Iran war. The leaders urged ending war in Iran as a matter of utmost urgency. By David Brennan May 20, 2026, 8:51 AM LONDON",
            2,
        );
        assert!(
            claims
                .iter()
                .any(|claim| claim.to_ascii_lowercase().contains("leaders urged")),
            "{claims:#?}"
        );
    }

    #[test]
    fn broad_current_claim_hints_use_candidate_locator_date_signal() {
        let mut candidate = candidate(
            "https://www.cnn.example/2026/05/21/politics/iran-military-rebuild",
            "Iran rebuilding military industrial base faster than expected, already producing drones, according to US intel. CNN Politics window",
        );
        candidate.source_kind = "web_conduit_fetch_page_enriched".to_string();
        candidate.permissions = Some("public_web;page_enriched".to_string());

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "Give me the biggest world news from this week.",
            &[],
            1,
            &[(candidate, 0.97)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{first:#?}"
        );
        assert!(
            first
                .get("claim_hints")
                .and_then(Value::as_array)
                .map(|claims| !claims.is_empty())
                .unwrap_or(false),
            "{first:#?}"
        );
    }

    #[test]
    fn web_summary_does_not_fallback_to_title_when_claim_units_are_absent() {
        let query = "what are some scientific breakthroughs 2026?";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "provider": "google_news_rss",
                    "results": [{
                        "title": "7 Space Science And Technology Breakthroughs To Watch For In 2026",
                        "url": "https://science.example.org/breakthroughs-2026",
                        "snippet": "7 Space Science And Technology Breakthroughs To Watch For In 2026 Science Example Published: Thu, 01 Jan 2026 08:00:00 GMT"
                    }],
                    "status_code": 200
                }
            }),
            query,
        );

        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(
            summary.contains("no usable findings were extracted"),
            "{summary}"
        );
        assert!(
            !summary.contains("7 Space Science And Technology Breakthroughs"),
            "{summary}"
        );
        assert_eq!(
            out.get("evidence_claims")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(0),
            "{out:#?}"
        );
    }

    #[test]
    fn rss_structured_rows_keep_feed_evidence_provenance() {
        let payload = json!({
            "ok": true,
            "provider": "google_news_rss",
            "results": [{
                "title": "World policy update",
                "url": "https://news.example.org/world-policy-update",
                "snippet": "The world policy update report says governments announced new agreements, implementation deadlines, public reactions, and agency follow-up steps during the current news cycle."
            }],
            "status_code": 200
        });
        let candidates =
            candidates_from_structured_search_payload("world policy update", &payload, 4);
        let candidate = candidates.first().expect("candidate").clone();
        assert_eq!(candidate.source_kind, "google_news_rss");

        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "world policy update",
            &[],
            1,
            &[(candidate, 0.84)],
            1,
        );
        assert_eq!(
            pack.pointer("/0/materialization_quality")
                .and_then(Value::as_str),
            Some("trusted_structured_feed")
        );
        assert_eq!(
            pack.pointer("/0/counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn evidence_pack_rows_emit_answer_ready_packet_contract_fields() {
        let candidate = materialized_candidate(
            "https://science.example/reports/battery-breakthrough",
            "The science report says researchers demonstrated a battery chemistry milestone in 2026, measured cycle-life improvements, and described limitations that still need independent replication.",
        );
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "scientific breakthroughs reported in 2026",
            &[],
            1,
            &[(candidate, 0.92)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        assert_eq!(
            first
                .pointer("/evidence_packet_version")
                .and_then(Value::as_str),
            Some("evidence_packet_v1")
        );
        assert!(
            first
                .pointer("/source_type")
                .and_then(Value::as_str)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
            "{first:#?}"
        );
        assert!(
            first
                .pointer("/relevant_extract")
                .and_then(Value::as_str)
                .map(|value| value.contains("battery chemistry milestone"))
                .unwrap_or(false),
            "{first:#?}"
        );
        assert!(
            first
                .pointer("/why_relevant_to_query")
                .and_then(Value::as_str)
                .map(|value| value.split_whitespace().count() >= 4)
                .unwrap_or(false),
            "{first:#?}"
        );
        assert!(
            first
                .pointer("/claim_hints")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{first:#?}"
        );
    }

    #[test]
    fn structured_feed_titles_can_supply_claim_hints_for_relevant_rows() {
        let mut candidate = structured_feed_candidate(
            "https://www.nbcnews.com/science/example",
            "Inside a daily science briefing NBC News Published: Wed, 25 Mar 2026 07:00:00 GMT. Source: NBC News (www.nbcnews.com).",
        );
        candidate.title =
            "Inside the daily science briefing on 2026 climate research - NBC News".to_string();
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "daily science briefing on 2026 climate research",
            &[],
            1,
            &[(candidate, 0.72)],
            1,
        );
        let first = pack.pointer("/0").expect("evidence row");
        let hints = first
            .pointer("/claim_hints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!hints.is_empty(), "{first:#?}");
        assert_eq!(
            first
                .pointer("/counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{first:#?}"
        );
    }

    #[test]
    fn evidence_claims_promote_materialized_rows_into_claim_units() {
        let pack = json!([{
            "title": "Agent SDK docs",
            "locator": "https://example.test/docs/agent-sdk",
            "source_domain": "example.test",
            "source_kind": "browser_materialized_page",
            "snippet": "The Agent SDK docs say the SDK supports typed outputs and structured tool execution for agent workflows.",
            "claim_hints": ["The SDK supports typed outputs.", "The SDK supports structured tool execution."],
            "confidence": "usable",
            "materialization_quality": "full_materialized",
            "counts_as_usable_evidence": true,
            "quality_flags": ["trusted_source"],
            "coverage_facets": ["facet_01"],
            "timestamp": "2026-05-20T00:00:00Z"
        }]);
        let claims = evidence_claims_from_pack(
            &BatchQueryKeywordPack {
                entities: vec!["Agent SDK".to_string()],
                facets: vec!["typed outputs".to_string()],
                ..BatchQueryKeywordPack::default()
            },
            &pack,
            6,
        );
        assert_eq!(claims.as_array().map(Vec::len), Some(2), "{claims:#?}");
        assert_eq!(
            claims.pointer("/0/claim").and_then(Value::as_str),
            Some("The SDK supports typed outputs.")
        );
        assert_eq!(
            claims
                .pointer("/0/source_ref/materialization_quality")
                .and_then(Value::as_str),
            Some("full_materialized")
        );
        assert_eq!(
            claims.pointer("/0/entities/0").and_then(Value::as_str),
            Some("Agent SDK")
        );
    }

    #[test]
    fn claim_hints_reject_url_tails_and_subscription_boilerplate() {
        let hints = evidence_pack_claim_hints(
            "institution research funding impact",
            "MIT President Sally Kornbluth warned that the institution is doing less research and enrolling fewer graduate students as a result of federal actions, reports College and University Breaking News and Press Releases - https://www.example.com/newsroom/industry/college-news - Sign up for a Business Wire account today! Explore ways to use Business Wire features for your next news release.",
            4,
        );

        let joined = hints.join(" ").to_ascii_lowercase();
        assert!(
            joined.contains("less research") || joined.contains("fewer graduate"),
            "{hints:#?}"
        );
        assert!(!joined.contains("com/newsroom"), "{hints:#?}");
        assert!(!joined.contains("sign up"), "{hints:#?}");
        assert!(!joined.contains("business wire features"), "{hints:#?}");

        let affiliate_hints = evidence_pack_claim_hints(
            "compare vacuums for pet hair",
            "Home Appliances Top 12 Best Vacuum For Pet Hair You Need Today. As an Amazon Associate, I earn from qualifying purchases. This post contains affiliate links.",
            4,
        );
        assert!(affiliate_hints.is_empty(), "{affiliate_hints:#?}");
    }

    #[test]
    fn claim_hints_decode_html_entities_before_splitting_segments() {
        let hints = evidence_pack_claim_hints(
            "latest world news this week",
            "Trump says he will speak to Taiwan&#x27;s president in break from protocol 18 hours ago Share Save. US President Donald Trump said the call would discuss a possible arms sale.",
            2,
        );

        let joined = hints.join(" ");
        assert!(joined.contains("Taiwan's president"), "{hints:#?}");
        assert!(!joined.starts_with("s president"), "{hints:#?}");
    }

    #[test]
    fn claim_hints_reject_title_bar_source_mashups() {
        let hints = evidence_pack_claim_hints(
            "biggest world news this week",
            "US and Taiwan ‘Multipolar world’: What Xi and Putin announced after Beijing summit | Politics News | Al Jazeera —",
            2,
        );

        assert!(hints.is_empty(), "{hints:#?}");
    }

    #[test]
    fn current_freshness_accepts_relative_hour_signals() {
        let candidate = materialized_candidate(
            "https://www.bbc.com/news/articles/c78qv3w4xzqo",
            "Trump says he will speak to Taiwan's president in break from protocol 18 hours ago. US President Donald Trump said the call would discuss a possible arms sale.",
        );

        assert_eq!(
            evidence_pack_freshness_status("biggest world news this week", &candidate),
            "current_signal_present"
        );
        assert!(
            !candidate_quality_flags("biggest world news this week", &candidate, 0.7)
                .iter()
                .any(|flag| flag == "freshness_unproven")
        );
    }

    #[test]
    fn current_freshness_accepts_abbreviated_relative_hour_signals() {
        let candidate = materialized_candidate(
            "https://www.bbc.com/news/articles/c5y74lwx395o",
            "Russia's Putin vows retaliation after accusing Ukraine of hitting student dormitory 4 hrs ago. Rubio tries to reassure Nato allies over US troop deployments.",
        );

        assert_eq!(
            evidence_pack_freshness_status("give me major news from this week", &candidate),
            "current_signal_present"
        );
        assert!(
            !candidate_quality_flags("give me major news from this week", &candidate, 0.7)
                .iter()
                .any(|flag| flag == "freshness_unproven")
        );
    }

    #[test]
    fn current_freshness_accepts_weekday_structured_feed_without_stale_date() {
        let mut candidate = structured_feed_candidate(
            "https://www.pbs.org/newshour/world/nato-allies-bewildered-by-troop-moves",
            "HELSINGBORG, Sweden (AP) — NATO allies and defense officials expressed bewilderment Friday at U.S. President Donald Trump's announcement that he would send 5,000 U.S. troops to Poland just weeks after ordering the same number of forces pulled out of Europe.",
        );
        candidate.source_kind = "exa_api_search_result".to_string();
        candidate.permissions = Some("public_web;structured_feed".to_string());

        assert_eq!(
            evidence_pack_freshness_status("give me major news from this week", &candidate),
            "current_signal_present"
        );
        assert!(
            !candidate_quality_flags("give me major news from this week", &candidate, 0.7)
                .iter()
                .any(|flag| flag == "freshness_unproven")
        );
    }

    #[test]
    fn broad_current_weekday_structured_feed_avoids_thin_overlap() {
        let mut candidate = structured_feed_candidate(
            "https://www.pbs.org/newshour/world/nato-allies-bewildered-by-troop-moves",
            "HELSINGBORG, Sweden (AP) — NATO allies and defense officials expressed bewilderment Friday at U.S. President Donald Trump's announcement that he would send 5,000 U.S. troops to Poland just weeks after ordering the same number of forces pulled out of Europe.",
        );
        candidate.source_kind = "exa_api_search_result".to_string();
        candidate.permissions = Some("public_web;structured_feed".to_string());

        let flags = candidate_quality_flags("give me major news from this week", &candidate, 0.7);
        assert!(
            !flags.iter().any(|flag| flag == "thin_query_overlap"),
            "{flags:#?}"
        );
        assert!(
            !flags.iter().any(|flag| flag == "freshness_unproven"),
            "{flags:#?}"
        );
    }

    #[test]
    fn current_freshness_rejects_weekday_structured_feed_with_stale_date() {
        let mut candidate = structured_feed_candidate(
            "https://example.org/news/2026-03-22/world-roundup",
            "World News Digest 2026-03-22: Officials said Friday that trade talks would continue after a week of diplomatic meetings and market volatility.",
        );
        candidate.source_kind = "exa_api_search_result".to_string();
        candidate.permissions = Some("public_web;structured_feed".to_string());

        assert_eq!(
            evidence_pack_freshness_status("give me news from this week", &candidate),
            "freshness_unproven"
        );
        assert!(
            candidate_quality_flags("give me news from this week", &candidate, 0.7)
                .iter()
                .any(|flag| flag == "freshness_unproven")
        );
    }

    #[test]
    fn relative_current_queries_do_not_accept_bare_year_as_freshness() {
        let candidate = materialized_candidate(
            "https://example.org/analysis/2026-03-22",
            "The 2026 roundup describes spring policy developments and background context from an earlier archived edition.",
        );

        assert_eq!(
            evidence_pack_freshness_status("give me news from this week", &candidate),
            "freshness_unproven"
        );
        assert!(
            !candidate_counts_as_query_usable_evidence(
                "give me news from this week",
                &candidate,
                0.8,
            ),
            "bare current-year text should not satisfy a relative freshness request"
        );
    }

    #[test]
    fn year_scoped_queries_can_use_bare_year_freshness() {
        let candidate = materialized_candidate(
            "https://example.org/research/breakthroughs-2026",
            "The 2026 research review reports that lab teams demonstrated a new instrument for measuring protein interactions in living cells.",
        );

        assert_eq!(
            evidence_pack_freshness_status("scientific breakthroughs 2026", &candidate),
            "current_signal_present"
        );
    }

    #[test]
    fn evidence_claims_reject_url_tails_titles_and_cta_boilerplate() {
        let pack = json!([{
            "title": "How Trump's IRS settlement could block tax audits of him, his family and their businesses - BBC News",
            "locator": "https://www.bbc.com/news/articles/example",
            "source_domain": "bbc.com",
            "source_kind": "browser_materialized_page",
            "snippet": "The US Department of Justice has announced that this week's settlement blocks the IRS from reviewing tax filings connected to Trump and his businesses.",
            "claim_hints": [
                "How Trump's IRS settlement could block tax audits of him, his family and their businesses - BBC News",
                "com/tavily-ai/tavily-agent-wab - This repository provides a simple yet powerful example of building a conversational",
                "See the latest features and releases",
                "The US Department of Justice has announced that this week's settlement blocks the IRS from reviewing tax filings connected to Trump and his businesses."
            ],
            "confidence": "usable",
            "materialization_quality": "full_materialized",
            "counts_as_usable_evidence": true,
            "quality_flags": [],
            "coverage_facets": [],
            "timestamp": "2026-05-20T00:00:00Z"
        }]);

        let claims = evidence_claims_from_pack(&BatchQueryKeywordPack::default(), &pack, 8);
        let rows = claims.as_array().expect("claims");
        assert_eq!(rows.len(), 1, "{claims:#?}");
        assert_eq!(
            rows[0].get("claim").and_then(Value::as_str),
            Some("The US Department of Justice has announced that this week's settlement blocks the IRS from reviewing tax filings connected to Trump and his businesses.")
        );
    }

    #[test]
    fn quality_report_marks_comparison_sources_for_careful_synthesis() {
        let ranked = vec![
            (
                candidate(
                    "https://docs.example.com/agent-a",
                    "Agent A is faster and stronger for multi-agent task execution in 2026.",
                ),
                0.88,
            ),
            (
                candidate(
                    "https://docs.example.org/agent-b",
                    "Agent B has limitations and is slower but offers stronger integrations.",
                ),
                0.84,
            ),
        ];
        let report = web_tool_quality_report(
            "compare agent A vs agent B in 2026",
            "ok",
            2,
            2,
            &[],
            &[],
            &ranked,
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(flags
            .iter()
            .any(|flag| flag == "comparative_synthesis_required"));
        assert!(flags.iter().any(|flag| flag == "potential_source_conflict"));
        assert!(report
            .pointer("/candidate_quality/0/snippet_preview")
            .is_some());
        assert_eq!(
            report
                .pointer("/coverage/bucket_status")
                .and_then(Value::as_str),
            Some("covered")
        );
    }

    #[test]
    fn usable_evidence_does_not_misclassify_missing_premium_provider_as_starvation() {
        let ranked = vec![
            (
                structured_feed_candidate(
                    "https://news.google.com/rss/articles/example-a",
                    "Published: Mon, 20 Apr 2026 07:00:00 GMT. Source: Nature (www.nature.com). New tools drive scientific discovery with evidence from major breakthroughs, publication metadata, institution context, and direct research findings suitable for bounded synthesis.",
                ),
                0.88,
            ),
            (
                structured_feed_candidate(
                    "https://news.google.com/rss/articles/example-b",
                    "Published: Wed, 01 Apr 2026 07:00:00 GMT. Source: Phys.org (phys.org). A large-scale analysis identifies disruptive innovations in research history, describes the method used to detect breakthroughs, and gives enough context for evidence-backed synthesis.",
                ),
                0.82,
            ),
        ];
        let report = web_tool_quality_report(
            "scientific breakthroughs 2026",
            "partial",
            8,
            2,
            &["serperdev:search_providers_exhausted".to_string()],
            &["serperdev:search_providers_exhausted".to_string()],
            &ranked,
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            !flags.iter().any(|flag| flag == "provider_starved"),
            "{flags:?}"
        );
        assert!(
            flags
                .iter()
                .any(|flag| flag == "credentialed_provider_unavailable_nonblocking"),
            "{flags:?}"
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("none")
        );
    }

    #[test]
    fn tool_recovery_queries_do_not_become_required_coverage_facets() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "scientific breakthroughs 2026",
            &[
                "scientific breakthroughs 2026".to_string(),
                "scientific breakthroughs 2026 primary evidence".to_string(),
                "scientific breakthroughs 2026 official sources".to_string(),
                "scientific breakthroughs 2026 technical reports".to_string(),
            ],
            &BatchQueryKeywordPack {
                keywords: vec![
                    "scientific".to_string(),
                    "breakthroughs".to_string(),
                    "2026".to_string(),
                ],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true}}}),
            budget,
        );
        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].requested_text, "scientific breakthroughs 2026");
    }

    #[test]
    fn weak_metadata_facets_do_not_drive_coverage_selection() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "Give me the biggest world news from this week.",
            &[
                "Give me the biggest world news from this week.".to_string(),
                "world news recent developments".to_string(),
                "this week source-backed evidence".to_string(),
            ],
            &BatchQueryKeywordPack {
                keywords: vec![
                    "world news".to_string(),
                    "this week".to_string(),
                    "biggest".to_string(),
                ],
                facets: vec!["world news".to_string(), "this week".to_string()],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true,"max_facets":8}}}),
            budget,
        );
        assert!(
            facets.is_empty(),
            "pure breadth/freshness metadata should not force evidence selection: {facets:?}"
        );
    }

    #[test]
    fn source_and_presentation_metadata_do_not_become_retrieval_facets() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "What are some scientific breakthroughs reported in 2026? Give examples across different fields and cite sources.",
            &[],
            &BatchQueryKeywordPack {
                entities: vec!["2026".to_string()],
                facets: vec![
                    "multiple scientific fields".to_string(),
                    "cited sources".to_string(),
                    "peer-reviewed publications".to_string(),
                    "verified reporting".to_string(),
                ],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true,"max_facets":8}}}),
            budget,
        );
        let requested = facets
            .iter()
            .map(|facet| facet.requested_text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(requested, vec!["2026"], "{facets:?}");
    }

    #[test]
    fn concrete_product_comparison_facets_still_drive_coverage_selection() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "Compare two cordless vacuums for pet hair.",
            &[],
            &BatchQueryKeywordPack {
                entities: vec!["Dyson V15 Detect".to_string(), "Shark Stratos".to_string()],
                facets: vec![
                    "pet hair removal".to_string(),
                    "suction power".to_string(),
                    "battery runtime".to_string(),
                    "price comparison".to_string(),
                    "brushroll design".to_string(),
                    "maintenance".to_string(),
                    "reliability".to_string(),
                    "2025-2026 models".to_string(),
                ],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true,"max_facets":8}}}),
            budget,
        );
        let requested = facets
            .iter()
            .map(|facet| facet.requested_text.as_str())
            .collect::<Vec<_>>();

        for expected in [
            "Dyson V15 Detect",
            "Shark Stratos",
            "pet hair removal",
            "suction power",
            "battery runtime",
            "price comparison",
            "brushroll design",
            "maintenance",
        ] {
            assert!(requested.contains(&expected), "{requested:?}");
        }
        assert!(
            !requested.contains(&"2025-2026 models"),
            "freshness/model-year constraints should stay out of retrieval topic facets: {requested:?}"
        );
    }

    #[test]
    fn generated_query_lanes_do_not_expand_declared_coverage_obligations() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "Compare the current OpenAI Agents SDK with LangChain/LangGraph for production customer-support agents. Focus on tool orchestration, tracing, safety controls, and vendor lock-in.",
            &[
                "OpenAI Agents SDK tool orchestration".to_string(),
                "LangChain tool orchestration".to_string(),
                "LangGraph tool orchestration".to_string(),
                "OpenAI Agents SDK tracing".to_string(),
                "LangChain tracing".to_string(),
                "LangGraph tracing".to_string(),
                "OpenAI Agents SDK safety controls".to_string(),
                "LangChain safety controls".to_string(),
                "LangGraph safety controls".to_string(),
                "OpenAI Agents SDK vendor lock-in".to_string(),
                "LangChain vendor lock-in".to_string(),
            ],
            &BatchQueryKeywordPack {
                keywords: vec![
                    "production".to_string(),
                    "customer-support".to_string(),
                    "agents".to_string(),
                ],
                entities: vec![
                    "OpenAI Agents SDK".to_string(),
                    "LangChain".to_string(),
                    "LangGraph".to_string(),
                ],
                facets: vec![
                    "tool orchestration".to_string(),
                    "tracing".to_string(),
                    "safety controls".to_string(),
                    "vendor lock-in".to_string(),
                ],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true,"max_facets":8}}}),
            budget,
        );
        let requested = facets
            .iter()
            .map(|facet| facet.requested_text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(facets.len(), 7, "{requested:?}");
        assert_eq!(
            facets.iter().filter(|facet| facet.kind == "entity").count(),
            3
        );
        assert_eq!(
            facets.iter().filter(|facet| facet.kind == "facet").count(),
            4
        );
        for expected in [
            "OpenAI Agents SDK",
            "LangChain",
            "LangGraph",
            "tool orchestration",
            "tracing",
            "safety controls",
            "vendor lock-in",
        ] {
            assert!(requested.contains(&expected), "{requested:?}");
        }
        assert!(
            !requested
                .iter()
                .any(|text| text.contains("OpenAI Agents SDK tool orchestration")),
            "{requested:?}"
        );
    }

    #[test]
    fn coverage_gap_recovery_spreads_budget_across_missing_facets() {
        let budget = aperture_budget("medium").expect("medium budget");
        let policy = json!({
            "batch_query": {
                "coverage_aware_evidence": {
                    "enabled": true,
                    "max_facets": 8
                },
                "coverage_gap_recovery": {
                    "enabled": true,
                    "max_queries": 4,
                    "min_usable_evidence": 3,
                    "min_covered_facets": 3,
                    "min_covered_facet_ratio": 0.75,
                    "templates": [
                        "{facet} source-backed evidence",
                        "{facet} primary or official source",
                        "{facet} independent analysis evidence",
                        "{facet} examples reports data"
                    ]
                }
            }
        });
        let metadata = BatchQueryKeywordPack {
            facets: vec![
                "LangChain".to_string(),
                "tool orchestration".to_string(),
                "tracing".to_string(),
                "safety controls".to_string(),
            ],
            metadata_authority: "tool_structured_from_user_query_terms".to_string(),
            ..BatchQueryKeywordPack::default()
        };
        let facets = infer_research_facets(
            "Compare frameworks for production agents.",
            &[],
            &metadata,
            &policy,
            budget,
        );
        let queries = coverage_gap_recovery_queries(
            &policy,
            "Compare frameworks for production agents.",
            &[],
            &facets,
            &[candidate(
                "https://example.org/noise",
                "Garden irrigation tips and unrelated seasonal watering advice.",
            )],
            budget,
        );
        assert_eq!(
            queries,
            vec![
                "LangChain source-backed evidence",
                "tool orchestration source-backed evidence",
                "tracing source-backed evidence",
                "safety controls source-backed evidence",
            ]
        );
    }

    #[test]
    fn coverage_gap_recovery_uses_compact_entity_context_when_declared() {
        let budget = aperture_budget("medium").expect("medium budget");
        let policy = json!({
            "batch_query": {
                "coverage_aware_evidence": {
                    "enabled": true,
                    "max_facets": 8
                },
                "coverage_gap_recovery": {
                    "enabled": true,
                    "max_queries": 4,
                    "min_usable_evidence": 3,
                    "min_covered_facets": 3,
                    "min_covered_facet_ratio": 1.0,
                    "templates": [
                        "{entities} {facet} official documentation",
                        "{query} {facet} source-backed evidence"
                    ]
                }
            }
        });
        let metadata = BatchQueryKeywordPack {
            entities: vec![
                "OpenAI Agents SDK".to_string(),
                "LangChain".to_string(),
                "LangGraph".to_string(),
            ],
            facets: vec![
                "tool orchestration".to_string(),
                "safety controls".to_string(),
            ],
            metadata_authority: "tool_structured_from_user_query_terms".to_string(),
            ..BatchQueryKeywordPack::default()
        };
        let facets = infer_research_facets(
            "Compare frameworks for production customer support agents.",
            &[],
            &metadata,
            &policy,
            budget,
        );
        let queries = coverage_gap_recovery_queries(
            &policy,
            "Compare frameworks for production customer support agents.",
            &[],
            &facets,
            &[
                materialized_candidate(
                    "https://example.org/openai-agents",
                    "OpenAI Agents SDK release notes for production agents.",
                ),
                materialized_candidate(
                    "https://example.org/langchain",
                    "LangChain platform documentation for production agents.",
                ),
                materialized_candidate(
                    "https://example.org/langgraph",
                    "LangGraph runtime documentation for production agents.",
                ),
            ],
            budget,
        );

        assert_eq!(
            queries.first().map(String::as_str),
            Some("\"OpenAI Agents SDK\" LangChain LangGraph tool orchestration official documentation"),
            "{queries:?}"
        );
        assert!(
            queries
                .iter()
                .any(|query| query.contains("safety controls official documentation")),
            "{queries:?}"
        );
        assert!(
            queries.iter().take(2).all(|query| !query
                .contains("Compare frameworks for production customer support agents")),
            "{queries:?}"
        );
    }

    #[test]
    fn two_word_non_entity_facets_require_more_than_one_generic_term() {
        let mut facets = vec![
            research_facet_from_metadata_text("Exa", 0, "entity").expect("entity facet"),
            research_facet_from_metadata_text("evidence gathering", 1, "facet")
                .expect("coverage facet"),
        ];
        assign_distinctive_facet_terms(&mut facets);
        let evidence_candidate = candidate(
            "https://example.org/evidence",
            "This article mentions evidence but does not discuss collection workflows.",
        );
        let exa_candidate = candidate("https://exa.ai/docs", "Exa search documentation.");

        assert!(candidate_matches_facet(&facets[0], &exa_candidate, 2));
        assert!(
            !candidate_matches_facet(&facets[1], &evidence_candidate, 2),
            "coverage facets should not be satisfied by one generic token"
        );
    }

    #[test]
    fn candidate_truncation_preserves_late_coverage_rows() {
        let mut facets = vec![
            research_facet_from_metadata_text("Firecrawl", 0, "entity").expect("firecrawl"),
            research_facet_from_metadata_text("Tavily", 1, "entity").expect("tavily"),
            research_facet_from_metadata_text("Exa", 2, "entity").expect("exa"),
            research_facet_from_metadata_text("evidence gathering", 3, "facet")
                .expect("coverage facet"),
        ];
        assign_distinctive_facet_terms(&mut facets);
        let mut candidates = (0..10)
            .map(|index| {
                candidate(
                    &format!("https://example.org/noise-{index}"),
                    "Generic search article with no requested product coverage.",
                )
            })
            .collect::<Vec<_>>();
        candidates.push(candidate(
            "https://docs.firecrawl.dev",
            "Firecrawl crawler documentation for web extraction.",
        ));
        candidates.push(candidate(
            "https://docs.tavily.com",
            "Tavily search API documentation for agent retrieval.",
        ));
        candidates.push(candidate(
            "https://docs.exa.ai",
            "Exa neural search documentation for agent retrieval.",
        ));
        candidates.push(candidate(
            "https://example.org/evidence-gathering",
            "Evidence gathering workflows for research agents.",
        ));

        truncate_candidates_preserving_facet_coverage(
            "Firecrawl Tavily Exa evidence gathering",
            &facets,
            &mut candidates,
            6,
            2,
        );
        let joined = candidates
            .iter()
            .map(|row| format!("{} {}", row.locator, row.snippet))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();

        for expected in ["firecrawl", "tavily", "exa", "evidence gathering"] {
            assert!(joined.contains(expected), "{joined}");
        }
    }

    #[test]
    fn weak_single_research_source_recommends_agent_retry() {
        let report = web_tool_quality_report(
            "CrewAI multi agent framework documentation",
            "ok",
            1,
            1,
            &[],
            &[],
            &[(
                candidate(
                    "https://www.crewai.io/lander",
                    "AI and automation are revolutionizing workforce training by reshaping job roles, necessitating reskilling, and enhancing learning experiences.",
                ),
                0.52,
            )],
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            flags.iter().any(|flag| flag == "weak_single_source"),
            "{flags:?}"
        );
        assert_eq!(
            report
                .pointer("/retry/recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("weak_single_source")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("agent_refine_query_pack")
        );
        assert_eq!(
            report
                .pointer("/retry/query_refinement_signals/hidden_query_generation")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn candidate_without_promoted_evidence_recommends_direct_fetch() {
        let report = web_tool_quality_report(
            "public agency science breakthrough report",
            "no_results",
            2,
            0,
            &[],
            &[],
            &[(
                candidate(
                    "https://agency.example.gov/reports/science-breakthroughs",
                    "Annual science breakthroughs report with program milestones and publication links.",
                ),
                0.71,
            )],
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("direct_fetch_candidate")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/inputs/candidate_url_state")
                .and_then(Value::as_str),
            Some("candidate_url_ref_available")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/candidate_refs/0/url_safety_status")
                .and_then(Value::as_str),
            Some("allowed_public_http_https")
        );
    }

    #[test]
    fn unsafe_candidate_url_blocks_browser_materialization_recommendation() {
        let report = web_tool_quality_report(
            "public agency science breakthrough report",
            "no_results",
            1,
            0,
            &["needs_js: please enable javascript before content renders".to_string()],
            &[],
            &[(
                candidate(
                    "http://user:pass@127.0.0.1/admin",
                    "Public agency science breakthrough report shell requiring JavaScript.",
                ),
                0.74,
            )],
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/inputs/candidate_url_state")
                .and_then(Value::as_str),
            Some("candidate_url_ref_blocked_by_safety")
        );
        assert_eq!(
            report
                .pointer("/browser_materialization/url_safety/materializable_candidate_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            report
                .pointer("/browser_materialization/url_safety/candidate_refs/0/url_safety/status")
                .and_then(Value::as_str),
            Some("blocked_internal_host_hint")
        );
    }

    #[test]
    fn cached_weak_single_source_is_not_replayed_as_clean_success() {
        let report = cached_web_tool_quality_report(
            "CrewAI multi agent framework documentation",
            "ok",
            &json!([]),
            &json!([
                {
                    "title": "AI and Automation Impact on Workforce Training | .Training - crewai.io",
                    "locator": "https://www.crewai.io/lander",
                    "score": 0.52
                }
            ]),
        );
        assert_eq!(
            report.get("version").and_then(Value::as_str),
            Some(web_tool_quality_version())
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("weak_single_source")
        );
        assert_eq!(
            report
                .pointer("/retry/recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn comparison_research_requires_more_than_one_evidence_source() {
        let report = web_tool_quality_report(
            "compare CrewAI and LangGraph agent frameworks",
            "ok",
            1,
            1,
            &[],
            &[],
            &[(
                candidate(
                    "https://www.langchain.com/langgraph",
                    "LangGraph is an agent orchestration framework for reliable AI agents.",
                ),
                0.92,
            )],
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            flags
                .iter()
                .any(|flag| flag == "comparison_evidence_insufficient"),
            "{flags:?}"
        );
        assert_eq!(
            report
                .pointer("/retry/recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("comparison_evidence_insufficient")
        );
    }

    #[test]
    fn comparison_partial_preserves_actionable_evidence_when_two_entities_are_covered() {
        let comparison_entities = vec![
            "Infring".to_string(),
            "LangGraph".to_string(),
            "CrewAI".to_string(),
            "AutoGen".to_string(),
        ];
        let actionable_ranked = vec![
            (
                materialized_candidate(
                    "https://docs.langchain.com/langgraph",
                    "LangGraph supports multi-agent workflow coordination with durable execution, observability, and human review.",
                ),
                0.91,
            ),
            (
                materialized_candidate(
                    "https://docs.crewai.com/overview",
                    "CrewAI offers multi-agent workflow coordination and deployment guides.",
                ),
                0.88,
            ),
        ];
        let retained_ranked = actionable_ranked.clone();
        assert!(comparison_partial_preserves_actionable_evidence(
            "Compare LangGraph and CrewAI for multi-agent workflow coordination",
            &comparison_entities,
            &actionable_ranked,
            &retained_ranked,
        ));
    }

    #[test]
    fn comparison_partial_does_not_preserve_when_only_one_entity_is_covered() {
        let comparison_entities = vec!["LangGraph".to_string(), "CrewAI".to_string()];
        let actionable_ranked = vec![(
            candidate(
                "https://docs.langchain.com/langgraph",
                "LangGraph supports durable execution and checkpointing.",
            ),
            0.91,
        )];
        let retained_ranked = actionable_ranked.clone();
        assert!(!comparison_partial_preserves_actionable_evidence(
            "Compare LangGraph and CrewAI",
            &comparison_entities,
            &actionable_ranked,
            &retained_ranked,
        ));
    }

    #[test]
    fn comparison_partial_preserves_citable_rows_when_retained_candidates_cover_missing_side() {
        let comparison_entities = vec!["LangGraph".to_string(), "CrewAI".to_string()];
        let actionable_ranked = vec![(
            materialized_candidate(
                "https://docs.langchain.com/langgraph",
                "LangGraph supports durable multi-agent workflow coordination with checkpointing and human review.",
            ),
            0.91,
        )];
        let retained_ranked = vec![
            actionable_ranked[0].clone(),
            (
                candidate(
                    "https://docs.crewai.com/overview",
                    "CrewAI documentation describes multi-agent teams, process orchestration, tools, and deployment options.",
                ),
                0.82,
            ),
        ];
        assert!(comparison_partial_preserves_actionable_evidence(
            "Compare LangGraph and CrewAI for multi-agent orchestration",
            &comparison_entities,
            &actionable_ranked,
            &retained_ranked,
        ));
    }

    #[test]
    fn current_comparison_can_use_full_materialized_page_with_freshness_caveat() {
        let query =
            "Compare Dyson V15 Detect and Shark Stratos cordless vacuums for pet hair in 2026";
        let mut candidate = materialized_candidate(
            "https://example.com/vacuums/shark-stratos-vs-dyson-v15",
            "Shark Stratos vs. Dyson V15 - A Side-by-Side Comparison covering pet hair pickup, cordless cleaning, brush maintenance, and practical buying tradeoffs.",
        );
        candidate.title = "Shark Stratos vs. Dyson V15 - A Side-by-Side Comparison".to_string();
        let score = rerank_score(query, &candidate);
        assert!(candidate_quality_flags(query, &candidate, score)
            .iter()
            .any(|flag| flag == "freshness_unproven"));
        assert!(candidate_counts_as_query_usable_evidence(
            query, &candidate, score
        ));
    }

    #[test]
    fn current_broad_discovery_query_can_use_doi_article_titles_as_claims() {
        let query = "scientific breakthroughs reported 2026 different fields";
        let mut candidate = structured_feed_candidate(
            "https://www.science.org/doi/10.1126/science.example",
            "Scalable-manufactured randomized glass-polymer hybrid metamaterial for daytime radiative cooling. Published: 2026-05-13T10:24:00+00:00. Passive radiative cooling requires a material that radiates heat away while allowing solar radiation to pass through, and the paper describes a manufactured hybrid metamaterial.",
        );
        candidate.title =
            "Scalable-manufactured randomized glass-polymer hybrid metamaterial for daytime radiative cooling | Science"
                .to_string();
        let score = rerank_score(query, &candidate);
        let flags = candidate_quality_flags(query, &candidate, score);

        assert!(
            !flags.iter().any(|flag| flag == "thin_query_overlap"),
            "{flags:#?}"
        );
        assert!(
            candidate_counts_as_query_usable_evidence(query, &candidate, score),
            "{flags:#?}"
        );
        let hints = evidence_pack_claim_hints_for_candidate(query, &candidate, 2);
        assert!(
            hints
                .iter()
                .any(|hint| hint.to_ascii_lowercase().contains("hybrid metamaterial")),
            "{hints:#?}"
        );
    }

    #[test]
    fn comparison_query_entities_stop_before_dimension_tail() {
        let query = "Compare LangGraph vs CrewAI on reliability and deployment";
        let request = json!({
            "source": "web",
            "query": query,
            "aperture": "medium"
        });
        let budget = aperture_budget("medium").expect("medium budget");
        let plan = resolve_query_plan(&json!({}), &request, query, budget);

        assert_eq!(plan.query_metadata.entities, vec!["LangGraph", "CrewAI"]);
        assert!(
            plan.query_metadata
                .keywords
                .iter()
                .any(|keyword| keyword == "reliability"),
            "{:#?}",
            plan.query_metadata
        );
    }

    #[test]
    fn comparison_guard_keeps_hidden_search_results_when_only_one_side_retrieves() {
        let out = run_request_with_fixture(
            json!({
                "LangGraph official docs reliability deployment": {
                    "ok": true,
                    "summary": "LangGraph documentation covers durable execution, checkpoints, deployment controls, and human-in-the-loop review for reliable agents.",
                    "requested_url": "https://docs.langchain.com/langgraph",
                    "status_code": 200
                },
                "CrewAI official docs reliability deployment": {
                    "ok": false,
                    "error": "query_result_mismatch"
                }
            }),
            &json!({
                "source":"web",
                "query":"Compare LangGraph vs CrewAI on reliability and deployment",
                "queries":[
                    "LangGraph official docs reliability deployment",
                    "CrewAI official docs reliability deployment"
                ],
                "aperture":"medium"
            }),
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert!(summary_lowered(&out).contains("retrieval-quality miss"));
        assert_eq!(
            out.pointer("/search_results/0/title")
                .and_then(Value::as_str),
            Some("Web result from docs.langchain.com")
        );
        assert!(out
            .pointer("/search_results/0/snippet")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("langgraph"));
    }

    #[test]
    fn comparison_guard_keeps_ranked_preview_when_official_docs_are_too_generic_for_evidence() {
        let out = run_request_with_fixture(
            json!({
                "LangGraph official docs reliability observability human-in-the-loop deployment": {
                    "ok": true,
                    "summary": "LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain",
                    "requested_url": "https://www.langchain.com/langgraph",
                    "status_code": 200
                },
                "CrewAI official docs reliability observability human-in-the-loop deployment": {
                    "ok": false,
                    "error": "query_result_mismatch"
                }
            }),
            &json!({
                "source":"web",
                "query":"Compare LangGraph vs CrewAI on reliability, observability, human-in-the-loop, and deployment.",
                "queries":[
                    "LangGraph official docs reliability observability human-in-the-loop deployment",
                    "CrewAI official docs reliability observability human-in-the-loop deployment"
                ],
                "aperture":"medium"
            }),
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/evidence_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            out.pointer("/search_results/0/locator")
                .and_then(Value::as_str),
            Some("https://www.langchain.com/langgraph")
        );
        assert!(out
            .pointer("/tool_result_quality/flags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|flag| flag == "comparison_evidence_insufficient"));
    }

    #[test]
    fn broad_current_materialized_action_sentence_becomes_claim_hint() {
        let query = "Give me a concise briefing on major news from this week. Prioritize broadly important stories.";
        let candidate = materialized_candidate(
            "https://www.bbc.co.uk/news/articles/c0729d374mx",
            "Rubio tries to reassure Nato allies over US troop deployments 8 hours ago. Nato Secretary General Mark Rutte and US Secretary of State Marco Rubio spoke to the press ahead of a meeting with foreign ministers.",
        );

        assert!(
            candidate_counts_as_query_usable_evidence(
                query,
                &candidate,
                rerank_score(query, &candidate)
            ),
            "materialized current article text should survive generic query-usable filtering"
        );
        let hints = evidence_pack_claim_hints_for_candidate(query, &candidate, 2);

        assert!(
            hints
                .iter()
                .any(|hint| hint.contains("Rubio tries to reassure Nato allies")),
            "hints: {hints:#?}"
        );
    }

    #[test]
    fn metric_rich_structured_specs_become_claim_hints() {
        let query = "Compare Dyson V15 Detect and Shark Stratos battery runtime.";
        let mut candidate = candidate(
            "https://example.com/v15-detect",
            "V15 Detect cordless vacuum lists 60min run time and a seven-cell battery for fade-free suction. It also lists 240 Air Watts suction.",
        );
        candidate.source_kind = "exa_api_search_result".to_string();
        candidate.permissions = Some("public_web;headline_feed".to_string());

        let hints = evidence_pack_claim_hints_for_candidate(query, &candidate, 1);

        assert!(
            hints
                .iter()
                .any(|hint| hint.to_ascii_lowercase().contains("60min run time")),
            "hints: {hints:#?}"
        );
    }

    #[test]
    fn metric_rich_bullet_specs_become_claim_hints() {
        let query = "Compare Example Pro and Other Model battery runtime.";
        let mut candidate = candidate(
            "https://example.com/example-pro",
            "Example Pro device page [...] # Example Pro cordless model [...] - 240 Air Watts suction [...] - 60min run time [...] - Seven-cell battery for fade-free power.",
        );
        candidate.source_kind = "exa_api_search_result".to_string();
        candidate.permissions = Some("public_web;headline_feed".to_string());

        let hints = evidence_pack_claim_hints_for_candidate(query, &candidate, 1);

        assert!(
            hints
                .iter()
                .any(|hint| hint.to_ascii_lowercase().contains("60min run time")),
            "hints: {hints:#?}"
        );
    }

    #[test]
    fn freshness_unproven_metric_specs_can_support_comparison_evidence() {
        let query = "Compare Example Pro and Other Model battery runtime 2026.";
        let mut candidate = candidate(
            "https://example.com/example-pro",
            "Example Pro cordless model lists 60min run time and 240 Air Watts suction for its standard configuration.",
        );
        candidate.source_kind = "exa_api_search_result".to_string();
        candidate.permissions = Some("public_web;headline_feed".to_string());

        let flags = candidate_quality_flags(query, &candidate, 0.9);
        assert!(
            flags.iter().any(|flag| flag == "freshness_unproven"),
            "{flags:#?}"
        );
        assert!(
            candidate_counts_as_query_usable_evidence(query, &candidate, 0.9),
            "stable metric/spec rows should be usable comparison evidence even without a fresh date; flags: {flags:#?}"
        );
    }

    #[test]
    fn coverage_facets_match_compound_runtime_and_brushroll_variants() {
        let runtime = research_facet_from_metadata_text("battery runtime", 0, "facet").unwrap();
        let brushroll = research_facet_from_metadata_text("brushroll design", 1, "facet").unwrap();
        let runtime_candidate = candidate(
            "https://example.com/runtime",
            "The product page lists a seven-cell battery, 60min run time, and fade-free suction for cordless use.",
        );
        let brushroll_candidate = candidate(
            "https://example.com/brushbar",
            "The review describes an anti-tangle brush bar design for hair pickup and easier cleaning.",
        );

        assert!(
            candidate_matches_facet(&runtime, &runtime_candidate, 2),
            "runtime facet should match run time metric language"
        );
        assert!(
            candidate_matches_facet(&brushroll, &brushroll_candidate, 2),
            "brushroll facet should match brush bar compound language"
        );
    }
}
