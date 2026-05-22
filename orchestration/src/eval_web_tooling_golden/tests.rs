use serde_json::{json, Value};

use super::super::eval_research_golden_scoring::grade_case;
use super::super::eval_research_golden_utils::str_at;
use super::request_packs::{load_request_pack_index, request_pack_for_case};
use super::synthetic::synthesize_tooling_eval_payload;

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
