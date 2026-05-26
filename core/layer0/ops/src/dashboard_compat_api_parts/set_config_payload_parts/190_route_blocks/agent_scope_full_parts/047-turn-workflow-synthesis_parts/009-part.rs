fn response_looks_truncated_or_incomplete_for_verifier(response_text: &str) -> bool {
    let trimmed = clean_chat_text(response_text, 32_000);
    if trimmed.is_empty() {
        return false;
    }
    let tail = trimmed
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(trimmed.as_str());
    let terminal_punctuation = tail
        .chars()
        .rev()
        .find(|ch| !ch.is_ascii_whitespace())
        .map(|ch| matches!(ch, '.' | '!' | '?' | ')' | ']' | '"' | '\''))
        .unwrap_or(false)
        || tail.ends_with("```")
        || tail.ends_with('|');
    let table_tail_incomplete = tail.contains('|') && !terminal_punctuation;
    let open_parens = tail.matches('(').count() > tail.matches(')').count();
    let open_brackets = tail.matches('[').count() > tail.matches(']').count();
    let dangling_connector = clean_text(tail, 300)
        .to_ascii_lowercase()
        .split_whitespace()
        .last()
        .map(|last| {
            matches!(
                last,
                "and" | "or" | "with" | "for" | "from" | "because" | "while" | "including"
            )
        })
        .unwrap_or(false);
    (table_tail_incomplete || open_parens || open_brackets || dangling_connector)
        && !terminal_punctuation
}

fn response_looks_like_retrieval_recap_substituted_for_answer(response_text: &str) -> bool {
    let normalized = clean_text(response_text, 8_000).to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    let first = first_sentence(&normalized, 700);
    let trace_marker_count = [
        "recorded evidence so far",
        "here's what i found",
        "here s what i found",
        "web search:",
        "from web retrieval",
        "tool trace complete",
        "search surfaced",
        "retrieval state",
        "coverage is fragmented",
        "provider timeouts",
        "provider starvation",
        "results are incomplete",
    ]
    .iter()
    .filter(|marker| normalized.contains(**marker))
    .count();
    let opens_as_status_or_inventory = [
        "the safest bounded answer",
        "i found some",
        "here's what",
        "here s what",
        "the current retrieval",
        "recorded evidence",
        "web search",
    ]
    .iter()
    .any(|marker| first.contains(*marker));
    trace_marker_count >= 2 || (trace_marker_count >= 1 && opens_as_status_or_inventory)
}

fn workflow_final_answer_explicitly_refuses_unsupported_recommendation(normalized: &str) -> bool {
    [
        "not enough to recommend",
        "cannot recommend",
        "can't recommend",
        "no source backed basis to choose",
        "no source-backed basis to choose",
        "no source backed basis to recommend",
        "no source-backed basis to recommend",
        "do not use this as a recommendation",
        "should not be used as a recommendation",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn response_has_public_source_signal(response_text: &str) -> bool {
    let normalized = clean_text(response_text, 8_000).to_ascii_lowercase();
    [
        "http://",
        "https://",
        "source:",
        "sources:",
        "citation",
        "citations",
        "according to",
        "recorded source",
        "recorded evidence",
        "retrieved evidence",
        "source supports",
        "evidence supports",
        "source-backed",
        "the docs",
        "official docs",
        "release notes",
        "changelog",
        "paper",
        "study",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
        || response_text_contains_domain_like_source_marker(&normalized)
}

fn response_text_contains_domain_like_source_marker(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let cleaned = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '/' && ch != ':' && ch != '-'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = cleaned
            .split('/')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '-')
            .collect::<String>();
        let labels = host
            .split('.')
            .filter(|label| !label.is_empty())
            .collect::<Vec<_>>();
        if labels.len() < 2 {
            return false;
        }
        let tld = labels.last().copied().unwrap_or("");
        if !(2..=24).contains(&tld.len()) || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return false;
        }
        labels
            .iter()
            .any(|label| label.chars().any(|ch| ch.is_ascii_alphabetic()))
    })
}

fn primary_query_texts_for_coverage_tool(tool: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for key in ["query", "primary_query", "user_goal", "prompt", "question"] {
        let value = clean_text(tool.get(key).and_then(Value::as_str).unwrap_or(""), 1_200);
        if !value.is_empty() {
            out.push(value);
        }
    }
    let raw_input = clean_text(
        tool.get("input").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    if raw_input.is_empty() {
        return out;
    }
    match serde_json::from_str::<Value>(&raw_input) {
        Ok(Value::Object(map)) => {
            for key in ["query", "primary_query", "user_goal", "prompt", "question"] {
                let value = clean_text(map.get(key).and_then(Value::as_str).unwrap_or(""), 1_200);
                if !value.is_empty() {
                    out.push(value);
                }
            }
        }
        _ => out.push(raw_input),
    }
    out
}

fn coverage_lane_should_be_hard_required(requested: &str, response_tools: &[Value]) -> bool {
    let mut saw_primary_query = false;
    for tool in response_tools {
        for query in primary_query_texts_for_coverage_tool(tool) {
            let normalized_query = normalize_coverage_lane_text(&query);
            if normalized_query.is_empty() {
                continue;
            }
            saw_primary_query = true;
            if normalized_response_covers_coverage_lane(&normalized_query, requested) {
                return true;
            }
        }
    }
    !saw_primary_query
}

fn response_missing_required_entity_lanes(
    response_text: &str,
    response_tools: &[Value],
) -> Vec<String> {
    let normalized_response = normalize_coverage_lane_text(response_text);
    if normalized_response.is_empty() {
        return Vec::new();
    }
    let mut missing = Vec::<String>::new();
    for lane in synthesis_coverage_lanes_for_tools(response_tools, 24) {
        let kind = clean_text(lane.get("kind").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        if kind != "entity" {
            continue;
        }
        let requested = clean_text(
            lane.get("requested_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if requested.is_empty()
            || missing
                .iter()
                .any(|row| row.eq_ignore_ascii_case(&requested))
        {
            continue;
        }
        if !coverage_lane_should_be_hard_required(&requested, response_tools) {
            continue;
        }
        if !normalized_response_covers_coverage_lane(&normalized_response, &requested) {
            missing.push(requested);
        }
    }
    missing.into_iter().take(8).collect()
}

fn normalized_response_covers_coverage_lane(normalized_response: &str, lane: &str) -> bool {
    let normalized_lane = normalize_coverage_lane_text(lane);
    if normalized_lane.is_empty() {
        return false;
    }
    if normalized_response.contains(&normalized_lane)
        || normalized_response.contains(&simple_coverage_plural_variant(&normalized_lane))
        || normalized_response.contains(&simple_coverage_singular_variant(&normalized_lane))
    {
        return true;
    }
    let tokens = normalized_lane
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| coverage_token_or_simple_variant_present(normalized_response, token))
}

fn coverage_token_or_simple_variant_present(normalized_response: &str, token: &str) -> bool {
    normalized_response.contains(token)
        || normalized_response.contains(&simple_coverage_plural_variant(token))
        || normalized_response.contains(&simple_coverage_singular_variant(token))
}

fn simple_coverage_plural_variant(value: &str) -> String {
    if value.ends_with('s') {
        value.to_string()
    } else {
        format!("{value}s")
    }
}

fn simple_coverage_singular_variant(value: &str) -> String {
    value.strip_suffix('s').unwrap_or(value).to_string()
}

fn normalize_coverage_lane_text(value: &str) -> String {
    clean_text(value, 4_000)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn workflow_final_synthesis_retry_prompt_context(
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
) -> String {
    if last_reject_reason.trim().is_empty() {
        return String::new();
    }
    clean_text(
        &format!(
            "Internal final-response verifier retry. The previous candidate failed `{}`. Previous excerpt: {}. Produce the user-facing answer from the same recorded evidence and user goal. The visible answer must still stand on its own and be useful if formatting is stripped away; any presentation shape is fine if it fits the query, but finish the structure you start. Lead with the best bounded answer the evidence supports, then state limits or gaps. If the failure names missing coverage lanes, cover each named lane or explicitly mark its evidence as weak or missing. If the failure names missing citation/source signal, preserve compact source grounding for claims supported by recorded evidence, using whatever natural wording fits the answer. If the failure names answer_units_not_traceable_to_evidence, keep concrete claim wording closer to recorded claim_hints, relevant_extract, and source titles; remove unsupported category labels, exact dates, ages, numbers, or other modifiers unless they are present in the recorded evidence, or clearly mark them as inference. If the failure names retrieval recap or the previous candidate opened by reporting tool/search/retrieval status, convert EvidencePacket claim_hints/relevant_extract/source refs into answer units instead of listing sources or tool status. Do not mention this verifier, workflow gates, tool traces, internal outcome posture, or a required output format.",
            clean_text(last_reject_reason, 120),
            clean_text(last_invalid_excerpt, 240)
        ),
        1_000,
    )
}

fn mark_workflow_pending_gate_without_final_synthesis(
    workflow: &mut Value,
    status: &str,
    diagnostic_source: &str,
    gate_attempt_count: u64,
) {
    let visible_response_preserved = workflow
        .get("response")
        .and_then(Value::as_str)
        .map(|raw| !clean_text(raw, 1_000).is_empty())
        .unwrap_or(false);
    workflow["final_llm_response"]["required"] = Value::Bool(false);
    workflow["final_llm_response"]["attempted"] = Value::Bool(false);
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    workflow["final_llm_response"]["attempt_count"] = json!(0);
    workflow["final_llm_response"]["gate_attempt_count"] = json!(gate_attempt_count);
    workflow["final_llm_response"]["status"] = Value::String(clean_text(status, 80));
    workflow["final_llm_response"]["diagnostic_source"] =
        Value::String(clean_text(diagnostic_source, 120));
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] =
        Value::Bool(visible_response_preserved);
    set_turn_workflow_final_stage_status(workflow, status);
}

fn workflow_final_synthesis_attempt_limit(workflow: &Value, response_tools: &[Value]) -> u64 {
    if response_tools.is_empty() {
        return 1;
    }
    workflow
        .pointer("/selected_workflow/tool_menu_interface_contract/final_synthesis_attempt_limit")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 3)
}

