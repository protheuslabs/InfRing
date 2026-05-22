use super::eval_research_golden_utils::*;
use serde_json::{json, Value};

pub(super) fn append_failure_events(
    events: &mut Vec<Value>,
    case_id: &str,
    prompt: &str,
    agent_id: &str,
    live: bool,
    response_text: &str,
    failures: &[String],
    setup_failures: &[String],
) {
    for reason in failures.iter().chain(setup_failures.iter()) {
        events.push(json!({
            "type": "research_golden_failure",
            "case_id": case_id,
            "reason": reason,
            "prompt_preview": clean_text(prompt, 240),
            "response_preview": clean_text(response_text, 320),
            "agent_id": agent_id,
            "live": live,
            "generated_at": now_iso_like(),
        }));
    }
}

pub(super) fn markdown_report(report: &Value) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = format!(
        "# Research Golden Eval (Current)\n\n- generated_at: {}\n- ok: {}\n- mode: {}\n- cases: {}\n- average_score: {:.3}\n- research_success_rate_raw: {:.3}\n- research_success_rate_transport_adjusted: {:.3}\n- gate_path_ok: {}\n- gate_transition_path_ok: {}\n- safety_ok: {}\n- failure_count: {}\n\n",
        str_at(report, &["generated_at"], ""),
        bool_at(report, &["ok"], false),
        str_at(report, &["mode"], ""),
        u64_at(summary, &["cases"], 0),
        f64_at(summary, &["average_score"], 0.0),
        f64_at(summary, &["research_success_rate"], 0.0),
        f64_at(summary, &["transport_adjusted_research_success_rate"], 0.0),
        bool_at(summary, &["gate_path_ok"], false),
        bool_at(summary, &["gate_transition_path_ok"], false),
        bool_at(summary, &["safety_ok"], false),
        u64_at(summary, &["failure_count"], 0),
    );
    out.push_str("## Gate Pass Rates\n\n");
    if let Some(rows) = report
        .get("workflow_gate_pass_rates")
        .and_then(Value::as_array)
    {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3}) ok={}\n",
                str_at(row, &["gate"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
                bool_at(row, &["ok"], false)
            ));
        }
    }
    out.push_str("\n## Gate Transition Diagnostics\n\n");
    if let Some(rows) = report
        .get("gate_transition_pass_rates")
        .and_then(Value::as_array)
    {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3})\n",
                str_at(row, &["checkpoint"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
            ));
        }
    }
    out.push_str("\n## Category Pass Rates\n\n");
    if let Some(rows) = report.get("category_pass_rates").and_then(Value::as_array) {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3}) excellent={}/{}\n",
                str_at(row, &["category"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
                u64_at(row, &["excellent"], 0),
                u64_at(row, &["total"], 0),
            ));
        }
    }
    out.push_str("\n## Tag Pass Rates\n\n");
    if let Some(rows) = report.get("tag_pass_rates").and_then(Value::as_array) {
        for row in rows {
            out.push_str(&format!(
                "- {}: {}/{} ({:.3}) excellent={}/{}\n",
                str_at(row, &["tag"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0),
                u64_at(row, &["excellent"], 0),
                u64_at(row, &["total"], 0),
            ));
        }
    }
    let split = report.get("measurement_split").unwrap_or(&Value::Null);
    out.push_str("\n## Measurement Split\n\n");
    out.push_str(&format!(
        "- deterministic_workflow_path: ok={} hard_failure_cases={}\n",
        bool_at(split, &["deterministic_workflow_path", "ok"], false),
        u64_at(
            split,
            &["deterministic_workflow_path", "hard_failure_cases"],
            0
        )
    ));
    out.push_str(&format!(
        "- live_retrieval_health: status={} tool_execution_rate={:.3} evidence_context_rate={:.3}\n",
        str_at(split, &["live_retrieval_health", "status"], "unknown"),
        f64_at(
            split,
            &["live_retrieval_health", "tool_execution_rate"],
            0.0
        ),
        f64_at(
            split,
            &["live_retrieval_health", "evidence_context_rate"],
            0.0
        )
    ));
    let query_metadata_eligible = u64_at(
        split,
        &["query_metadata_planning", "eligible_web_retrieval_requests"],
        u64_at(
            split,
            &["query_metadata_planning", "eligible_batch_query_requests"],
            0,
        ),
    );
    out.push_str(&format!(
        "- query_metadata_planning: metadata={}/{} ({:.3}) rich_or_marked={}/{} ({:.3})\n",
        u64_at(
            split,
            &["query_metadata_planning", "metadata_present_cases"],
            0
        ),
        query_metadata_eligible,
        f64_at(
            split,
            &["query_metadata_planning", "metadata_present_rate"],
            0.0
        ),
        u64_at(
            split,
            &[
                "query_metadata_planning",
                "rich_query_pack_or_narrow_marker_cases"
            ],
            0
        ),
        query_metadata_eligible,
        f64_at(
            split,
            &[
                "query_metadata_planning",
                "rich_query_pack_or_narrow_marker_rate"
            ],
            0.0
        )
    ));
    out.push_str(&format!(
        "- answer_quality: citation_signal={}/{} ({:.3}) query_satisfaction={}/{} ({:.3}) ignored_citable_evidence={}\n",
        u64_at(split, &["answer_quality", "citation_signal_cases"], 0),
        u64_at(split, &["answer_quality", "citation_ready_cases"], 0),
        f64_at(split, &["answer_quality", "citation_signal_rate"], 0.0),
        u64_at(split, &["answer_quality", "query_satisfaction_cases"], 0),
        u64_at(summary, &["cases"], 0),
        f64_at(split, &["answer_quality", "query_satisfaction_rate"], 0.0),
        u64_at(
            split,
            &[
                "answer_quality",
                "synthesis_ignored_citable_evidence_cases"
            ],
            0
        )
    ));
    out.push_str(&format!(
        "- response_grading_layers: generic={}/{} ({:.3}) evidence={}/{} ({:.3}) rubric={}/{} ({:.3})\n",
        u64_at(
            split,
            &["response_grading_layers", "generic_response_contract_pass_cases"],
            0
        ),
        u64_at(summary, &["cases"], 0),
        f64_at(
            split,
            &["response_grading_layers", "generic_response_contract_pass_rate"],
            0.0
        ),
        u64_at(
            split,
            &[
                "response_grading_layers",
                "tool_backed_evidence_contract_pass_cases"
            ],
            0
        ),
        u64_at(summary, &["cases"], 0),
        f64_at(
            split,
            &[
                "response_grading_layers",
                "tool_backed_evidence_contract_pass_rate"
            ],
            0.0
        ),
        u64_at(
            split,
            &["response_grading_layers", "workflow_specific_rubric_pass_cases"],
            0
        ),
        u64_at(summary, &["cases"], 0),
        f64_at(
            split,
            &["response_grading_layers", "workflow_specific_rubric_pass_rate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "- soft_quality_smoke: pass={}/{} ({:.3}) flagged={}/{} ({:.3}) top_blocker={}\n",
        u64_at(split, &["soft_quality_smoke", "pass_cases"], 0),
        u64_at(summary, &["cases"], 0),
        f64_at(split, &["soft_quality_smoke", "pass_rate"], 0.0),
        u64_at(split, &["soft_quality_smoke", "flagged_cases"], 0),
        u64_at(summary, &["cases"], 0),
        f64_at(split, &["soft_quality_smoke", "flagged_rate"], 0.0),
        str_at(
            split,
            &["soft_quality_smoke", "top_blocker", "name"],
            "none"
        )
    ));
    out.push_str(&format!(
        "- answer_unit_evidence_alignment: evaluated={}/{} pass={}/{} ({:.3}) flagged={}/{} ({:.3}) support={:.3} top_blocker={}\n",
        u64_at(split, &["answer_unit_evidence_alignment", "evaluated_cases"], 0),
        u64_at(summary, &["cases"], 0),
        u64_at(split, &["answer_unit_evidence_alignment", "evaluated_pass_cases"], 0),
        u64_at(split, &["answer_unit_evidence_alignment", "evaluated_cases"], 0),
        f64_at(
            split,
            &["answer_unit_evidence_alignment", "evaluated_pass_rate"],
            0.0
        ),
        u64_at(split, &["answer_unit_evidence_alignment", "flagged_cases"], 0),
        u64_at(summary, &["cases"], 0),
        f64_at(
            split,
            &["answer_unit_evidence_alignment", "flagged_rate"],
            0.0
        ),
        f64_at(
            split,
            &["answer_unit_evidence_alignment", "average_term_support_rate"],
            0.0
        ),
        str_at(
            split,
            &["answer_unit_evidence_alignment", "top_blocker", "name"],
            "none"
        )
    ));
    out.push_str(&format!(
        "- web_tooling: top_layer={} top_failure={} raw_candidates/case={:.3} usable_evidence_rate={:.3}\n",
        str_at(
            split,
            &["web_tooling", "operator_metrics", "top_layer"],
            "none"
        ),
        str_at(
            split,
            &["web_tooling", "operator_metrics", "top_first_failure", "name"],
            "none"
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "averages", "raw_candidates_per_case"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "conversion_rates", "usable_evidence_case_rate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "  - query_planning: query_lanes/case={:.3} followups/case={:.3} keywords/case={:.3} multi_query_rate={:.3}\n",
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "query_planning", "query_lanes_per_case"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "query_planning", "followup_queries_per_case"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "query_planning", "keywords_per_case"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "query_planning", "multi_query_case_rate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "  - candidate_supply: unique_domains/case={:.3} official_case_rate={:.3} relevant_per_candidate={:.3}\n",
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "candidate_supply", "unique_source_domains_per_case"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "candidate_supply", "official_or_primary_case_rate"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "candidate_supply", "relevant_evidence_per_candidate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "  - access_materialization: blocked_rate={:.3} content_rich_per_candidate={:.3}\n",
        f64_at(
            split,
            &[
                "web_tooling",
                "operator_metrics",
                "blocker_rates",
                "access_blocked_or_throttled_case_rate"
            ],
            0.0
        ),
        f64_at(
            split,
            &[
                "web_tooling",
                "operator_metrics",
                "conversion_rates",
                "content_rich_per_candidate"
            ],
            0.0
        )
    ));
    out.push_str(&format!(
        "  - claim_extraction: claim_hints_per_evidence={:.3}\n",
        f64_at(
            split,
            &[
                "web_tooling",
                "operator_metrics",
                "conversion_rates",
                "claim_hints_per_evidence"
            ],
            0.0
        )
    ));
    out.push_str(&format!(
        "  - usable_evidence_packaging: evidence_per_candidate={:.3} synthesis_handoff_rate={:.3}\n",
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "conversion_rates", "evidence_per_candidate"],
            0.0
        ),
        f64_at(
            split,
            &["web_tooling", "operator_metrics", "conversion_rates", "synthesis_handoff_case_rate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "- upstream_failure_localization: top_layer={} run_stability={} workflow_path={} retrieval_mechanics={} evidence_carrythrough={} synthesis_quality={} ux_smoke={}\n",
        str_at(split, &["upstream_failure_localization", "top_layer"], "none"),
        layer_count(split, "run_stability"),
        layer_count(split, "workflow_path"),
        layer_count(split, "retrieval_mechanics"),
        layer_count(split, "evidence_carrythrough"),
        layer_count(split, "synthesis_quality"),
        layer_count(split, "ux_smoke")
    ));
    let excellent_quality = split.get("excellent_quality").unwrap_or(&Value::Null);
    out.push_str(&format!(
        "- excellent_quality: excellent={}/{} ({:.3}) top_blocker={}\n",
        u64_at(excellent_quality, &["excellent_cases"], 0),
        u64_at(excellent_quality, &["total_cases"], 0),
        f64_at(excellent_quality, &["excellent_rate"], 0.0),
        str_at(excellent_quality, &["top_blocker"], "none")
    ));
    if let Some(rows) = excellent_quality
        .get("subgate_pass_rates")
        .and_then(Value::as_array)
    {
        for row in rows {
            out.push_str(&format!(
                "  - {}: {}/{} ({:.3})\n",
                str_at(row, &["gate"], "unknown"),
                u64_at(row, &["passed"], 0),
                u64_at(row, &["total"], 0),
                f64_at(row, &["pass_rate"], 0.0)
            ));
        }
    }
    out.push_str(&format!(
        "- end_to_end_golden: mode={} raw_success_rate={:.3} transport_adjusted_success_rate={:.3} transport_failures={} soft_failure_cases={}\n",
        str_at(split, &["end_to_end_golden", "mode"], "unknown"),
        f64_at(split, &["end_to_end_golden", "research_success_rate"], 0.0),
        f64_at(
            split,
            &["end_to_end_golden", "transport_adjusted_research_success_rate"],
            0.0
        ),
        u64_at(split, &["failure_classification", "transport_failure_cases"], 0),
        u64_at(split, &["failure_classification", "soft_failure_cases"], 0)
    ));
    let lifecycle = report.get("observation_lifecycle").unwrap_or(&Value::Null);
    out.push_str("\n## Observation Lifecycle\n\n");
    out.push_str(&format!(
        "- enabled={} ok={} events_recorded={}\n",
        bool_at(lifecycle, &["enabled"], false),
        bool_at(lifecycle, &["ok"], false),
        u64_at(lifecycle, &["events_recorded"], 0)
    ));
    out.push_str(&format!(
        "- archive: open_subjects={} archived_subjects={} reemerged_subjects={}\n",
        u64_at(lifecycle, &["summary", "archive", "open_subjects"], 0),
        u64_at(lifecycle, &["summary", "archive", "archived_subjects"], 0),
        u64_at(lifecycle, &["summary", "archive", "reemerged_subjects"], 0)
    ));
    out.push_str("\n## Lowest Cases\n\n");
    let mut case_rows = report
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    case_rows.sort_by_key(|row| u64_at(row, &["score"], 0));
    for row in case_rows.iter().take(8) {
        out.push_str(&format!(
            "- {}: score={} score_pass={} lifecycle_complete={} pass={} failures={}\n",
            str_at(row, &["case_id"], "unknown"),
            u64_at(row, &["score"], 0),
            bool_at(row, &["score_pass"], false),
            bool_at(row, &["lifecycle_gate_path_complete"], false),
            bool_at(row, &["pass"], false),
            row.get("failures")
                .and_then(Value::as_array)
                .map(|failures| failures
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_default()
        ));
    }
    out
}

fn layer_count(split: &Value, layer: &str) -> u64 {
    split
        .pointer("/upstream_failure_localization/layer_counts")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("layer").and_then(Value::as_str) == Some(layer))
        })
        .and_then(|row| row.get("count").and_then(Value::as_u64))
        .unwrap_or(0)
}
