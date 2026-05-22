use super::super::eval_research_golden_utils::*;
use serde_json::Value;

pub(super) fn raw_provider_result_present(payload: &Value) -> bool {
    if !has_tool_execution(payload) {
        return false;
    }
    tool_rows(payload)
        .iter()
        .any(|row| tool_row_has_raw_provider_result(row))
        || raw_provider_result_paths(payload).iter().any(|path| {
            let pointer = format!("/{}", path.replace('.', "/"));
            payload
                .pointer(&pointer)
                .map(value_has_substantive_result)
                .unwrap_or(false)
        })
}

pub(super) fn packaged_tool_result_present(payload: &Value) -> bool {
    if !has_tool_execution(payload) {
        return false;
    }
    if bool_pointer_any(
        payload,
        &[
            "/response_finalization/findings_available",
            "/response_finalization/tool_completion/findings_available",
        ],
    ) {
        return true;
    }
    tool_rows(payload)
        .iter()
        .any(|row| tool_row_has_packaged_result(row))
}

pub(super) fn evidence_extracted(payload: &Value) -> bool {
    evidence_paths(payload).iter().any(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
    })
}

pub(super) fn agent_received_evidence_context(payload: &Value) -> bool {
    if !evidence_extracted(payload) {
        return false;
    }
    if agent_evidence_context_paths(payload).iter().any(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
    }) {
        return true;
    }
    response_has_source_signal(&normalize_for_compare(&assistant_text(payload)))
}

pub(super) fn synthesis_uses_evidence_or_low_evidence_fallback(
    _case: &Value,
    payload: &Value,
    packaged_tool_result: bool,
    evidence_extracted: bool,
) -> bool {
    let response = assistant_text(payload);
    let normalized = normalize_for_compare(&response);
    if normalized.is_empty() {
        return false;
    }
    if !has_tool_execution(payload)
        && response_matches_explicit_missing_tool_context_contract(&normalized)
    {
        return true;
    }
    if !has_tool_execution(payload) && response_acknowledges_missing_tool_context(&normalized) {
        let has_bounded_missing_context_fallback =
            response_has_missing_tool_context_shape(&normalized)
                || response_has_research_shape(&normalized)
                || response_has_low_evidence_signal(&normalized)
                || normalized.contains("what i know")
                || normalized.contains("what we know");
        return has_bounded_missing_context_fallback
            && !response_uses_internal_runtime_context_as_evidence(&normalized)
            && !response_requests_more_scope_without_substance(&normalized);
    }
    if tool_result_low_signal(payload) {
        return response_has_low_evidence_signal(&normalized)
            && response_has_research_shape(&normalized)
            && !response_overleads_with_tool_status(&normalized)
            && !response_uses_internal_runtime_context_as_evidence(&normalized)
            && !response_requests_more_scope_without_substance(&normalized);
    }
    if evidence_extracted || packaged_tool_result {
        let bounded_partial_shortform = payload_evidence_outcome_posture(payload).as_deref()
            == Some("bounded_partial_answer")
            && response_has_low_evidence_signal(&normalized)
            && response_has_bounded_partial_shortform(&normalized);
        return (response_has_source_signal(&normalized)
            || payload_has_final_citation_signal(payload))
            && (response_has_research_shape(&normalized) || bounded_partial_shortform)
            && !response_overleads_with_tool_status(&normalized)
            && !response_uses_internal_runtime_context_as_evidence(&normalized)
            && !response_requests_more_scope_without_substance(&normalized);
    }
    false
}

fn payload_evidence_outcome_posture(payload: &Value) -> Option<String> {
    payload
        .pointer("/response_workflow/final_llm_response/evidence_outcome_posture")
        .or_else(|| {
            payload.pointer("/response_finalization/final_llm_response/evidence_outcome_posture")
        })
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 120))
        .filter(|raw| !raw.is_empty())
}

fn payload_has_final_citation_signal(payload: &Value) -> bool {
    [
        "/citations",
        "/source_refs",
        "/response_workflow/citations",
        "/response_workflow/source_refs",
        "/response_workflow/final_llm_response/citations",
        "/response_workflow/final_llm_response/source_refs",
        "/response_finalization/citations",
        "/response_finalization/source_refs",
        "/response_finalization/tool_completion/citations",
        "/response_finalization/tool_completion/source_refs",
    ]
    .iter()
    .any(|pointer| {
        payload
            .pointer(pointer)
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(value_has_content))
            .unwrap_or(false)
    })
}

pub(super) fn raw_provider_result_paths(payload: &Value) -> Vec<String> {
    post_tool_paths(
        payload,
        &[
            "raw",
            "raw_result",
            "raw_results",
            "provider_result",
            "provider_results",
            "search_results",
            "organic_results",
            "web_results",
            "raw_result_ref",
            "raw_result_refs",
        ],
        value_has_raw_provider_artifact,
    )
}

pub(super) fn packaged_tool_result_paths(payload: &Value) -> Vec<String> {
    post_tool_paths(
        payload,
        &[
            "result",
            "summary",
            "findings",
            "sources",
            "citations",
            "evidence",
            "evidence_refs",
            "items",
            "results",
            "data",
        ],
        value_has_substantive_result,
    )
}

pub(super) fn evidence_paths(payload: &Value) -> Vec<String> {
    let mut paths = [
        "evidence",
        "evidence_bundle",
        "evidence_refs",
        "sources",
        "citations",
        "response_workflow.evidence",
        "response_workflow.evidence_bundle",
        "response_workflow.evidence_refs",
        "response_workflow.sources",
        "response_workflow.citations",
        "response_finalization.evidence",
        "response_finalization.evidence_bundle",
        "response_finalization.evidence_refs",
        "response_finalization.citations",
        "response_finalization.source_refs",
        "response_finalization.tool_completion.evidence_refs",
        "response_finalization.tool_completion.findings",
        "response_finalization.tool_completion.citations",
        "response_finalization.tool_completion.source_refs",
    ]
    .iter()
    .filter_map(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
            .then(|| (*path).to_string())
    })
    .collect::<Vec<_>>();
    for path in post_tool_paths(
        payload,
        &[
            "evidence",
            "evidence_bundle",
            "evidence_refs",
            "sources",
            "citations",
            "findings",
        ],
        value_has_content,
    ) {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
    paths
}

pub(super) fn agent_evidence_context_paths(payload: &Value) -> Vec<String> {
    [
        "response_workflow.final_llm_response.evidence_refs",
        "response_workflow.final_llm_response.evidence_refs_used",
        "response_workflow.final_llm_response.sources",
        "response_workflow.final_prompt_context.evidence_refs",
        "response_workflow.synthesis_context.evidence_refs",
        "response_finalization.evidence_context",
        "response_finalization.synthesis_context.evidence_refs",
        "response_finalization.tool_completion.evidence_refs_used",
    ]
    .iter()
    .filter_map(|path| {
        let pointer = format!("/{}", path.replace('.', "/"));
        payload
            .pointer(&pointer)
            .map(value_has_content)
            .unwrap_or(false)
            .then(|| (*path).to_string())
    })
    .collect()
}

fn has_tool_execution(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn tool_rows(payload: &Value) -> Vec<&Value> {
    let mut rows = Vec::new();
    if let Some(items) = payload.get("tools").and_then(Value::as_array) {
        rows.extend(items.iter());
    }
    if let Some(items) = payload
        .pointer("/response_finalization/tool_completion/tool_attempts")
        .and_then(Value::as_array)
    {
        rows.extend(items.iter());
    }
    rows
}

fn tool_row_has_raw_provider_result(row: &Value) -> bool {
    [
        "raw",
        "raw_result",
        "raw_results",
        "provider_result",
        "provider_results",
        "search_results",
        "organic_results",
        "web_results",
        "raw_result_ref",
        "raw_result_refs",
    ]
    .iter()
    .any(|key| {
        row.get(*key)
            .map(value_has_raw_provider_artifact)
            .unwrap_or(false)
    })
}

fn tool_row_has_packaged_result(row: &Value) -> bool {
    for key in [
        "sources",
        "citations",
        "evidence",
        "evidence_refs",
        "items",
        "results",
        "data",
    ] {
        if value_has_content(row.get(key).unwrap_or(&Value::Null)) {
            return true;
        }
    }
    let result = str_at(row, &["result"], "");
    value_has_substantive_result(&Value::String(result))
}

fn tool_result_low_signal(payload: &Value) -> bool {
    if !has_tool_execution(payload) {
        return false;
    }
    if tool_result_has_usable_quality(payload) {
        return false;
    }
    let finalization =
        normalize_for_compare(&response_finalization_outcome(payload).unwrap_or_default());
    if finalization.contains("low_signal")
        || finalization.contains("no_results")
        || finalization.contains("tool_failure")
    {
        return true;
    }
    for pointer in [
        "/response_finalization/tool_completion/completion_state",
        "/response_finalization/tool_completion/reasoning",
    ] {
        if payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(text_has_low_signal_only)
            .unwrap_or(false)
        {
            return true;
        }
    }
    bool_pointer_any(
        payload,
        &[
            "/response_finalization/tool_completion/final_no_findings",
            "/response_finalization/tool_completion/final_requests_more_tooling",
        ],
    ) || payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().all(tool_row_is_low_signal))
        .unwrap_or(false)
}

fn tool_result_has_usable_quality(payload: &Value) -> bool {
    tool_rows(payload)
        .iter()
        .any(|row| tool_row_has_usable_quality(row))
}

fn tool_row_has_usable_quality(row: &Value) -> bool {
    row.get("tool_result_quality")
        .map(tool_quality_value_is_usable)
        .unwrap_or(false)
}

fn tool_quality_value_is_usable(quality: &Value) -> bool {
    let status = normalize_for_compare(&str_at(quality, &["status"], ""));
    if status == "usable" || bool_at(quality, &["usable_evidence"], false) {
        return true;
    }
    let evidence_count = u64_at(quality, &["evidence_count"], 0);
    let materialized_count = u64_at(
        quality,
        &["materialized_candidate_count"],
        u64_at(quality, &["content_rich_candidate_count"], 0),
    );
    let claim_hint_count = u64_at(quality, &["claim_hint_count"], 0);
    evidence_count > 0 && materialized_count > 0 && claim_hint_count > 0
}

fn tool_row_is_low_signal(row: &Value) -> bool {
    let status = normalize_for_compare(&str_at(row, &["status"], ""));
    row_status_is_failure_or_empty(&status)
        || row
            .get("result")
            .and_then(Value::as_str)
            .map(text_has_low_signal_only)
            .unwrap_or(false)
}

fn row_status_is_failure_or_empty(status: &str) -> bool {
    matches!(
        status,
        "low_signal"
            | "no_results"
            | "partial_no_results"
            | "error"
            | "failed"
            | "timeout"
            | "blocked"
            | "policy_denied"
    )
}

fn text_has_low_signal_only(raw: &str) -> bool {
    let normalized = normalize_for_compare(raw);
    [
        "low signal",
        "low-signal",
        "low relevance",
        "low-relevance",
        "no usable findings",
        "no usable result",
        "no usable results",
        "no usable snippet",
        "no usable snippets",
        "no results",
        "no source-backed",
        "not source-backed",
        "zero evidence",
        "zero snippets",
        "zero recorded results",
        "not enough source coverage",
        "limited evidence",
        "retrieval results are limited",
        "retrieval result is limited",
        "limited results",
        "weak evidence",
        "off topic",
        "off-topic",
        "off target",
        "off-target",
        "irrelevant",
        "inconclusive",
        "retrieval missed",
        "retrieval miss",
        "retrieval gap",
        "did not produce enough",
        "could not find enough",
        "provider degradation",
        "provider degraded",
        "narrow the query",
        "need a tighter query",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_source_signal(normalized: &str) -> bool {
    [
        "source",
        "evidence",
        "according",
        "docs",
        "release",
        "citation",
        "http://",
        "https://",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_low_evidence_signal(normalized: &str) -> bool {
    [
        "low signal",
        "low-signal",
        "low relevance",
        "low-relevance",
        "limited evidence",
        "retrieval results are limited",
        "retrieval result is limited",
        "source coverage",
        "limited results",
        "limited source",
        "weak evidence",
        "off topic",
        "off-topic",
        "off target",
        "off-target",
        "retrieval missed",
        "retrieval miss",
        "retrieval gap",
        "inconclusive",
        "insufficient",
        "not enough",
        "no results",
        "no_results",
        "no qualifying results",
        "no directly relevant results",
        "no source",
        "no sources",
        "no source snippets",
        "no usable source",
        "no usable sources",
        "no usable evidence",
        "no usable retrieved evidence",
        "no usable result",
        "no usable results",
        "no usable snippet",
        "no usable snippets",
        "no usable catalog evidence",
        "no usable independent evidence",
        "no usable source coverage",
        "no source-backed",
        "not source-backed",
        "cannot source",
        "can't source",
        "zero evidence",
        "zero snippets",
        "zero candidate snippets",
        "zero source snippets",
        "zero recorded results",
        "cannot cite",
        "can't cite",
        "mismatched to returned content",
        "retrieval attempt failed",
        "retrieval failure",
        "retrieval failed",
        "provider degradation",
        "provider degraded",
        "tool error",
        "retrieval-quality miss",
        "retrieval quality miss",
        "no retrievable results",
        "no retrievable evidence",
        "no usable findings",
        "retrieved snippets don't contain",
        "retrieved snippets do not contain",
        "retrieved evidence doesn't contain",
        "retrieved evidence does not contain",
        "evidence falls short",
        "evidence fell short",
        "direct evidence is missing",
        "direct source coverage is missing",
        "caveat",
        "uncertain",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_research_shape(normalized: &str) -> bool {
    normalized.split_whitespace().count() >= 40
        && [
            "tradeoff",
            "trade-off",
            "compare",
            "comparison",
            "versus",
            "vs",
            "recommend",
            "ranking",
            "selection",
            "best for",
            "criteria",
            "dimension",
            "decision",
            "decision boundary",
            "bounded conclusion",
            "bounded guidance",
            "secondary inference",
            "labeled inference",
            "practical implication",
            "current state",
            "supports",
            "does not support",
            "what the evidence covers",
            "what the evidence misses",
            "what the evidence supports",
            "risk",
            "limitation",
            "uncertainty",
            "evidence",
            "source-backed",
            "maturity",
            "security",
            "evaluate",
            "avoid",
        ]
        .iter()
        .any(|needle| normalized.contains(*needle))
}

fn response_has_bounded_partial_shortform(normalized: &str) -> bool {
    normalized.split_whitespace().count() >= 30
        && [
            "practical answer",
            "partial conclusion",
            "coverage gap",
            "coverage gaps",
            "supports only",
            "does not support",
            "evaluation plan",
            "next best search query",
            "next best query",
            "next useful action",
        ]
        .iter()
        .any(|needle| normalized.contains(*needle))
}

fn response_overleads_with_tool_status(normalized: &str) -> bool {
    let first = normalized.split(['.', '\n']).next().unwrap_or("").trim();
    if first.is_empty() {
        return false;
    }
    let status_first = [
        "the web search",
        "the search",
        "based on the search",
        "based on search",
        "search attempt",
        "search returned",
        "search did not",
        "search results",
        "the retrieval",
        "web retrieval",
        "based on the retrieval",
        "based on retrieval",
        "retrieval attempt",
        "retrieval returned",
        "retrieval did not",
        "retrieval results",
        "the tool",
        "tool result",
        "provider degradation",
        "provider degraded",
        "i ran a search",
        "i ran a batch search",
        "i wasn't able to retrieve",
        "i was not able to retrieve",
        "i couldn't retrieve",
        "i could not retrieve",
    ]
    .iter()
    .any(|needle| first.contains(*needle));
    if !status_first {
        return false;
    }
    ![
        "bottom line",
        "my recommendation",
        "practical answer",
        "bounded conclusion",
        "decision",
        "best",
        "use",
        "avoid",
        "treat",
        "should",
        "risk",
        "tradeoff",
        "trade-off",
    ]
    .iter()
    .any(|needle| first.contains(*needle))
}

fn response_requests_more_scope_without_substance(normalized: &str) -> bool {
    let has_scope_request = [
        "narrow the query",
        "pick 2",
        "pick two",
        "which specific",
        "would you prefer",
        "need a tighter query",
        "provide a specific source",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle));
    if !has_scope_request {
        return false;
    }
    let has_bounded_substance = normalized.split_whitespace().count() >= 45
        && (response_has_research_shape(normalized)
            || response_has_low_evidence_signal(normalized)
            || normalized.contains("supports")
            || normalized.contains("does not support")
            || normalized.contains("bounded"));
    !has_bounded_substance
}

fn response_acknowledges_missing_tool_context(normalized: &str) -> bool {
    [
        "no live web data",
        "no returned tool result",
        "tool result is not present in this turn",
        "tool result is not available in this turn",
        "no retrieved snippets",
        "no retrieved results",
        "i havent actually executed any web search",
        "i do not have the tool result",
        "i don't have the tool result",
        "no recorded tool outcome",
        "would require live research",
        "requires live research",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_missing_tool_context_shape(normalized: &str) -> bool {
    let has_knowns = normalized.contains("what i know")
        || normalized.contains("what we know")
        || normalized.contains("from my current context");
    let has_unknowns = normalized.contains("what i do not know")
        || normalized.contains("what we do not know")
        || normalized.contains("would require live research")
        || normalized.contains("requires live research");
    let has_next_step = normalized.contains("next best")
        || normalized.contains("next useful action")
        || normalized.contains("next step")
        || normalized.contains("next query")
        || normalized.contains("follow up query")
        || normalized.contains("follow-up query")
        || normalized.contains("search query");
    normalized.split_whitespace().count() >= 35 && has_knowns && (has_unknowns || has_next_step)
}

fn response_matches_explicit_missing_tool_context_contract(normalized: &str) -> bool {
    normalized.starts_with(
        "no returned tool result is available in this turn so no receipt backed synthesis is available yet",
    ) && normalized.contains("what we know")
        && normalized.contains("what we do not know")
        && (normalized.contains("source") || normalized.contains("evidence"))
        && normalized.contains("recommend")
        && normalized.contains("next best search query")
}

fn response_uses_internal_runtime_context_as_evidence(normalized: &str) -> bool {
    let internal_subject = [
        "identity context",
        "runtime context",
        "workspace metadata",
        "platform identity",
        "agent name",
        "hosting this conversation",
        "this conversation",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle));
    let evidence_claim = [
        "evident from",
        "based on",
        "according to",
        "as evidence",
        "tells me",
        "shows that",
        "proves that",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle));
    if internal_subject && evidence_claim {
        return true;
    }

    [
        "my system instruction",
        "my system instructions",
        "this system instruction",
        "these system instructions",
        "the system instruction says",
        "the system instructions say",
        "evident from system instruction",
        "evident from system instructions",
        "based on system instruction",
        "based on system instructions",
        "according to system instruction",
        "according to system instructions",
        "from internal context",
        "from workspace metadata",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn post_tool_paths(payload: &Value, keys: &[&str], predicate: fn(&Value) -> bool) -> Vec<String> {
    let mut paths = Vec::new();
    for (prefix, rows) in [
        ("tools", payload.get("tools").and_then(Value::as_array)),
        (
            "response_finalization.tool_completion.tool_attempts",
            payload
                .pointer("/response_finalization/tool_completion/tool_attempts")
                .and_then(Value::as_array),
        ),
    ] {
        if let Some(rows) = rows {
            for (idx, row) in rows.iter().enumerate() {
                for key in keys {
                    if row.get(*key).map(predicate).unwrap_or(false) {
                        paths.push(format!("{prefix}.{idx}.{key}"));
                    }
                }
            }
        }
    }
    paths
}

fn bool_pointer_any(payload: &Value, pointers: &[&str]) -> bool {
    pointers.iter().any(|pointer| {
        payload
            .pointer(pointer)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn value_has_substantive_result(value: &Value) -> bool {
    match value {
        Value::String(raw) => {
            !raw.trim().is_empty()
                && raw.split_whitespace().count() >= 8
                && !text_has_low_signal_only(raw)
        }
        Value::Array(rows) => !rows.is_empty() && rows.iter().any(value_has_substantive_result),
        Value::Object(map) => !map.is_empty() && map.values().any(value_has_substantive_result),
        other => value_has_content(other),
    }
}

fn value_has_raw_provider_artifact(value: &Value) -> bool {
    match value {
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => !rows.is_empty() && rows.iter().any(value_has_raw_provider_artifact),
        Value::Object(map) => {
            [
                "provider",
                "query",
                "summary",
                "error",
                "links",
                "locator",
                "snippet",
                "title",
                "provider_raw_count",
                "provider_filtered_count",
            ]
            .iter()
            .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
                || map.values().any(value_has_raw_provider_artifact)
        }
        other => value_has_content(other),
    }
}

fn value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => !rows.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

fn response_finalization_outcome(payload: &Value) -> Option<String> {
    payload
        .pointer("/response_finalization/outcome")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 600))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn synthesis_gate_accepts_final_package_citations_as_source_signal() {
        let payload = json!({
            "response": "For a small team building RAG, I recommend LlamaIndex with Postgres and pgvector as the default. The tradeoff is that this keeps ingestion and retrieval focused while avoiding early managed-vector lock-in. LangChain is better when broad integration is more important than a narrow retrieval stack, while a managed vector database is a later scaling choice.",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "result": "RAG stack findings",
                "evidence_refs": [{
                    "title": "LlamaIndex docs",
                    "locator": "https://docs.llamaindex.ai/",
                    "snippet": "LlamaIndex retrieval workflow notes."
                }]
            }],
            "response_finalization": {
                "citations": [{
                    "citation_id": "source_1",
                    "title": "LlamaIndex docs",
                    "locator": "https://docs.llamaindex.ai/"
                }]
            }
        });
        assert!(synthesis_uses_evidence_or_low_evidence_fallback(
            &json!({}),
            &payload,
            true,
            true
        ));
    }

    #[test]
    fn synthesis_gate_accepts_bounded_partial_shortform_with_evidence_posture() {
        let payload = json!({
            "response": "The practical answer is that the current evidence supports only a partial conclusion. Here's what I found: coverage gaps still matter for LangGraph, CrewAI, AutoGen, and LlamaIndex, so the safest next step is a practical evaluation plan rather than a firm benchmark ranking.",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "result": "Web benchmark synthesis",
                "evidence_refs": [{
                    "title": "Benchmark roundup",
                    "locator": "https://example.com/benchmarks",
                    "snippet": "Coverage is partial and methodology varies across frameworks."
                }]
            }],
            "response_workflow": {
                "final_llm_response": {
                    "evidence_outcome_posture": "bounded_partial_answer"
                }
            }
        });
        assert!(synthesis_uses_evidence_or_low_evidence_fallback(
            &json!({}),
            &payload,
            true,
            true
        ));
    }
}
