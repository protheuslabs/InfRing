#[test]
fn answer_alignment_ignores_generic_words_inside_expanded_law_names() {
    let terms = answer_unit_specific_terms(
        "The EU Digital Services Act includes dedicated protections for minors.",
    );

    assert!(!terms.contains(&"digital".to_string()), "{terms:?}");
    assert!(!terms.contains(&"services".to_string()), "{terms:?}");
}

#[test]
fn answer_alignment_ignores_heading_and_scope_words() {
    let terms = answer_unit_specific_terms(
        "LNG Market Dynamics: Global LNG markets are influenced by export terminal capacity.",
    );

    assert!(!terms.contains(&"dynamics".to_string()), "{terms:?}");
    assert!(!terms.contains(&"global".to_string()), "{terms:?}");
}

#[test]
fn answer_alignment_ignores_generic_actor_and_scope_adjectives() {
    let terms = answer_unit_specific_terms(
        "Researchers and scholars reported that Western institutions were highly influential in the policy context.",
    );

    assert!(!terms.contains(&"researcher".to_string()), "{terms:?}");
    assert!(!terms.contains(&"researchers".to_string()), "{terms:?}");
    assert!(!terms.contains(&"scholar".to_string()), "{terms:?}");
    assert!(!terms.contains(&"scholars".to_string()), "{terms:?}");
    assert!(!terms.contains(&"western".to_string()), "{terms:?}");
    assert!(!terms.contains(&"highly".to_string()), "{terms:?}");
}

#[test]
fn answer_alignment_expands_common_scope_initialisms() {
    let payload = json!({
        "response": "For a first-time visitor to New York City, the Upper West Side is a useful base.",
        "pending_tool_request": {
            "input": {
                "query": "Compare NYC neighborhoods for a first-time visitor."
            }
        },
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 3,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 3,
            "evidence_refs": [{
                "title": "NYC first-time visitor neighborhoods",
                "locator": "https://example.test/nyc",
                "snippet": "Upper West Side works well for museums and subway access while avoiding Times Square.",
                "claim_hints": ["Upper West Side is a useful first-time visitor base."]
            }]
        }]
    });
    let retrieval_quality = retrieval_provider_quality(&payload, "nyc neighborhoods");
    let alignment = answer_unit_evidence_alignment(
        &payload,
        "For a first-time visitor to New York City, the Upper West Side is a useful base.",
        &retrieval_quality,
    );

    assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true));
    assert_eq!(
        alignment
            .get("top_blocker")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[test]
fn answer_alignment_ignores_generic_action_verbs_as_claim_terms() {
    let payload = json!({
        "response": "What looks supported: Individualized screening and management tailors care around symptom flares. Pacing specifically supports patients whose symptoms worsen after exertion. Where people are overgeneralizing: Prescribing uniform exercise, pushing fixed escalation, applying aggressive graded exercise, or treating patient reports as trial data all overstate the evidence.",
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 3,
            "materialized_candidate_count": 3,
            "content_rich_candidate_count": 3,
            "claim_hint_count": 3,
            "evidence_refs": [{
                "title": "Long COVID clinical management review",
                "locator": "https://example.test/long-covid",
                "snippet": "The review supports individualized symptom management, screening for post-exertional symptom exacerbation, rehabilitation matched to tolerance, and pacing for patients whose symptoms worsen after exertion. It warns against one-size-fits-all exercise prescriptions.",
                "claim_hints": ["Pacing is supported for patients with post-exertional symptom worsening."]
            }, {
                "title": "Rehabilitation caution statement",
                "locator": "https://example.test/rehab",
                "snippet": "The guidance distinguishes graded rehabilitation for some deconditioned patients from aggressive graded exercise for patients with post-exertional malaise. Programs should monitor setbacks and stop escalation when symptoms flare, rather than forcing progression on a fixed schedule.",
                "claim_hints": ["Fixed escalation can be harmful for some patients."]
            }, {
                "title": "Patient advocacy evidence synthesis",
                "locator": "https://example.test/patient-reports",
                "snippet": "Patient reports highlight relapses after overexertion and practical management harms. Anecdotal reports are not trials but are important for identifying harms and practical management needs.",
                "claim_hints": ["Patient experience should inform caution without replacing clinical evidence."]
            }]
        }]
    });
    let retrieval_quality = retrieval_provider_quality(&payload, "long covid pacing exercise");
    let alignment = answer_unit_evidence_alignment(
        &payload,
        "What looks supported: Individualized screening and management tailors care around symptom flares. Pacing specifically supports patients whose symptoms worsen after exertion. Where people are overgeneralizing: Prescribing uniform exercise, pushing fixed escalation, applying aggressive graded exercise, or treating patient reports as trial data all overstate the evidence.",
        &retrieval_quality,
    );

    assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true), "{alignment}");
}

#[test]
fn answer_alignment_treats_negative_timeline_caveats_as_hedged() {
    let payload = json!({
        "response": "These accounts do not claim a simple, direct pipeline from the 1920s to the 1950s and 1960s.",
        "tools": [{
            "name": "batch_query",
            "status": "ok",
            "candidate_count": 1,
            "materialized_candidate_count": 1,
            "content_rich_candidate_count": 1,
            "claim_hint_count": 1,
            "evidence_refs": [{
                "title": "Civil-rights movement continuity essay",
                "locator": "https://example.test/harlem",
                "snippet": "The essay links Harlem Renaissance networks, publications, patronage, and public intellectual life to later civil-rights organizing and Black political consciousness. It avoids claiming a simple direct pipeline, instead describing a cultural and institutional foundation that later movements drew from.",
                "claim_hints": ["A useful answer should avoid a simplistic cause-effect chain."]
            }]
        }]
    });
    let retrieval_quality = retrieval_provider_quality(&payload, "harlem renaissance political impact");
    let alignment = answer_unit_evidence_alignment(
        &payload,
        "These accounts do not claim a simple, direct pipeline from the 1920s to the 1950s and 1960s.",
        &retrieval_quality,
    );

    assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true), "{alignment}");
}

#[test]
fn response_truncation_detector_flags_incomplete_table_tail() {
    assert!(response_looks_truncated_or_incomplete(
        "Comparison:\n| Dimension | Best signal |\n| SDK ecosystem | Tavily (AWS"
    ));
    assert!(!response_looks_truncated_or_incomplete(
        "Comparison:\n| Dimension | Best signal |\n| SDK ecosystem | Tavily (AWS partnership). |"
    ));
}

#[test]
fn excellent_diagnostics_call_out_missing_final_citation_signal() {
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
        "response": "Alpha is the better default for production when reliability matters, while Beta is more useful for exploratory workflows. Alpha has stronger deployment and maintenance tradeoffs; Beta remains useful when speed of experimentation matters. The practical recommendation is to use Alpha for steady production and Beta for prototypes.",
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
            "evidence_refs": [
                {
                    "title": "Alpha and Beta production comparison",
                    "locator": "https://example.test/alpha-beta-production",
                    "snippet": "A substantive source comparing Alpha and Beta for reliability, deployment, maintenance, and experimentation tradeoffs.",
                    "claim_hints": ["Alpha is better suited to production reliability."]
                },
                {
                    "title": "Alpha and Beta experimentation comparison",
                    "locator": "https://example.test/alpha-beta-experimentation",
                    "snippet": "A second substantive source comparing Alpha and Beta for experimentation speed and prototype workflows.",
                    "claim_hints": ["Beta is more useful for exploratory workflows."]
                }
            ]
        }]
    });

    let grade = grade_case(&case, &payload, 85, 95);
    assert!(grade.pass, "{:?}", grade.failures);
    assert!(!grade.excellent);
    assert_eq!(
        grade
            .excellent_diagnostics
            .pointer("/subgates/excellent_3_citations_used_in_final")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        grade
            .excellent_diagnostics
            .get("top_blocker")
            .and_then(Value::as_str),
        Some("missing_final_citation_or_source_signal")
    );
}
