use super::*;

pub(super) fn record_gate_counts(
    gates: &BTreeMap<String, bool>,
    total_counts: &mut BTreeMap<String, u64>,
    pass_counts: &mut BTreeMap<String, u64>,
) {
    for (gate, ok) in gates.iter() {
        *total_counts.entry(gate.clone()).or_insert(0) += 1;
        if *ok {
            *pass_counts.entry(gate.clone()).or_insert(0) += 1;
        }
    }
}

pub(super) fn query_metadata_diagnostics(payload: &Value) -> Value {
    let request = research_pending_request(payload);
    let Some(request) = request else {
        return json!({
            "eligible_batch_query_request": false,
            "metadata_present": false,
            "rich_query_pack_or_narrow_marker": false,
            "query_lane_count": 0,
            "followup_query_count": 0,
            "multi_query_present": false,
            "keyword_count": 0,
            "alias_count": 0,
            "negative_term_count": 0,
            "required_coverage_entities_count": 0,
            "required_coverage_facets_count": 0,
            "fields_present": [],
            "source": "none"
        });
    };
    let mut tool = str_at(request, &["selected_tool_key"], "");
    if tool.is_empty() {
        tool = str_at(request, &["tool_key"], "");
    }
    if tool.is_empty() {
        tool = str_at(request, &["tool_name"], "");
    }
    let input = request.get("input").unwrap_or(&Value::Null);
    let normalized_tool = normalize_for_compare(&tool);
    let eligible_batch_query = normalized_tool == "batch_query";
    let eligible_web_retrieval = matches!(normalized_tool.as_str(), "batch_query" | "web_search");
    let query_lane_count = array_len(input.get("queries"));
    let followup_query_count = query_lane_count.saturating_sub(1);
    let keyword_count = array_len(input.get("keywords"));
    let alias_count = array_len(input.get("aliases"));
    let negative_term_count = array_len(input.get("negative_terms"));
    let required_coverage_entities_count =
        required_coverage_count(input.get("required_coverage"), "entities");
    let required_coverage_facets_count =
        required_coverage_count(input.get("required_coverage"), "facets");
    let fields_present = input
        .as_object()
        .map(|map| {
            [
                "queries",
                "keywords",
                "required_coverage",
                "aliases",
                "negative_terms",
                "query_metadata_policy",
            ]
            .iter()
            .filter(|field| map.contains_key(**field))
            .map(|field| (*field).to_string())
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let metadata_present = fields_present.iter().any(|field| {
        matches!(
            field.as_str(),
            "keywords"
                | "required_coverage"
                | "aliases"
                | "negative_terms"
                | "query_metadata_policy"
        )
    });
    let rich_query_pack = !json_array_empty(input.get("queries"))
        && (!json_array_empty(input.get("keywords"))
            || required_coverage_nonempty(input.get("required_coverage")));
    let narrow_or_expanded_marker = input
        .pointer("/query_metadata_policy/classification")
        .and_then(Value::as_str)
        .map(|raw| {
            matches!(
                raw,
                "expanded_query_pack" | "narrow_lookup_or_initial_discovery"
            )
        })
        .unwrap_or(false);
    json!({
        "eligible_batch_query_request": eligible_batch_query,
        "eligible_web_retrieval_request": eligible_web_retrieval,
        "metadata_present": eligible_web_retrieval && metadata_present,
        "rich_query_pack_or_narrow_marker": eligible_web_retrieval && (rich_query_pack || narrow_or_expanded_marker),
        "query_lane_count": query_lane_count,
        "followup_query_count": followup_query_count,
        "multi_query_present": query_lane_count > 1,
        "keyword_count": keyword_count,
        "alias_count": alias_count,
        "negative_term_count": negative_term_count,
        "required_coverage_entities_count": required_coverage_entities_count,
        "required_coverage_facets_count": required_coverage_facets_count,
        "fields_present": fields_present,
        "tool": normalized_tool,
        "source": str_at(request, &["source"], "unknown"),
        "classification": input
            .pointer("/query_metadata_policy/classification")
            .and_then(Value::as_str)
            .unwrap_or("")
    })
}

pub(super) fn research_pending_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

pub(super) fn json_array_empty(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
}

pub(super) fn array_len(value: Option<&Value>) -> u64 {
    value
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

pub(super) fn required_coverage_nonempty(value: Option<&Value>) -> bool {
    let Some(map) = value.and_then(Value::as_object) else {
        return false;
    };
    !json_array_empty(map.get("entities")) || !json_array_empty(map.get("facets"))
}

pub(super) fn required_coverage_count(value: Option<&Value>, field: &str) -> u64 {
    value
        .and_then(Value::as_object)
        .and_then(|map| map.get(field))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

pub(super) fn transition_first_failed_checkpoint(diagnostics: &Value) -> Option<String> {
    diagnostics
        .pointer("/first_failed_checkpoint")
        .and_then(Value::as_str)
        .map(|raw| raw.trim())
        .filter(|raw| !raw.is_empty())
        .map(ToString::to_string)
}

pub(super) fn record_checkpoint_counts(
    diagnostics: &Value,
    total_counts: &mut BTreeMap<String, u64>,
    pass_counts: &mut BTreeMap<String, u64>,
) {
    let Some(checkpoints) = diagnostics.get("checkpoints").and_then(Value::as_array) else {
        return;
    };
    for checkpoint in checkpoints {
        let Some(name) = checkpoint.get("checkpoint").and_then(Value::as_str) else {
            continue;
        };
        *total_counts.entry(name.to_string()).or_insert(0) += 1;
        if checkpoint.get("status").and_then(Value::as_str) == Some("pass") {
            *pass_counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

pub(super) fn case_failure_classification(
    case_pass: bool,
    case_failures: &[String],
    setup_failures: &[String],
    transition_diagnostics: &Value,
    empty_response: bool,
    raw_tool_leak: bool,
    tool_choice_final_response: bool,
) -> &'static str {
    if case_pass {
        return "none";
    }
    if case_failures
        .iter()
        .any(|failure| failure == "transport_timeout" || failure == "transport_failure")
    {
        return "transport";
    }
    if !setup_failures.is_empty()
        || empty_response
        || raw_tool_leak
        || tool_choice_final_response
        || case_failures.iter().any(|failure| {
            matches!(
                failure.as_str(),
                "raw_tool_payload_leaked"
                    | "internal_workflow_state_leaked"
                    | "tool_choice_visible_as_final_response"
            )
        })
    {
        return "hard";
    }
    let checkpoint = transition_first_failed_checkpoint(transition_diagnostics).unwrap_or_default();
    if checkpoint.starts_with('4')
        || checkpoint.starts_with('5')
        || checkpoint == "terminal_artifact_present"
    {
        return "hard";
    }
    if transition_diagnostics
        .get("synthesis_failure_hardness")
        .and_then(Value::as_str)
        == Some("hard")
    {
        return "hard";
    }
    "soft"
}
