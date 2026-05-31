fn passive_attention_context_for_message(
    root: &Path,
    agent_id: &str,
    message: &str,
    max_items: usize,
) -> String {
    let path = attention_queue_path_for_dashboard(root);
    if !path.exists() {
        return String::new();
    }
    let message_terms = important_memory_terms(message, 20)
        .into_iter()
        .collect::<HashSet<_>>();
    let mut related = Vec::<String>::new();
    for row in read_jsonl_loose(&path, 1200) {
        let source = clean_text(row.get("source").and_then(Value::as_str).unwrap_or(""), 180);
        if source != format!("agent:{agent_id}") {
            continue;
        }
        let source_type = clean_text(
            row.get("source_type").and_then(Value::as_str).unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if source_type != "passive_memory_turn" {
            continue;
        }
        let summary = clean_text(
            row.get("summary").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        if summary.is_empty() {
            continue;
        }
        if internal_context_metadata_phrase(&summary)
            || persistent_memory_denied_phrase(&summary)
            || runtime_access_denied_phrase(&summary)
            || contains_deprecated_workflow_ghost_phrase(&summary)
        {
            continue;
        }
        let assistant_text = clean_text(
            row.pointer("/raw_event/assistant_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1_200,
        );
        if contains_deprecated_workflow_ghost_phrase(&assistant_text)
            || response_contains_cross_project_assimilation_bleed(message, &assistant_text)
        {
            continue;
        }
        let terms = row
            .pointer("/raw_event/terms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| {
                value
                    .as_str()
                    .map(|raw| clean_text(raw, 120).to_ascii_lowercase())
            })
            .filter(|term| !term.is_empty())
            .collect::<HashSet<_>>();
        let effective_terms = if terms.is_empty() {
            important_memory_terms(&summary, 16)
                .into_iter()
                .collect::<HashSet<_>>()
        } else {
            terms
        };
        if !message_terms.is_empty() {
            if effective_terms.is_empty() || message_terms.is_disjoint(&effective_terms) {
                continue;
            }
        }
        if !related.iter().any(|item| item == &summary) {
            related.push(summary);
        }
        if related.len() >= max_items.max(1) {
            break;
        }
    }
    if related.is_empty() {
        String::new()
    } else {
        format!(
            "Relevant passive memory cues:\n{}",
            related
                .iter()
                .map(|row| format!("- {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn response_contains_project_dump_sections(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let markers = ["project overview", "data source", "tools used", "key features", "sql queries", "future work", "how to use"];
    let hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn response_contains_short_unrelated_project_title(user_message: &str, response_text: &str) -> bool {
    let cleaned = clean_chat_text(response_text, 1_000);
    if cleaned.len() < 8 || cleaned.len() > 220 {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let looks_like_project_artifact = cleaned
        .lines()
        .take(3)
        .any(|line| line.trim_start().starts_with("# "))
        || lowered.contains("project-")
        || lowered.contains("project 2 for")
        || lowered.contains("itmd-")
        || lowered.contains("readme.md");
    if !looks_like_project_artifact {
        return false;
    }
    let user_lowered = clean_text(user_message, 600).to_ascii_lowercase();
    if [
        "project",
        "readme",
        "assignment",
        "itmd",
        "repository title",
        "repo title",
    ]
    .iter()
    .any(|needle| user_lowered.contains(*needle))
    {
        return false;
    }
    let user_terms = important_memory_terms(user_message, 20)
        .into_iter()
        .collect::<HashSet<_>>();
    let response_terms = important_memory_terms(&cleaned, 20)
        .into_iter()
        .collect::<HashSet<_>>();
    user_terms.is_empty() || response_terms.is_empty() || user_terms.is_disjoint(&response_terms)
}

fn response_contains_internal_prompt_dump(lowered: &str) -> bool {
    lowered.contains("you are the currently selected infring agent instance")
        || (lowered.contains("<instructions>") && lowered.contains("manual toolbox gate"))
        || lowered.contains("you are the llm and you author the visible chat text")
        || lowered.contains("available workflow/tool candidates")
        || lowered.contains("<output_format type=\"json\">")
        || lowered.contains("do not expose raw gate choices")
        || lowered.contains("hardcoded agent workflow: you are writing the final assistant response")
        || (lowered.contains("use tool output as context and synthesize a direct answer")
            && lowered.contains("never output capability-denial claims"))
        || (lowered.contains("recorded tool outcomes")
            && lowered.contains("workflow events")
            && lowered.contains("write the final assistant response now"))
}

fn response_contains_tool_telemetry_dump(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    if response_contains_internal_prompt_dump(&lowered) {
        return true;
    }
    let noisy_markers = ["at duckduckgo all regions", "duckduckgo all regions", "all regions argentina", "all regions australia", "spawn_subagents failed:", "tool_explicit_signoff_required", "tool_confirmation_required", "\"decision_audit_receipt\"", "\"turn_loop_tracking\"", "\"turn_transaction\"", "\"response_finalization\"", "\"latent_tool_candidates\"", "\"workspace_hints\"", "\"nexus_connection\""];
    let hits = noisy_markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn parse_json_payload_dump(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut candidate = trimmed.to_string();
    if candidate.starts_with("```") && candidate.ends_with("```") {
        candidate = candidate
            .trim_start_matches("```json")
            .trim_start_matches("```JSON")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(&candidate) {
        return Some(parsed);
    }
    let start = candidate.find('{')?;
    let end = candidate.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<Value>(&candidate[start..=end]).ok()
}

fn looks_like_internal_agent_payload_dump(payload: &Value) -> bool {
    let Value::Object(map) = payload else {
        return false;
    };
    let marker_keys = ["agent_id", "decision_audit_receipt", "response_finalization", "turn_loop_tracking", "turn_transaction", "tools", "nexus_connection", "latent_tool_candidates", "workspace_hints", "input_tokens", "output_tokens", "runtime_model", "provider"];
    let hits = marker_keys
        .iter()
        .filter(|key| map.contains_key(**key))
        .count();
    hits >= 3 || (map.contains_key("tools") && map.contains_key("turn_transaction"))
}

fn normalize_raw_response_payload_dump(text: &str) -> Option<String> {
    let payload = parse_json_payload_dump(text)?;
    if !looks_like_internal_agent_payload_dump(&payload) {
        return None;
    }
    let synthesized = clean_text(
        payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        32_000,
    );
    if !synthesized.is_empty() {
        return Some(synthesized);
    }
    let _tools = payload.get("tools").and_then(Value::as_array).cloned().unwrap_or_default();
    None
}

fn response_field_from_json_wrapper(payload: &Value) -> Option<String> {
    let Value::Object(map) = payload else {
        return None;
    };
    let output_keys = [
        "response",
        "text",
        "answer",
        "message",
        "content",
        "output",
        "final_answer",
    ];
    let output_key = output_keys
        .iter()
        .find(|key| map.contains_key(**key))?;
    // Gate-submission guard: if every non-output key is a gate/tool routing key, this is a gate
    // submission that happens to have an output-key field — don't extract it.
    // Extra semantic keys (e.g. "substantive_response", "engagement_prompt") are allowed so that
    // structured LLM responses have their primary field extracted rather than leaking full JSON.
    let gate_routing_keys = [
        "gate", "category", "tool", "tool_family", "tool_name", "request_payload", "payload",
    ];
    let has_extra_keys = map.keys().any(|k| !output_keys.contains(&k.as_str()));
    let extra_keys_are_all_gate = map
        .keys()
        .filter(|k| !output_keys.contains(&k.as_str()))
        .all(|k| gate_routing_keys.contains(&k.as_str()));
    if has_extra_keys
        && extra_keys_are_all_gate
        && *output_key != "final_answer"
    {
        return None;
    }
    let response = clean_chat_text(
        map.get(*output_key).and_then(Value::as_str).unwrap_or(""),
        32_000,
    );
    if response.trim().is_empty() {
        return None;
    }
    Some(response)
}

fn response_wrapper_starts_with_output_key(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    ["response", "text", "answer", "message", "content", "output", "final_answer"].iter().any(|key| {
        lowered.starts_with(&format!("\"{key}\""))
            || lowered.starts_with(&format!("'{key}'"))
            || lowered.starts_with(&format!("{key}:"))
    })
}

fn normalize_response_field_json_wrapper(text: &str) -> Option<String> {
    let cleaned = clean_text(text.trim(), 32_000);
    if cleaned.is_empty() {
        return None;
    }
    if let Some(payload) = parse_json_payload_dump(&cleaned) {
        if let Some(response) = response_field_from_json_wrapper(&payload) {
            return Some(response);
        }
        if let Some(inner) = payload.as_str() {
            if let Some(inner_payload) = parse_json_payload_dump(inner) {
                return response_field_from_json_wrapper(&inner_payload);
            }
        }
    }

    let trimmed = cleaned.trim();
    if !response_wrapper_starts_with_output_key(trimmed) {
        return None;
    }
    let jsonish = if trimmed.ends_with('}') {
        format!("{{{trimmed}")
    } else {
        format!("{{{trimmed}}}")
    };
    parse_json_payload_dump(&jsonish).and_then(|payload| response_field_from_json_wrapper(&payload))
}

fn with_payload_normalization_outcome(outcome: &str, payload_normalized: bool) -> String {
    let cleaned = clean_text(outcome, 200);
    if !payload_normalized {
        return if cleaned.is_empty() {
            "unchanged".to_string()
        } else {
            cleaned
        };
    }
    if cleaned.is_empty() || cleaned == "unchanged" {
        return "normalized_raw_payload_json".to_string();
    }
    format!("normalized_raw_payload_json+{cleaned}")
}

fn response_contains_peer_review_template_dump(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let markers = [
        "aiffel campus online",
        "peerreviewtemplate",
        "prt(peerreviewtemplate)",
        "코더",
        "리뷰어",
        "각 항목을 스스로 확인",
        "코드가 정상적으로 동작",
        "chatbotdata.csv",
        "tensorflow.keras",
    ];
    let hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn response_contains_large_code_dump_with_low_overlap(
    user_message: &str,
    response_text: &str,
) -> bool {
    if response_text.len() < 1_400 {
        return false;
    }
    let code_like_lines = response_text
        .lines()
        .filter(|line| {
            let lowered = line.trim_start().to_ascii_lowercase();
            lowered.starts_with("import ")
                || lowered.starts_with("from ")
                || lowered.starts_with("def ")
                || lowered.starts_with("class ")
                || lowered.starts_with("#include")
                || lowered.starts_with("public class ")
                || lowered.starts_with("int main")
                || lowered.starts_with("typedef ")
                || lowered.starts_with("using namespace ")
                || lowered.starts_with("fn main(")
        })
        .count();
    if code_like_lines < 4 {
        return false;
    }
    let user_terms = important_memory_terms(user_message, 20)
        .into_iter()
        .collect::<HashSet<_>>();
    let response_terms = important_memory_terms(response_text, 48)
        .into_iter()
        .collect::<HashSet<_>>();
    if user_terms.is_empty() || response_terms.is_empty() {
        return false;
    }
    user_terms.is_disjoint(&response_terms)
}

fn response_contains_competitive_programming_template_dump(
    user_message: &str,
    response_text: &str,
) -> bool {
    let lowered = clean_text(response_text, 12_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "<|begin_of_sentence|>",
        "<｜begin▁of▁sentence｜>",
        "you are an expert python programmer",
        "translate the following java code to python",
        "input specification:",
        "output specification:",
        "sample input:",
        "sample output:",
        "智能推荐",
        "猜你喜欢",
        "03-树2 list leaves",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 2 {
        return false;
    }
    let user_lowered = clean_text(user_message, 600).to_ascii_lowercase();
    let user_requested_programming = user_lowered.contains("translate")
        || user_lowered.contains("java")
        || user_lowered.contains("python")
        || user_lowered.contains("coding problem")
        || user_lowered.contains("algorithm")
        || user_lowered.contains("list leaves")
        || user_lowered.contains("competitive");
    !user_requested_programming
}

fn response_contains_kernel_patch_thread_dump(
    user_message: &str,
    response_text: &str,
) -> bool {
    let lowered = clean_text(response_text, 20_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "[patch",
        "subject:",
        "from:",
        "to:",
        "in-reply-to:",
        "references:",
        "signed-off-by:",
        "diff --git",
        "@@ -",
        "index ",
        "[date prev]",
        "[thread index]",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 4 {
        return false;
    }
    let user_lowered = clean_text(user_message, 600).to_ascii_lowercase();
    let user_requested_patch_context = user_lowered.contains("linux kernel")
        || user_lowered.contains("patch review")
        || user_lowered.contains("git diff")
        || user_lowered.contains("signed-off-by")
        || user_lowered.contains("mailing list patch");
    !user_requested_patch_context
}

fn response_contains_role_preamble_prompt_dump(
    user_message: &str,
    response_text: &str,
) -> bool {
    let lowered = clean_text(response_text, 12_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "i am an expert in the field",
        "my role is to",
        "the user has provided",
        "my task is to refine",
        "workflow metadata",
        "source: the model's training data",
        "mechanism: faulty pattern retrieval",
        "the error: context collapse",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if marker_hits < 2 {
        return false;
    }
    let user_lowered = clean_text(user_message, 600).to_ascii_lowercase();
    let user_requested_prompting_context = user_lowered.contains("write a prompt")
        || user_lowered.contains("system prompt")
        || user_lowered.contains("role prompt")
        || user_lowered.contains("persona prompt");
    !user_requested_prompting_context
}

fn response_matches_previous_message_self_check(user_message: &str, response_text: &str) -> bool {
    let ignored_terms = [
        "the", "a", "an", "and", "or", "to", "for", "of", "in", "on", "is", "are", "was",
        "were", "it", "this", "that", "with", "from", "as", "by", "you", "your", "we", "our",
        "can", "could", "should", "would", "do", "did", "does", "system", "agent", "llm",
        "tool", "tools", "message", "response",
    ]
    .into_iter()
    .collect::<HashSet<_>>();
    let user_terms = important_memory_terms(user_message, 28)
        .into_iter()
        .map(|term| clean_text(&term, 80).to_ascii_lowercase())
        .filter(|term| !term.is_empty() && !ignored_terms.contains(term.as_str()))
        .collect::<HashSet<_>>();
    if user_terms.len() < 2 {
        return true;
    }
    let response_terms = important_memory_terms(response_text, 72)
        .into_iter()
        .map(|term| clean_text(&term, 80).to_ascii_lowercase())
        .filter(|term| !term.is_empty())
        .collect::<HashSet<_>>();
    if response_terms.is_empty() {
        return false;
    }
    user_terms
        .intersection(&response_terms)
        .next()
        .is_some()
}

fn response_is_unrelated_context_dump(user_message: &str, response_text: &str) -> bool {
    if response_contains_cross_project_assimilation_bleed(user_message, response_text) {
        return true;
    }
    if response_contains_kernel_patch_thread_dump(user_message, response_text) {
        return true;
    }
    if response_contains_role_preamble_prompt_dump(user_message, response_text) {
        return true;
    }
    if response_text.contains("<function=") || response_text.contains("</function>") {
        if response_contains_tool_telemetry_dump(response_text)
            || response_contains_peer_review_template_dump(response_text)
        {
            return true;
        }
        return false;
    }
    if response_contains_tool_telemetry_dump(response_text) {
        return true;
    }
    if response_contains_peer_review_template_dump(response_text) {
        return true;
    }
    if response_contains_large_code_dump_with_low_overlap(user_message, response_text) {
        return true;
    }
    if response_contains_competitive_programming_template_dump(user_message, response_text) {
        return true;
    }
    if response_contains_short_unrelated_project_title(user_message, response_text) {
        return true;
    }
    if response_text.len() < 220 {
        return false;
    }
    if response_contains_project_dump_sections(response_text) {
        let user_terms = important_memory_terms(user_message, 20)
            .into_iter()
            .collect::<HashSet<_>>();
        let response_terms = important_memory_terms(response_text, 48)
            .into_iter()
            .collect::<HashSet<_>>();
        if user_terms.is_empty() || response_terms.is_empty() {
            return false;
        }
        return user_terms.is_disjoint(&response_terms);
    }
    false
}

fn response_low_alignment_with_turn_context(
    user_message: &str,
    recent_context: &str,
    response_text: &str,
) -> bool {
    let cleaned_response = clean_text(response_text, 12_000);
    if cleaned_response.is_empty() {
        return true;
    }
    if response_is_unrelated_context_dump(user_message, &cleaned_response) {
        return true;
    }
    if cleaned_response.len() > 220
        && !response_matches_previous_message_self_check(user_message, &cleaned_response)
    {
        return true;
    }
    let contextual_seed = clean_text(
        &format!("{}\n{}", clean_text(user_message, 1_000), clean_text(recent_context, 2_000)),
        4_000,
    );
    let context_terms = important_memory_terms(&contextual_seed, 48)
        .into_iter()
        .collect::<HashSet<_>>();
    if context_terms.len() < 3 {
        return false;
    }
    let response_terms = important_memory_terms(&cleaned_response, 72)
        .into_iter()
        .collect::<HashSet<_>>();
    if response_terms.is_empty() {
        return true;
    }
    let overlap_count = context_terms.intersection(&response_terms).count();
    overlap_count == 0 && cleaned_response.len() > 260
}

fn response_has_rich_findings_markers(lowered: &str) -> bool {
    lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains("according to")
        || lowered.contains("source:")
        || lowered.contains("sources:")
        || lowered.contains("1.")
        || lowered.contains("2.")
}

fn normalize_ack_detector_text(text: &str, max_len: usize) -> String {
    clean_text(text, max_len)
        .to_ascii_lowercase()
        .replace('’', "'")
        .replace('‘', "'")
        .replace('`', "'")
        .replace('“', "\"")
        .replace('”', "\"")
}

fn response_is_deferred_execution_preamble(text: &str) -> bool {
    let lowered = normalize_ack_detector_text(text, 2_000);
    if lowered.is_empty() {
        return false;
    }
    let token_count = lowered.split_whitespace().count();
    if token_count > 80 {
        return false;
    }
    if response_has_rich_findings_markers(&lowered) {
        return false;
    }
    [
        "i'll get you an update",
        "i will get you an update",
        "let me get you an update",
        "i'll look into",
        "i will look into",
        "let me look into",
        "i'll check",
        "i will check",
        "let me check",
        "i'm going to check",
        "i am going to check",
        "i'm checking now",
        "i am checking now",
        "getting that now",
        "working on it",
        "one moment",
        "just a moment",
        "stand by",
        "i'll report back",
        "i will report back",
    ]
    .iter()
    .any(|marker| lowered.starts_with(marker))
}

fn response_is_deferred_retry_prompt(text: &str) -> bool {
    let lowered = normalize_ack_detector_text(text, 2_000);
    if lowered.is_empty() {
        return false;
    }
    if response_has_rich_findings_markers(&lowered) {
        return false;
    }
    let mentions_retry_language = [
        "would you like me to try",
        "would you like me to retry",
        "would you like me to run",
        "should i retry",
        "should i rerun",
        "i can retry with",
        "i can rerun with",
        "if you'd like, i can retry",
        "if you would like, i can retry",
        "if you'd like, i can rerun",
        "if you would like, i can rerun",
        "i can try a narrower query",
        "i can run a narrower query",
        "i can try a more specific query",
        "i can search again",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    if !mentions_retry_language {
        return false;
    }
    lowered.contains("search")
        || lowered.contains("web")
        || lowered.contains("tool")
        || lowered.contains("query")
        || lowered.contains("source url")
}

fn response_looks_like_tool_ack_without_findings(text: &str) -> bool {
    let cleaned = clean_text(text, 1200);
    let lowered = normalize_ack_detector_text(&cleaned, 1200);
    let potential_source_mentions = lowered.matches("potential sources:").count();
    if lowered.is_empty() {
        return true;
    }
    if response_is_no_findings_placeholder(&cleaned) {
        return false;
    }
    if response_is_actionable_tool_diagnostic(&cleaned) {
        return false;
    }
    if response_looks_like_unsynthesized_web_snippet_dump(&cleaned)
        || response_looks_like_raw_web_artifact_dump(&cleaned)
        || response_contains_tool_telemetry_dump(&cleaned)
    {
        return true;
    }
    if parse_json_payload_dump(&cleaned)
        .map(|payload| looks_like_internal_agent_payload_dump(&payload))
        .unwrap_or(false)
    {
        return true;
    }
    if lowered.contains("key findings for") && potential_source_mentions >= 1 {
        return true;
    }
    if potential_source_mentions >= 1
        && !lowered.contains("http://")
        && !lowered.contains("https://")
    {
        return true;
    }
    if lowered.starts_with("web search for")
        && lowered.contains("found sources:")
        && !lowered.contains("http://")
        && !lowered.contains("https://")
    {
        return true;
    }
    if crate::tool_output_match_filter::matches_ack_placeholder(&cleaned) {
        return true;
    }
    if response_looks_like_off_topic_web_results(&cleaned) {
        return true;
    }
    let has_rich_findings = response_has_rich_findings_markers(&lowered);
    let mentions_tooling = lowered.contains("search")
        || lowered.contains("web")
        || lowered.contains("tool")
        || lowered.contains("found some results")
        || lowered.contains("looked up")
        || lowered.contains("called")
        || lowered.contains("executed")
        || lowered.contains("reading files")
        || lowered.contains("searching the internet")
        || lowered.contains("running terminal commands");
    let plain_failure_explanation = lowered.contains("didn't return")
        || lowered.contains("did not return")
        || lowered.contains("low-signal")
        || lowered.contains("no source-backed")
        || lowered.contains("only returned");
    let mainly_ack_language = lowered.contains("i searched")
        || lowered.contains("searched the internet")
        || lowered.contains("i looked up")
        || lowered.contains("i found some results")
        || lowered.contains("found some results")
        || lowered.contains("will summarize them")
        || lowered.contains("i called")
        || lowered.contains("i executed")
        || lowered.contains("web search completed")
        || lowered.contains("tool completed")
        || lowered.contains("batch execution initiated")
        || lowered.contains("concurrent searches running")
        || lowered.contains("will execute all searches in parallel")
        || lowered.contains("would execute concurrently")
        || lowered.contains("this demonstrates the full pipeline");
    if mentions_tooling && plain_failure_explanation {
        return false;
    }
    if response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
    {
        return true;
    }
    if !mentions_tooling {
        return false;
    }
    mentions_tooling && !has_rich_findings && mainly_ack_language
}

fn no_findings_user_facing_response() -> String {
    String::new()
}

fn response_is_no_findings_placeholder(text: &str) -> bool {
    let lowered = clean_text(text, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if lowered.contains("error_code: web_tool_low_signal")
        || (lowered.contains("error_code:") && lowered.contains("web_status:"))
    {
        return false;
    }
    if lowered.contains("http://") || lowered.contains("https://") {
        return false;
    }
    lowered.contains("no relevant results found for that request yet")
        || lowered.contains("couldn't produce source-backed findings in this turn")
        || lowered.contains("don't have usable tool findings from this turn yet")
        || lowered.contains("low-signal or no-result output")
        || lowered.contains("couldn't extract usable findings")
        || lowered.contains("could not extract usable findings")
        || lowered.contains("search returned no useful information")
        || lowered.contains("couldn't extract reliable findings")
        || lowered.contains("could not extract reliable findings")
        || lowered.contains("no usable findings yet")
}

fn response_contains_speculative_web_blocker_language(text: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let mentions_blocked_lane = lowered.contains("blocked the function calls")
        || lowered.contains("function calls from executing entirely")
        || lowered.contains("function calls entirely")
        || lowered.contains("wouldn't even attempt to execute")
        || lowered.contains("would not even attempt to execute")
        || lowered.contains("web search operations")
        || lowered.contains("web tool access")
        || lowered.contains("external tool execution")
        || lowered.contains("function execution level")
        || lowered.contains("invalid response attempt")
        || lowered.contains("processing the queries");
    if !mentions_blocked_lane {
        return false;
    }
    lowered.contains("security controls")
        || lowered.contains("policy change")
        || lowered.contains("policy restrictions")
        || lowered.contains("temporary system restriction")
        || lowered.contains("broader policy change")
        || lowered.contains("would you like me to try a different approach")
}

fn response_looks_like_off_topic_web_results(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let has_web_context = lowered.contains("web search")
        || lowered.contains("search results")
        || lowered.contains("query");
    if !has_web_context {
        return false;
    }
    let off_topic_markers = lowered.contains("qrz.com")
        || lowered.contains("callsign")
        || lowered.contains("ham radio")
        || lowered.contains("qso");
    off_topic_markers
}

fn sanitize_findings_for_final_response(findings: Option<String>) -> Option<String> {
    let raw = findings.unwrap_or_default();
    let cleaned = clean_text(raw.trim(), 24_000);
    if cleaned.is_empty() {
        return None;
    }
    if response_looks_like_tool_ack_without_findings(&cleaned) {
        return None;
    }
    if response_looks_like_off_topic_web_results(&cleaned) {
        return None;
    }
    Some(cleaned)
}

fn finalize_user_facing_response_with_outcome(
    output: String,
    findings: Option<String>,
) -> (String, String, bool) {
    let _findings_cleaned = sanitize_findings_for_final_response(findings);
    let mut cleaned = clean_text(output.trim(), 32_000);
    let mut payload_normalized = false;
    if let Some(unwrapped) = normalize_response_field_json_wrapper(&cleaned) {
        cleaned = clean_text(unwrapped.trim(), 32_000);
        payload_normalized = true;
    }
    cleaned = strip_disallowed_source_tags(&cleaned);
    if contains_deprecated_workflow_ghost_phrase(&cleaned) {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_deprecated_workflow_ghost_text",
                payload_normalized,
            ),
            true,
        );
    }
    if let Some((_, rule_id)) = crate::tool_output_match_filter::rewrite_failure_placeholder(&cleaned)
    {
        return (
            cleaned,
            with_payload_normalization_outcome(
                &format!("flagged_failure_placeholder:{rule_id}"),
                payload_normalized,
            ),
            false,
        );
    }
    if response_looks_like_raw_web_artifact_dump(&cleaned) {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_raw_web_artifact_dump",
                payload_normalized,
            ),
            false,
        );
    }
    if response_looks_like_unsynthesized_web_snippet_dump(&cleaned) {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_unsynthesized_web_snippet_dump",
                payload_normalized,
            ),
            false,
        );
    }
    if response_looks_like_off_topic_web_results(&cleaned) {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_off_topic_web_results",
                payload_normalized,
            ),
            true,
        );
    }
    let speculative_blocker_copy = response_contains_speculative_web_blocker_language(&cleaned);
    let deferred_execution_copy = response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned);
    let input_ack_only = response_looks_like_tool_ack_without_findings(&cleaned);
    if speculative_blocker_copy {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_speculative_web_blocker_copy",
                payload_normalized,
            ),
            true,
        );
    }
    if deferred_execution_copy {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_deferred_execution_copy",
                payload_normalized,
            ),
            true,
        );
    }
    if cleaned.is_empty() {
        return (
            String::new(),
            with_payload_normalization_outcome(
                "empty_final_response",
                payload_normalized,
            ),
            false,
        );
    }
    if input_ack_only {
        return (
            cleaned,
            with_payload_normalization_outcome("flagged_ack_only_response", payload_normalized),
            true,
        );
    }
    let cleaned_without_legacy_gate = scrub_deprecated_workflow_ghost_text(&cleaned);
    if cleaned_without_legacy_gate.is_empty() {
        return (
            cleaned,
            with_payload_normalization_outcome(
                "flagged_deprecated_workflow_gate_token",
                payload_normalized,
            ),
            true,
        );
    }
    if cleaned_without_legacy_gate != cleaned {
        return (
            strip_disallowed_source_tags(&cleaned),
            with_payload_normalization_outcome(
                "flagged_deprecated_workflow_gate_token",
                payload_normalized,
            ),
            true,
        );
    }
    (
        strip_disallowed_source_tags(&cleaned),
        with_payload_normalization_outcome("unchanged", payload_normalized),
        false,
    )
}

#[allow(dead_code)]
fn finalize_user_facing_response(output: String, findings: Option<String>) -> String {
    finalize_user_facing_response_with_outcome(output, findings).0
}

fn response_prompt_echo_detected(user_message: &str, response_text: &str) -> bool {
    let message = clean_text(user_message, 1_200).to_ascii_lowercase();
    let response = clean_text(response_text, 1_200).to_ascii_lowercase();
    if message.is_empty() || response.is_empty() {
        return false;
    }
    if response_contains_prompt_scaffold(&response) {
        return true;
    }
    // Only apply verbatim-echo checks when the message is long enough to rule out
    // natural greeting overlaps like "hey" → "Hey! How can I help?"
    if message.len() >= 8 && (response == message || response.starts_with(&message)) {
        return true;
    }
    if response.contains(&format!("\"{message}\"")) {
        return true;
    }
    let message_terms = important_memory_terms(&message, 24)
        .into_iter()
        .collect::<HashSet<_>>();
    let response_terms = important_memory_terms(&response, 36)
        .into_iter()
        .collect::<HashSet<_>>();
    // Overlap-based echo detection requires enough message terms to avoid false positives
    // on short greetings ("hey" → ["hey"] → 100% overlap with any response mentioning "hey").
    if message_terms.len() < 3 || response_terms.is_empty() {
        return false;
    }
    let overlap = message_terms.intersection(&response_terms).count() as f64;
    let denominator = message_terms.len().max(1) as f64;
    let overlap_ratio = overlap / denominator;
    let response_word_count = response.split_whitespace().count();
    response_word_count <= 60 && overlap_ratio >= 0.9
}

fn response_contains_prompt_scaffold(response_text: &str) -> bool {
    let response = clean_text(response_text, 2_400).to_ascii_lowercase();
    !response.is_empty()
        && ((response.contains("<instructions>")
            && response.contains("</instructions>")
            && response.contains("<output_format"))
            || (response.contains("reply naturally as the assistant")
                && response.contains("write the assistant reply now"))
            || (response.contains("user:")
                && response.contains("write the assistant reply now")))
}

fn response_has_evidence_tags(text: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    lowered.contains("[source:local_context]") || lowered.contains("[source:tool_receipt:")
}

fn first_two_sentences(text: &str, max_len: usize) -> String {
    let cleaned = clean_text(text, max_len.max(1));
    if cleaned.is_empty() {
        return cleaned;
    }
    let mut boundaries = cleaned
        .char_indices()
        .filter(|(_, ch)| matches!(ch, '.' | '!' | '?' | '\n'))
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    boundaries.sort_unstable();
    if boundaries.is_empty() {
        return cleaned;
    }
    let cut_idx = if boundaries.len() >= 2 {
        boundaries[1].saturating_add(1)
    } else {
        boundaries[0].saturating_add(1)
    };
    clean_text(&cleaned[..cut_idx.min(cleaned.len())], max_len.max(1))
}

fn response_answers_user_early(user_message: &str, response_text: &str) -> bool {
    let early = first_two_sentences(response_text, 800);
    if early.is_empty() {
        return false;
    }
    let lowered_message = clean_text(user_message, 1_000).to_ascii_lowercase();
    let strict_question_shape = lowered_message.contains('?')
        || lowered_message.starts_with("what")
        || lowered_message.starts_with("why")
        || lowered_message.starts_with("how")
        || lowered_message.starts_with("did")
        || lowered_message.starts_with("can")
        || lowered_message.starts_with("could")
        || lowered_message.starts_with("would")
        || lowered_message.contains("status")
        || lowered_message.contains("compare");
    if !strict_question_shape {
        return true;
    }
    let question_terms = important_memory_terms(user_message, 24)
        .into_iter()
        .collect::<HashSet<_>>();
    let early_terms = important_memory_terms(&early, 30)
        .into_iter()
        .collect::<HashSet<_>>();
    if question_terms.is_empty() {
        return !early.is_empty();
    }
    if early_terms.is_empty() {
        return false;
    }
    let overlap = question_terms.intersection(&early_terms).count();
    overlap >= 1
        || early
            .to_ascii_lowercase()
            .starts_with("yes")
        || early
            .to_ascii_lowercase()
            .starts_with("no")
        || early
            .to_ascii_lowercase()
            .starts_with("based on")
}

#[cfg(test)]
mod tests_json_wrapper {
    use super::*;

    #[test]
    fn normalize_response_field_json_wrapper_extracts_final_answer() {
        assert_eq!(
            normalize_response_field_json_wrapper(
                r#"{"gate":"respond_directly","final_answer":"Hello! How can I help?"}"#
            ),
            Some("Hello! How can I help?".to_string())
        );
    }

    #[test]
    fn normalize_response_field_json_wrapper_rejects_tool_request_like_json() {
        assert_eq!(
            normalize_response_field_json_wrapper(
                r#"{"tool_family":"web_research","tool":"web_search","request_payload":{"query":"agentic frameworks"}}"#,
            ),
            None
        );
    }
}
