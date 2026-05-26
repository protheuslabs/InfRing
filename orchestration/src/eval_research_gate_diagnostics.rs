use super::eval_research_golden_utils::*;
#[path = "eval_research_gate_post_tool.rs"]
mod eval_research_gate_post_tool;
use eval_research_gate_post_tool::{
    agent_evidence_context_paths, agent_received_evidence_context, evidence_extracted,
    evidence_paths, packaged_tool_result_paths, packaged_tool_result_present,
    raw_provider_result_paths, raw_provider_result_present,
    synthesis_uses_evidence_or_low_evidence_fallback,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) fn gate_transition_diagnostics(case: &Value, payload: &Value) -> Value {
    let required_fields = string_array_at(case, &["expected_gate_path", "gate_4_required_fields"]);
    let pending_request = pending_tool_request(payload);
    let candidate = pending_request
        .or_else(|| latent_tool_candidate(payload))
        .or_else(|| response_workflow_candidate(payload));
    let candidate_payload = candidate.and_then(candidate_input_object);
    let candidate_fields = candidate_payload
        .and_then(Value::as_object)
        .map(|input| {
            input
                .keys()
                .map(|key| normalize_for_compare(key))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let serialized = payload.to_string().to_ascii_lowercase();
    let final_response_present = !assistant_text(payload).trim().is_empty();
    let case_allows_existing_tool_state =
        case_allows_existing_tool_state_without_new_candidate(case);
    let synthesis_only_without_new_candidate =
        case_allows_existing_tool_state && candidate.is_none();
    let template_signaled = required_fields.iter().all(|field| {
        let field = normalize_for_compare(field);
        candidate_fields.iter().any(|key| key == &field)
            || serialized.contains(&format!("\"{field}\""))
    });
    let candidate_present = candidate.is_some();
    let candidate_payload_present = candidate_payload.map(Value::is_object).unwrap_or(false);
    let schema_fields_present = !required_fields.is_empty()
        && required_fields.iter().all(|field| {
            let field = normalize_for_compare(field);
            candidate_fields.iter().any(|key| key == &field)
        });
    let pending_promoted = pending_request
        .and_then(|request| request.get("status").and_then(Value::as_str))
        .map(|status| matches!(status, "pending_confirmation" | "executed" | "allowed"))
        .unwrap_or(false);
    let tool_attempted = has_tool_execution(payload);
    let raw_provider_result = raw_provider_result_present(payload);
    let packaged_tool_result = packaged_tool_result_present(payload);
    let evidence_extracted = evidence_extracted(payload);
    let agent_received_evidence_context = agent_received_evidence_context(payload);
    let synthesis_ok = synthesis_uses_evidence_or_low_evidence_fallback(
        case,
        payload,
        packaged_tool_result,
        evidence_extracted,
    );
    let synthesis_failure_class = synthesis_failure_class(
        case,
        payload,
        packaged_tool_result,
        evidence_extracted,
        agent_received_evidence_context,
        synthesis_ok,
    );
    let synthesis_failure_hardness =
        synthesis_failure_hardness(&synthesis_failure_class, &first_failure_stage_hint(payload));
    let structured_terminal_present = final_response_present
        || pending_promoted
        || tool_attempted
        || response_finalization_outcome(payload)
            .map(|outcome| outcome.contains("structured_failure"))
            .unwrap_or(false);

    let checkpoints = vec![
        checkpoint(
            "4a_request_template_signaled",
            template_signaled || synthesis_only_without_new_candidate,
            template_signaled || synthesis_only_without_new_candidate,
            if template_signaled {
                "required request fields are visible in request-state artifacts"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh request template is required"
            } else {
                "required request fields are absent from request-state artifacts"
            },
            vec!["expected_gate_path.gate_4_required_fields"],
        ),
        checkpoint(
            "4b_tool_request_candidate_present",
            candidate_present || synthesis_only_without_new_candidate,
            candidate_present || synthesis_only_without_new_candidate,
            if candidate_present {
                "a request candidate exists before broker admission"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh request candidate is required"
            } else {
                "no pending request, latent candidate, or workflow request candidate exists"
            },
            candidate_paths(payload),
        ),
        checkpoint(
            "4c_candidate_payload_object",
            candidate_present || synthesis_only_without_new_candidate,
            candidate_payload_present || synthesis_only_without_new_candidate,
            if candidate_payload_present {
                "candidate contains an object payload/input"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh candidate payload is required"
            } else if candidate_present {
                "candidate exists but does not contain an object payload/input"
            } else {
                "no candidate exists to parse"
            },
            vec!["input", "request_payload", "payload"],
        ),
        checkpoint(
            "4d_candidate_schema_fields_present",
            candidate_payload_present || synthesis_only_without_new_candidate,
            schema_fields_present || synthesis_only_without_new_candidate,
            if schema_fields_present {
                "candidate payload includes all declared Gate 4 fields"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh Gate 4 payload fields are required"
            } else if required_fields.is_empty() {
                "case declares no Gate 4 required fields"
            } else {
                "candidate payload is missing one or more declared Gate 4 fields"
            },
            required_fields.clone(),
        ),
        checkpoint(
            "4e_pending_request_promoted",
            candidate_payload_present || synthesis_only_without_new_candidate,
            pending_promoted || synthesis_only_without_new_candidate,
            if pending_promoted {
                "candidate was promoted to a pending/admitted tool request"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no new pending request promotion is required"
            } else if candidate_payload_present {
                "candidate payload exists but was not promoted to pending_tool_request"
            } else {
                "promotion cannot happen without a payload candidate"
            },
            pending_request_paths(payload),
        ),
        checkpoint(
            "5a_tool_execution_recorded",
            pending_promoted || tool_attempted || synthesis_only_without_new_candidate,
            tool_attempted || synthesis_only_without_new_candidate,
            if tool_attempted {
                "tool execution evidence is present"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh tool execution evidence is required"
            } else if pending_promoted {
                "tool request is pending but no execution evidence is present yet"
            } else {
                "tool execution cannot be expected before request promotion"
            },
            vec![
                "tools",
                "response_finalization.tool_completion.tool_attempts",
            ],
        ),
        checkpoint(
            "5b_raw_provider_result_present",
            tool_attempted || synthesis_only_without_new_candidate,
            raw_provider_result || synthesis_only_without_new_candidate,
            if raw_provider_result {
                "raw provider/tool result contains substantive rows or snippets"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh raw provider artifact is required"
            } else if tool_attempted {
                "tool executed but no substantive raw provider rows/snippets were found"
            } else {
                "raw provider result cannot be inspected before execution"
            },
            raw_provider_result_paths(payload),
        ),
        checkpoint(
            "5c_packaged_tool_result_present",
            raw_provider_result || tool_attempted || synthesis_only_without_new_candidate,
            packaged_tool_result || synthesis_only_without_new_candidate,
            if packaged_tool_result {
                "raw/provider output was normalized into a substantive packaged tool result"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh packaged tool artifact is required"
            } else if raw_provider_result {
                "raw/provider output exists but no substantive packaged tool result was found"
            } else if tool_attempted {
                "packaging cannot be trusted because raw/provider output is absent or low-signal"
            } else {
                "packaged tool result cannot be inspected before execution"
            },
            packaged_tool_result_paths(payload),
        ),
        checkpoint(
            "5d_evidence_refs_extracted",
            packaged_tool_result || synthesis_only_without_new_candidate,
            evidence_extracted || synthesis_only_without_new_candidate,
            if evidence_extracted {
                "packaged tool result was converted into evidence artifacts or refs"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh evidence extraction artifact is required"
            } else if packaged_tool_result {
                "packaged tool result exists but no evidence artifact/ref was found"
            } else {
                "evidence extraction cannot happen without a packaged tool result"
            },
            evidence_paths(payload),
        ),
        checkpoint(
            "5e_agent_received_evidence_context",
            evidence_extracted || synthesis_only_without_new_candidate,
            agent_received_evidence_context || synthesis_only_without_new_candidate,
            if agent_received_evidence_context {
                "final synthesis turn shows evidence context was available to the agent"
            } else if synthesis_only_without_new_candidate {
                "case expects synthesis from existing or pending tool state, so no fresh agent evidence-context marker is required"
            } else if evidence_extracted {
                "evidence exists but no agent-visible evidence context marker was found"
            } else {
                "agent evidence context cannot be checked before evidence extraction"
            },
            agent_evidence_context_paths(payload),
        ),
        checkpoint(
            "6a_synthesis_uses_evidence_or_low_evidence_fallback",
            final_response_present,
            synthesis_ok,
            if synthesis_ok {
                "final response uses evidence or gives a substantive low-evidence fallback"
            } else if final_response_present {
                "final response does not satisfy evidence-backed or substantive low-evidence synthesis"
            } else {
                "synthesis cannot be checked without a final response"
            },
            vec![
                "response",
                "response_workflow.final_llm_response",
                "response_finalization.tool_completion",
            ],
        ),
        checkpoint(
            "terminal_artifact_present",
            true,
            structured_terminal_present,
            if structured_terminal_present {
                "turn ended with final response, pending request, tool evidence, or structured failure"
            } else {
                "turn has no allowed terminal artifact"
            },
            vec![
                "response",
                "pending_tool_request",
                "tools",
                "response_finalization",
            ],
        ),
    ];
    let first_failed_checkpoint = checkpoints
        .iter()
        .find(|row| row.get("status").and_then(Value::as_str) == Some("fail"))
        .and_then(|row| row.get("checkpoint").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    json!({
        "first_failed_checkpoint": if first_failed_checkpoint.is_empty() {
            Value::Null
        } else {
            Value::String(first_failed_checkpoint.clone())
        },
        "inferred_failure_boundary": failure_boundary(&first_failed_checkpoint),
        "required_gate_4_fields": required_fields,
        "candidate_payload_fields": candidate_fields,
        "final_llm_status": payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        "final_llm_attempt_count": payload
            .pointer("/response_workflow/final_llm_response/attempt_count")
            .and_then(Value::as_u64),
        "final_llm_reject_reason": final_llm_diagnostic_text(
            payload,
            &[
                "/response_workflow/final_llm_response/original_reject_reason",
                "/response_workflow/final_llm_response/diagnostic_reject_reason",
                "/response_workflow/final_llm_response/last_reject_reason",
            ],
            240,
        ),
        "final_llm_reject_excerpt": final_llm_diagnostic_text(
            payload,
            &[
                "/response_workflow/final_llm_response/original_reject_excerpt",
                "/response_workflow/final_llm_response/diagnostic_invalid_excerpt",
                "/response_workflow/final_llm_response/error",
            ],
            500,
        ),
        "suppressed_runtime_visible_fallback_excerpt": final_llm_diagnostic_text(
            payload,
            &[
                "/response_workflow/final_llm_response/suppressed_replacement_response_excerpt",
            ],
            500,
        ),
        "finalization_outcome": response_finalization_outcome(payload),
        "synthesis_failure_class": synthesis_failure_class,
        "synthesis_failure_hardness": synthesis_failure_hardness,
        "post_tool_pipeline": {
            "raw_provider_result_present": raw_provider_result,
            "raw_provider_result_paths": raw_provider_result_paths(payload),
            "packaged_tool_result_present": packaged_tool_result,
            "packaged_tool_result_paths": packaged_tool_result_paths(payload),
            "evidence_extracted": evidence_extracted,
            "evidence_paths": evidence_paths(payload),
            "agent_received_evidence_context": agent_received_evidence_context,
            "agent_evidence_context_paths": agent_evidence_context_paths(payload)
        },
        "checkpoints": checkpoints
    })
}

fn final_llm_diagnostic_text(payload: &Value, pointers: &[&str], max_len: usize) -> Option<String> {
    pointers.iter().find_map(|pointer| {
        let cleaned = clean_text(
            payload
                .pointer(pointer)
                .and_then(Value::as_str)
                .unwrap_or(""),
            max_len,
        );
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    })
}

fn synthesis_failure_class(
    case: &Value,
    payload: &Value,
    packaged_tool_result: bool,
    evidence_extracted: bool,
    agent_received_evidence_context: bool,
    synthesis_ok: bool,
) -> String {
    if synthesis_ok {
        return "none".to_string();
    }
    let response = assistant_text(payload);
    let normalized = normalize_for_compare(&response);
    if normalized.is_empty() {
        return "missing_final_response".to_string();
    }
    if response_uses_internal_runtime_context_as_evidence_diag(&normalized) {
        return "internal_context_used_as_evidence".to_string();
    }
    if !has_tool_execution(payload) {
        if response_acknowledges_missing_tool_context_diag(&normalized) {
            return "missing_tool_context_fallback_insufficient".to_string();
        }
        return "tool_required_but_not_executed".to_string();
    }
    if evidence_extracted && !agent_received_evidence_context {
        return "evidence_context_not_reaching_synthesis".to_string();
    }
    if tool_result_low_signal_diag(payload) {
        if response_requests_more_scope_without_substance_diag(&normalized) {
            return "low_signal_delegated_to_user".to_string();
        }
        if !response_has_low_evidence_signal_diag(&normalized) {
            return "low_signal_not_acknowledged".to_string();
        }
        if required_entity_coverage_diag(case, &normalized) < 0.75 {
            return "low_signal_entity_coverage_gap".to_string();
        }
        if !response_has_research_shape_diag(&normalized) {
            return "low_signal_synthesis_too_thin".to_string();
        }
        return "low_signal_synthesis_contract_miss".to_string();
    }
    if evidence_extracted || packaged_tool_result {
        if !response_has_source_signal_diag(&normalized) {
            return "evidence_present_but_not_used".to_string();
        }
        if required_entity_coverage_diag(case, &normalized) < 0.75 {
            return "evidence_entity_coverage_gap".to_string();
        }
        if !response_has_research_shape_diag(&normalized) {
            return "evidence_present_but_not_synthesized".to_string();
        }
        return "evidence_synthesis_contract_miss".to_string();
    }
    "no_packaged_evidence_for_synthesis".to_string()
}

fn synthesis_failure_hardness(class: &str, first_failure_stage: &str) -> &'static str {
    if first_failure_stage.starts_with('4') || first_failure_stage.starts_with('5') {
        return "hard";
    }
    match class {
        "none" => "none",
        "missing_final_response"
        | "tool_required_but_not_executed"
        | "internal_context_used_as_evidence"
        | "evidence_context_not_reaching_synthesis"
        | "no_packaged_evidence_for_synthesis" => "hard",
        _ => "soft",
    }
}

fn first_failure_stage_hint(payload: &Value) -> String {
    payload
        .pointer("/response_finalization/outcome")
        .and_then(Value::as_str)
        .map(|raw| {
            if raw.contains("missing_tool_attempt") || raw.contains("route_parse_failed") {
                "4e".to_string()
            } else if raw.contains("tool_failure") || raw.contains("low_signal") {
                "6a".to_string()
            } else {
                String::new()
            }
        })
        .unwrap_or_default()
}

fn tool_result_low_signal_diag(payload: &Value) -> bool {
    if tool_result_has_usable_quality_diag(payload) {
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
            .map(text_has_low_signal_only_diag)
            .unwrap_or(false)
        {
            return true;
        }
    }
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().all(tool_row_is_low_signal_diag))
        .unwrap_or(false)
}

fn tool_result_has_usable_quality_diag(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(tool_row_has_usable_quality_diag)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(tool_row_has_usable_quality_diag)
}

fn tool_row_has_usable_quality_diag(row: &Value) -> bool {
    row.get("tool_result_quality")
        .map(tool_quality_value_is_usable_diag)
        .unwrap_or(false)
}

fn tool_quality_value_is_usable_diag(quality: &Value) -> bool {
    let status = normalize_for_compare(&str_at(quality, &["status"], ""));
    if status == "usable" || bool_at(quality, &["usable_evidence"], false) {
        return true;
    }
    let evidence_count = u64_at(quality, &["evidence_count"], 0);
    let content_rich_count = u64_at(quality, &["content_rich_candidate_count"], 0);
    let claim_hint_count = u64_at(quality, &["claim_hint_count"], 0);
    evidence_count > 0 && content_rich_count > 0 && claim_hint_count > 0
}

fn tool_row_is_low_signal_diag(row: &Value) -> bool {
    let status = normalize_for_compare(&str_at(row, &["status"], ""));
    matches!(
        status.as_str(),
        "low_signal"
            | "no_results"
            | "partial_no_results"
            | "error"
            | "failed"
            | "timeout"
            | "blocked"
            | "policy_denied"
    ) || row
        .get("result")
        .and_then(Value::as_str)
        .map(text_has_low_signal_only_diag)
        .unwrap_or(false)
}

fn text_has_low_signal_only_diag(raw: &str) -> bool {
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
        "off target",
        "irrelevant",
        "inconclusive",
        "retrieval missed",
        "retrieval miss",
        "retrieval gap",
        "narrow the query",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_source_signal_diag(normalized: &str) -> bool {
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

fn response_has_low_evidence_signal_diag(normalized: &str) -> bool {
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
        "off target",
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

fn response_has_research_shape_diag(normalized: &str) -> bool {
    normalized.split_whitespace().count() >= 40
        && [
            "tradeoff",
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
            "what is known",
            "what is not known",
        ]
        .iter()
        .any(|needle| normalized.contains(*needle))
}

fn response_requests_more_scope_without_substance_diag(normalized: &str) -> bool {
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
        && (response_has_research_shape_diag(normalized)
            || response_has_low_evidence_signal_diag(normalized)
            || normalized.contains("supports")
            || normalized.contains("does not support")
            || normalized.contains("bounded"));
    !has_bounded_substance
}

fn response_acknowledges_missing_tool_context_diag(normalized: &str) -> bool {
    [
        "no live web data",
        "no returned tool result",
        "tool result is not present in this turn",
        "tool result is not available in this turn",
        "no retrieved snippets",
        "no retrieved results",
        "no recorded tool outcome",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_uses_internal_runtime_context_as_evidence_diag(normalized: &str) -> bool {
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

fn required_entity_coverage_diag(case: &Value, normalized_response: &str) -> f64 {
    let entities = string_array_at(case, &["required_entities"]);
    let prompt = normalize_for_compare(&str_at(case, &["prompt"], ""));
    let user_stated_entities = entities
        .iter()
        .filter(|entity| normalized_response_covers_entity_diag(&prompt, entity))
        .collect::<Vec<_>>();
    if user_stated_entities.is_empty() {
        return 1.0;
    }
    let covered = user_stated_entities
        .iter()
        .filter(|entity| normalized_response_covers_entity_diag(normalized_response, entity))
        .count() as u64;
    ratio(covered, user_stated_entities.len() as u64)
}

fn normalized_response_covers_entity_diag(normalized_response: &str, entity: &str) -> bool {
    let normalized_entity = normalize_for_compare(entity);
    if normalized_entity.is_empty() {
        return false;
    }
    if normalized_response.contains(&normalized_entity) {
        return true;
    }
    if normalized_response.contains(&simple_plural_variant_diag(&normalized_entity))
        || normalized_response.contains(&simple_singular_variant_diag(&normalized_entity))
    {
        return true;
    }
    let tokens = normalized_entity
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| token_or_simple_variant_present_diag(normalized_response, token))
}

fn token_or_simple_variant_present_diag(normalized_response: &str, token: &str) -> bool {
    normalized_response.contains(token)
        || normalized_response.contains(&simple_plural_variant_diag(token))
        || normalized_response.contains(&simple_singular_variant_diag(token))
}

fn simple_plural_variant_diag(value: &str) -> String {
    if value.ends_with('s') {
        value.to_string()
    } else {
        format!("{value}s")
    }
}

fn simple_singular_variant_diag(value: &str) -> String {
    value.strip_suffix('s').unwrap_or(value).to_string()
}

pub(super) fn gate_transition_rate_rows(
    total_counts: &BTreeMap<String, u64>,
    pass_counts: &BTreeMap<String, u64>,
) -> Vec<Value> {
    total_counts
        .iter()
        .map(|(checkpoint, total)| {
            let passed = *pass_counts.get(checkpoint).unwrap_or(&0);
            json!({
                "checkpoint": checkpoint,
                "passed": passed,
                "total": total,
                "pass_rate": ratio(passed, *total)
            })
        })
        .collect()
}

fn checkpoint(
    name: &str,
    artifact_present: bool,
    passed: bool,
    reason: &str,
    artifact_refs: Vec<impl Into<String>>,
) -> Value {
    json!({
        "checkpoint": name,
        "status": if passed { "pass" } else { "fail" },
        "artifact_present": artifact_present,
        "reason": reason,
        "artifact_refs": artifact_refs
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>()
    })
}

fn case_allows_existing_tool_state_without_new_candidate(case: &Value) -> bool {
    let gate_1 = normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_1"], ""));
    let post_tool = normalize_for_compare(&str_at(case, &["expected_gate_path", "post_tool"], ""));
    gate_1.contains("pending_tool_result") || post_tool.starts_with("must_synthesize_from")
}

pub(super) fn failure_boundary(first_failed_checkpoint: &str) -> &'static str {
    match first_failed_checkpoint {
        "" => "no_failure_detected",
        "4a_request_template_signaled" => "gate_4_template_not_visible",
        "4b_tool_request_candidate_present" => "gate_4_candidate_not_emitted",
        "4c_candidate_payload_object" => "gate_4_candidate_parse_failed",
        "4d_candidate_schema_fields_present" => "gate_4_schema_mismatch",
        "4e_pending_request_promoted" => "request_candidate_not_promoted",
        "5a_tool_execution_recorded" => "pending_request_not_executed",
        "5b_raw_provider_result_present" => "raw_provider_result_absent_or_low_signal",
        "5c_packaged_tool_result_present" => "tool_result_packaging_missing_or_low_signal",
        "5d_evidence_refs_extracted" => "packaged_result_not_extracted_to_evidence",
        "5e_agent_received_evidence_context" => "evidence_context_not_reaching_agent",
        "6a_synthesis_uses_evidence_or_low_evidence_fallback" => "post_tool_synthesis_not_useful",
        "terminal_artifact_present" => "empty_terminal_projection",
        _ => "unknown_gate_transition_failure",
    }
}

fn pending_tool_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

fn latent_tool_candidate(payload: &Value) -> Option<&Value> {
    payload
        .get("latent_tool_candidates")
        .and_then(Value::as_array)
        .and_then(|rows| rows.iter().find(|row| row.is_object()))
}

fn response_workflow_candidate(payload: &Value) -> Option<&Value> {
    payload
        .pointer("/response_workflow/tool_request_candidate")
        .or_else(|| payload.pointer("/response_workflow/gate_4_tool_request_candidate"))
        .or_else(|| payload.pointer("/response_workflow/request_payload_candidate"))
}

fn candidate_input_object(candidate: &Value) -> Option<&Value> {
    candidate
        .get("input")
        .or_else(|| candidate.get("request_payload"))
        .or_else(|| candidate.get("payload"))
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

fn response_finalization_outcome(payload: &Value) -> Option<String> {
    payload
        .pointer("/response_finalization/outcome")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 600))
}

fn pending_request_paths(payload: &Value) -> Vec<String> {
    [
        ("pending_tool_request", payload.get("pending_tool_request")),
        (
            "response_workflow.pending_tool_request",
            payload.pointer("/response_workflow/pending_tool_request"),
        ),
        (
            "response_workflow.manual_toolbox_pending_tool_request",
            payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"),
        ),
        (
            "response_finalization.pending_tool_request",
            payload.pointer("/response_finalization/pending_tool_request"),
        ),
    ]
    .iter()
    .filter_map(|(path, value)| value.is_some().then(|| (*path).to_string()))
    .collect()
}

fn candidate_paths(payload: &Value) -> Vec<String> {
    let mut paths = pending_request_paths(payload);
    if payload
        .get("latent_tool_candidates")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
    {
        paths.push("latent_tool_candidates".to_string());
    }
    for (path, value) in [
        (
            "response_workflow.tool_request_candidate",
            payload.pointer("/response_workflow/tool_request_candidate"),
        ),
        (
            "response_workflow.gate_4_tool_request_candidate",
            payload.pointer("/response_workflow/gate_4_tool_request_candidate"),
        ),
        (
            "response_workflow.request_payload_candidate",
            payload.pointer("/response_workflow/request_payload_candidate"),
        ),
    ] {
        if value.is_some() {
            paths.push(path.to_string());
        }
    }
    paths
}
