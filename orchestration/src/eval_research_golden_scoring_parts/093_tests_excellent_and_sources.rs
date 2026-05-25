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
