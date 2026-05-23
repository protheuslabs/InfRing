use super::*;

pub(super) fn excellent_quality_report(rows: &[Value]) -> Value {
    let total_cases = rows.len() as u64;
    let excellent_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["excellent"], false))
        .count() as u64;
    let mut subgate_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut subgate_passes: BTreeMap<String, u64> = BTreeMap::new();
    let mut blocker_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut top_blocker_counts: BTreeMap<String, u64> = BTreeMap::new();
    for row in rows {
        if let Some(subgates) = row
            .pointer("/excellent_diagnostics/subgates")
            .and_then(Value::as_object)
        {
            for (gate, value) in subgates {
                *subgate_totals.entry(gate.clone()).or_insert(0) += 1;
                if value.as_bool().unwrap_or(false) {
                    *subgate_passes.entry(gate.clone()).or_insert(0) += 1;
                }
            }
        }
        if let Some(blockers) = row
            .pointer("/excellent_diagnostics/blockers")
            .and_then(Value::as_array)
        {
            for blocker in blockers.iter().filter_map(Value::as_str) {
                let blocker = clean_text(blocker, 120);
                if !blocker.is_empty() {
                    *blocker_counts.entry(blocker).or_insert(0) += 1;
                }
            }
        }
        let top_blocker = str_at(row, &["excellent_diagnostics", "top_blocker"], "");
        if !top_blocker.is_empty() && top_blocker != "none" {
            *top_blocker_counts.entry(top_blocker).or_insert(0) += 1;
        }
    }
    let subgate_rates = subgate_totals
        .iter()
        .map(|(gate, total)| {
            let passed = *subgate_passes.get(gate).unwrap_or(&0);
            json!({
                "gate": gate,
                "passed": passed,
                "total": total,
                "pass_rate": ratio(passed, *total)
            })
        })
        .collect::<Vec<_>>();
    let blocker_rows = map_count_rows(&blocker_counts, "blocker");
    let top_blocker_rows = map_count_rows(&top_blocker_counts, "blocker");
    let top_blocker = top_blocker_rows
        .first()
        .and_then(|row| row.get("blocker"))
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    json!({
        "schema_version": 1,
        "excellent_cases": excellent_cases,
        "total_cases": total_cases,
        "excellent_rate": ratio(excellent_cases, total_cases),
        "subgate_pass_rates": subgate_rates,
        "blocker_counts": blocker_rows,
        "top_blocker_counts": top_blocker_rows,
        "top_blocker": top_blocker,
        "note": "Excellent diagnostics isolate generic quality blockers after workflow and tooling gates pass."
    })
}

const UPSTREAM_FAILURE_LAYER_ORDER: &[&str] = &[
    "run_stability",
    "workflow_path",
    "retrieval_mechanics",
    "evidence_carrythrough",
    "synthesis_quality",
    "ux_smoke",
    "none",
];

pub(super) fn upstream_failure_localization(row: &Value) -> Value {
    let case_pass = bool_at(row, &["pass"], false);
    let failure_classification = str_at(row, &["failure_classification"], "");
    let transport_failure = bool_at(row, &["transport_failure"], false);
    let response_error = str_at(row, &["response_diagnostics", "error"], "");
    let transport_error = str_at(row, &["response_diagnostics", "transport_error"], "");
    let first_failed_checkpoint = str_at(
        row,
        &["gate_transition_diagnostics", "first_failed_checkpoint"],
        "",
    );
    let workflow_boundary = str_at(
        row,
        &["gate_transition_diagnostics", "inferred_failure_boundary"],
        "",
    );
    let web_first_failed_gate =
        str_at(row, &["web_tool_gate_diagnostics", "first_failed_gate"], "");
    let web_boundary = str_at(
        row,
        &["web_tool_gate_diagnostics", "inferred_failure_boundary"],
        "",
    );
    let evidence_top_blocker = str_at(
        row,
        &[
            "response_grading_layers",
            "tool_backed_evidence_contract",
            "top_blocker",
        ],
        "",
    );
    let rubric_top_blocker = str_at(
        row,
        &[
            "response_grading_layers",
            "workflow_specific_rubric",
            "top_blocker",
        ],
        "",
    );
    let smoke_top_blocker = str_at(row, &["soft_quality_smoke", "top_blocker"], "");
    let answer_unit_alignment_top_blocker =
        str_at(row, &["answer_unit_evidence_alignment", "top_blocker"], "");
    let answer_unit_usefulness_top_blocker =
        str_at(row, &["answer_unit_usefulness", "top_blocker"], "");
    let evidence_layer_failed = !bool_at(
        row,
        &[
            "response_grading_layers",
            "tool_backed_evidence_contract",
            "pass",
        ],
        true,
    );
    let rubric_failed = !bool_at(
        row,
        &[
            "response_grading_layers",
            "workflow_specific_rubric",
            "pass",
        ],
        true,
    );
    let smoke_failed = !bool_at(row, &["soft_quality_smoke", "pass"], true);
    let answer_unit_alignment_failed =
        !bool_at(row, &["answer_unit_evidence_alignment", "pass"], true)
            && bool_at(row, &["answer_unit_evidence_alignment", "evaluated"], false);
    let answer_unit_usefulness_failed = !bool_at(row, &["answer_unit_usefulness", "pass"], true)
        && bool_at(row, &["answer_unit_usefulness", "evaluated"], false);
    let authoritative_contract_failures = collect_authoritative_contract_failures(row);
    let mut soft_smoke_flags = string_array_at(row, &["soft_quality_smoke", "blockers"]);
    soft_smoke_flags.extend(
        string_array_at(row, &["answer_unit_evidence_alignment", "blockers"])
            .into_iter()
            .map(|blocker| format!("answer_unit_evidence_alignment:{blocker}")),
    );
    soft_smoke_flags.extend(
        string_array_at(row, &["answer_unit_usefulness", "blockers"])
            .into_iter()
            .map(|blocker| format!("answer_unit_usefulness:{blocker}")),
    );
    soft_smoke_flags.sort();
    soft_smoke_flags.dedup();

    let (earliest_failure_layer, earliest_failure_boundary) = if case_pass
        && !smoke_failed
        && !answer_unit_alignment_failed
        && !answer_unit_usefulness_failed
    {
        ("none".to_string(), "none".to_string())
    } else if transport_failure
        || !transport_error.is_empty()
        || response_error == "agent_not_found"
        || failure_classification == "transport"
    {
        let boundary = if !response_error.is_empty() {
            response_error
        } else if !transport_error.is_empty() {
            transport_error
        } else {
            "transport_or_agent_lifecycle_failure".to_string()
        };
        ("run_stability".to_string(), boundary)
    } else if workflow_path_failed(row, &first_failed_checkpoint) {
        let boundary = if !workflow_boundary.is_empty() {
            workflow_boundary
        } else if !first_failed_checkpoint.is_empty() {
            failure_boundary(&first_failed_checkpoint).to_string()
        } else {
            "workflow_path_failure".to_string()
        };
        ("workflow_path".to_string(), boundary)
    } else if retrieval_mechanics_failed(row, &web_first_failed_gate, &first_failed_checkpoint) {
        let boundary = if !web_boundary.is_empty() {
            web_boundary
        } else if !web_first_failed_gate.is_empty() {
            web_failure_boundary(&web_first_failed_gate).to_string()
        } else if !first_failed_checkpoint.is_empty() {
            failure_boundary(&first_failed_checkpoint).to_string()
        } else {
            "retrieval_mechanics_failure".to_string()
        };
        ("retrieval_mechanics".to_string(), boundary)
    } else if evidence_layer_failed
        || matches!(
            first_failed_checkpoint.as_str(),
            "5e_agent_received_evidence_context"
        )
    {
        let boundary = if !evidence_top_blocker.is_empty() && evidence_top_blocker != "none" {
            evidence_top_blocker
        } else if !first_failed_checkpoint.is_empty() {
            failure_boundary(&first_failed_checkpoint).to_string()
        } else {
            "evidence_carrythrough_failure".to_string()
        };
        ("evidence_carrythrough".to_string(), boundary)
    } else if rubric_failed
        || matches!(
            first_failed_checkpoint.as_str(),
            "6a_synthesis_uses_evidence_or_low_evidence_fallback"
        )
    {
        let boundary = if !rubric_top_blocker.is_empty() && rubric_top_blocker != "none" {
            rubric_top_blocker
        } else if !first_failed_checkpoint.is_empty() {
            str_at(
                row,
                &["gate_transition_diagnostics", "synthesis_failure_class"],
                &failure_boundary(&first_failed_checkpoint),
            )
        } else {
            "synthesis_quality_failure".to_string()
        };
        ("synthesis_quality".to_string(), boundary)
    } else if smoke_failed {
        let boundary = if !smoke_top_blocker.is_empty() && smoke_top_blocker != "none" {
            smoke_top_blocker
        } else {
            "soft_quality_smoke_flagged".to_string()
        };
        ("ux_smoke".to_string(), boundary)
    } else if answer_unit_alignment_failed {
        let boundary = if !answer_unit_alignment_top_blocker.is_empty()
            && answer_unit_alignment_top_blocker != "none"
        {
            answer_unit_alignment_top_blocker
        } else {
            "answer_unit_evidence_alignment_flagged".to_string()
        };
        ("synthesis_quality".to_string(), boundary)
    } else if answer_unit_usefulness_failed {
        let boundary = if !answer_unit_usefulness_top_blocker.is_empty()
            && answer_unit_usefulness_top_blocker != "none"
        {
            answer_unit_usefulness_top_blocker
        } else {
            "answer_unit_usefulness_flagged".to_string()
        };
        ("synthesis_quality".to_string(), boundary)
    } else {
        ("none".to_string(), "none".to_string())
    };

    json!({
        "schema_version": 1,
        "layer_order": UPSTREAM_FAILURE_LAYER_ORDER,
        "earliest_failure_layer": earliest_failure_layer,
        "earliest_failure_boundary": earliest_failure_boundary,
        "hardness": if failure_classification.is_empty() { "none" } else { &failure_classification },
        "authoritative_contract_failures": authoritative_contract_failures,
        "soft_smoke_flags": soft_smoke_flags,
        "note": "Earliest broken layer is the canonical debugging entrypoint. Work this layer to stability before moving downstream."
    })
}

pub(super) fn workflow_path_failed(row: &Value, first_failed_checkpoint: &str) -> bool {
    if row
        .get("gates")
        .and_then(Value::as_object)
        .map(|gates| gates.values().any(|value| value.as_bool() == Some(false)))
        .unwrap_or(false)
    {
        return true;
    }
    matches!(
        first_failed_checkpoint,
        "4a_request_template_signaled"
            | "4b_tool_request_candidate_present"
            | "4c_candidate_payload_object"
            | "4d_candidate_schema_fields_present"
            | "4e_pending_request_promoted"
            | "5a_tool_execution_recorded"
    )
}

pub(super) fn retrieval_mechanics_failed(
    row: &Value,
    web_first_failed_gate: &str,
    first_failed_checkpoint: &str,
) -> bool {
    !web_first_failed_gate.is_empty()
        || matches!(
            first_failed_checkpoint,
            "5b_raw_provider_result_present"
                | "5c_packaged_tool_result_present"
                | "5d_evidence_refs_extracted"
        )
        || matches!(
            str_at(row, &["retrieval_quality", "status"], "").as_str(),
            "low_relevance"
                | "low_signal"
                | "provider_degraded"
                | "no_results"
                | "raw_provider_absent"
        )
}

pub(super) fn collect_authoritative_contract_failures(row: &Value) -> Vec<String> {
    let mut failures = string_array_at(row, &["failures"]);
    for path in [
        &[
            "response_grading_layers",
            "generic_response_contract",
            "blockers",
        ][..],
        &[
            "response_grading_layers",
            "tool_backed_evidence_contract",
            "blockers",
        ][..],
        &[
            "response_grading_layers",
            "workflow_specific_rubric",
            "blockers",
        ][..],
    ] {
        failures.extend(string_array_at(row, path));
    }
    failures.sort();
    failures.dedup();
    failures
}

pub(super) fn upstream_failure_localization_report(rows: &[Value]) -> Value {
    let mut layer_counts = BTreeMap::<String, u64>::new();
    let mut boundary_counts = BTreeMap::<String, u64>::new();
    for row in rows {
        let layer = str_at(
            row,
            &["upstream_failure_localization", "earliest_failure_layer"],
            "none",
        );
        let boundary = str_at(
            row,
            &["upstream_failure_localization", "earliest_failure_boundary"],
            "none",
        );
        *layer_counts.entry(layer).or_insert(0) += 1;
        *boundary_counts.entry(boundary).or_insert(0) += 1;
    }
    let mut layer_rows = UPSTREAM_FAILURE_LAYER_ORDER
        .iter()
        .map(|layer| {
            let count = *layer_counts.get(*layer).unwrap_or(&0);
            json!({
                "layer": layer,
                "count": count,
                "rate": ratio(count, rows.len() as u64)
            })
        })
        .collect::<Vec<_>>();
    layer_rows.retain(|row| u64_at(row, &["count"], 0) > 0);
    let top_layer = layer_rows
        .iter()
        .find(|row| str_at(row, &["layer"], "") != "none")
        .and_then(|row| row.get("layer"))
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    json!({
        "schema_version": 1,
        "layer_order": UPSTREAM_FAILURE_LAYER_ORDER,
        "layer_counts": layer_rows,
        "boundary_counts": map_count_rows(&boundary_counts, "boundary"),
        "top_layer": top_layer,
        "note": "Use the earliest broken layer as the only authorized starting point for fixes. Do not optimize downstream layers while an upstream layer is unstable."
    })
}

pub(super) fn map_count_rows(counts: &BTreeMap<String, u64>, key_name: &str) -> Vec<Value> {
    let mut rows = counts
        .iter()
        .map(|(key, count)| {
            let mut row = serde_json::Map::new();
            row.insert(key_name.to_string(), Value::String(key.clone()));
            row.insert("count".to_string(), json!(count));
            Value::Object(row)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        u64_at(right, &["count"], 0)
            .cmp(&u64_at(left, &["count"], 0))
            .then_with(|| str_at(left, &[key_name], "").cmp(&str_at(right, &[key_name], "")))
    });
    rows
}

pub(super) fn retrieval_quality_status_counts(rows: &[Value]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for row in rows {
        let status = str_at(row, &["retrieval_quality", "status"], "unknown");
        *counts.entry(status).or_insert(0) += 1;
    }
    counts
}

pub(super) fn checkpoint_rate(rows: &[Value], checkpoint: &str) -> f64 {
    rows.iter()
        .find(|row| row.get("checkpoint").and_then(Value::as_str) == Some(checkpoint))
        .map(|row| f64_at(row, &["pass_rate"], 0.0))
        .unwrap_or(0.0)
}

pub(super) fn failure_rows_for_classification(rows: &[Value], classification: &str) -> Vec<Value> {
    rows.iter()
        .filter(|row| str_at(row, &["failure_classification"], "") == classification)
        .map(|row| {
            json!({
                "case_id": str_at(row, &["case_id"], "unknown"),
                "score": u64_at(row, &["score"], 0),
                "first_failed_checkpoint": row.pointer("/gate_transition_diagnostics/first_failed_checkpoint").cloned().unwrap_or(Value::Null),
                "failure_boundary": str_at(row, &["gate_transition_diagnostics", "inferred_failure_boundary"], ""),
                "synthesis_failure_class": str_at(row, &["gate_transition_diagnostics", "synthesis_failure_class"], ""),
                "failures": row.get("failures").cloned().unwrap_or_else(|| json!([]))
            })
        })
        .collect()
}

pub(super) fn case_has_retrieval_quality_signal(row: &Value) -> bool {
    let failures = row
        .get("failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let failure_text = failures
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let synthesis_class = str_at(
        row,
        &["gate_transition_diagnostics", "synthesis_failure_class"],
        "",
    );
    let response = normalize_for_compare(&str_at(row, &["response_preview"], ""));
    [
        "entity_coverage_low",
        "low_signal",
        "coverage",
        "retrieval",
        "no usable",
        "no results",
        "zero",
        "provider",
        "source",
    ]
    .iter()
    .any(|needle| {
        failure_text.contains(needle)
            || synthesis_class.contains(needle)
            || response.contains(needle)
    })
}

pub(super) fn gate_transition_diagnostics_for_sequence(
    case: &Value,
    initial_payload: &Value,
    final_payload: &Value,
    confirmation_payload_used: bool,
) -> Value {
    let final_diagnostics = gate_transition_diagnostics(case, final_payload);
    if !confirmation_payload_used {
        return final_diagnostics;
    }
    let initial_diagnostics = gate_transition_diagnostics(case, initial_payload);
    let mut checkpoints = Vec::new();
    for checkpoint_name in [
        "4a_request_template_signaled",
        "4b_tool_request_candidate_present",
        "4c_candidate_payload_object",
        "4d_candidate_schema_fields_present",
        "4e_pending_request_promoted",
        "5a_tool_execution_recorded",
        "5b_raw_provider_result_present",
        "5c_packaged_tool_result_present",
        "5d_evidence_refs_extracted",
        "5e_agent_received_evidence_context",
        "6a_synthesis_uses_evidence_or_low_evidence_fallback",
        "terminal_artifact_present",
    ] {
        let source = if matches!(
            checkpoint_name,
            "5a_tool_execution_recorded"
                | "5b_raw_provider_result_present"
                | "5c_packaged_tool_result_present"
                | "5d_evidence_refs_extracted"
                | "5e_agent_received_evidence_context"
                | "6a_synthesis_uses_evidence_or_low_evidence_fallback"
                | "terminal_artifact_present"
        ) {
            &final_diagnostics
        } else {
            &initial_diagnostics
        };
        if let Some(row) = checkpoint_by_name(source, checkpoint_name) {
            checkpoints.push(row.clone());
        }
    }
    let first_failed_checkpoint = checkpoints
        .iter()
        .find(|row| row.get("status").and_then(Value::as_str) == Some("fail"))
        .and_then(|row| row.get("checkpoint").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    json!({
        "diagnostic_mode": "sequenced_confirmation",
        "first_failed_checkpoint": if first_failed_checkpoint.is_empty() {
            Value::Null
        } else {
            Value::String(first_failed_checkpoint.clone())
        },
        "inferred_failure_boundary": failure_boundary(&first_failed_checkpoint),
        "required_gate_4_fields": initial_diagnostics
            .get("required_gate_4_fields")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "candidate_payload_fields": initial_diagnostics
            .get("candidate_payload_fields")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "final_llm_status": final_diagnostics.get("final_llm_status").cloned().unwrap_or(Value::Null),
        "finalization_outcome": final_diagnostics
            .get("finalization_outcome")
            .cloned()
            .unwrap_or(Value::Null),
        "checkpoints": checkpoints
    })
}

pub(super) fn post_tool_web_tooling_setup_prompt(case: &Value) -> Option<String> {
    if str_at(case, &["category"], "") != "post_tool_synthesis" {
        return None;
    }
    str_opt(case, &["web_tooling_setup", "prompt"])
        .map(|raw| clean_text(raw, 2_000))
        .filter(|raw| !raw.is_empty())
}

pub(super) fn checkpoint_by_name<'a>(diagnostics: &'a Value, name: &str) -> Option<&'a Value> {
    diagnostics
        .get("checkpoints")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("checkpoint").and_then(Value::as_str) == Some(name))
        })
}
