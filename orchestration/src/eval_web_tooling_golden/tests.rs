use serde_json::{json, Value};

use super::super::eval_research_golden_scoring::grade_case;
use super::super::eval_research_golden_utils::str_at;
use super::super::eval_web_retrieval_gate_diagnostics::web_retrieval_gate_diagnostics;
use super::direct_tool::direct_tool_payload_sample;
use super::request_packs::{load_request_pack_index, request_pack_for_case};
use super::synthetic::{
    query_metadata_diagnostics, synthesize_tooling_eval_payload, synthetic_transition_diagnostics,
};
use super::tooling_eval_request_input;

#[test]
fn extracts_request_pack_from_research_report_case() {
    let path = std::env::temp_dir().join("web_tooling_request_pack_extract.json");
    let report = json!({
        "cases": [
            {
                "case_id": "case_a",
                "response_diagnostics": {
                    "pending_tool_request": {
                        "tool_name": "batch_query",
                        "input": {
                            "query": "hello",
                            "queries": ["hello"]
                        }
                    }
                }
            }
        ]
    });
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).expect("write report");
    let index = load_request_pack_index(path.to_str().expect("utf8"));
    assert_eq!(
        str_at(index.get("case_a").expect("case"), &["tool_name"], ""),
        "batch_query"
    );
    assert_eq!(
        str_at(index.get("case_a").expect("case"), &["input", "query"], ""),
        "hello"
    );
}

#[test]
fn ignores_null_pending_tool_request_in_research_report() {
    let path = std::env::temp_dir().join("web_tooling_request_pack_ignore_null.json");
    let report = json!({
        "cases": [
            {
                "case_id": "case_a",
                "response_diagnostics": {
                    "pending_tool_request": null
                }
            }
        ]
    });
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).expect("write report");
    let index = load_request_pack_index(path.to_str().expect("utf8"));
    assert!(index.get("case_a").is_none());
}

#[test]
fn synthetic_payload_exposes_direct_tool_artifacts_to_retrieval_grader() {
    let case = json!({
        "id": "case_a",
        "prompt": "Compare LangGraph and CrewAI",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query",
            "gate_4_required_fields": ["query", "aperture"]
        },
        "required_entities": ["LangGraph", "CrewAI"]
    });
    let request = json!({
        "query": "Compare LangGraph and CrewAI",
        "queries": ["Compare LangGraph and CrewAI"],
        "keywords": ["LangGraph", "CrewAI"],
        "required_coverage": {
            "entities": ["LangGraph", "CrewAI"],
            "facets": ["comparison"]
        },
        "aperture": "medium",
        "source": "web"
    });
    let direct_payload = json!({
        "status": "ok",
        "provider_results": [
            {"title": "LangGraph vs CrewAI docs", "snippet": "LangGraph and CrewAI are both agent frameworks used for production AI agents, with LangGraph emphasizing stateful orchestration and CrewAI emphasizing role-based coordination."}
        ],
        "evidence_refs": [
            {"title": "LangGraph vs CrewAI docs", "snippet": "LangGraph and CrewAI are both agent frameworks used for production AI agents, with LangGraph emphasizing stateful orchestration and CrewAI emphasizing role-based coordination.", "claim_hints": ["stateful orchestration", "role-based coordination"], "source_domain": "langchain.com", "materialization_quality": "full_materialized", "counts_as_usable_evidence": true}
        ],
        "evidence_claims": [
            {"claim": "LangGraph and CrewAI are both production AI agent frameworks.", "source_domain": "langchain.com", "evidence_ref": "LangGraph vs CrewAI docs"},
            {"claim": "LangGraph emphasizes stateful orchestration while CrewAI emphasizes role-based coordination.", "source_domain": "langchain.com", "evidence_ref": "LangGraph vs CrewAI docs"}
        ],
        "tool_result_quality": {
            "claim_hint_count": 2,
            "content_rich_candidate_count": 1,
            "materialized_candidate_count": 1,
            "usable_evidence": true
        }
    });
    let payload = synthesize_tooling_eval_payload("batch_query", &request, &direct_payload);
    let grade = grade_case(&case, &payload, 85, 95);
    assert_eq!(str_at(&grade.retrieval_quality, &["status"], ""), "usable");
    assert!(grade
        .retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false));
}

#[test]
fn direct_transport_timeout_is_measured_as_transport_gate_failure() {
    let case = json!({
        "id": "case_timeout",
        "prompt": "Give me news from this week",
        "expected_gate_path": {
            "gate_1": "tool_required",
            "gate_2": "web_research",
            "gate_3": "batch_query"
        }
    });
    let request = json!({
        "query": "Give me news from this week",
        "queries": ["Give me news from this week"],
        "keywords": ["news", "this week"],
        "required_coverage": {
            "entities": [],
            "facets": ["news", "this week"]
        },
        "source": "web",
        "aperture": "medium"
    });
    let direct_payload = json!({
        "ok": false,
        "transport_error": "curl_failed",
        "stderr": "curl: (28) Operation timed out after 120005 milliseconds with 0 bytes received"
    });
    let payload = synthesize_tooling_eval_payload("batch_query", &request, &direct_payload);
    let grade = grade_case(&case, &payload, 85, 95);
    let query_metadata = query_metadata_diagnostics(&payload);
    let transition_diagnostics =
        synthetic_transition_diagnostics(&payload, &grade.retrieval_quality);
    let gate_diagnostics = web_retrieval_gate_diagnostics(
        &payload,
        &grade.retrieval_quality,
        &query_metadata,
        &transition_diagnostics,
    );

    assert_eq!(
        gate_diagnostics
            .pointer("/first_failed_gate")
            .and_then(Value::as_str),
        Some("web_3a_tool_transport_completed")
    );
    assert_eq!(
        gate_diagnostics
            .pointer("/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("tool_transport_failed")
    );
}

#[test]
fn live_batch_query_tooling_eval_bypasses_cache_by_default() {
    let request = json!({
        "query": "fresh generic research prompt",
        "source": "web",
        "aperture": "medium"
    });
    let out = tooling_eval_request_input("batch_query", &request);
    assert_eq!(
        out.pointer("/cache_mode").and_then(Value::as_str),
        Some("disabled")
    );

    let explicit = json!({
        "query": "cached prompt",
        "cache": {"mode": "refresh"}
    });
    let preserved = tooling_eval_request_input("batch_query", &explicit);
    assert_eq!(
        preserved.pointer("/cache/mode").and_then(Value::as_str),
        Some("refresh")
    );

    let fetch = tooling_eval_request_input("web_fetch", &request);
    assert!(fetch.pointer("/cache_mode").is_none());
}

#[test]
fn direct_tool_sample_preserves_cache_and_query_metadata_diagnostics() {
    let payload = json!({
        "status": "ok",
        "ok": true,
        "cache_status": "disabled",
        "cache_mode": "disabled",
        "query_metadata": {
            "required_coverage": {
                "entities": ["QuickBooks", "Xero"],
                "facets": ["workflow fit"]
            },
            "compilation": {
                "authority": "agent_submitted_request_metadata"
            }
        },
        "query_plan": ["QuickBooks Xero workflow fit"]
    });
    let sample = direct_tool_payload_sample(&payload);
    assert_eq!(
        sample.pointer("/cache_status").and_then(Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        sample.pointer("/cache_mode").and_then(Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        sample
            .pointer("/query_metadata/required_coverage/entities/0")
            .and_then(Value::as_str),
        Some("QuickBooks")
    );
}

#[test]
fn derived_request_pack_moves_generic_required_entities_into_facets() {
    let case = json!({
        "id": "case_sparse",
        "prompt": "Find recent benchmarks comparing agent frameworks. If the benchmark evidence is weak, explain why and suggest a practical evaluation plan.",
        "required_entities": ["benchmark", "agent framework"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    assert_eq!(
        pack.pointer("/input/required_coverage/entities")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        pack.pointer("/input/required_coverage/facets")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert!(pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .map(|rows| rows.len() > 1)
        .unwrap_or(false));
}

#[test]
fn report_request_pack_repairs_instruction_scaffold_pollution() {
    let case = json!({
        "id": "research_pool_083_semiconductor_moves_this_month",
        "prompt": "Research the biggest semiconductor industry moves this month. Focus on developments that would matter to builders or investors, not generic stock chatter.",
        "required_entities": ["semiconductor industry"]
    });
    let report_request = json!({
        "tool_name": "batch_query",
        "input": {
            "source": "web",
            "query": "Research the biggest semiconductor industry moves this month. Focus on developments that would matter to builders or investors, not generic stock chatter.",
            "queries": [
                "Research the biggest semiconductor industry moves this month. Focus on developments that would matter to builders or investors, not generic stock chatter.",
                "Focus research biggest semiconductor",
                "Focus Focus official documentation"
            ],
            "keywords": ["Focus", "research", "biggest", "semiconductor"],
            "required_coverage": {
                "entities": ["Focus"],
                "facets": ["research", "biggest", "semiconductor", "industry", "moves", "month", "focus", "developments", "matter", "builders"]
            }
        }
    });

    let pack = request_pack_for_case(&case, Some(&report_request), "batch_query");
    let entities = pack
        .pointer("/input/required_coverage/entities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let queries = pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let query_strings = queries.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(
        entities.iter().all(|row| row.as_str() != Some("Focus")),
        "{pack:#?}"
    );
    assert!(
        query_strings
            .iter()
            .all(|row| !row.starts_with("Focus ") && !row.contains("Focus Focus")),
        "{pack:#?}"
    );
    assert_eq!(
        pack.pointer("/input/query_metadata_policy/eval_request_pack_repair/status")
            .and_then(Value::as_str),
        Some("repaired_instruction_scaffold_pollution"),
        "{pack:#?}"
    );
}

#[test]
fn derived_request_pack_uses_facet_lanes_for_broad_current_prompts() {
    let case = json!({
        "id": "case_broad",
        "prompt": "Give me an update on the AI agentic landscape in May 2026.",
        "required_facets": ["AI agentic landscape", "May 2026"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    let queries = pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let query_strings = queries.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(
        pack.pointer("/input/required_coverage/entities")
            .and_then(Value::as_array)
            .map(Vec::is_empty)
            .unwrap_or(false),
        "{pack:#?}"
    );
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("AI agentic landscape recent developments")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("AI agentic landscape independent analysis")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .all(|row| !row.contains("May 2026 official site")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .all(|row| !row.starts_with("May 2026 ")),
        "{query_strings:#?}"
    );
}

#[test]
fn derived_request_pack_adds_public_sentiment_lanes_for_sentiment_prompts() {
    let case = json!({
        "id": "case_sentiment",
        "prompt": "Summarize public sentiment around Figma AI features in 2026.",
        "required_entities": ["Figma AI"],
        "required_facets": ["public sentiment", "2026"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    let queries = pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let query_strings = queries.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("public sentiment user reports")),
        "{query_strings:#?}"
    );
}

#[test]
fn derived_request_pack_uses_travel_lanes_for_local_stay_prompts() {
    let case = json!({
        "id": "case_local_stay",
        "prompt": "Research family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability. Compare a few options and tradeoffs.",
        "required_entities": ["Chicago"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    let queries = pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let query_strings = queries.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("travel guide comparison")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("where to stay guide")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("neighborhood guide")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .all(|row| !row.contains("official site")),
        "{query_strings:#?}"
    );
}

#[test]
fn derived_request_pack_keeps_multi_entity_comparison_lanes_together() {
    let case = json!({
        "id": "case_compare",
        "prompt": "Compare Dyson, Roborock, and iRobot for pet hair in apartments.",
        "required_entities": ["Dyson", "Roborock", "iRobot"],
        "required_facets": ["pet hair", "apartments"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    let queries = pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let query_strings = queries.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("Dyson Roborock iRobot pet hair apartments comparison")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .any(|row| *row == "iRobot official documentation"),
        "{query_strings:#?}"
    );
}

#[test]
fn request_pack_prefers_web_tooling_setup_prompt_when_present() {
    let case = json!({
        "id": "case_post_tool",
        "category": "post_tool_synthesis",
        "prompt": "After the web tool returns low-signal results for Infring, synthesize a useful answer anyway.",
        "web_tooling_setup": {
            "prompt": "Use web research to gather public source evidence about Infring."
        },
        "required_entities": ["Infring"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    assert_eq!(
        str_at(&pack, &["request_pack_source"], ""),
        "case_web_tooling_setup_prompt"
    );
    assert_eq!(
        str_at(&pack, &["input", "query"], ""),
        "Use web research to gather public source evidence about Infring."
    );
    assert_eq!(
        str_at(
            &pack,
            &["input", "query_metadata_policy", "classification"],
            ""
        ),
        "tooling_setup_prompt_request"
    );
}

#[test]
fn derived_request_pack_adds_entity_discovery_lanes_for_named_subjects() {
    let case = json!({
        "id": "case_browser_agents",
        "prompt": "Research browser-use, Playwright-based browser agents, and OpenHands for browser task automation. Which is most appropriate for repeatable QA-style workflows?",
        "required_entities": ["browser-use", "Playwright", "OpenHands"]
    });
    let pack = request_pack_for_case(&case, None, "batch_query");
    let queries = pack
        .pointer("/input/queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 4, "{queries:#?}");
    let query_strings = queries.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("browser-use official site")),
        "{query_strings:#?}"
    );
    assert!(
        query_strings
            .iter()
            .any(|row| row.contains("OpenHands official documentation")),
        "{query_strings:#?}"
    );
}
