fn strip_no_tool_gate_prefix_from_visible_response(response: &str) -> String {
    let cleaned = clean_text(response, 8_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    if response_is_no_tool_category_gate_submission(&cleaned)
        || response_is_tool_bearing_category_gate_submission(&cleaned)
    {
        return String::new();
    }
    cleaned
}

fn tool_payload_shape_looks_raw(response_payload: &Value) -> bool {
    match response_payload {
        Value::Object(map) => {
            let visible_fields = [
                "answer",
                "final_answer",
                "visible_response",
                "final_response",
                "message",
                "text",
                "response",
                "content",
                "output",
            ];
            let has_visible = map
                .keys()
                .any(|key| visible_fields.iter().any(|visible| key == visible));
            let toolish_fields = [
                "tool",
                "tool_name",
                "name",
                "query",
                "input",
                "arguments",
                "args",
                "payload",
                "params",
                "parameters",
                "request_payload",
                "result",
                "status",
                "error",
                "blocked",
                "is_error",
                "source",
                "findings",
                "results",
                "tool_attempt_receipt",
                "tool_call",
                "tool_call_id",
                "tool_receipt",
            ];
            let toolish_count = toolish_fields
                .iter()
                .filter(|field| map.contains_key(**field))
                .count();
            if map.contains_key("tool_attempt_receipt") {
                return true;
            }
            if let Some(tool_value) = map.get("tool").and_then(Value::as_str) {
                if !tool_value.trim().is_empty()
                    && (map.contains_key("input")
                        || map.contains_key("arguments")
                        || map.contains_key("query")
                        || map.contains_key("result")
                        || map.contains_key("request_payload"))
                {
                    return true;
                }
            }
            if map.contains_key("tool_call")
                && (map.contains_key("tool")
                    || map.contains_key("tool_name")
                    || map.contains_key("name"))
            {
                return true;
            }
            if map.contains_key("tool_call_id")
                && (map.contains_key("status") || map.contains_key("result"))
            {
                return true;
            }
            if map.contains_key("name")
                && (map.contains_key("result")
                    || map.contains_key("status")
                    || map.contains_key("request_payload")
                    || map.contains_key("query")
                    || map.contains_key("input"))
            {
                return true;
            }
            if map.contains_key("query") && map.contains_key("source") && !has_visible {
                return true;
            }
            if map.contains_key("findings")
                && (map.contains_key("sources")
                    || map.contains_key("results")
                    || map.contains_key("tool"))
                && !has_visible
            {
                return true;
            }
            (toolish_count >= 2 && !has_visible) || (toolish_count >= 3)
        }
        Value::Array(rows) => rows.iter().any(|entry| tool_payload_shape_looks_raw(entry)),
        _ => false,
    }
}

fn sanitize_workflow_visible_response_text(response_text: &str) -> String {
    let cleaned = strip_trailing_research_follow_up_offer(&strip_internal_evidence_posture_prefix(
        &sanitize_workflow_final_response_candidate(&strip_internal_cache_control_markup(
            &strip_internal_context_metadata_prefix(response_text),
        )),
    ));
    if response_looks_like_raw_tool_payload_dump(&cleaned) {
        String::new()
    } else {
        cleaned
    }
}

fn strip_internal_evidence_posture_prefix(response_text: &str) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    let trimmed = cleaned
        .trim_start()
        .trim_start_matches(|ch: char| ch == '*' || ch == '`' || ch == '_' || ch.is_whitespace());
    for posture in [
        "supported_answer",
        "bounded_partial_answer",
        "evidence_insufficient_answer",
    ] {
        let Some(after_posture) = trimmed.strip_prefix(posture) else {
            continue;
        };
        let after_posture = after_posture.trim_start_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, ':' | '-' | '.' | ';' | '*' | '`' | '_')
        });
        return clean_chat_text(after_posture.trim_start(), 32_000);
    }
    strip_internal_evidence_posture_disclosure(&cleaned)
}

fn strip_internal_evidence_posture_disclosure(response_text: &str) -> String {
    let mut cleaned = clean_chat_text(response_text, 32_000);
    for posture in [
        "supported_answer",
        "bounded_partial_answer",
        "evidence_insufficient_answer",
    ] {
        for marker in [
            format!("**Posture: `{posture}`** — "),
            format!("**Posture: `{posture}`** - "),
            format!("**Posture: `{posture}`** "),
            format!("**Posture:** `{posture}` — "),
            format!("**Posture:** `{posture}` - "),
            format!("**Posture:** `{posture}` "),
            format!("Posture: `{posture}` — "),
            format!("Posture: `{posture}` - "),
            format!("Posture: `{posture}` "),
            format!("Posture: {posture} — "),
            format!("Posture: {posture} - "),
            format!("Posture: {posture} "),
            format!("**Outcome posture: `{posture}`** — "),
            format!("**Outcome posture: `{posture}`** - "),
            format!("**Outcome posture: `{posture}`** "),
            format!("**Outcome posture:** `{posture}` — "),
            format!("**Outcome posture:** `{posture}` - "),
            format!("**Outcome posture:** `{posture}` "),
            format!("Outcome posture: `{posture}` — "),
            format!("Outcome posture: `{posture}` - "),
            format!("Outcome posture: `{posture}` "),
            format!("Outcome posture: {posture} — "),
            format!("Outcome posture: {posture} - "),
            format!("Outcome posture: {posture} "),
            format!("**Outcome posture: {posture}** "),
            format!("`{posture}` — "),
            format!("`{posture}` - "),
        ] {
            cleaned = cleaned.replace(&marker, "");
        }
    }
    cleaned
}

fn strip_trailing_research_follow_up_offer(response_text: &str) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    if cleaned.is_empty() || cleaned.split_whitespace().count() < 40 {
        return cleaned;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let looks_like_substantive_research_answer = lowered.contains("tradeoff")
        || lowered.contains("evidence")
        || lowered.contains("source-backed")
        || lowered.contains("current evidence")
        || lowered.contains("receipt-backed")
        || lowered.contains("production")
        || lowered.contains("benchmark");
    if !looks_like_substantive_research_answer {
        return cleaned;
    }

    let trailing_offer_markers = [
        "\nif you want",
        "\nif you'd like",
        "\nif you would like",
        "\nwould you prefer",
        "\nwould you like",
        "\nis there a specific",
        "\nwhich angle matters more",
        "\nwhich framework pair",
        "\nwhat task domain",
        "\ni can narrow",
        "\ni can retry",
        "\ni can rerun",
        " if you want",
        " if you'd like",
        " if you would like",
        " would you prefer",
        " would you like",
        " is there a specific",
        " which angle matters more",
        " which framework pair",
        " what task domain",
    ];
    let offer_start = trailing_offer_markers
        .iter()
        .filter_map(|marker| lowered.rfind(marker))
        .filter(|idx| *idx >= cleaned.len() / 2)
        .min();
    let Some(offer_start) = offer_start else {
        return cleaned;
    };

    let trimmed = cleaned[..offer_start].trim();
    if trimmed.split_whitespace().count() < 35 {
        return cleaned;
    }
    if matches!(
        trimmed.chars().last(),
        Some('.') | Some('!') | Some('?') | Some(':')
    ) {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

fn workflow_final_visible_response_text(response_text: &str) -> String {
    let sanitized = sanitize_workflow_visible_response_text(response_text);
    if let Some(unwrapped) = normalize_response_field_json_wrapper(&sanitized) {
        return clean_chat_text(unwrapped.as_str(), 32_000);
    }
    sanitized
}

fn response_looks_like_raw_tool_payload_dump(response_text: &str) -> bool {
    let cleaned = clean_text(response_text, 2_000);
    let lowered = cleaned.to_ascii_lowercase();
    lowered.starts_with("<?xml")
        || lowered.starts_with("<!doctype")
        || lowered.starts_with("<html")
        || lowered.contains("<custommetadata")
        || lowered.contains("xmlns=")
        || lowered.contains("<function=")
        || lowered.contains("<tool")
        || lowered.contains("</tool>")
        || response_contains_provider_completion_dump(&cleaned)
        || parse_json_payload_dump(&cleaned)
            .is_some_and(|payload| tool_payload_shape_looks_raw(&payload))
        || looks_like_tool_payload_json_literal(&cleaned)
}

fn response_contains_provider_completion_dump(response_text: &str) -> bool {
    let lowered = clean_text(response_text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let markers = [
        "\"finish_reason\"",
        "\"prompt_tokens\"",
        "\"completion_tokens\"",
        "\"total_tokens\"",
        "\"logprob\"",
        "\"usage\":{",
        "\"prediction\"",
        "\"refusal\"",
        "i am kimi, an ai assistant created by moonshot ai",
    ];
    markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count()
        >= 3
}

fn looks_like_tool_payload_json_literal(response_text: &str) -> bool {
    let compact = response_text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    if compact.is_empty() {
        return false;
    }
    if !(compact.starts_with('{') || compact.starts_with('[')) {
        return false;
    }
    let compact_lower = compact.to_ascii_lowercase();
    let raw_signals = [
        "\"tool\"",
        "\"tool_name\"",
        "\"name\"",
        "\"query\"",
        "\"input\"",
        "\"arguments\"",
        "\"args\"",
        "\"params\"",
        "\"parameters\"",
        "\"result\"",
        "\"status\"",
        "\"error\"",
        "\"tool_attempt_receipt\"",
        "\"receipt\"",
        "\"toolcall\"",
        "\"request_payload\"",
        "\"findings\"",
        "\"sources\"",
        "\"results\"",
        "\"blocked\"",
        "\"is_error\"",
        "\"tool_call\"",
        "\"tool_call_id\"",
    ];
    let signal_hits = raw_signals
        .iter()
        .filter(|signal| compact_lower.contains(*signal))
        .count();
    signal_hits >= 3 || {
        let has_tool_token =
            compact_lower.contains("\"tool\"") || compact_lower.contains("\"tool_name\"");
        let has_name_token =
            compact_lower.contains("\"name\"") || compact_lower.contains("\"tool_name\"");
        let has_payload_token = compact_lower.contains("\"request_payload\"")
            || compact_lower.contains("\"input\"")
            || compact_lower.contains("\"query\"")
            || compact_lower.contains("\"arguments\"")
            || compact_lower.contains("\"args\"");
        has_tool_token && has_name_token && has_payload_token
    }
}

#[cfg(test)]
mod visible_response_sanitizer_tests {
    use super::*;

    #[test]
    fn sanitizer_strips_trailing_research_follow_up_offer_from_substantive_answer() {
        let response = "The main tradeoff is between benchmark performance and production maintainability for open-source coding agents. The evidence is still partial, but Aider looks strongest for real repositories while OpenHands appears more exploratory for broader agent loops. My recommendation is to start with Aider for real repository work and treat OpenHands as a secondary evaluation track. If you want, I can narrow this to SWE-bench-style evidence next.";
        let cleaned = sanitize_workflow_visible_response_text(response);
        assert!(cleaned.contains("My recommendation is to start with Aider"));
        assert!(!cleaned
            .to_ascii_lowercase()
            .contains("if you want, i can narrow"));
    }

    #[test]
    fn sanitizer_strips_internal_evidence_posture_prefix_from_visible_answer() {
        let response =
            "bounded_partial_answer Here is the useful answer from the recorded evidence.";
        assert_eq!(
            sanitize_workflow_visible_response_text(response),
            "Here is the useful answer from the recorded evidence."
        );
    }

    #[test]
    fn sanitizer_strips_markdown_internal_evidence_posture_prefix_from_visible_answer() {
        let response =
            "**bounded_partial_answer** Based on the retrieved evidence, here is the useful answer.";
        assert_eq!(
            sanitize_workflow_visible_response_text(response),
            "Based on the retrieved evidence, here is the useful answer."
        );
    }

    #[test]
    fn sanitizer_strips_inline_internal_evidence_posture_disclosure() {
        let response = "Here is the answer. **Posture: `bounded_partial_answer`** — I have some relevant signals, but gaps remain.";
        assert_eq!(
            sanitize_workflow_visible_response_text(response),
            "Here is the answer. I have some relevant signals, but gaps remain."
        );
    }

    #[test]
    fn sanitizer_strips_outcome_posture_disclosure() {
        let response = "**Outcome posture: bounded_partial_answer** Here is the answer grounded in the retrieved evidence.";
        assert_eq!(
            sanitize_workflow_visible_response_text(response),
            "Here is the answer grounded in the retrieved evidence."
        );
    }

    #[test]
    fn sanitizer_keeps_plain_short_follow_up_question_when_no_substantive_answer_exists() {
        let response = "Would you prefer a narrower query?";
        assert_eq!(
            sanitize_workflow_visible_response_text(response),
            "Would you prefer a narrower query?"
        );
    }

    #[test]
    fn sanitizer_strips_trailing_research_follow_up_question_from_substantive_answer() {
        let response = "I wasn't able to run the benchmark search successfully. The main tradeoff in agent frameworks is between ease of orchestration and deep tool integration. CrewAI is easier to compose for role-based workflows, while LangGraph offers more control for stateful tool loops. For a practical evaluation plan, compare task success rate, latency, observability, and integration depth on one real workflow. Is there a specific framework pair or task domain you're trying to evaluate?";
        let cleaned = sanitize_workflow_visible_response_text(response);
        assert!(cleaned.contains("For a practical evaluation plan"));
        assert!(!cleaned
            .to_ascii_lowercase()
            .contains("is there a specific framework pair"));
        assert!(cleaned.ends_with('.'));
    }
}
