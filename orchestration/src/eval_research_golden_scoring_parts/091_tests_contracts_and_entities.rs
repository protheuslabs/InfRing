#[test]
fn query_satisfaction_does_not_mark_goal_coverage_gap_as_excellent_answer() {
    let response = normalize_for_compare(
            "I don't have usable source-backed evidence for this turn. What the evidence covers: none. \
             What the evidence misses: everything specific to your research goal. Next search direction: try a narrower query.",
        );
    let entities = vec!["Mastra".to_string(), "LangGraph".to_string()];
    let coverage = entity_coverage(&response, &entities);
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research Mastra and compare it with LangGraph for TypeScript agent workflows.",
        ),
        &response,
        &entities,
        coverage,
        true,
        true,
        true,
        true,
    );
    assert_eq!(
        satisfaction
            .get("coverage_gap_prevents_answer")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        satisfaction.get("intent_answered").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        satisfaction
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or(10)
            < 9
    );
}

#[test]
fn grade_case_counts_initialism_alias_as_user_entity_coverage() {
    let case = json!({
        "prompt": "Research the current Model Context Protocol ecosystem and summarize maturity and risk.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["Model Context Protocol"]
    });
    let payload = json!({
        "response": "According to source evidence, the MCP ecosystem has strong integration momentum, but product teams should avoid overcommitting to immature server behavior. The practical recommendation is to design around the pattern while keeping adapters replaceable and treating security boundaries as still evolving.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Research query pack",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "input": {
                "source": "web",
                "query": "Research the current Model Context Protocol ecosystem.",
                "queries": ["Model Context Protocol ecosystem maturity risk"],
                "keywords": ["Model Context Protocol", "MCP", "maturity", "risk"],
                "required_coverage": {"entities": ["Model Context Protocol"], "facets": ["maturity", "risk"]},
                "aliases": ["MCP"],
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
                "title": "MCP ecosystem source",
                "locator": "https://example.test/mcp",
                "snippet": "This source describes the MCP ecosystem, maturity signals, risks, and integration behavior with enough detail to support synthesis.",
                "claim_hints": ["MCP ecosystem maturity varies by implementation."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(grade.coverage_entities, vec!["Model Context Protocol"]);
    assert!(!grade
        .failures
        .iter()
        .any(|failure| failure.starts_with("entity_coverage_low")));
    assert_eq!(
        grade
            .query_satisfaction
            .get("scope_covered")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn grade_case_prefers_sanitized_final_visible_response_text() {
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
        "response": "**Outcome posture: bounded_partial_answer** Alpha looks better for production use based on the retrieved evidence.",
        "response_finalization": {
            "final_response": {
                "text": "Alpha looks better for production use based on the retrieved evidence, while Beta remains the better fit for exploratory work."
            }
        },
        "pending_tool_request": {
            "status": "executed",
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
                "claim_hints": ["Alpha is better suited to production reliability.", "Beta is more useful for exploratory workflows."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(
            grade.response_text,
            "Alpha looks better for production use based on the retrieved evidence, while Beta remains the better fit for exploratory work."
        );
    assert!(!grade
        .failures
        .iter()
        .any(|failure| failure == "internal_workflow_state_leaked"));
}

#[test]
fn short_derived_initialisms_are_not_used_as_loose_entity_aliases() {
    assert_eq!(derived_initialism_alias("Artificial Intelligence"), None);
    let response =
        normalize_for_compare("AI safety is discussed, but no country coverage appears.");
    assert!(!normalized_response_covers_entity(
        &response,
        "Artificial Intelligence"
    ));
}

#[test]
fn hidden_fixture_entities_do_not_hard_fail_broad_discovery_prompts() {
    let case = json!({
        "prompt": "Research the strongest open-source coding agents right now and explain which are useful for real repositories versus demos.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["OpenHands", "Aider"]
    });
    let payload = json!({
        "response": "The source-backed finding is that repository usefulness depends less on demo polish and more on repeatability, reviewability, and how well the agent can work against an existing codebase. For real repositories, choose tools with explicit edit loops, test feedback, and clear rollback behavior; treat demo-first agents as exploratory unless their docs show durable project workflows. Caveat: current source coverage is uneven, so verify recent releases before committing.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "open-source coding agents real repositories demos",
                "aperture": "web"
            }
        },
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 3,
            "content_rich_candidate_count": 2,
            "claim_hint_count": 2,
            "evidence_refs": [{
                "title": "Coding agent project workflow docs",
                "locator": "https://example.test/coding-agent-docs",
                "snippet": "This source contains enough detail about edit loops, repository workflows, tests, review, and rollback behavior to support a practical synthesis for repository use.",
                "claim_hints": ["Repository usefulness depends on repeatable edit and test loops."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(grade.coverage_entities.is_empty());
    assert!(!grade
        .failures
        .iter()
        .any(|failure| failure.starts_with("entity_coverage_low")));
    assert_eq!(
        grade
            .query_satisfaction
            .get("scope_covered")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(grade.pass, "{:?}", grade.failures);
}

#[test]
fn real_conversation_source_summary_is_not_a_passing_research_answer() {
    let case = json!({
        "prompt": "what are some scientific breakthroughs 2026?",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        }
    });
    let payload = json!({
        "response": "The safest bounded answer is that the current retrieval state does not support a source-backed conclusion yet; any decision should stay conservative until coverage improves. Recorded evidence so far: Here's what I found:\n\nweb search: From web retrieval: www.nature.com: New tools drive scientific discovery: evidence from all nobel-prize and major non-nobel breakthroughs Nature; Spring 2026 University of Miami Medicine Magazine Highlights Breakthroughs in Heart, Vision and Cancer Research; Nine scientific breakthroughs I’d like to see in 2026. The current turn does not yet support a complete answer to: what are some scientific breakthroughs 2026?. The current tradeoff is breadth versus confidence: we can stay narrow and source-backed on the covered evidence, or broaden retrieval before making a stronger claim. My recommendation is to treat this as a partial answer.",
        "pending_tool_request": {
            "status": "executed",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "what are some scientific breakthroughs 2026?",
                "keywords": ["scientific breakthroughs", "2026"],
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
                "title": "New tools drive scientific discovery",
                "locator": "https://www.nature.com/example",
                "snippet": "New tools drive scientific discovery: evidence from Nobel-prize and major non-Nobel breakthroughs.",
                "claim_hints": ["Scientific discovery depends on new tools."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(!grade.pass, "{:?}", grade.failures);
    assert!(grade
        .failures
        .iter()
        .any(|failure| failure == "source_summary_without_user_answer"));
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("pass")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        grade
            .soft_quality_smoke
            .get("top_blocker")
            .and_then(Value::as_str),
        Some("meta_process_talk_visible")
    );
}

#[test]
fn off_topic_evidence_does_not_count_as_usable_research_data() {
    let payload = json!({
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 3,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "Most Concerning Question Mark Ravens Face With Rookie TE Matthew Hibner",
                    "locator": "https://www.si.com/example",
                    "snippet": "Sports Illustrated published a story about the Baltimore Ravens and a rookie tight end.",
                    "claim_hints": ["The Ravens have a roster question."]
                },
                {
                    "title": "Clinical gaps and legal loopholes paved the way for the Virginia Tech tragedy",
                    "locator": "https://www.psychologytoday.com/example",
                    "snippet": "A psychology article discusses clinical gaps and legal loopholes.",
                    "claim_hints": ["Clinical gaps shaped a tragedy."]
                },
                {
                    "title": "Leaders Seek to Address Big Question Mark Around Private Markets",
                    "locator": "https://www.thinkadvisor.com/example",
                    "snippet": "A finance article discusses private market uncertainty.",
                    "claim_hints": ["Private markets face uncertainty."]
                }
            ]
        }]
    });

    let quality = retrieval_provider_quality(
        &payload,
        &normalize_for_compare("give me an update on the AI agentic landscape in May 2026"),
    );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("low_relevance"),
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
fn prompt_relevance_strips_instruction_words_and_punctuation() {
    let relevance = evidence_prompt_relevance(
            &json!({
                "tools": [{
                    "name": "web_search",
                    "status": "ok",
                    "evidence_refs": [{
                        "title": "Retail result",
                        "locator": "https://example.test/best-buy",
                        "snippet": "Best Buy store page and shopping deals for electronics in 2026."
                    }]
                }]
            }),
            &normalize_for_compare(
                "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer."
            ),
        );
    let prompt_terms = relevance
        .get("prompt_terms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(prompt_terms.iter().any(|term| term == "agentic"));
    assert!(prompt_terms.iter().any(|term| term == "framework"));
    assert!(!prompt_terms.iter().any(|term| term == "search"));
    assert!(!prompt_terms.iter().any(|term| term == "best"));
    assert!(!prompt_terms.iter().any(|term| term == "trust"));
    assert!(!prompt_terms.iter().any(|term| term == "page"));
}

#[test]
fn prompt_relevance_does_not_require_broad_current_scaffold_terms() {
    let relevance = evidence_prompt_relevance_from_texts(
            &normalize_for_compare("Give me the biggest world news from this week."),
            vec![
                normalize_for_compare("NATO allies responded to a U.S. troop deployment shift in Europe after officials described surprise across the alliance."),
                normalize_for_compare("Ukraine recaptured territory after officials disabled illegal Starlink terminals used by Russian forces."),
            ],
            "broad current prompts should be graded by evidence availability, freshness, and source quality rather than literal scaffold overlap",
            true,
        );
    let prompt_terms = relevance
        .get("prompt_terms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(!prompt_terms.iter().any(|term| term == "world"));
    assert!(!prompt_terms.iter().any(|term| term == "week"));
    assert_eq!(
        relevance
            .get("topic_relevant_evidence")
            .and_then(Value::as_bool),
        Some(true),
        "{relevance:#?}"
    );
}

#[test]
fn prompt_relevance_ignores_comparison_and_glue_words() {
    let relevance = evidence_prompt_relevance(
        &json!({
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "evidence_refs": [{
                    "title": "Robot vacuum comparison",
                    "locator": "https://example.test/robot-vacuum",
                    "snippet": "Dyson, Roborock, and iRobot models are compared for pet hair pickup in small apartments."
                }]
            }]
        }),
        &normalize_for_compare("Compare Dyson, Roborock, and iRobot for pet hair in apartments."),
    );
    let prompt_terms = relevance
        .get("prompt_terms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(!prompt_terms.iter().any(|term| term == "compare"));
    assert!(!prompt_terms.iter().any(|term| term == "and"));
    assert!(!prompt_terms.iter().any(|term| term == "for"));
    assert_eq!(
        relevance
            .get("topic_relevant_evidence")
            .and_then(Value::as_bool),
        Some(true),
        "{relevance:#?}"
    );
}

#[test]
fn excellent_requires_more_than_one_relevant_evidence_item() {
    let payload = json!({
        "tools": [{
            "name": "web_search",
            "status": "ok",
            "candidate_count": 6,
            "materialized_candidate_count": 4,
            "content_rich_candidate_count": 4,
            "claim_hint_count": 3,
            "evidence_refs": [
                {
                    "title": "Single relevant framework page",
                    "locator": "https://example.test/framework",
                    "snippet": "This page discusses one agentic framework and its 2026 roadmap.",
                    "claim_hints": ["One framework has a 2026 roadmap."]
                },
                {
                    "title": "Retail page",
                    "locator": "https://example.test/store",
                    "snippet": "Best Buy store page for electronics.",
                    "claim_hints": ["Retail result."]
                }
            ]
        }]
    });

    let quality = retrieval_provider_quality(
            &payload,
            &normalize_for_compare(
                "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer."
            ),
        );
    assert_eq!(
        quality.get("status").and_then(Value::as_str),
        Some("usable"),
        "{quality:#?}"
    );
    assert_eq!(
        quality.get("allows_excellent").and_then(Value::as_bool),
        Some(false),
        "{quality:#?}"
    );
    assert_eq!(
        quality
            .pointer("/classification_inputs/relevant_evidence_count")
            .and_then(Value::as_u64),
        Some(1),
        "{quality:#?}"
    );
}

#[test]
fn user_stated_entities_remain_query_scope() {
    let case = json!({
        "prompt": "Compare OpenHands and Aider for existing repository maintenance.",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "web_search",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["OpenHands", "Aider"]
    });
    let payload = json!({
        "response": "According to source evidence, OpenHands has useful repository-maintenance affordances, but the comparison is incomplete. I would verify release docs before choosing because source coverage is limited and the available evidence only supports a bounded recommendation.",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Web search",
            "tool_name": "web_search",
            "tool_key": "web_search",
            "input": {
                "query": "OpenHands Aider repository maintenance",
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
                "title": "Repository maintenance source",
                "locator": "https://example.test/repo-maintenance",
                "snippet": "This source contains enough detail about repository maintenance workflows, review, test loops, and coding agent operational concerns to support synthesis.",
                "claim_hints": ["Existing repository work requires reviewable edit loops."]
            }]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(grade.coverage_entities, vec!["OpenHands", "Aider"]);
    assert!(grade
        .failures
        .iter()
        .any(|failure| failure.starts_with("entity_coverage_low")));
}
