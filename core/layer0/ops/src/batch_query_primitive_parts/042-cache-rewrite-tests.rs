mod cache_rewrite_tests {
    use super::*;
    use std::sync::Mutex;

    static CACHE_REWRITE_TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

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
        let _guard = CACHE_REWRITE_TEST_ENV_MUTEX.lock().expect("lock");
        let _fixture = ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&fixture).expect("encode fixture"),
        );
        run()
    }

    fn run_request_with_fixture(fixture: Value, request: &Value) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || api_batch_query(tmp.path(), request))
    }

    #[test]
    fn cached_framework_summary_is_rewritten_from_evidence_refs_on_hit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let query = "top AI agentic frameworks";
        let key = cache_key_with_query_plan(
            "web",
            query,
            "medium",
            &policy,
            &[
                "top AI agentic frameworks".to_string(),
                "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                    .to_string(),
            ],
        );
        let now_ts = chrono::Utc::now().timestamp();
        let payload = json!({
            "version": 1,
            "entries": {
                key: {
                    "stored_at": now_ts,
                    "expires_at": now_ts + 120,
                    "response": {
                        "status": "ok",
                        "summary": "Key findings: langchain.com: LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain",
                        "evidence_refs": [
                            {"title":"Web result from langchain.com","locator":"https://www.langchain.com/langgraph","score":0.78},
                            {"title":"Web result from crewai.com","locator":"https://crewai.com/","score":0.66}
                        ],
                        "rewrite_set": ["AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"],
                        "query_plan": [
                            "top AI agentic frameworks",
                            "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                        ],
                        "query_plan_source": "explicit_request_pack",
                        "parallel_retrieval_used": true,
                        "partial_failure_details": []
                    }
                }
            }
        });
        write_json_atomic(&cache_path(tmp.path()), &payload).expect("write cache");

        let out = api_batch_query(
            tmp.path(),
            &json!({
                "source":"web",
                "query": query,
                "queries": [
                    "top AI agentic frameworks",
                    "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                ],
                "aperture":"medium"
            }),
        );

        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("langgraph"), "{summary}");
        assert!(lowered.contains("crewai"), "{summary}");
        assert!(!lowered.contains("zhihu.com"), "{summary}");
    }

    #[test]
    fn official_source_lane_recovers_with_verified_domain_probe() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let query = "Firecrawl web search apis official";
        let fixture = json!({
            "official_domain_probe::https://firecrawl.dev/": {
                "ok": true,
                "type": "web_conduit_fetch",
                "source_kind": "web_conduit_fetch",
                "requested_url": "https://firecrawl.dev/",
                "resolved_url": "https://firecrawl.dev/",
                "final_url": "https://www.firecrawl.dev/",
                "status_code": 200,
                "summary": "Firecrawl is an API to search, scrape, crawl, and extract clean web data for AI agents and web research workflows.",
                "content": "Firecrawl API docs describe search, scrape, crawl, and extract endpoints for converting websites into LLM-ready data.",
                "error": null
            }
        });

        let (candidates, issues, provider_results) = with_fixture(fixture, || {
            retrieve_web_candidates_for_query(
                tmp.path(),
                query,
                &policy,
                &BatchQuerySearchScope::default(),
                PageExtractionFetchBudget::new(&policy),
            )
        });

        assert!(
            candidates
                .iter()
                .any(|row| row.locator.contains("firecrawl.dev")),
            "{candidates:#?}"
        );
        assert!(
            issues.iter().any(|row| row == "official_domain_probe:attempted"),
            "{issues:#?}"
        );
        assert!(
            provider_results.iter().any(|row| {
                row.get("stage").and_then(Value::as_str) == Some("official_domain_probe")
                    && row.get("synthesis_candidate_count").and_then(Value::as_u64) == Some(1)
            }),
            "{provider_results:#?}"
        );
    }

    #[test]
    fn disabled_cache_mode_bypasses_read_and_write_for_isolated_runs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let query = "cache isolation test";
        let key = cache_key("web", query, "small", &policy);
        let now_ts = chrono::Utc::now().timestamp();
        write_json_atomic(
            &cache_path(tmp.path()),
            &json!({
                "version": 1,
                "entries": {
                    key.clone(): {
                        "stored_at": now_ts,
                        "expires_at": now_ts + 120,
                        "status": "ok",
                        "response": {
                            "status": "ok",
                            "summary": "stale cached only text",
                            "evidence_refs": [
                                {"title":"Stale cache row","locator":"https://stale.example/cache","score":0.9}
                            ],
                            "partial_failure_details": []
                        }
                    }
                }
            }),
        )
        .expect("write cache");

        let out = run_request_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "cache isolation test fresh retrieval evidence",
                    "content": "Cache isolation test — https://example.org/cache-isolation — Cache isolation keeps eval runs from replaying old evidence.",
                    "links": ["https://example.org/cache-isolation"],
                    "requested_url": "https://example.org/cache-isolation",
                    "status_code": 200
                }
            }),
            &json!({
                "source": "web",
                "query": query,
                "aperture": "small",
                "cache": {"mode": "disabled"}
            }),
        );

        assert_eq!(
            out.get("cache_status").and_then(Value::as_str),
            Some("disabled")
        );
        assert_eq!(
            out.get("cache_mode").and_then(Value::as_str),
            Some("disabled")
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(!summary.contains("stale cached only text"), "{summary}");

        let cache = read_json_or(&cache_path(tmp.path()), json!({}));
        let stored_summary = cache
            .pointer(&format!("/entries/{key}/response/summary"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert_eq!(stored_summary, "stale cached only text");
    }

    #[test]
    fn cache_cleanup_prunes_expired_entries_and_caps_retained_entries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let now_ts = chrono::Utc::now().timestamp();
        write_json_atomic(
            &cache_path(tmp.path()),
            &json!({
                "version": 1,
                "entries": {
                    "expired": {
                        "stored_at": now_ts - 400,
                        "expires_at": now_ts - 1,
                        "status": "ok",
                        "response": {"status": "ok"}
                    },
                    "oldest": {
                        "stored_at": now_ts - 300,
                        "expires_at": now_ts + 300,
                        "status": "ok",
                        "response": {"status": "ok"}
                    },
                    "middle": {
                        "stored_at": now_ts - 200,
                        "expires_at": now_ts + 300,
                        "status": "ok",
                        "response": {"status": "ok"}
                    },
                    "newest": {
                        "stored_at": now_ts - 100,
                        "expires_at": now_ts + 300,
                        "status": "ok",
                        "response": {"status": "ok"}
                    }
                }
            }),
        )
        .expect("write cache");
        let control = BatchQueryCacheControl {
            mode: "enabled".to_string(),
            ttl_success_secs: 1800,
            ttl_no_results_secs: 120,
            max_entries: 2,
        };

        let report = prune_batch_query_cache(tmp.path(), &control);

        assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            report.get("removed_entries").and_then(Value::as_u64),
            Some(2)
        );
        let cache = read_json_or(&cache_path(tmp.path()), json!({}));
        let entries = cache
            .get("entries")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(entries.len(), 2, "{entries:?}");
        assert!(entries.contains_key("middle"), "{entries:?}");
        assert!(entries.contains_key("newest"), "{entries:?}");
    }

    #[test]
    fn cache_identity_ignores_lifecycle_policy_changes() {
        let base = json!({
            "batch_query": {
                "enabled_sources": ["web"],
                "allow_large": false,
                "cache": {"mode": "enabled", "max_entries": 240}
            }
        });
        let changed_lifecycle = json!({
            "batch_query": {
                "enabled_sources": ["web"],
                "allow_large": false,
                "cache": {"mode": "disabled", "max_entries": 12}
            }
        });

        assert_eq!(
            cache_key("web", "cache key test", "small", &base),
            cache_key("web", "cache key test", "small", &changed_lifecycle)
        );
    }

    #[test]
    fn framework_catalog_query_plan_preserves_official_domain_queries() {
        let payload = json!({
            "source": "web",
            "query": "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
            "queries": [
                "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
                "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
                "site:langchain.com LangGraph agent framework overview",
                "site:openai.github.io/openai-agents-python OpenAI Agents SDK overview",
                "site:crewai.com CrewAI agent framework overview",
                "site:microsoft.github.io AutoGen framework overview",
                "site:github.com huggingface/smolagents smolagents framework overview",
                "OpenAI Agents SDK official docs overview"
            ],
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );
        assert!(
            matches!(
                plan.query_plan_source,
                "explicit_request_pack" | "tool_inferred_query_pack_from_user_query"
            ),
            "{}",
            plan.query_plan_source
        );
        assert_eq!(plan.rerank_query, query);
        assert!(plan.queries.len() >= 8, "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:openai.github.io/openai-agents-python")), "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:microsoft.github.io")), "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:github.com huggingface/smolagents")), "{:?}", plan.queries);
    }

    #[test]
    fn explicit_metadata_query_plan_does_not_promote_criterion_tail() {
        let payload = json!({
            "source": "web",
            "query": "Research current bookkeeping and finance automation options for a small services business. Compare QuickBooks, Xero, Pilot, and Puzzle on practicality and workflow fit.",
            "queries": [
                "Research current bookkeeping and finance automation options for a small services business. Compare QuickBooks, Xero, Pilot, and Puzzle on practicality and workflow fit.",
                "QuickBooks Xero Pilot Puzzle comparison",
                "QuickBooks Xero Pilot Puzzle independent comparison"
            ],
            "keywords": [
                "QuickBooks",
                "Xero",
                "Pilot",
                "Puzzle",
                "bookkeeping",
                "finance",
                "automation"
            ],
            "required_coverage": {
                "entities": ["QuickBooks", "Xero", "Pilot", "Puzzle"],
                "facets": []
            },
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );

        assert_eq!(
            plan.query_plan_source,
            "explicit_request_pack_with_metadata"
        );
        assert!(
            plan.queries
                .first()
                .map(|row| row.contains("QuickBooks") && row.contains("Xero"))
                .unwrap_or(false),
            "{:?}",
            plan.queries
        );
        assert!(
            plan.queries
                .iter()
                .take(3)
                .all(|row| row != "practicality and workflow fit"),
            "{:?}",
            plan.queries
        );
    }

    #[test]
    fn multi_entity_initial_execution_frontloads_entity_lanes() {
        let payload = json!({
            "source": "web",
            "query": "Compare Firecrawl, Tavily, and Exa as web research APIs.",
            "keywords": ["web research APIs", "official docs", "pricing"],
            "required_coverage": {
                "entities": ["Firecrawl", "Tavily", "Exa"],
                "facets": []
            },
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let budget = aperture_budget("medium").expect("budget");
        let plan = resolve_query_plan(&default_policy(), &payload, &query, budget);
        let initial = execution_limited_initial_queries(
            &default_policy(),
            budget,
            &plan.query_metadata,
            &plan.queries,
        );

        assert_eq!(initial.len(), 4, "{initial:?}");
        for entity in ["firecrawl", "tavily", "exa"] {
            assert!(
                initial
                    .iter()
                    .skip(1)
                    .any(|row| row.to_ascii_lowercase().contains(entity)),
                "{entity} missing from initial execution lanes: {initial:?}"
            );
        }
        assert!(
            initial
                .iter()
                .skip(1)
                .all(|row| !row.to_ascii_lowercase().contains("independent comparison")),
            "broad comparison lanes should wait until entity coverage has a first chance: {initial:?}"
        );
    }

    #[test]
    fn metadata_query_plan_does_not_frontload_instruction_scaffold() {
        let payload = json!({
            "source": "web",
            "query": "Use web research to gather public source evidence about Alpha Runtime. If retrieval is sparse, preserve that as evidence state instead of inventing claims.",
            "queries": [
                "Use web research to gather public source evidence about Alpha Runtime. If retrieval is sparse, preserve that as evidence state instead of inventing claims.",
                "Alpha Runtime public source evidence official",
                "Alpha Runtime official site"
            ],
            "keywords": [
                "Alpha Runtime",
                "public source evidence",
                "official documentation"
            ],
            "required_coverage": {
                "entities": ["Alpha Runtime"],
                "facets": []
            },
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );

        assert_eq!(plan.query_plan_source, "explicit_request_pack_with_metadata");
        assert!(
            plan
                .queries
                .first()
                .map(|row| row.to_ascii_lowercase().contains("alpha runtime"))
                .unwrap_or(false),
            "{:?}",
            plan.queries
        );
        assert!(
            plan
                .queries
                .first()
                .map(|row| !row.to_ascii_lowercase().starts_with("use web research"))
                .unwrap_or(false),
            "{:?}",
            plan.queries
        );
        assert!(
            !plan
                .rerank_query
                .to_ascii_lowercase()
                .starts_with("use web research"),
            "{}",
            plan.rerank_query
        );
    }

    #[test]
    fn deferred_recovery_prioritizes_official_lanes_with_subject_diversity() {
        let policy = json!({
            "batch_query": {
                "query_execution_budget": {
                    "deferred_recovery": {
                        "enabled": true,
                        "max_lanes": 2
                    }
                }
            }
        });
        let budget = aperture_budget("medium").expect("budget");
        let submitted = vec![
            "AlphaSearch BetaSearch comparison".to_string(),
            "AlphaSearch BetaSearch reviews".to_string(),
            "AlphaSearch official site".to_string(),
            "AlphaSearch official documentation".to_string(),
            "BetaSearch official site".to_string(),
        ];
        let executed = vec![
            "AlphaSearch BetaSearch comparison".to_string(),
            "AlphaSearch BetaSearch reviews".to_string(),
        ];

        let queries = deferred_query_recovery_queries(&policy, budget, &submitted, &executed);

        assert_eq!(
            queries,
            vec![
                "AlphaSearch official site".to_string(),
                "BetaSearch official site".to_string()
            ]
        );
    }

    #[test]
    fn inferred_comparison_pack_splits_short_unseparated_entity_list() {
        let pack = inferred_comparison_query_pack(
            "compare Firecrawl Tavily Exa web search APIs",
            aperture_budget("medium").expect("budget"),
        )
        .expect("comparison pack");

        for entity in ["Firecrawl", "Tavily", "Exa"] {
            assert!(
                pack.entities.iter().any(|row| row == entity),
                "{entity} missing from inferred entities: {:?}",
                pack.entities
            );
        }
        assert!(
            !pack
                .entities
                .iter()
                .any(|row| row == "Firecrawl Tavily Exa"),
            "unseparated entity lists should not collapse into one facet: {:?}",
            pack.entities
        );
    }

    #[test]
    fn raw_multi_entity_research_query_frontloads_each_named_subject() {
        let payload = json!({
            "source": "web",
            "query": "Research Firecrawl, Tavily, and Exa as data tools for AI research agents. Which should we use for search, crawling, and evidence gathering?",
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let budget = aperture_budget("medium").expect("budget");
        let plan = resolve_query_plan(&default_policy(), &payload, &query, budget);
        let initial = execution_limited_initial_queries(
            &default_policy(),
            budget,
            &plan.query_metadata,
            &plan.queries,
        );
        let initial_text = initial.join("\n").to_ascii_lowercase();

        for entity in ["firecrawl", "tavily", "exa"] {
            assert!(
                initial_text.contains(entity),
                "{entity} missing from raw-query initial lanes: {initial:?}; metadata={:?}",
                plan.query_metadata
            );
        }
        assert!(
            plan.query_metadata.entities.len() >= 3,
            "raw-query metadata should preserve named subjects before facet recovery: {:?}",
            plan.query_metadata
        );
    }

    #[test]
    fn unseparated_entity_split_keeps_product_phrase_continuations_together() {
        assert_eq!(
            split_unseparated_comparison_entity_variants("OpenAI Agents SDK"),
            vec!["OpenAI Agents SDK".to_string()]
        );
    }

    #[test]
    fn local_stay_query_plan_avoids_generic_official_site_lane() {
        let payload = json!({
            "source": "web",
            "query": "Research family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability. Compare a few options and tradeoffs.",
            "keywords": [
                "Chicago",
                "family-friendly",
                "neighborhoods",
                "stay",
                "museums",
                "transit",
                "access",
                "walkability"
            ],
            "required_coverage": {
                "entities": ["Chicago"],
                "facets": []
            },
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );

        assert_eq!(plan.query_plan_source, "explicit_request_pack_with_metadata");
        assert!(
            plan.queries
                .iter()
                .any(|row| row.contains("travel guide comparison")),
            "{:?}",
            plan.queries
        );
        assert!(
            plan.queries
                .iter()
                .any(|row| row.contains("where to stay guide")
                    || row.contains("neighborhood guide")),
            "{:?}",
            plan.queries
        );
        assert!(
            plan.queries
                .iter()
                .all(|row| !row.contains("official site")
                    && !row.ends_with(" official")),
            "{:?}",
            plan.queries
        );
    }

    #[test]
    fn broad_multi_facet_query_plan_allocates_each_requested_facet() {
        let payload = json!({
            "source": "web",
            "query": "Compare the current evidence and commercialization status for direct air capture, mineralization, and biochar as carbon removal approaches.",
            "queries": [
                "Compare the current evidence and commercialization status for direct air capture, mineralization, and biochar as carbon removal approaches",
                "direct air capture recent developments",
                "direct air capture independent analysis",
                "mineralization recent developments",
                "mineralization independent analysis"
            ],
            "keywords": [
                "direct air capture",
                "mineralization",
                "biochar",
                "current",
                "evidence",
                "commercialization"
            ],
            "required_coverage": {
                "entities": [],
                "facets": ["direct air capture", "mineralization", "biochar"]
            },
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );

        assert_eq!(
            plan.query_plan_source,
            "explicit_request_pack_with_metadata"
        );
        let primary_key = query_plan_dedup_key(
            "Compare the current evidence and commercialization status for direct air capture, mineralization, and biochar as carbon removal approaches.",
        );
        assert_eq!(
            plan.queries
                .iter()
                .filter(|row| query_plan_dedup_key(row) == primary_key)
                .count(),
            1,
            "{:?}",
            plan.queries
        );
        for facet in ["direct air capture", "mineralization", "biochar"] {
            assert!(
                plan.queries
                    .iter()
                    .take(5)
                    .any(|row| row.to_ascii_lowercase().contains(facet)),
                "{facet} missing from query plan: {:?}",
                plan.queries
            );
        }
        assert!(
            plan.queries
                .iter()
                .any(|row| row.to_ascii_lowercase().contains("biochar source-backed evidence")),
            "{:?}",
            plan.queries
        );
    }

    #[test]
    fn single_subject_multi_facet_query_plan_frontloads_subject_facet_lanes() {
        let payload = json!({
            "source": "web",
            "query": "Research the current status of right-to-repair legislation in the US for electronics and farm equipment. Where is momentum strongest, and where are the carve-outs?",
            "queries": [
                "Research the current status of right-to-repair legislation in the US for electronics and farm equipment. Where is momentum strongest, and where are the carve-outs?",
                "right-to-repair official site",
                "right-to-repair official documentation",
                "electronics source-backed evidence",
                "farm equipment source-backed evidence"
            ],
            "keywords": [
                "right-to-repair",
                "electronics",
                "farm equipment",
                "legislation",
                "carve-outs"
            ],
            "required_coverage": {
                "entities": ["right-to-repair"],
                "facets": ["electronics", "farm equipment"]
            },
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );

        assert_eq!(
            plan.query_plan_source,
            "explicit_request_pack_with_metadata"
        );
        let primary_key = query_plan_dedup_key(
            "the current status of right-to-repair legislation in the US for electronics and farm equipment. Where is momentum strongest, and where are the carve-outs",
        );
        assert_eq!(
            plan.queries
                .iter()
                .filter(|row| query_plan_dedup_key(row) == primary_key)
                .count(),
            1,
            "{:?}",
            plan.queries
        );
        assert!(
            plan.queries.iter().take(3).any(|row| {
                let lowered = row.to_ascii_lowercase();
                lowered.contains("right-to-repair")
                    && lowered.contains("electronics")
                    && lowered.contains("farm equipment")
            }),
            "subject+facet lane should fit inside the first execution window: {:?}",
            plan.queries
        );
        assert!(
            !plan
                .queries
                .iter()
                .take(3)
                .any(|row| row.eq_ignore_ascii_case("right-to-repair official site")),
            "generic official-site lane should not consume the first coverage slot: {:?}",
            plan.queries
        );
    }

    #[test]
    fn single_framework_catalog_query_plan_does_not_add_hidden_queries() {
        let payload = json!({
            "source": "web",
            "query": "top AI agentic frameworks",
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );
        assert_eq!(plan.query_plan_source, "agent_submitted_single_query");
        assert_eq!(plan.queries, vec!["top AI agentic frameworks".to_string()]);
        assert!(plan.rewrite_set.is_empty(), "{:?}", plan.rewrite_set);
    }

    #[test]
    fn relative_current_query_plan_adds_visible_local_month_lane() {
        let payload = json!({
            "source": "web",
            "query": "give me global news from this week",
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(
            &default_policy(),
            &payload,
            &query,
            aperture_budget("medium").expect("budget"),
        );
        let year = current_year();
        let month = chrono::Local::now().format("%B").to_string().to_ascii_lowercase();
        let date = current_date_iso();
        assert!(
            plan.queries
                .iter()
                .any(|row| row.to_ascii_lowercase().contains(&month)
                    && row.contains(&year)),
            "{:?}",
            plan.queries
        );
        assert!(
            plan.queries.iter().any(|row| row.contains(&date)),
            "relative week queries should expose an exact-date lane: {:?}",
            plan.queries
        );
        assert!(
            plan.queries
                .iter()
                .any(|row| row.to_ascii_lowercase().contains("week of")
                    && row.to_ascii_lowercase().contains(&month)
                    && row.contains(&year)),
            "relative week queries should expose a week-of lane: {:?}",
            plan.queries
        );
        assert!(
            plan.query_plan_source.contains("recovery")
                || plan.query_plan_source.contains("request_pack"),
            "{} {:?}",
            plan.query_plan_source,
            plan.queries
        );
    }

    #[test]
    fn relative_current_stage_search_request_sets_freshness_filter() {
        let request = stage_search_request(
            "give me global news from this week",
            None,
            &default_policy(),
            &BatchQuerySearchScope::default(),
        );
        assert_eq!(
            request.get("freshness").and_then(Value::as_str),
            Some("week")
        );
        assert_eq!(
            request.get("freshness_source").and_then(Value::as_str),
            Some("batch_query_relative_current_window")
        );
    }

    #[test]
    fn framework_catalog_source_adjustment_penalizes_support_noise() {
        let candidate = Candidate {
            source_kind: "web".to_string(),
            title: "Contact Us - Microsoft Support".to_string(),
            locator: "https://support.microsoft.com/en-us/contactus".to_string(),
            snippet: "Contact Microsoft Support. Find solutions to common problems, or get help from a support agent.".to_string(),
            excerpt_hash: "support-noise".to_string(),
            timestamp: None,
            permissions: None,
            status_code: 200,
        };
        assert!(framework_catalog_source_adjustment(&candidate) < 0.0);
    }

    #[test]
    fn framework_catalog_source_adjustment_penalizes_mirror_domains() {
        let candidate = Candidate {
            source_kind: "web".to_string(),
            title: "LangGraph - LangChain Framework".to_string(),
            locator: "https://langgraph.com.cn/index.html".to_string(),
            snippet: "LangGraph mirror documentation in Chinese.".to_string(),
            excerpt_hash: "mirror-noise".to_string(),
            timestamp: None,
            permissions: None,
            status_code: 200,
        };
        assert!(framework_catalog_source_adjustment(&candidate) < 0.0);
    }

    #[test]
    fn framework_catalog_source_adjustment_penalizes_competitive_programming_dump() {
        let candidate = Candidate {
            source_kind: "web".to_string(),
            title: "03-Tree List Leaves".to_string(),
            locator: "https://example.com/tree-problem".to_string(),
            snippet: "Given a tree, list leaves. Input Specification: ... Sample Input ... Sample Output ... #include <stdio.h> int main()".to_string(),
            excerpt_hash: "competitive-dump".to_string(),
            timestamp: None,
            permissions: None,
            status_code: 200,
        };
        assert!(framework_catalog_source_adjustment(&candidate) < -0.2);
    }

    #[test]
    fn rendered_search_payload_extracts_multiple_framework_candidates() {
        let payload = json!({
            "ok": true,
            "content": concat!(
                "LangGraph: Agent Orchestration Framework for Reliable AI Agents — https://www.langchain.com/langgraph — LangGraph sets the foundation for reliable agent workflows.\n",
                "OpenAI Agents SDK overview — https://openai.github.io/openai-agents-python/ — OpenAI Agents SDK helps build tool-using agents.\n",
                "crewAI — https://crewai.com/ — CrewAI enables multiple agents to collaborate on tasks."
            ),
            "status_code": 200
        });
        let candidates =
            candidates_from_rendered_search_payload("top AI agentic frameworks", &payload, 4);
        assert!(candidates.len() >= 3, "{candidates:?}");
        let joined = candidates
            .iter()
            .map(|row| format!("{} {}", row.title, row.locator))
            .collect::<Vec<_>>()
            .join(" | ")
            .to_ascii_lowercase();
        assert!(joined.contains("langchain.com"), "{joined}");
        assert!(joined.contains("openai.github.io"), "{joined}");
        assert!(joined.contains("crewai.com"), "{joined}");
    }

    #[test]
    fn batch_query_synthesizes_multiple_frameworks_from_single_search_payload() {
        let out = run_request_with_fixture(
            json!({
                "top AI agentic frameworks": {
                    "ok": true,
                    "summary": "top ai agentic frameworks official docs",
                    "content": concat!(
                        "LangGraph: Agent Orchestration Framework for Reliable AI Agents — https://www.langchain.com/langgraph — LangGraph sets the foundation for reliable agent workflows.\n",
                        "OpenAI Agents SDK overview — https://openai.github.io/openai-agents-python/ — OpenAI Agents SDK helps build tool-using agents.\n",
                        "crewAI — https://crewai.com/ — CrewAI enables multiple agents to collaborate on tasks."
                    ),
                    "links": [
                        "https://www.langchain.com/langgraph",
                        "https://openai.github.io/openai-agents-python/",
                        "https://crewai.com/"
                    ],
                    "requested_url": "https://example.com/frameworks",
                    "status_code": 200
                }
            }),
            &json!({
                "source":"web",
                "query":"top AI agentic frameworks",
                "aperture":"medium"
            }),
        );
        let lowered = out
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(lowered.contains("langgraph"), "{lowered}");
        assert!(lowered.contains("openai agents sdk"), "{lowered}");
        assert!(lowered.contains("crewai"), "{lowered}");
    }
}
