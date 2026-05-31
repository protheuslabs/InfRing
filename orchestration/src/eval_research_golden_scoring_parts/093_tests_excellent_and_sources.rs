// Layer ownership: orchestration (research eval authority)

#[test]
fn excellent_diagnostics_accept_public_source_signal_without_format_lock() {
    let case = json!({
        "prompt": "Compare Alpha and Beta for production use.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta"]
    });
    let payload = json!({
        "response": "According to the project docs and release notes, Alpha is the better production default when reliability and maintenance matter, while Beta is stronger for exploratory workflows. Alpha's deployment story is steadier; Beta is useful for fast prototypes. The practical recommendation is Alpha for production and Beta for experimentation.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "Alpha Beta production comparison",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 2,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "Alpha and Beta production comparison",
                "locator": "https://example.test/alpha-beta-production",
                "snippet": "A substantive source comparing Alpha and Beta for reliability, deployment, maintenance, and experimentation tradeoffs.",
                "claim_hints": ["Alpha is better suited to production reliability."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(grade.pass, "{:?}", grade.failures);
    assert_eq!(
        grade
            .excellent_diagnostics
            .pointer("/subgates/excellent_3_citations_used_in_final")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(!grade
        .excellent_blockers
        .contains(&"missing_final_citation_or_source_signal".to_string()));
}

#[test]
fn bounded_low_evidence_fallback_can_pass_without_earning_excellent() {
    let case = json!({
        "prompt": "Research Mastra for TypeScript agent workflows and compare it with LangGraph.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Mastra", "TypeScript", "LangGraph"]
    });
    let payload = json!({
        "response": "I don't have usable source-backed evidence about Mastra for this turn. The search returned largely off-topic snippets that do not cover Mastra's architecture, strengths, weaknesses, or how it compares to LangGraph for TypeScript agent workflows. Safe boundary given current limits: do not choose between Mastra and LangGraph from this retrieval state; verify Mastra directly against its official documentation or repository before making a source-backed comparison. Next search direction: try a narrower query for Mastra framework documentation or repository material.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "Mastra LangGraph TypeScript agent workflows",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 4,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 1,
            "evidence_refs": [{
                "title": "Generic AI agent roundup",
                "locator": "https://example.test/agent-roundup",
                "snippet": "Mentions LangGraph for agent workflows and generic TypeScript tooling, but not Mastra itself.",
                "claim_hints": ["LangGraph is used for agent workflows."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(grade.pass, "{:?}", grade.failures);
    assert!(!grade.excellent);
    assert!(grade
        .excellent_blockers
        .contains(&"query_satisfaction_below_excellent".to_string()));
}

#[test]
fn assistant_owned_retry_offer_does_not_block_bounded_low_evidence_answer() {
    let case = json!({
        "prompt": "What notable labor actions or strike developments are actually moving this month in major US industries, versus just being threatened?",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["source", "query", "aperture"]
        },
        "required_entities": ["US"]
    });
    let payload = json!({
        "response": "I don't have usable, source-backed evidence for current May 2026 labor actions or strikes in major US industries. The search returned only low-signal results—one February 2026 Cornell Chronicle item on health care strike trends and an unrelated GitHub page—with no materialized coverage of actual walkouts, threatened strikes, or industry-specific developments for this month. Given the current evidence state, a more targeted approach would help: searching specific entities with current-month date filters, or checking primary sources like labor trackers directly for May 2026 entries. If you have a particular industry or union in mind, I can narrow the query and retry.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "source": "web",
                "query": "notable labor actions or strike developments this month in major US industries",
                "queries": ["current labor actions major US industries this month"],
                "keywords": ["labor actions", "strikes", "US industries", "current month"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 6,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 1,
            "evidence_refs": [
                {
                    "title": "Cornell Chronicle on strike trends",
                    "locator": "https://example.test/cornell-strike-trends",
                    "source_kind": "news",
                    "snippet": "A February 2026 Cornell Chronicle article discusses health care strike trends rather than current walkouts across major US industries."
                },
                {
                    "title": "Unrelated GitHub page",
                    "locator": "https://example.test/unrelated-github",
                    "source_kind": "web_page",
                    "snippet": "An unrelated GitHub page that does not cover current labor actions."
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .answer_unit_evidence_alignment
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.answer_unit_evidence_alignment
    );
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        !string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "source_or_process_recap_visible"
                || blocker == "concrete_units_not_traceable_enough"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
}

#[test]
fn outside_evidence_inference_cannot_carry_final_recommendation() {
    let case = json!({
        "prompt": "Compare Alpha, Beta, and Gamma for a purchasing decision.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta", "Gamma"]
    });
    let payload = json!({
        "response": "Based on the available evidence, the retrieved snippets do not provide a direct three-way comparison. General positioning (well-established, not source-backed in this turn): Alpha is known for reliability, Beta is known for flexibility, and Gamma is historically stronger for low-cost deployments. Bottom line: choose Alpha for production unless price is the only criterion.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "Alpha Beta Gamma purchasing comparison",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 3,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 0,
            "evidence_refs": [{
                "title": "General category roundup",
                "locator": "https://example.test/category-roundup",
                "snippet": "This roundup mentions the category but does not compare Alpha, Beta, or Gamma for the user's purchasing criteria."
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(!grade.pass, "{:?}", grade.failures);
    assert!(grade
        .failures
        .contains(&"outside_evidence_used_for_decision".to_string()));
    assert_eq!(
        grade
            .response_grading_layers
            .pointer("/tool_backed_evidence_contract/subgates/evidence_6_respects_source_boundary")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn limitation_heavy_opening_blocks_excellent_even_when_answer_is_structured() {
    let case = json!({
        "prompt": "Compare Alpha and Beta for production use.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta"]
    });
    let payload = json!({
        "response": "I found very limited evidence for this comparison, and the recorded evidence is insufficient for a fully source-backed conclusion. What the recorded evidence actually shows is narrow, but the practical tradeoff still points one way: Alpha looks steadier for production reliability, while Beta is better for exploratory flexibility. My bounded recommendation is Alpha for production and Beta for experiments.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "Alpha Beta production comparison",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 3,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "Alpha and Beta production comparison",
                "locator": "https://example.test/alpha-beta-production",
                "snippet": "A substantive source comparing Alpha and Beta for production reliability and experimentation tradeoffs.",
                "claim_hints": ["Alpha is the steadier production default.", "Beta is better for exploratory work."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(grade.pass, "{:?}", grade.failures);
    assert!(
        !string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "insufficiency_without_bounded_closure"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(!grade.excellent);
    assert!(grade
        .excellent_blockers
        .contains(&"limitation_heavy_answer_shape".to_string()));
    assert_eq!(
        grade
            .excellent_diagnostics
            .pointer("/subgates/excellent_10_answer_not_limitation_heavy")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn grade_case_emits_layered_response_grading_output() {
    let case = json!({
        "prompt": "Compare Alpha and Beta for production use.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta"]
    });
    let payload = json!({
        "response": "According to the docs and release notes, Alpha is the steadier production default, while Beta is stronger for exploration. The practical tradeoff is reliability versus flexibility. My recommendation is Alpha for production and Beta for experiments.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "Alpha Beta production comparison",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 2,
            "materialized_candidate_count": 2,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "Alpha and Beta production comparison",
                "locator": "https://example.test/alpha-beta-production",
                "snippet": "A substantive source comparing Alpha and Beta for reliability and flexibility.",
                "claim_hints": ["Alpha is steadier for production."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .response_grading_layers
            .pointer("/generic_response_contract/pass")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        grade
            .response_grading_layers
            .pointer("/tool_backed_evidence_contract/pass")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        grade
            .response_grading_layers
            .pointer("/workflow_specific_rubric/pass")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("pass")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn visible_internal_posture_label_is_stripped_before_grading() {
    let case = json!({
        "prompt": "What changed in the AI agent landscape this month?",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        }
    });
    let payload = json!({
        "response": "**Bounded_partial_answer** Based on the retrieved evidence, major agent platforms added background-agent and sandboxing features this month.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "AI agent landscape this month",
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 1,
            "content_rich_candidate_count": 1,
            "claim_hint_count": 1,
            "evidence_refs": [{
                "title": "AI agent platform update",
                "locator": "https://example.test/agent-update",
                "snippet": "Major agent platforms added background-agent and sandboxing features this month.",
                "claim_hints": ["Major agent platforms added background-agent and sandboxing features this month."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(!grade
        .response_text
        .to_ascii_lowercase()
        .contains("bounded_partial_answer"));
    assert!(!grade
        .failures
        .contains(&"internal_workflow_state_leaked".to_string()));
}

#[test]
fn soft_quality_smoke_allows_mild_evidence_caveat_when_answer_is_still_direct() {
    let case = json!({
        "prompt": "Compare Alpha and Beta for production use.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta"]
    });
    let payload = json!({
        "response": "Based on the limited evidence retrieved and the coverage gaps noted in the state, Alpha is still the safer production default, while Beta is better for exploratory work. The practical tradeoff is reliability versus flexibility, so I would choose Alpha for production and Beta for experiments.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "Alpha Beta production comparison",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 2,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "Alpha and Beta production comparison",
                "locator": "https://example.test/alpha-beta-production",
                "snippet": "A substantive source comparing Alpha and Beta for reliability and flexibility.",
                "claim_hints": ["Alpha is steadier for production."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("pass")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn unsupported_claim_signal_allows_explicit_low_signal_rejection_of_best_claim() {
    let case = json!({
        "prompt": "What is the best option for this research task?"
    });
    let response = "The retrieval was low-signal and off-topic, so the evidence does not support naming the best option. Claim: \"X is the best option\". Supported? No.";

    assert!(!unsupported_claim_signal(&case, response));
}

#[test]
fn source_dump_retry_template_is_not_a_good_user_answer() {
    let normalized = normalize_for_compare(
            "This retrieval attempt did not produce enough balanced evidence to make a source-backed comparison. Recorded evidence so far: Here's what I found: web search returned low-signal snippets. Retry with a narrower query.",
        );

    assert!(source_summary_without_answer_signal(&normalized));
}

#[test]
fn source_fragment_fallback_template_is_not_a_good_user_answer() {
    let normalized = normalize_for_compare(
        "Based on the retrieved evidence, the strongest supported answer is:
        - User Guide - Example / Shop Solar Generators Portable Power Stations Accessories Gift Card Source: Web result from example.test.
        - Description Summary Tour starts Reykjavik, Iceland Duration 5 days Source: Web result from travel.example.
        Limit: Coverage state: usable evidence is present for Iceland, Reykjavik.",
    );

    assert!(source_summary_without_answer_signal(&normalized));
}

#[test]
fn thin_source_inventory_after_answer_frame_counts_as_process_metadata() {
    let normalized = normalize_for_compare(
        "Here's what I found: - web search: Web benchmark synthesis: arxiv.",
    );

    assert!(answer_unit_is_process_or_metadata_fact(&normalized));
}

#[test]
fn single_source_fragment_fallback_template_is_not_a_good_user_answer() {
    let normalized = normalize_for_compare(
        "Based on the retrieved evidence, the strongest supported answer is:
        - Your accountant, clients, or existing workflow already leans QuickBooks and reducing friction matters more than squeezing out the strongest long-term fit Source: QuickBooks vs Xero 2026: Default Ecosystem or Better Long-Term Fit.
        Limit: Coverage state: usable evidence is present for QuickBooks, Xero, Pilot, Puzzle.",
    );

    assert!(source_summary_without_answer_signal(&normalized));
}

#[test]
fn answer_unit_alignment_handles_decimal_and_compound_tokens() {
    let units = answer_text_units(
        "Pricing starts around $29/seat/month with a usage-based component ($0.99 per Fin outcome). OpenTelemetry v1.56.0 is current for tracing. Freshdesk/Freshworks are compared with Salesforce-native workflows.",
    );
    assert!(
        units
            .iter()
            .any(|unit| unit.contains("$0.99 per Fin outcome")),
        "{units:?}"
    );
    assert!(
        units
            .iter()
            .any(|unit| unit.contains("OpenTelemetry v1.56.0")),
        "{units:?}"
    );
    let terms = answer_unit_specific_terms(&units.join(" "));
    assert!(terms.contains(&"freshdesk".to_string()), "{terms:?}");
    assert!(terms.contains(&"freshworks".to_string()), "{terms:?}");
    assert!(terms.contains(&"salesforce".to_string()), "{terms:?}");
}

#[test]
fn answer_unit_alignment_ignores_without_as_connective() {
    let terms = answer_unit_specific_terms(
        "Without source-backed updates on these entities, claims about credible versus promotional dates cannot be resolved this turn.",
    );
    assert!(!terms.contains(&"without".to_string()), "{terms:?}");
}

#[test]
fn retrieval_limitation_report_without_answer_is_not_successful_research_output() {
    let case = json!({
        "prompt": "Give me news from this week.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        }
    });
    let response = "I don't have enough usable evidence to deliver the concise weekly briefing you requested. What the search returned: one usable but very low-signal result from a section index page, plus one off-target article. There were no headline-level stories and no source-backed claims to cite. Bottom line: the current retrieval did not surface any directly citable major news stories from this week; narrower topic-specific queries would likely perform better.";
    let payload = json!({
        "response": response,
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "Give me news from this week.",
                "queries": ["major news stories this week"],
                "keywords": ["news", "this week"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 20,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [{
                "title": "Generic section index",
                "locator": "https://example.test/news",
                "snippet": "A section landing page that does not provide dated headline-level news stories.",
                "claim_hints": ["The page is a news index."]
            }]
        }]
    });

    let normalized = normalize_for_compare(response);
    assert!(source_summary_without_answer_signal(&normalized));
    let grade = grade_case(&case, &payload, 85, 95);
    assert!(!grade.pass);
    assert!(!grade.excellent);
    assert!(grade
        .failures
        .contains(&"source_summary_without_user_answer".to_string()));
    assert_eq!(
        grade
            .query_satisfaction
            .get("coverage_gap_prevents_answer")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("pass")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn evidence_layer_allows_qualified_relevance_denial() {
    let retrieval_quality = json!({
        "tool_executed": true,
        "usable_evidence": true,
        "status": "usable"
    });
    let citation_behavior = json!({
        "evidence_count": 2,
        "citation_signal": true,
        "response_source_signal": true,
        "synthesis_ignored_citable_evidence": false
    });
    let query_satisfaction = json!({
        "scope_covered": true
    });

    let layer = tool_backed_evidence_contract(
            &normalize_for_compare(
                "I found evidence, but it does not cover the named product. The retrieved rows are false positives, so there is no source-backed basis to choose a winner.",
            ),
            &retrieval_quality,
            &citation_behavior,
            true,
            &query_satisfaction,
            false,
            false,
        );
    assert_eq!(layer.get("pass").and_then(Value::as_bool), Some(true));
}

#[test]
fn evidence_layer_rejects_claim_that_recorded_evidence_does_not_exist() {
    let retrieval_quality = json!({
        "tool_executed": true,
        "usable_evidence": true,
        "status": "usable"
    });
    let citation_behavior = json!({
        "evidence_count": 2,
        "citation_signal": false,
        "response_source_signal": false,
        "synthesis_ignored_citable_evidence": true
    });
    let query_satisfaction = json!({
        "scope_covered": true
    });

    let layer = tool_backed_evidence_contract(
            &normalize_for_compare(
                "No source-backed findings are available yet, so I cannot answer this from the recorded state."
            ),
            &retrieval_quality,
            &citation_behavior,
            true,
            &query_satisfaction,
            false,
            false,
        );
    assert_eq!(layer.get("pass").and_then(Value::as_bool), Some(false));
    assert_eq!(
        layer.get("top_blocker").and_then(Value::as_str),
        Some("recorded_evidence_not_used")
    );
    assert_eq!(
        layer
            .pointer("/subgates/evidence_4_does_not_overclaim_or_deny_recorded_state")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn citation_behavior_accepts_domain_style_source_mentions() {
    let behavior = citation_behavior(
            &json!({}),
            "The strongest current signal favors Alpha for production (langchain.com) while Beta remains better for exploration.",
            &json!({
                "usable_evidence": true,
                "evidence_count": 2
            }),
        );
    assert_eq!(
        behavior
            .get("response_source_signal")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        behavior.get("citation_signal").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn user_facing_answer_quality_passes_coherent_useful_answer() {
    let case = json!({
        "prompt": "Compare Alpha and Beta for production versus exploratory workflows.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta"]
    });
    let payload = json!({
        "response": "For production, Alpha is the safer default because its deployment and maintenance docs emphasize repeatable release workflows (alpha.example). Beta looks better for exploratory work because its docs emphasize fast prototyping and flexible experiment loops (beta.example). The practical split is Alpha for stable repository maintenance and Beta for prototypes, with a quick release-doc check before committing to either.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "Alpha Beta production exploratory workflows",
                "queries": ["Alpha production maintenance docs", "Beta exploratory workflow docs"],
                "keywords": ["Alpha", "Beta", "production", "exploratory workflows"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 5,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "Alpha deployment and maintenance docs",
                    "locator": "https://alpha.example/docs",
                    "source_kind": "official_docs",
                    "snippet": "Alpha deployment and maintenance docs emphasize repeatable release workflows for stable repository maintenance.",
                    "claim_hints": ["Alpha is suited to stable repository maintenance and repeatable release workflows."]
                },
                {
                    "title": "Beta exploratory workflow docs",
                    "locator": "https://beta.example/docs",
                    "source_kind": "official_docs",
                    "snippet": "Beta docs emphasize fast prototyping and flexible experiment loops for exploratory workflows.",
                    "claim_hints": ["Beta is suited to prototypes and exploratory workflows."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("verdict")
            .and_then(Value::as_str),
        Some("sounds_good")
    );
}

#[test]
fn excellent_direct_answer_does_not_require_unneeded_gap_statement() {
    let case = json!({
        "prompt": "Compare Alpha and Beta for production versus exploratory workflows.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Alpha", "Beta"]
    });
    let payload = json!({
        "response": "For production, Alpha is the better default because its deployment docs emphasize repeatable release workflows and stable maintenance (alpha.example). Beta is the better exploratory choice because its docs emphasize fast prototyping and flexible experiment loops (beta.example). The practical recommendation is Alpha for stable repository maintenance and Beta for prototypes.",
        "evidence_pack_quality": {
            "status": "usable",
            "candidate_count": 5,
            "materialized_candidate_count": 2,
            "usable_count": 2,
            "content_rich_item_count": 2
        },
        "evidence_claims": [
            {
                "claim": "Alpha is suited to production and stable repository maintenance because its deployment docs emphasize repeatable release workflows.",
                "source_ref": "alpha-docs",
                "support_snippet": "Alpha deployment and maintenance docs emphasize repeatable release workflows for stable repository maintenance.",
                "source_domain": "alpha.example"
            },
            {
                "claim": "Beta is suited to exploratory workflows because its docs emphasize fast prototyping and flexible experiment loops.",
                "source_ref": "beta-docs",
                "support_snippet": "Beta docs emphasize fast prototyping and flexible experiment loops for exploratory workflows.",
                "source_domain": "beta.example"
            }
        ],
        "evidence_refs": [
            {
                "id": "alpha-docs",
                "title": "Alpha deployment and maintenance docs",
                "locator": "https://alpha.example/docs",
                "source_kind": "official_docs",
                "materialization_quality": "trusted_structured_feed",
                "counts_as_usable_evidence": true,
                "snippet": "Alpha deployment and maintenance docs emphasize repeatable release workflows for stable repository maintenance. The source explains that production use depends on predictable release procedures, stable update paths, and maintainable operational routines."
            },
            {
                "id": "beta-docs",
                "title": "Beta exploratory workflow docs",
                "locator": "https://beta.example/docs",
                "source_kind": "official_docs",
                "materialization_quality": "trusted_structured_feed",
                "counts_as_usable_evidence": true,
                "snippet": "Beta docs emphasize fast prototyping and flexible experiment loops for exploratory workflows. The source describes Beta as useful when teams want quick trials, adaptable experiments, and less ceremony before deciding whether a workflow should become production-grade."
            }
        ],
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "Alpha Beta production exploratory workflows",
                "queries": ["Alpha production maintenance docs", "Beta exploratory workflow docs"],
                "keywords": ["Alpha", "Beta", "production", "exploratory workflows"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 5,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "Alpha deployment and maintenance docs",
                    "locator": "https://alpha.example/docs",
                    "source_kind": "official_docs",
                    "materialization_quality": "trusted_structured_feed",
                    "counts_as_usable_evidence": true,
                    "snippet": "Alpha deployment and maintenance docs emphasize repeatable release workflows for stable repository maintenance. The source explains that production use depends on predictable release procedures, stable update paths, and maintainable operational routines.",
                    "claim_hints": ["Alpha is suited to stable repository maintenance and repeatable release workflows."]
                },
                {
                    "title": "Beta exploratory workflow docs",
                    "locator": "https://beta.example/docs",
                    "source_kind": "official_docs",
                    "materialization_quality": "trusted_structured_feed",
                    "counts_as_usable_evidence": true,
                    "snippet": "Beta docs emphasize fast prototyping and flexible experiment loops for exploratory workflows. The source describes Beta as useful when teams want quick trials, adaptable experiments, and less ceremony before deciding whether a workflow should become production-grade.",
                    "claim_hints": ["Beta is suited to prototypes and exploratory workflows."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(
        grade.score >= 95,
        "score={} blockers={:#?} diagnostics={:#?}",
        grade.score,
        grade.excellent_blockers,
        grade.excellent_diagnostics
    );
    assert!(
        grade.excellent,
        "blockers={:#?} diagnostics={:#?}",
        grade.excellent_blockers,
        grade.excellent_diagnostics
    );
}

#[test]
fn answer_alignment_ignores_generic_presentation_terms() {
    let payload = json!({
        "evidence_claims": [
            {
                "claim": "Amazon Q Developer fits AWS-heavy environments where cloud-specific integration adds value.",
                "support_snippet": "Amazon Q Developer is useful for AWS-heavy teams because cloud-specific context and integration add value.",
                "source_domain": "vendor-notes.example"
            },
            {
                "claim": "EV charging viability depends on utilization, location quality, cost burden, and driver density.",
                "support_snippet": "EV charging business model viability depends on utilization, location quality, fixed operating costs, and driver density.",
                "source_domain": "market-notes.example"
            }
        ]
    });
    let retrieval_quality = json!({
        "usable_evidence": true
    });
    let response = "Amazon Q Developer — Strengths: tuned for AWS-heavy environments. \
        Best fit: Organizations with significant AWS investment where cloud-specific context adds value. \
        EV charging profitability depends on location quality, utilization, and fixed operating costs. \
        The available context does not include 2026-specific U.S. forecasts. \
        Bottom line: filter the landscape through your product risk. \
        Call-focused option: make microphone clarity the primary filter.";

    let alignment = answer_unit_evidence_alignment(&payload, response, &retrieval_quality);
    assert_eq!(
        alignment.get("pass").and_then(Value::as_bool),
        Some(true),
        "{alignment:#?}"
    );
}

#[test]
fn user_facing_answer_quality_allows_light_source_framing_when_answer_is_direct() {
    let case = json!({
        "prompt": "Compare LangGraph and CrewAI for long-running production agents.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["LangGraph", "CrewAI"]
    });
    let payload = json!({
        "response": "Based on the retrieved evidence, here's how LangGraph and CrewAI compare for long-running production agents: LangGraph is the safer default when you need explicit state handling, recovery, and control over execution graphs, while CrewAI is better when you want lighter team-style orchestration with less custom workflow wiring. The practical recommendation is LangGraph for durable, stateful production flows and CrewAI for faster multi-agent coordination where strict graph control matters less.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "LangGraph CrewAI long-running production agents",
                "queries": ["LangGraph durable execution docs", "CrewAI orchestration docs"],
                "keywords": ["LangGraph", "CrewAI", "long-running", "production agents"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 6,
            "content_rich_candidate_count": 5,
            "claim_hint_count": 4,
            "evidence_refs": [
                {
                    "title": "LangGraph durable execution docs",
                    "locator": "https://langgraph.example/durable",
                    "source_kind": "official_docs",
                    "snippet": "LangGraph emphasizes durable execution, recovery, state handling, and explicit control over execution graphs for long-running systems.",
                    "claim_hints": ["LangGraph is suited to long-running stateful agents with durable execution and recovery."]
                },
                {
                    "title": "CrewAI orchestration docs",
                    "locator": "https://crewai.example/orchestration",
                    "source_kind": "official_docs",
                    "snippet": "CrewAI emphasizes lighter team-style orchestration and faster multi-agent coordination with less graph wiring.",
                    "claim_hints": ["CrewAI is better for lighter orchestration and quicker multi-agent coordination."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        !string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "source_or_process_recap_visible")
    );
}

#[test]
fn source_title_fragment_contamination_blocks_user_facing_and_excellent() {
    let case = json!({
        "prompt": "Compare PydanticAI with LangChain for production agent development.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["PydanticAI", "LangChain"]
    });
    let payload = json!({
        "response": "PydanticAI looks stronger when typed validation and predictable model I/O are your main concerns, while LangChain is broader when you need a larger integration surface. Other supported points: Pydantic AI vs LangChain: Which Framework is Better for Production AI Agents. Choosing an agent framework: LangChain vs LangGraph for production teams. The practical split is PydanticAI for typed reliability and LangChain for ecosystem breadth.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "PydanticAI vs LangChain production agent development",
                "queries": ["PydanticAI validation docs", "LangChain integration docs"],
                "keywords": ["PydanticAI", "LangChain", "production agents", "typed validation"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 5,
            "content_rich_candidate_count": 5,
            "claim_hint_count": 4,
            "evidence_refs": [
                {
                    "title": "PydanticAI validation docs",
                    "locator": "https://pydanticai.example/docs",
                    "source_kind": "official_docs",
                    "snippet": "PydanticAI emphasizes typed validation and predictable model I/O for production agent development.",
                    "claim_hints": ["PydanticAI is strong when typed validation and predictable model I/O are required."]
                },
                {
                    "title": "LangChain integration docs",
                    "locator": "https://langchain.example/docs",
                    "source_kind": "official_docs",
                    "snippet": "LangChain provides a broad integration surface for production agent systems.",
                    "claim_hints": ["LangChain is broader when integration surface is the main concern."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "source_title_fragment_contamination")
    );
    assert!(grade
        .failures
        .contains(&"user_facing_answer_not_good_enough".to_string()));
    assert!(!grade.excellent);
    assert!(grade
        .excellent_blockers
        .contains(&"user_facing_quality_not_excellent_ready".to_string()));
}

#[test]
fn single_source_title_with_byline_counts_as_contamination() {
    let case = json!({
        "prompt": "Explain the current data residency and sovereignty requirements that matter to SaaS buyers selling in Europe and the public sector.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        }
    });
    let payload = json!({
        "response": "Self-Hosted EU Data Residency Laws Are Breaking Your SaaS Stack (Here's How to Fix It) /author/michael/ Michael Soto 12 Feb 2026 •",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "data residency sovereignty SaaS buyers Europe public sector",
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 4,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "EU data sovereignty guidance",
                    "locator": "https://example.test/eu-data-sovereignty",
                    "source_kind": "official_docs",
                    "snippet": "European public-sector and regulated SaaS buyers increasingly require data location controls, subprocessor transparency, and clear jurisdiction boundaries.",
                    "claim_hints": ["European public-sector and regulated SaaS buyers increasingly require data location controls, subprocessor transparency, and clear jurisdiction boundaries."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "source_title_fragment_contamination")
    );
}

#[test]
fn user_facing_answer_quality_flags_source_recap_as_not_good() {
    let case = json!({
        "prompt": "Give me news from this week.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        }
    });
    let payload = json!({
        "response": "Here's what I found: web search returned a weather headline, a boating safety interview, and a hospital week resolution. The current turn does not yet support a complete answer, so broaden retrieval before making a stronger claim.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "news from this week",
                "queries": ["news from this week"],
                "keywords": ["news", "week"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 3,
            "content_rich_candidate_count": 1,
            "claim_hint_count": 1,
            "evidence_refs": [{
                "title": "Generic weekly section",
                "locator": "https://example.test/week",
                "snippet": "A generic weekly section page."
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "source_or_process_recap_visible")
    );
}

#[test]
fn thin_source_inventory_answer_frame_fails_user_facing_quality() {
    let case = json!({
        "prompt": "Find recent benchmarks comparing agent frameworks. If the benchmark evidence is weak, explain why and suggest a practical evaluation plan.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["source", "query", "aperture"]
        }
    });
    let payload = json!({
        "response": "The practical answer is that the current evidence supports only a partial conclusion. Here's what I found: - web search: Web benchmark synthesis: arxiv.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "source": "web",
                "query": "Find recent benchmarks comparing agent frameworks.",
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 4,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 1,
            "evidence_refs": [{
                "title": "Agentic Frameworks for Reasoning Tasks",
                "locator": "https://arxiv.org/abs/2604.16646",
                "source_kind": "paper",
                "snippet": "A benchmark-style paper comparing agentic frameworks for reasoning tasks."
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| {
                blocker == "source_or_process_recap_visible"
                    || blocker == "answer_units_not_prompt_useful"
                    || blocker == "substantive_user_value_missing"
            }),
        "{:#?}",
        grade.user_facing_answer_quality
    );
}

#[test]
fn thin_partial_answer_with_value_and_detail_blockers_does_not_pass_user_facing_quality() {
    let case = json!({
        "prompt": "Research data-residency and sovereignty requirements that matter for SaaS buyers in 2026. I want the practical picture for selling into Europe and the US public sector.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["data residency", "Europe", "US public sector"]
    });
    let payload = json!({
        "response": "Europe appears to require stronger residency and sovereignty readiness (ec.europa.eu), but I do not have enough usable evidence yet to give a practical selling recommendation for the US public sector.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "data residency sovereignty SaaS Europe US public sector 2026",
                "queries": ["Europe data residency sovereignty SaaS", "US public sector SaaS data residency requirements"],
                "keywords": ["data residency", "Europe", "US public sector", "sovereignty"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 4,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "European sovereignty guidance",
                "locator": "https://ec.europa.eu/example",
                "source_kind": "government",
                "snippet": "European guidance emphasizes localization, sovereignty posture, and controllable residency expectations for cloud and SaaS vendors.",
                "claim_hints": ["Europe requires stronger residency and sovereignty readiness."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "substantive_user_value_missing"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "wrong_level_of_detail"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert_ne!(
        grade
            .user_facing_answer_quality
            .get("verdict")
            .and_then(Value::as_str),
        Some("sounds_good")
    );
}

#[test]
fn eu_shorthand_does_not_trigger_false_entity_coverage_failure() {
    let case = json!({
        "prompt": "Research data-residency and sovereignty requirements that matter for SaaS buyers in 2026. I want the practical picture for selling into Europe and the US public sector.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["data residency", "Europe", "US public sector"]
    });
    let payload = json!({
        "response": "The EU-US Data Privacy Framework remains the current transfer path for certified US SaaS vendors, while the EU Data Act adds separate portability and interoperability duties that affect European deals. For US public sector work, FedRAMP Rev5 remains the practical baseline. The current gap is that the retrieved evidence does not yet close the loop on narrower StateRAMP or ITAR edge cases.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "data residency sovereignty SaaS Europe US public sector 2026",
                "queries": ["EU Data Act SaaS Europe 2026", "FedRAMP SaaS public sector 2026"],
                "keywords": ["Europe", "EU", "US public sector", "FedRAMP", "data residency"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 5,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [{
                "title": "European sovereignty guidance",
                "locator": "https://example.test/eu-data-act",
                "source_kind": "government",
                "snippet": "EU guidance emphasizes transfer controls, interoperability obligations, and cloud sovereignty posture for SaaS vendors.",
                "claim_hints": [
                    "The EU-US Data Privacy Framework remains a practical transfer path.",
                    "The EU Data Act adds portability and interoperability duties."
                ]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(grade.coverage_entities, vec!["Europe".to_string()]);
    assert!(!grade
        .failures
        .iter()
        .any(|failure| failure.starts_with("entity_coverage_low")));
    assert_eq!(
        grade
            .query_satisfaction
            .get("scope_covered")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.query_satisfaction
    );
}

#[test]
fn useful_explanatory_answer_is_not_blocked_only_by_soft_smoke_decision_signal() {
    let case = json!({
        "prompt": "Research data-residency and sovereignty requirements that matter for SaaS buyers in 2026. I want the practical picture for selling into Europe and the US public sector.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["source", "query", "aperture"]
        },
        "required_entities": ["data residency", "Europe", "US public sector"]
    });
    let payload = json!({
        "response": "For the US public sector, FedRAMP is the baseline gate for cloud and SaaS offerings that handle unclassified agency data. For Europe, the evidence points to tighter data-localization and sovereign-cloud pressure around enterprise procurement, even though the retrieved sources do not fully close the loop on post-Schrems transfer mechanics or DORA operational specifics. Key gaps remain around StateRAMP timing and how EU sovereignty requirements are being enforced in actual contract language.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "source": "web",
                "query": "data residency sovereignty SaaS Europe US public sector 2026",
                "queries": ["EU data localization sovereign cloud SaaS 2026", "FedRAMP SaaS public sector 2026"],
                "keywords": ["Europe", "EU", "US public sector", "FedRAMP", "data residency", "sovereign cloud"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 6,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "EU cloud regulations explained",
                    "locator": "https://example.test/eu-cloud",
                    "source_kind": "analysis",
                    "snippet": "European procurement and sovereignty pressure is increasing around localization, sovereign cloud posture, and transfer controls.",
                    "claim_hints": [
                        "Europe is tightening localization and sovereignty expectations.",
                        "Transfer and procurement expectations are still evolving."
                    ]
                },
                {
                    "title": "FedRAMP public sector scope",
                    "locator": "https://example.test/fedramp-scope",
                    "source_kind": "government",
                    "snippet": "FedRAMP remains the baseline assessment and authorization framework for cloud services used by US federal agencies.",
                    "claim_hints": [
                        "FedRAMP is the baseline gate for US public sector cloud procurement."
                    ]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.soft_quality_smoke
    );
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("top_blocker")
            .and_then(Value::as_str),
        Some("none"),
        "{:#?}",
        grade.soft_quality_smoke
    );
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("verdict")
            .and_then(Value::as_str),
        Some("sounds_good"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("max_score")
            .and_then(Value::as_u64),
        Some(12),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        !grade
            .failures
            .iter()
            .any(|failure| failure == "user_facing_answer_not_good_enough"),
        "{:?}",
        grade.failures
    );
    assert!(
        !grade
            .failures
            .iter()
            .any(|failure| failure.starts_with("research_score_below_pass")),
        "{:?}",
        grade.failures
    );
    assert!(grade.score >= 85, "{:?}", grade.failures);
}

#[test]
fn comparison_gap_without_bounded_closure_fails_user_facing_quality() {
    let case = json!({
        "prompt": "Compare online course platforms for an independent expert who wants to sell structured courses without building a full custom site.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["online course platforms"]
    });
    let payload = json!({
        "response": "For an independent expert selling structured courses without building a custom site, the main split is between platforms optimized for course delivery and ones optimized for marketing funnels. Teachable and Thinkific both look workable if you already have an external site. But the retrieved evidence is insufficient for a direct ranking across the broader field, and I cannot rank them directly for your use case from what was retrieved.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "compare online course platforms structured courses without custom site",
                "queries": ["Teachable Thinkific Kajabi Podia comparison", "online course platform structured course sales comparison"],
                "keywords": ["course platforms", "structured courses", "independent expert", "no custom site"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 5,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 2,
            "evidence_refs": [
                {
                    "title": "Course platform comparison overview",
                    "locator": "https://example.test/course-platforms-overview",
                    "source_kind": "blog",
                    "snippet": "Some platforms emphasize course delivery while others lean harder into funnel and marketing tooling.",
                    "claim_hints": ["Course platforms often split between instructional design focus and marketing-funnel focus."]
                },
                {
                    "title": "Teachable documentation for creators with existing websites",
                    "locator": "https://example.test/teachable-existing-site",
                    "source_kind": "official_docs",
                    "snippet": "Teachable supports creators who already operate an external website and want to embed or link course sales paths.",
                    "claim_hints": ["Teachable works for creators who already have an external site."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "insufficiency_without_bounded_closure"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
}

#[test]
fn insufficiency_only_answer_does_not_count_as_user_facing_good() {
    let case = json!({
        "prompt": "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["meeting overload", "remote teams"]
    });
    let payload = json!({
        "response": "The current evidence is insufficient for a direct source-backed conclusion on which remote-team interventions are strongest. I cannot compare the interventions directly from what was retrieved.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "remote teams meeting overload stronger evidence interventions",
                "queries": ["meeting overload remote teams evidence interventions"],
                "keywords": ["meeting overload", "remote teams", "interventions", "evidence"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 4,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 1,
            "evidence_refs": [{
                "title": "Remote team collaboration guidance",
                "locator": "https://example.test/remote-team-guidance",
                "source_kind": "guide",
                "snippet": "General guidance discusses asynchronous communication and meeting-free blocks, but without a direct comparative ranking.",
                "claim_hints": ["Asynchronous communication and meeting-free blocks are commonly discussed interventions."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "insufficiency_without_bounded_closure"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert_ne!(
        grade
            .user_facing_answer_quality
            .get("verdict")
            .and_then(Value::as_str),
        Some("sounds_good")
    );
}

#[test]
fn off_topic_retrieval_gap_answer_does_not_count_as_user_facing_good() {
    let case = json!({
        "prompt": "Research the current wave of youth social-media restriction bills in the US. What approaches are states trying, and what legal uncertainty remains?",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["youth social-media restriction bills", "US"]
    });
    let payload = json!({
        "response": "I don't have usable evidence about current youth social-media restriction bills in the US. The only material returned is a 2026 Utah driver handbook from the state's Driver License Division, which doesn't address social-media legislation, age verification approaches, or ongoing First Amendment legal challenges. To answer your question properly, I would need source-backed information on state bills, age verification methods, parental consent requirements, and the current status of constitutional challenges—none of which appear in the evidence for this turn.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "youth social media restriction bills US legal uncertainty",
                "queries": ["state social media restriction bills age verification minors"],
                "keywords": ["youth social media restriction bills", "US", "age verification", "legal uncertainty"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 8,
            "content_rich_candidate_count": 6,
            "claim_hint_count": 3,
            "evidence_refs": [{
                "title": "Utah Driver Handbook",
                "locator": "https://example.test/utah-driver-handbook",
                "source_kind": "document_page_artifact_page_enriched",
                "snippet": "The Utah Driver Handbook explains licensing rules and road procedures for state drivers in 2026."
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "insufficiency_without_bounded_closure"
                || blocker == "answer_units_not_prompt_useful"
                || blocker == "substantive_user_value_missing"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert_eq!(
        grade
            .answer_unit_usefulness
            .get("direct_useful_units")
            .and_then(Value::as_u64),
        Some(0),
        "{:#?}",
        grade.answer_unit_usefulness
    );
}

#[test]
fn gap_dominated_partial_answer_does_not_count_as_prompt_useful() {
    let case = json!({
        "prompt": "Research the current wave of youth social-media restriction bills in the US. What approaches are states trying, and what legal uncertainty remains?",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["youth social-media restriction bills", "US"]
    });
    let payload = json!({
        "response": "State-level youth social media restriction legislation is an active but legally unsettled area, though the specific bill mechanisms and current court challenges for Utah, Arkansas, Texas, Florida, and California weren't retrievable in this search. The recorded evidence points to general state references rather than substantive legislative details on age verification mandates, parental consent requirements, or under-18 usage limits that have been central to these laws. Without retrieved specifics on enacted approaches or pending litigation outcomes, I can't characterize which state models are prevailing or how courts are weighing the compelled age-verification against constitutional scrutiny standards.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "youth social media restriction bills US legal uncertainty",
                "queries": ["state social media restriction bills age verification minors", "Utah Arkansas Texas Florida California social media law challenges"],
                "keywords": ["youth social media restriction bills", "US", "age verification", "legal uncertainty"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 12,
            "content_rich_candidate_count": 7,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "State policy references",
                    "locator": "https://example.test/state-policy-overview",
                    "source_kind": "analysis",
                    "snippet": "Several states are considering restrictions touching youth access and social media governance, but this summary does not spell out the bill mechanisms."
                },
                {
                    "title": "Constitutional challenge overview",
                    "locator": "https://example.test/constitutional-overview",
                    "source_kind": "analysis",
                    "snippet": "Legal analysis notes continuing First Amendment questions around compelled age verification and parental consent rules."
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .answer_unit_usefulness
            .get("direct_useful_units")
            .and_then(Value::as_u64),
        Some(0),
        "{:#?}",
        grade.answer_unit_usefulness
    );
    assert_eq!(
        grade
            .user_facing_answer_quality
            .get("pass")
            .and_then(Value::as_bool),
        Some(false),
        "{:#?}",
        grade.user_facing_answer_quality
    );
    assert!(
        string_array_at(&grade.user_facing_answer_quality, &["blockers"])
            .iter()
            .any(|blocker| blocker == "answer_units_not_prompt_useful"
                || blocker == "substantive_user_value_missing"),
        "{:#?}",
        grade.user_facing_answer_quality
    );
}

#[test]
fn traceability_ignores_advisory_individual_factor_framing() {
    let case = json!({
        "prompt": "Summarize the current evidence on menopausal hormone therapy risk so a patient understands what matters before a clinical discussion.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["menopausal hormone therapy"]
    });
    let payload = json!({
        "response": "The FDA removed black box warnings from six menopausal hormone therapy products in February 2026, reflecting updated safety reassessments. Individual factors—personal and family health history, timing of treatment initiation, and formulation—remain critical to weighing benefits against potential harms.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "menopausal hormone therapy risk FDA black box warning 2026",
                "keywords": ["menopausal hormone therapy", "FDA", "black box warning", "2026"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 5,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "FDA warning update",
                "locator": "https://example.test/fda-warning-update",
                "source_kind": "news",
                "snippet": "The FDA removed black box warnings from six menopausal hormone therapy products in February 2026 after updated safety review."
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .answer_unit_evidence_alignment
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.answer_unit_evidence_alignment
    );
}

#[test]
fn traceability_ignores_dash_join_artifacts_and_decision_framing() {
    let case = json!({
        "prompt": "Compare Playwright, browser-use, and Selenium for browser QA automation in 2026. I care about repeatability, CI cost, and current fit for LLM-driven workflows.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Playwright", "browser-use", "Selenium"]
    });
    let payload = json!({
        "response": "For stable scripted testing in 2026, Playwright leads on measurable momentum—30M downloads versus Cypress's 6.5M as of March 2026—with stronger CI efficiency signals. One Medium piece positions browser-use alongside Playwright as part of an emerging browser-agents stack integrating LLMs, but no hard stability metrics for browser-use were returned. Decision boundary: choose Playwright if you need proven, repeatable scripted QA today.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "query": "Playwright browser-use Selenium QA automation 2026",
                "keywords": ["Playwright", "browser-use", "Selenium", "QA automation", "2026", "LLM"],
                "aperture": "medium"
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 8,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 4,
            "evidence_refs": [
                {
                    "title": "Playwright vs Cypress CI costs",
                    "locator": "https://example.test/playwright-ci",
                    "source_kind": "analysis",
                    "snippet": "Playwright reached 30M downloads versus Cypress at 6.5M in March 2026, with lower CI cost in comparative workflow benchmarks."
                },
                {
                    "title": "Browser agents landscape",
                    "locator": "https://example.test/browser-agents",
                    "source_kind": "analysis",
                    "snippet": "Browser-use is discussed alongside Playwright in emerging browser-agent stacks for LLM-driven workflows, but repeatability metrics remain thin."
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
        grade
            .answer_unit_evidence_alignment
            .get("pass")
            .and_then(Value::as_bool),
        Some(true),
        "{:#?}",
        grade.answer_unit_evidence_alignment
    );
}
