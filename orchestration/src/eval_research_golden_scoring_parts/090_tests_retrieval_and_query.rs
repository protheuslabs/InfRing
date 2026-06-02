#[test]
fn materialized_evidence_candidates_count_as_retrieval_quality() {
    let payload = json!({
        "tools": [{
            "name": "browser_materialize_page",
            "status": "ok",
            "evidence_pack_candidates": [{
                "source_kind": "browser_materialized_page",
                "title": "Rendered research page",
                "locator": "https://example.test/rendered",
                "snippet": "This rendered page includes enough extracted body text to support a normal source-backed synthesis after materialization packaging succeeds, including context, terms, source scope, and a concrete claim for the user question.",
                "claim_hints": ["Rendered source supports a concrete research claim."],
                "score": 76.0,
                "confidence": "usable"
            }]
        }]
    });

    let quality =
        retrieval_provider_quality(&payload, "rendered research page source backed synthesis");
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable")
    );
    assert_eq!(
        quality
            .get("materialized_evidence_available")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        quality.get("allows_excellent").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn error_only_provider_rows_do_not_count_as_retrieval_evidence() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "error",
            "input": {
                "query": "Research current RAG stack options for a small team",
                "keywords": ["RAG", "LlamaIndex", "LangChain"]
            },
            "provider_results": [{
                "provider": "web",
                "query": "Research current RAG stack options for a small team",
                "status": "error",
                "error": "tool_execution_failed"
            }],
            "evidence_refs": [{
                "provider": "web",
                "query": "Research current RAG stack options for a small team",
                "status": "error",
                "error": "tool_execution_failed"
            }]
        }]
    });

    let quality = retrieval_provider_quality(&payload, "rag stack options");
    assert_eq!(
        quality.get("candidate_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        quality.get("evidence_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        quality
            .get("content_rich_candidate_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("provider_degraded")
    );
}

#[test]
fn direct_evidence_claim_contract_overrides_candidate_title_relevance() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 8,
            "evidence_count": 3,
            "materialized_candidate_count": 2,
            "content_rich_candidate_count": 2,
            "flags": ["partial_results"]
        },
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 2,
            "claim_hint_count": 1,
            "content_rich_item_count": 1
        },
        "evidence_claims": [{
            "claim": "In today's digital world, news sources are everywhere",
            "support_snippet": "In today's digital world, news sources are everywhere.",
            "source_domain": "example.test"
        }],
        "evidence_refs": [{
            "title": "Major world news story from this week",
            "snippet": "A candidate title that overlaps the query but is not the citable claim."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("Give me the biggest world news from this week."),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("low_relevance"),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/direct_evidence_claim_count")
            .and_then(Value::as_u64),
        Some(1),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/prompt_relevance/topic_relevant_evidence")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn direct_evidence_claim_relevance_uses_citable_support_fields() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 12,
            "evidence_count": 2,
            "materialized_candidate_count": 2,
            "content_rich_candidate_count": 2,
            "flags": ["partial_results"]
        },
        "evidence_pack_quality": {
            "status": "thin",
            "usable_count": 1,
            "claim_hint_count": 2,
            "content_rich_item_count": 1
        },
        "evidence_claims": [{
            "claim": "Anti-hair wrap technology is specifically useful for pet owners.",
            "support_snippet": "The comparison says Shark counters with anti-hair-wrap technology while Dyson emphasizes particle detection.",
            "source_title": "Dyson V15 Detect vs Shark Stratos cordless vacuum comparison",
            "source_domain": "example.test"
        }],
        "evidence_refs": [{
            "title": "Dyson V15 Detect vs Shark Stratos cordless vacuum comparison",
            "snippet": "The comparison says Shark counters with anti-hair-wrap technology while Dyson emphasizes particle detection."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare(
            "Compare Dyson V15 Detect and Shark Stratos cordless vacuums for pet hair.",
        ),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable"),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/direct_pack_thin_blocks_signal")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/prompt_relevance/relevant_evidence_count")
            .and_then(Value::as_u64),
        Some(1),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("allows_excellent").and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn direct_structured_evidence_overrides_stale_missing_quality_flags() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 18,
            "evidence_count": 1,
            "materialized_candidate_count": 7,
            "content_rich_candidate_count": 7,
            "flags": [
                "claim_hints_missing",
                "content_rich_evidence_missing",
                "provider_starved"
            ]
        },
        "evidence_pack_quality": {
            "status": "thin",
            "usable_count": 1,
            "claim_hint_count": 2,
            "content_rich_item_count": 1
        },
        "evidence_claims": [{
            "claim": "The attack on a nuclear power plant in the United Arab Emirates raised fears about Iran's retaliation.",
            "support_snippet": "The attack on a nuclear power plant in the United Arab Emirates raised fears about Iran's retaliation and the role of militias.",
            "source_title": "Iran War: Attack on Barakah Nuclear Plant From Iraq Is Warning Shot",
            "source_domain": "bloomberg.com"
        }],
        "evidence_refs": [{
            "title": "Iran War: Attack on Barakah Nuclear Plant From Iraq Is Warning Shot",
            "snippet": "The attack on a nuclear power plant in the United Arab Emirates raised fears about Iran's retaliation and the role of militias."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare(
            "Give me concise major news from this week, group by theme and cite sources.",
        ),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable"),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/direct_low_signal_marker")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn generic_prompt_shape_terms_do_not_force_false_relevance_failures() {
    let relevance = evidence_prompt_relevance_from_texts(
        &normalize_for_compare(
            "Give me concise major news from this week, group by theme and cite sources.",
        ),
        vec![normalize_for_compare(
            "Senate Republicans canceled a planned vote on immigration enforcement funding.",
        )],
        "test",
        true,
    );
    assert_eq!(
        relevance
            .get("topic_relevant_evidence")
            .and_then(Value::as_bool),
        Some(true),
        "{relevance:#?}"
    );
    assert_eq!(
        relevance.get("min_overlap_terms").and_then(Value::as_u64),
        Some(0),
        "{relevance:#?}"
    );
}

#[test]
fn prompt_relevance_supports_compact_compound_topic_terms() {
    let relevance = evidence_prompt_relevance_from_texts(
        &normalize_for_compare(
            "Research youth social-media restriction bills and current gig-worker classification rules.",
        ),
        vec![
            normalize_for_compare(
                "States passed bills addressing minors' access to social media platforms.",
            ),
            normalize_for_compare(
                "Gig worker classification remains split between employee and contractor tests.",
            ),
        ],
        "compound prompt terms should match equivalent spaced or hyphenated evidence text",
        true,
    );

    assert_eq!(
        relevance
            .get("topic_relevant_evidence")
            .and_then(Value::as_bool),
        Some(true),
        "{relevance:#?}"
    );
    assert!(
        relevance
            .get("relevant_evidence_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 2,
        "{relevance:#?}"
    );
}

#[test]
fn direct_usable_evidence_pack_overrides_stale_low_signal_flag() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "partial"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 24,
            "evidence_count": 5,
            "materialized_candidate_count": 5,
            "content_rich_candidate_count": 5,
            "flags": ["low_signal"]
        },
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 5,
            "claim_hint_count": 5,
            "content_rich_item_count": 5
        },
        "evidence_claims": [{
            "claim": "Cursor is an AI coding assistant used inside the IDE.",
            "support_snippet": "Cursor is an AI coding assistant used inside the IDE for code editing and refactoring.",
            "source_title": "Cursor coding assistant overview",
            "source_domain": "cursor.com"
        }, {
            "claim": "GitHub Copilot is an enterprise coding assistant used by engineering teams.",
            "support_snippet": "GitHub Copilot is positioned as an enterprise coding assistant for engineering teams.",
            "source_title": "GitHub Copilot enterprise overview",
            "source_domain": "github.com"
        }],
        "evidence_refs": [{
            "title": "Cursor coding assistant overview",
            "snippet": "Cursor is an AI coding assistant used inside the IDE for code editing and refactoring."
        }, {
            "title": "GitHub Copilot enterprise overview",
            "snippet": "GitHub Copilot is positioned as an enterprise coding assistant for engineering teams."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare(
            "Research the current enterprise AI coding assistant market. Compare GitHub Copilot and Cursor.",
        ),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable"),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("usable_evidence").and_then(Value::as_bool),
        Some(true),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/direct_low_signal_marker")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn direct_evidence_claim_contract_zero_claims_is_low_signal() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 12,
            "evidence_count": 3,
            "materialized_candidate_count": 3,
            "content_rich_candidate_count": 3,
            "flags": ["claim_hints_missing", "partial_results"]
        },
        "evidence_pack_quality": {
            "status": "thin",
            "usable_count": 0,
            "claim_hint_count": 0,
            "content_rich_item_count": 0
        },
        "evidence_claims": [],
        "evidence_refs": [{
            "title": "Firecrawl Tavily Exa API comparison",
            "snippet": "A title-level source row with no extracted claim."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("Compare Firecrawl, Tavily, and Exa for web research APIs."),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("low_signal"),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("claim_hint_count").and_then(Value::as_u64),
        Some(0),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("usable_evidence").and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn direct_provider_starved_contract_is_nonblocking_when_evidence_arrived() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 10,
            "evidence_count": 2,
            "materialized_candidate_count": 2,
            "content_rich_candidate_count": 2,
            "flags": ["provider_starved", "provider_timeout"]
        },
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 2,
            "claim_hint_count": 2,
            "content_rich_item_count": 2
        },
        "evidence_claims": [{
            "claim": "Scientific breakthroughs in 2026 include a methane chemistry result.",
            "source_domain": "example.test"
        }],
        "evidence_refs": [{
            "title": "Scientific breakthroughs in 2026",
            "snippet": "Scientific breakthroughs in 2026 include a methane chemistry result."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("What are some scientific breakthroughs in 2026 so far?"),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable"),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("usable_evidence").and_then(Value::as_bool),
        Some(true),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .get("quality_flags")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|flag| flag.as_str() == Some("provider_degradation_nonblocking")),
        true,
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/provider_degradation_blocks_supply")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn direct_usable_pack_overrides_stale_missing_flags_when_counts_are_present() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "partial"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 76,
            "evidence_count": 36,
            "materialized_candidate_count": 74,
            "content_rich_candidate_count": 15,
            "flags": [
                "quota_exhausted",
                "provider_starved",
                "materialized_evidence_missing",
                "content_rich_evidence_missing",
                "claim_hints_missing"
            ]
        },
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 6,
            "claim_hint_count": 6,
            "content_rich_item_count": 5
        },
        "evidence_claims": [{
            "claim": "A scientific breakthrough in April 2026 reported improved methane conversion selectivity.",
            "support_snippet": "The April 2026 scientific breakthrough report described improved methane conversion selectivity.",
            "source_title": "April 2026 scientific breakthrough result",
            "source_domain": "example.test"
        }, {
            "claim": "Another scientific breakthrough in April 2026 reported a materials discovery with measured performance gains.",
            "support_snippet": "The April 2026 article described a scientific materials breakthrough with measured performance gains.",
            "source_title": "April 2026 materials scientific breakthrough",
            "source_domain": "example.test"
        }],
        "evidence_refs": [{
            "title": "April 2026 scientific breakthrough result",
            "snippet": "The April 2026 scientific breakthrough report described improved methane conversion selectivity."
        }, {
            "title": "April 2026 materials scientific breakthrough",
            "snippet": "The April 2026 article described a scientific materials breakthrough with measured performance gains."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("What are some scientific breakthroughs in April 2026?"),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable"),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("usable_evidence").and_then(Value::as_bool),
        Some(true),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/direct_quality_contract_conflict")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/provider_degradation_blocks_supply")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
    assert!(
        quality
            .get("quality_flags")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|flag| flag.as_str() == Some("provider_degradation_nonblocking")),
        "{quality:#?}"
    );
    assert!(
        !quality
            .get("quality_flags")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|flag| flag.as_str() == Some("direct_tool_quality_contract_conflict")),
        "{quality:#?}"
    );
}

#[test]
fn direct_usable_pack_conflicts_with_missing_quality_flags() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 12,
            "evidence_count": 5,
            "materialized_candidate_count": 0,
            "content_rich_candidate_count": 0,
            "flags": [
                "claim_hints_missing",
                "content_rich_evidence_missing",
                "materialized_evidence_missing"
            ]
        },
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 5,
            "claim_hint_count": 5,
            "content_rich_item_count": 5
        },
        "evidence_claims": [{
            "claim": "The source says a named clinical milestone happened in April 2026.",
            "source_domain": "example.test"
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("What source-backed science milestones happened in April 2026?"),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("conflicting_provider_state"),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("usable_evidence").and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .get("quality_flags")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|flag| flag.as_str() == Some("direct_tool_quality_contract_conflict")),
        true,
        "{quality:#?}"
    );
}

#[test]
fn direct_provider_starved_with_evidence_but_no_claims_is_low_signal() {
    let payload = json!({
        "tools": [{
            "name": "batch_query",
            "status": "ok"
        }],
        "tool_result_quality": {
            "status": "partial",
            "candidate_count": 12,
            "evidence_count": 3,
            "materialized_candidate_count": 3,
            "content_rich_candidate_count": 3,
            "flags": ["provider_starved", "provider_timeout", "claim_hints_missing"]
        },
        "evidence_pack_quality": {
            "status": "thin",
            "usable_count": 0,
            "claim_hint_count": 0,
            "content_rich_item_count": 0
        },
        "evidence_claims": [],
        "evidence_refs": [{
            "title": "Firecrawl Tavily Exa API comparison",
            "snippet": "A title-level source row with no extracted claim."
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("Compare Firecrawl, Tavily, and Exa for web research APIs."),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("low_signal"),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/provider_degradation_blocks_supply")
            .and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("usable_evidence").and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
}

#[test]
fn web_tooling_gate_names_are_internal_leaks() {
    assert!(internal_workflow_leak(
        "web_gate_5_extraction_quality failed, so the final answer cannot use this source."
    ));
    assert!(internal_workflow_leak(
        "The web_tooling_gates summary says two gates passed."
    ));
    assert!(internal_workflow_leak(
        "Outcome posture: bounded_partial_answer. Here is the answer."
    ));
}

#[test]
fn scoring_shape_accepts_general_research_findings_and_plans() {
    let security = normalize_for_compare(
            "Here is what the evidence supports on AI browser agent security concerns. \
             Source-backed finding: prompt injection is a published risk, with gaps around credential handling.",
        );
    assert!(has_tradeoff_or_structure(&security));
    assert!(has_limitation_signal(&security));

    let sparse_benchmark = normalize_for_compare(
            "The benchmark evidence is weak and insufficient. \
             What the evidence shows is partial, so the practical evaluation plan should compare latency, cost, and reliability directly.",
        );
    assert!(has_tradeoff_or_structure(&sparse_benchmark));
    assert!(has_limitation_signal(&sparse_benchmark));
    assert!(has_recommendation_signal(&sparse_benchmark));

    let balanced_takeaway = normalize_for_compare(
        "The balanced view is that the technology is promising but not yet a finished replacement. \
         The key milestone to watch for is durable field performance at commercial scale.",
    );
    assert!(has_recommendation_signal(&balanced_takeaway));

    let priorities = normalize_for_compare(
        "The highest priorities are maintaining a data inventory and validating public privacy claims. \
         A key risk-reduction step is to focus on opt-out flows.",
    );
    assert!(has_recommendation_signal(&priorities));

    let historical_tension = normalize_for_compare(
        "The mainstream view is broadly democratizing. At the same time, scholars acknowledge \
         tensions over political symbolism, federal patronage, and representation.",
    );
    assert!(has_tradeoff_or_structure(&historical_tension));

    assert!(prompt_asks_for_process_or_schedule(
        &normalize_for_compare("Explain the implementation timeline and practical obligations for a SaaS company.")
    ));
}

#[test]
fn best_prompt_allows_bounded_ranked_strategy_answer() {
    let case = json!({
        "prompt": "Research the best neighborhoods to stay in Tokyo for a first-time visit."
    });
    let response = "The best strategy is to pick one convenient base rather than moving districts. \
        Strong options include Shinjuku and Ginza, but the right choice depends on transit, food, and luggage tradeoffs.";

    assert!(!unsupported_claim_signal(&case, response));
}

#[test]
fn outside_evidence_detector_ignores_unsettled_evidence_language() {
    let response = normalize_for_compare(
        "The evidence for precise links between exposure levels and health outcomes is not well established. \
        A practical recommendation is to reduce avoidable exposure without claiming diagnosed harm.",
    );

    assert!(!outside_evidence_used_for_decision_signal(&response));
}

#[test]
fn outside_evidence_detector_allows_grounded_known_for_phrasing() {
    let response = normalize_for_compare(
        "Arches is known for iconic short hikes near Moab. \
        A practical recommendation is to pick the Moab route if that matches your constraints.",
    );

    assert!(!outside_evidence_used_for_decision_signal(&response));
}

#[test]
fn entity_coverage_accepts_phrase_variants_without_case_specific_aliases() {
    let response = normalize_for_compare(
        "The evidence discusses agent evaluation frameworks and framework results, \
             but no head-to-head benchmark data was found.",
    );
    assert!(normalized_response_covers_entity(
        &response,
        "agent framework"
    ));
    assert_eq!(
        entity_coverage(
            &response,
            &["benchmark".to_string(), "agent framework".to_string()]
        ),
        1.0
    );
}

#[test]
fn entity_coverage_accepts_derived_initialism_aliases() {
    let response = normalize_for_compare(
        "The MCP ecosystem has strong momentum, but product teams should avoid \
             overcommitting to unstable server behavior without source-backed checks.",
    );
    assert!(normalized_response_covers_entity(
        &response,
        "Model Context Protocol"
    ));
    assert_eq!(
        entity_coverage(&response, &["Model Context Protocol".to_string()]),
        1.0
    );
}

#[test]
fn entity_coverage_accepts_generic_policy_phrase_aliases() {
    let response = normalize_for_compare(
        "US state AI legislation is clustering around automated decisions, \
             disclosure, procurement, and governance requirements.",
    );
    assert!(normalized_response_covers_entity(
        &response,
        "US state-level AI regulation"
    ));
    assert_eq!(
        entity_coverage(&response, &["US state-level AI regulation".to_string()]),
        1.0
    );
}

#[test]
fn entity_coverage_accepts_unicode_dash_variants() {
    let response = normalize_for_compare(
        "GLP\u{2011}1 and related incretin medications have evidence for \
         non-diabetic weight management when clinically supervised.",
    );

    assert!(normalized_response_covers_entity(
        &response,
        "GLP-1 medications"
    ));
    assert_eq!(
        entity_coverage(&response, &["GLP-1 medications".to_string()]),
        1.0
    );
}

#[test]
fn entity_coverage_accepts_named_place_in_negative_constraint() {
    let prompt = normalize_for_compare(
        "Compare NYC neighborhoods for a first-time visitor who wants food, museums, and \
         easy subway access without staying in Times Square.",
    );
    let response = normalize_for_compare(
        "For a first-time visitor who wants food, museums, and subway access while avoiding \
         Times Square, Upper West Side is the strongest all-around fit. Chelsea and Flatiron \
         also keep you well-connected without staying in Times Square.",
    );
    let entities = user_stated_required_entities(
        &prompt,
        &["NYC".to_string(), "Times Square".to_string()],
    );
    assert_eq!(entities, vec!["Times Square".to_string()]);
    assert!(normalized_response_covers_entity(&response, "Times Square"));
    assert_eq!(entity_coverage(&response, &entities), 1.0);
}

#[test]
fn query_satisfaction_reports_entity_aliases_without_requiring_format() {
    let response = normalize_for_compare(
        "According to source evidence, MCP is useful as an integration pattern, \
             but the ecosystem still has maturity and security gaps.",
    );
    let entities = vec!["Model Context Protocol".to_string()];
    let coverage = entity_coverage(&response, &entities);
    let satisfaction = query_satisfaction(
        &normalize_for_compare("Research the current Model Context Protocol ecosystem."),
        &response,
        &entities,
        coverage,
        true,
        true,
        true,
        true,
    );
    assert_eq!(
        satisfaction.get("scope_covered").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        satisfaction
            .pointer("/coverage_entity_aliases/Model Context Protocol/0")
            .and_then(Value::as_str),
        Some("MCP")
    );
}

#[test]
fn query_satisfaction_accepts_explanatory_prompt_overlap() {
    let response = normalize_for_compare(
        "The Works Progress Administration shaped American public art through \
             New Deal art programs, especially the Federal Art Project, and \
             historians generally interpret the legacy as democratizing access \
             to civic art while still noting political and regional tensions.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research how the Works Progress Administration influenced American public art. Which programs mattered most?",
        ),
        &response,
        &["Works Progress Administration".to_string()],
        1.0,
        true,
        true,
        true,
        false,
    );
    assert_eq!(
        satisfaction.get("intent_answered").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn decision_value_accepts_informational_selection_prompts() {
    let response = normalize_for_compare(
        "The Works Progress Administration shaped American public art primarily \
             through the Federal Art Project and related initiatives. The main \
             historical interpretation is that these programs democratized civic art.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research how the Works Progress Administration influenced American public art. Which programs mattered most?",
        ),
        &response,
        &["Works Progress Administration".to_string()],
        1.0,
        true,
        true,
        true,
        false,
    );
    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn decision_value_accepts_action_oriented_practical_guidance() {
    let response = normalize_for_compare(
        "Consolidate to one central cloud storage solution, adopt a unified filing system, \
             and decide clear organization rules for the family.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research practical approaches for moving a family from scattered cloud storage to a simple organized digital filing system.",
        ),
        &response,
        &[],
        1.0,
        true,
        true,
        true,
        false,
    );

    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(true),
        "{satisfaction:#?}"
    );
}

#[test]
fn decision_value_accepts_promising_approaches_selection_prompts() {
    let response = normalize_for_compare(
        "Progress centers on three frequently discussed approaches: lipid nanoparticles, \
             viral vectors, and virus-like particles. Lipid nanoparticles are attractive for \
             transient delivery, while viral vectors offer targeting experience. Broad use is \
             still blocked by tissue targeting, immune reactions, and manufacturing consistency.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research delivery progress. Which approaches look most promising, and what is still blocking broad use?",
        ),
        &response,
        &[],
        1.0,
        true,
        true,
        true,
        false,
    );

    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(true),
        "{satisfaction:#?}"
    );
}

#[test]
fn recommendation_signal_accepts_shortlist_selection_language() {
    let response = normalize_for_compare(
        "Travel headphone shortlist: Sony is the strongest all-round option, \
             Bose is the top performer for comfort, and Sennheiser is the value pick.",
    );

    assert!(has_recommendation_signal(&response));
}

#[test]
fn query_satisfaction_accepts_plain_contrast_for_comparison_prompts() {
    let response = normalize_for_compare(
        "NAWSA concentrated on state campaigns and lobbying, while the National \
             Woman's Party used picketing and public pressure. Ida B. Wells \
             exposes the movement's racial exclusions, and the 19th Amendment \
             was a constitutional milestone but did not end voter suppression.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research the US women's suffrage movement. Compare NAWSA, the National Woman's Party, Ida B. Wells, and the 19th Amendment.",
        ),
        &response,
        &[
            "NAWSA".to_string(),
            "National Woman's Party".to_string(),
            "Ida B. Wells".to_string(),
            "19th Amendment".to_string(),
        ],
        entity_coverage(
            &response,
            &[
                "NAWSA".to_string(),
                "National Woman's Party".to_string(),
                "Ida B. Wells".to_string(),
                "19th Amendment".to_string(),
            ],
        ),
        true,
        true,
        true,
        false,
    );
    assert_eq!(
        satisfaction.get("intent_answered").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        satisfaction
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 9
    );
}

#[test]
fn broad_scope_descriptors_do_not_get_derived_initialism_aliases() {
    assert_eq!(
        entity_coverage_aliases("AI agentic landscape"),
        vec!["AI agentic landscape".to_string()]
    );
    assert_eq!(
        entity_coverage_aliases("US public sector"),
        vec!["US public sector".to_string()]
    );
}
