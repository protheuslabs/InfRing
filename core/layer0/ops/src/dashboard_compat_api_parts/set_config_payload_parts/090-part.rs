fn merge_response_outcomes(primary: &str, secondary: &str, max_len: usize) -> String {
    let left = clean_text(primary, max_len.max(1));
    let right = clean_text(secondary, max_len.max(1));
    if left.is_empty() || left == "unchanged" {
        return if right.is_empty() {
            "unchanged".to_string()
        } else {
            right
        };
    }
    if right.is_empty() || right == "unchanged" {
        return left;
    }
    if left == right {
        return left;
    }
    clean_text(&format!("{left}+{right}"), max_len.max(1))
}

fn response_tool_receipt_id(row: &Value) -> String {
    clean_text(
        row.pointer("/tool_attempt_receipt/receipt_hash")
            .or_else(|| row.pointer("/tool_attempt_receipt/receipt_id"))
            .or_else(|| row.pointer("/tool_attempt_receipt/id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
}

fn response_fails_base_final_answer_contract(text: &str) -> bool {
    let cleaned = clean_text(text, 32_000);
    cleaned.trim().is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
}

fn response_workflow_quality_value<'a>(workflow: &'a Value, key: &str) -> Option<&'a Value> {
    workflow
        .get("quality_telemetry")
        .and_then(Value::as_object)
        .and_then(|telemetry| telemetry.get(key))
}

fn response_workflow_quality_rate(workflow: &Value, key: &str) -> f64 {
    match response_workflow_quality_value(workflow, key) {
        Some(Value::Number(number)) => number
            .as_f64()
            .or_else(|| number.as_u64().map(|value| value as f64))
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

fn response_workflow_quality_count(workflow: &Value, key: &str) -> u64 {
    match response_workflow_quality_value(workflow, key) {
        Some(Value::Number(number)) => number.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn build_response_finalization_payload(
    finalization_outcome: &str,
    initial_ack_only: bool,
    final_ack_only: bool,
    tool_completion: &Value,
    tooling_fallback_used: bool,
    comparative_fallback_used: bool,
    workflow_system_fallback_used: bool,
    visible_response_repaired: bool,
    response_quality_telemetry: &Value,
    tooling_invariant: &Value,
    web_invariant: &Value,
) -> Value {
    let citations = tool_completion
        .get("citations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let source_refs = tool_completion
        .get("source_refs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| citations.clone());
    json!({
        "applied": finalization_outcome != "unchanged",
        "outcome": finalization_outcome,
        "initial_ack_only": initial_ack_only,
        "final_ack_only": final_ack_only,
        "findings_available": tool_completion
            .get("findings_available")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "tool_completion": tool_completion,
        "citations": citations,
        "source_refs": source_refs,
        "final_answer_contract": tool_completion
            .get("final_answer_contract")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "retry_attempted": false,
        "retry_used": false,
        "tool_synthesis_retry_used": false,
        "synthesis_retry_used": false,
        "tooling_fallback_used": tooling_fallback_used,
        "comparative_fallback_used": comparative_fallback_used,
        "workflow_system_fallback_used": workflow_system_fallback_used,
        "visible_response_repaired": visible_response_repaired,
        "response_quality_telemetry": response_quality_telemetry,
        "tooling_invariant": tooling_invariant,
        "web_invariant": web_invariant
    })
}

fn build_response_quality_telemetry_payload(
    response_workflow: &Value,
    final_fallback_used: bool,
    tooling_invariant_repair_used: bool,
    tooling_failure_code: &str,
    direct_answer_rate: f64,
    retry_rate: f64,
    tool_overcall_rate: f64,
    off_topic_reject_rate: f64,
) -> Value {
    json!({
        "off_topic_reject": response_workflow_quality_count(response_workflow, "off_topic_reject"),
        "deferred_reply_reject": response_workflow_quality_count(response_workflow, "deferred_reply_reject"),
        "alignment_reject": response_workflow_quality_count(response_workflow, "alignment_reject"),
        "prompt_echo_reject": response_workflow_quality_count(response_workflow, "prompt_echo_reject"),
        "unsourced_claim_reject": response_workflow_quality_count(response_workflow, "unsourced_claim_reject"),
        "direct_answer_reject": response_workflow_quality_count(response_workflow, "direct_answer_reject"),
        "contamination_reject": response_workflow_quality_count(response_workflow, "contamination_reject"),
        "current_turn_dominance_reject": response_workflow_quality_count(response_workflow, "current_turn_dominance_reject"),
        "unsupported_tool_success_claim_reject": response_workflow_quality_count(response_workflow, "unsupported_tool_success_claim_reject"),
        "legacy_retry_template_detected": response_workflow_quality_count(response_workflow, "legacy_retry_template_detected"),
        "repeated_fallback_loop_detected": response_workflow_quality_count(response_workflow, "repeated_fallback_loop_detected"),
        "meta_control_tool_block": response_workflow_quality_value(response_workflow, "meta_control_tool_block")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "final_fallback_used": final_fallback_used,
        "tooling_contract_repair_used": tooling_invariant_repair_used,
        "tooling_failure_code_present": !tooling_failure_code.is_empty(),
        "direct_answer_rate": direct_answer_rate,
        "retry_rate": retry_rate,
        "tool_overcall_rate": tool_overcall_rate,
        "off_topic_reject_rate": off_topic_reject_rate
    })
}

fn claim_source_tags_for_report(response_tools: &[Value], max_items: usize) -> Vec<String> {
    let mut tags = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for row in response_tools.iter().take(max_items.clamp(1, 8)) {
        let receipt = response_tool_receipt_id(row);
        if receipt.is_empty() {
            continue;
        }
        let tag = format!("tool_receipt:{receipt}");
        if seen.insert(tag.clone()) {
            tags.push(tag);
        }
    }
    tags
}

fn response_has_public_source_signal_for_finalization(text: &str) -> bool {
    let lowered = clean_text(text, 8_000).to_ascii_lowercase();
    [
        "http://",
        "https://",
        "source:",
        "sources:",
        "citation",
        "citations",
        "according to",
        "official docs",
        "the docs",
        "release notes",
        "changelog",
        "paper",
        "study",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
        || text_contains_domain_like_source_marker(&lowered)
}

fn text_contains_domain_like_source_marker(text: &str) -> bool {
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

fn join_compact_label_list(labels: &[String]) -> String {
    match labels.len() {
        0 => String::new(),
        1 => labels[0].clone(),
        2 => format!("{} and {}", labels[0], labels[1]),
        _ => {
            let mut head = labels[..labels.len() - 1].join(", ");
            head.push_str(", and ");
            head.push_str(labels.last().map(String::as_str).unwrap_or(""));
            head
        }
    }
}

fn enforce_user_facing_finalization_contract(
    user_message: &str,
    output: String,
    response_tools: &[Value],
) -> (String, Value, String) {
    let findings = response_tools_summary_for_user(response_tools, 4);
    let findings = if findings.is_empty() { None } else { Some(findings) };
    let failure_reason = response_tools_failure_reason_for_user(response_tools, 4);
    let (mut prefinalized, mut pre_outcome, _) = finalize_user_facing_response_with_outcome(output, findings);
    let prefinalized_cleaned = clean_text(&prefinalized, 32_000);
    if !failure_reason.is_empty()
        && (prefinalized_cleaned.is_empty()
            || response_looks_like_tool_ack_without_findings(&prefinalized_cleaned)
            || response_is_no_findings_placeholder(&prefinalized_cleaned))
    {
        pre_outcome = merge_response_outcomes(
            &pre_outcome,
            "flagged_no_llm_response_due_tool_failure",
            220,
        );
    }
    let (mut finalized, mut report) =
        enforce_tool_completion_contract(prefinalized, response_tools);
    if !response_tools.is_empty()
        && !response_has_evidence_tags(&finalized)
        && !response_has_public_source_signal_for_finalization(&finalized)
    {
        let source_grounding = compact_source_grounding_sentence(response_tools, 3);
        if !source_grounding.is_empty() {
            finalized = clean_text(&format!("{finalized}\n\n{source_grounding}"), 32_000);
            if let Some(obj) = report.as_object_mut() {
                obj.insert("source_grounding_repair_used".to_string(), Value::Bool(true));
                obj.insert(
                    "source_grounding_sentence".to_string(),
                    Value::String(source_grounding),
                );
            }
        }
    }
    if !failure_reason.is_empty()
        && report.get("completion_state").and_then(Value::as_str) == Some("reported_no_findings")
    {
        if let Some(obj) = report.as_object_mut() {
            obj.insert("completion_state".to_string(), Value::String("flagged".to_string()));
            obj.insert("reasoning".to_string(), Value::String(first_sentence(&finalized, 220)));
        }
    }
    let deferred_execution = response_is_deferred_execution_preamble(&finalized)
        || response_is_deferred_retry_prompt(&finalized);
    if deferred_execution
        && report
            .get("completion_state")
            .and_then(Value::as_str)
            != Some("reported_findings")
    {
        if let Some(obj) = report.as_object_mut() {
            obj.insert(
                "completion_state".to_string(),
                Value::String("flagged".to_string()),
            );
            obj.insert(
                "reasoning".to_string(),
                Value::String(first_sentence(&finalized, 220)),
            );
            let prior = clean_text(obj.get("outcome").and_then(Value::as_str).unwrap_or(""), 200);
            obj.insert(
                "outcome".to_string(),
                Value::String(append_tool_completion_outcome(
                    &prior,
                    "tool_completion_flagged_deferred_execution",
                )),
            );
        }
    }
    let claim_sources = if response_tools.is_empty() {
        vec!["local_context".to_string()]
    } else {
        claim_source_tags_for_report(response_tools, 4)
    };
    let final_answer_contract = json!({
        "contract": "final_answer_contract_v1",
        "direct_answer_required": true,
        "direct_answer_in_first_two_sentences": response_answers_user_early(user_message, &finalized),
        "no_prompt_echo": !response_prompt_echo_detected(user_message, &finalized),
        "no_placeholder_copy": !response_is_no_findings_placeholder(&finalized)
            && !response_looks_like_tool_ack_without_findings(&finalized),
        "no_unsourced_claims": !claim_sources.is_empty()
            || response_has_evidence_tags(&finalized)
            || response_has_public_source_signal_for_finalization(&finalized),
        "claim_sources": claim_sources
    });
    if let Some(obj) = report.as_object_mut() {
        obj.insert("final_answer_contract".to_string(), final_answer_contract);
    }
    let contract_outcome = clean_text(report.get("outcome").and_then(Value::as_str).unwrap_or("unchanged"), 200);
    let merged_outcome = merge_response_outcomes(&pre_outcome, &contract_outcome, 220);
    (finalized, report, merged_outcome)
}

fn available_model_count(root: &Path, snapshot: &Value) -> usize {
    crate::dashboard_model_catalog::catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("available")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn no_models_available_payload(agent_id: &str) -> Value {
    json!({
        "ok": false,
        "error": "no_models_available",
        "error_code": "no_models_available",
        "agent_id": clean_agent_id(agent_id),
        "hint": "No usable LLMs are available yet. Install Ollama or add an API key.",
        "setup": {
            "steps": [
                "Install Ollama: https://ollama.com/download",
                "Start Ollama: ollama serve",
                "Pull or configure the exact model you want to use",
                "Or add API keys in Settings or via /apikey <key>"
            ]
        },
        "links": [
            {"label": "Ollama Download", "url": "https://ollama.com/download"},
            {"label": "Ollama Library", "url": "https://ollama.com/library"},
            {"label": "OpenRouter Keys", "url": "https://openrouter.ai/keys"},
            {"label": "OpenAI API Keys", "url": "https://platform.openai.com/api-keys"},
            {"label": "Anthropic API Keys", "url": "https://console.anthropic.com/settings/keys"},
            {"label": "Google AI Studio Keys", "url": "https://aistudio.google.com/app/apikey"}
        ]
    })
}

fn response_tool_summary_text_is_rejected(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("model attempted this call as text")
        || response_looks_like_tool_ack_without_findings(text)
        || response_is_no_findings_placeholder(text)
        || response_looks_like_unsynthesized_web_snippet_dump(text)
        || response_looks_like_raw_web_artifact_dump(text)
        || response_contains_tool_telemetry_dump(text)
        || looks_like_search_engine_chrome_summary(&lowered)
}

fn push_response_tool_summary_snippet(
    snippets: &mut Vec<String>,
    seen: &mut HashSet<String>,
    raw: &str,
    max_len: usize,
) {
    let cleaned = clean_text(raw, max_len.max(1));
    if cleaned.is_empty() {
        return;
    }
    let key = cleaned.to_ascii_lowercase();
    if seen.insert(key) {
        snippets.push(cleaned);
    }
}

fn collect_response_tool_evidence_snippets(
    value: &Value,
    snippets: &mut Vec<String>,
    seen: &mut HashSet<String>,
    limit: usize,
    depth: usize,
) {
    if snippets.len() >= limit || depth > 2 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                if snippets.len() >= limit {
                    break;
                }
                collect_response_tool_evidence_snippets(row, snippets, seen, limit, depth + 1);
            }
        }
        Value::Object(obj) => {
            if let Some(line) = compact_web_finding_line(value) {
                push_response_tool_summary_snippet(snippets, seen, &line, 160);
            }
            for key in [
                "evidence_refs",
                "search_results",
                "provider_results",
                "results",
                "items",
                "sources",
                "snippets",
            ] {
                let Some(nested) = obj.get(key) else {
                    continue;
                };
                if snippets.len() >= limit {
                    break;
                }
                collect_response_tool_evidence_snippets(nested, snippets, seen, limit, depth + 1);
            }
        }
        Value::String(raw) => {
            let line = first_sentence(&clean_text(raw, 200), 160);
            push_response_tool_summary_snippet(snippets, seen, &line, 160);
        }
        _ => {}
    }
}

fn response_tool_evidence_snippets_for_user(tool: &Value, max_items: usize) -> Vec<String> {
    let limit = max_items.clamp(1, 6);
    let mut snippets = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for key in [
        "evidence_refs",
        "evidence_pack",
        "evidence_pack_candidates",
        "search_results",
        "provider_results",
    ] {
        let Some(value) = tool.get(key) else {
            continue;
        };
        collect_response_tool_evidence_snippets(value, &mut snippets, &mut seen, limit, 0);
        if snippets.len() >= limit {
            break;
        }
    }
    snippets
}

fn response_tools_summary_for_user(response_tools: &[Value], max_items: usize) -> String {
    let limit = max_items.clamp(1, 8);
    let mut lines = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools {
        let raw_name = clean_text(
            tool.get("name").and_then(Value::as_str).unwrap_or("tool"),
            80,
        )
        .to_ascii_lowercase();
        let name = if raw_name == "batch_query" || raw_name == "batch-query" {
            "web_search".to_string()
        } else {
            raw_name.clone()
        };
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        if tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let raw_result = clean_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            2_000,
        );
        let mut snippets = Vec::<String>::new();
        let mut snippet_seen = HashSet::<String>::new();
        if !raw_result.is_empty() && !response_tool_summary_text_is_rejected(&raw_result) {
            let user_result =
                rewrite_tool_result_for_user_summary(&raw_name, &raw_result).unwrap_or(raw_result);
            if !response_tool_summary_text_is_rejected(&user_result) {
                let snippet = if user_result.starts_with("Key findings:") {
                    trim_text(&strip_redundant_key_findings_prefix(&user_result), 220)
                } else if let Some(rest) = user_result.strip_prefix("Web findings:") {
                    trim_text(rest.trim(), 220)
                } else {
                    first_sentence(&user_result, 220)
                };
                push_response_tool_summary_snippet(
                    &mut snippets,
                    &mut snippet_seen,
                    &snippet,
                    220,
                );
            }
        }
        for evidence_snippet in response_tool_evidence_snippets_for_user(tool, 4) {
            push_response_tool_summary_snippet(
                &mut snippets,
                &mut snippet_seen,
                &evidence_snippet,
                160,
            );
        }
        if snippets.is_empty() {
            continue;
        }
        let snippet = trim_text(&snippets.join(" | "), 520);
        let pretty_name = name.replace('_', " ");
        let line = format!("- {}: {}", clean_text(&pretty_name, 60), snippet);
        let key = line.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        lines.push(line);
        if lines.len() >= limit {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(
        &format!("Here's what I found:\n{}", lines.join("\n")),
        32_000,
    )
}

fn parse_tool_input_payload(raw_input: &str) -> Value {
    let cleaned = clean_text(raw_input, 12_000);
    if cleaned.is_empty() {
        return Value::Null;
    }
    serde_json::from_str::<Value>(&cleaned).unwrap_or_else(|_| Value::String(cleaned))
}

fn tool_payload_count(payload: &Value, keys: &[&str]) -> usize {
    for key in keys {
        let Some(value) = payload.get(*key) else {
            continue;
        };
        match value {
            Value::Array(rows) => {
                if !rows.is_empty() {
                    return rows.len().min(99);
                }
            }
            Value::Number(number) => {
                if let Some(raw) = number.as_u64() {
                    let bounded = raw.min(99) as usize;
                    if bounded > 0 {
                        return bounded;
                    }
                }
            }
            Value::String(text) => {
                if !text.trim().is_empty() {
                    return 1;
                }
            }
            Value::Object(map) => {
                if !map.is_empty() {
                    return 1;
                }
            }
            Value::Bool(flag) => {
                if *flag {
                    return 1;
                }
            }
            _ => {}
        }
    }
    0
}

fn tool_completion_status_for_tool(tool_name: &str, tool_input: &str) -> String {
    let normalized = normalize_tool_name(tool_name);
    if normalized == "thought_process" {
        return "Thinking".to_string();
    }
    let payload = parse_tool_input_payload(tool_input);
    let status = match normalized.as_str() {
        "batch_query" | "web_search" | "search_web" | "search" | "web_query" => {
            "Searching internet".to_string()
        }
        "web_fetch" | "browse" | "web_conduit_fetch" => "Reading web pages".to_string(),
        "file_read" | "read_file" | "file" => {
            let count = tool_payload_count(
                &payload,
                &["paths", "files", "file_paths", "targets", "path", "file"],
            );
            if count > 1 {
                format!("Scanning {count} files")
            } else if count == 1 {
                "Scanning 1 file".to_string()
            } else {
                "Scanning files".to_string()
            }
        }
        "file_read_many" => {
            let count = tool_payload_count(&payload, &["paths", "files", "file_paths", "targets"]);
            if count > 1 {
                format!("Scanning {count} files")
            } else if count == 1 {
                "Scanning 1 file".to_string()
            } else {
                "Scanning files".to_string()
            }
        }
        "folder_export" | "list_folder" | "folder_tree" | "folder" => {
            let count =
                tool_payload_count(&payload, &["folders", "paths", "targets", "path", "folder"]);
            if count > 1 {
                format!("Scanning {count} folders")
            } else if count == 1 {
                "Scanning 1 folder".to_string()
            } else {
                "Scanning folders".to_string()
            }
        }
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => {
            "Running terminal command".to_string()
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            let count =
                tool_payload_count(&payload, &["count", "agent_count", "num_agents", "agents"]);
            if count > 0 {
                format!("Summoning {count} agents")
            } else {
                "Summoning agents".to_string()
            }
        }
        "memory_semantic_query" => "Searching memory".to_string(),
        "cron_schedule" => "Scheduling follow-up work".to_string(),
        "cron_run" => "Running scheduled work".to_string(),
        "cron_list" => "Checking schedules".to_string(),
        "session_rollback_last_turn" => "Rewinding the last turn".to_string(),
        _ => {
            let cleaned = normalized.replace('_', " ");
            if cleaned.is_empty() {
                "Running tool".to_string()
            } else {
                format!("Running {cleaned}")
            }
        }
    };
    clean_text(&status, 180)
}

fn tool_completion_live_steps(response_tools: &[Value]) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for tool in response_tools {
        let name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        let input = clean_text(
            tool.get("input").and_then(Value::as_str).unwrap_or(""),
            12_000,
        );
        let status = tool_completion_status_for_tool(&name, &input);
        if status.is_empty() {
            continue;
        }
        out.push(json!({
            "tool": name,
            "status": status,
            "is_error": tool
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        }));
        if out.len() >= 16 {
            break;
        }
    }
    out
}

fn tool_terminal_transcript(response_tools: &[Value]) -> Vec<Value> {
    let mut rows = Vec::<Value>::new();
    for tool in response_tools {
        let name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or(""));
        if !is_terminal_tool_name(&name) {
            continue;
        }
        let parsed_input =
            serde_json::from_str::<Value>(tool.get("input").and_then(Value::as_str).unwrap_or(""))
                .unwrap_or_else(|_| json!({}));
        let command = clean_text(
            parsed_input
                .get("command")
                .or_else(|| parsed_input.get("cmd"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            12_000,
        );
        let output = trim_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            24_000,
        );
        let cwd = clean_text(
            parsed_input
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or(""),
            4_000,
        );
        let is_error = tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if command.is_empty() && output.trim().is_empty() {
            continue;
        }
        rows.push(json!({
            "tool": name,
            "command": command,
            "output": output,
            "cwd": cwd,
            "is_error": is_error
        }));
    }
    rows
}
fn append_turn_receipt_with_metadata(
    root: &Path,
    agent_id: &str,
    message: &str,
    finalized_response: &str,
    tools_payload: Value,
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    turn_transaction: &Value,
    terminal_transcript: &[Value],
) -> Value {
    let prior_messages = session_messages(&load_session_state(root, agent_id));
    let previous_assistant = latest_assistant_message_text(&prior_messages);
    let previous_user = latest_user_message_text(&prior_messages);
    let workflow_visibility = workflow_visibility_payload(response_workflow, response_finalization);
    let mut turn_receipt = append_turn_message(root, agent_id, message, finalized_response);
    turn_receipt["assistant_turn_patch"] = persist_last_assistant_turn_metadata(
        root,
        agent_id,
        finalized_response,
        &json!({
            "tools": tools_payload,
            "response_workflow": response_workflow.clone(),
            "response_finalization": response_finalization.clone(),
            "process_summary": process_summary.clone(),
            "workflow_visibility": workflow_visibility.clone(),
            "turn_transaction": turn_transaction.clone(),
            "terminal_transcript": terminal_transcript.to_vec()
        }),
    );
    turn_receipt["process_summary"] = process_summary.clone();
    turn_receipt["workflow_visibility"] = workflow_visibility;
    turn_receipt["response_finalization"] = response_finalization.clone();
    turn_receipt["live_eval_monitor"] = live_eval_monitor_turn(root, agent_id, message, finalized_response, &previous_assistant, &previous_user, response_finalization);
    turn_receipt
}
fn enrich_tool_completion_receipt(tool_completion: Value, response_tools: &[Value]) -> Value {
    let mut enriched = if tool_completion.is_object() {
        tool_completion
    } else {
        json!({})
    };
    let steps = tool_completion_live_steps(response_tools);
    let tool_attempts = response_tools
        .iter()
        .filter_map(enrich_tool_attempt_receipt_from_row)
        .take(16)
        .collect::<Vec<_>>();
    let evidence_refs_used = tool_attempts
        .iter()
        .filter_map(|row| row.get("evidence_refs").and_then(Value::as_array))
        .flatten()
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    let mut citations = compact_citations_from_evidence_refs(&evidence_refs_used);
    let mut citation_projection_source = "tool_completion_evidence_refs";
    if citations.is_empty() {
        citations = compact_citations_from_tool_attempts(&tool_attempts);
        citation_projection_source = "tool_completion_candidate_rows";
    }
    let live_tool_status = steps
        .first()
        .and_then(|row| row.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    enriched["live_tool_status"] = json!(clean_text(&live_tool_status, 180));
    enriched["live_tool_steps"] = Value::Array(steps);
    enriched["tool_attempts"] = Value::Array(tool_attempts);
    if !evidence_refs_used.is_empty() {
        enriched["evidence_refs_used"] = Value::Array(evidence_refs_used);
    }
    if !citations.is_empty() {
        enriched["citations"] = Value::Array(citations.clone());
        enriched["source_refs"] = Value::Array(citations);
        enriched["citation_projection_source"] = json!(citation_projection_source);
    }
    enriched["live_status_source"] = json!("tool_completion_receipt_v1");
    enriched
}

fn compact_citations_from_evidence_refs(evidence_refs: &[Value]) -> Vec<Value> {
    let mut citations = Vec::<Value>::new();
    for evidence_ref in evidence_refs {
        if citations.len() >= 8 {
            break;
        }
        if let Some(citation) = compact_citation_from_evidence_ref(citations.len() + 1, evidence_ref)
        {
            citations.push(citation);
        }
    }
    citations
}

fn compact_citations_from_tool_attempts(tool_attempts: &[Value]) -> Vec<Value> {
    let mut citations = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    for attempt in tool_attempts {
        for key in ["evidence_refs", "search_results", "provider_results"] {
            let Some(rows) = attempt.get(key).and_then(Value::as_array) else {
                continue;
            };
            for row in rows {
                if citations.len() >= 8 {
                    return citations;
                }
                let Some(citation) = compact_citation_from_evidence_ref(citations.len() + 1, row)
                else {
                    continue;
                };
                let dedupe_key = compact_citation_dedupe_key(&citation);
                if dedupe_key.is_empty() || seen.insert(dedupe_key) {
                    citations.push(citation);
                }
            }
        }
    }
    citations
}

fn compact_citation_dedupe_key(citation: &Value) -> String {
    clean_text(
        citation
            .get("locator")
            .or_else(|| citation.get("title"))
            .or_else(|| citation.get("snippet"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        500,
    )
    .to_ascii_lowercase()
}

fn compact_citation_from_evidence_ref(index: usize, evidence_ref: &Value) -> Option<Value> {
    if let Some(raw) = evidence_ref.as_str() {
        let value = clean_text(raw, 500);
        if value.is_empty() {
            return None;
        }
        let mut citation = serde_json::Map::new();
        citation.insert("citation_id".to_string(), json!(format!("source_{index}")));
        if value.starts_with("http://") || value.starts_with("https://") {
            citation.insert("locator".to_string(), json!(value));
        } else {
            citation.insert("source_ref".to_string(), json!(value));
        }
        return Some(Value::Object(citation));
    }
    let locator = clean_text(
        evidence_ref
            .get("locator")
            .or_else(|| evidence_ref.get("url"))
            .or_else(|| evidence_ref.get("source_url"))
            .or_else(|| evidence_ref.get("link"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        500,
    );
    let title = clean_text(
        evidence_ref
            .get("title")
            .or_else(|| evidence_ref.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let snippet = clean_text(
        evidence_ref
            .get("snippet")
            .or_else(|| evidence_ref.get("summary"))
            .or_else(|| evidence_ref.get("source_excerpt"))
            .or_else(|| evidence_ref.get("description"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    if locator.is_empty() && title.is_empty() && snippet.is_empty() {
        return None;
    }
    let source_domain = clean_text(
        evidence_ref
            .get("source_domain")
            .or_else(|| evidence_ref.get("domain"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let source_kind = clean_text(
        evidence_ref
            .get("source_kind")
            .or_else(|| evidence_ref.get("source_class"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let mut citation = serde_json::Map::new();
    citation.insert("citation_id".to_string(), json!(format!("source_{index}")));
    if !title.is_empty() {
        citation.insert("title".to_string(), json!(title));
    }
    if !locator.is_empty() {
        citation.insert("locator".to_string(), json!(locator));
    }
    if !source_domain.is_empty() {
        citation.insert("source_domain".to_string(), json!(source_domain));
    }
    if !source_kind.is_empty() {
        citation.insert("source_kind".to_string(), json!(source_kind));
    }
    if !snippet.is_empty() {
        citation.insert("snippet".to_string(), json!(snippet));
    }
    Some(Value::Object(citation))
}

fn compact_source_grounding_label(citation: &Value) -> String {
    let source_domain = clean_text(
        citation
            .get("source_domain")
            .or_else(|| citation.get("domain"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    if !source_domain.is_empty() && !source_domain.eq_ignore_ascii_case("news.google.com") {
        return source_domain;
    }
    let locator = clean_text(
        citation
            .get("locator")
            .or_else(|| citation.get("source_ref"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    let locator_host = locator
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or("");
    if !locator_host.is_empty() && !locator_host.eq_ignore_ascii_case("news.google.com") {
        return clean_text(locator_host, 80);
    }
    let title = clean_text(citation.get("title").and_then(Value::as_str).unwrap_or(""), 120);
    if let Some((_, tail)) = title.rsplit_once(" - ") {
        let tail = clean_text(tail, 80);
        if !tail.is_empty() {
            return tail;
        }
    }
    title
}

fn compact_source_grounding_sentence(response_tools: &[Value], max_items: usize) -> String {
    let tool_attempts = response_tools
        .iter()
        .filter_map(enrich_tool_attempt_receipt_from_row)
        .take(16)
        .collect::<Vec<_>>();
    let mut labels = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for citation in compact_citations_from_tool_attempts(&tool_attempts) {
        if labels.len() >= max_items.clamp(1, 4) {
            break;
        }
        let source_kind = clean_text(
            citation
                .get("source_kind")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        )
        .to_ascii_lowercase();
        if source_kind.contains("low_confidence") {
            continue;
        }
        let label = compact_source_grounding_label(&citation);
        if label.is_empty() {
            continue;
        }
        let dedupe = label.to_ascii_lowercase();
        if seen.insert(dedupe) {
            labels.push(label);
        }
    }
    if labels.is_empty() {
        return String::new();
    }
    if labels.len() == 1 {
        return format!("Source used here: {}.", labels[0]);
    }
    format!("Sources used here include {}.", join_compact_label_list(&labels))
}

fn enrich_tool_attempt_receipt_from_row(row: &Value) -> Option<Value> {
    let mut receipt = row
        .get("tool_attempt_receipt")
        .cloned()
        .or_else(|| row.pointer("/tool_attempt/attempt").cloned())
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| synthetic_tool_attempt_receipt_from_row(row));
    let Some(obj) = receipt.as_object_mut() else {
        return Some(receipt);
    };
    for key in [
        "search_results",
        "provider_results",
        "evidence_refs",
        "evidence_pack",
        "evidence_pack_candidates",
        "evidence_claims",
        "answer_units",
        "tool_result_quality",
    ] {
        if let Some(value) = row.get(key).cloned() {
            obj.insert(key.to_string(), value);
        }
    }
    if obj.get("evidence_refs").is_none() {
        for key in [
            "evidence_pack",
            "evidence_pack_candidates",
            "search_results",
            "provider_results",
        ] {
            let Some(rows) = row.get(key).and_then(Value::as_array) else {
                continue;
            };
            let projected = rows.iter().take(6).cloned().collect::<Vec<_>>();
            if !projected.is_empty() {
                obj.insert("evidence_refs".to_string(), Value::Array(projected));
                break;
            }
        }
    }
    Some(receipt)
}

fn synthetic_tool_attempt_receipt_from_row(row: &Value) -> Value {
    let tool_name = normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or("tool"));
    let is_error = row.get("is_error").and_then(Value::as_bool).unwrap_or(false);
    let status = clean_text(
        row.get("status")
            .and_then(Value::as_str)
            .unwrap_or(if is_error { "error" } else { "ok" }),
        80,
    );
    let result_excerpt = first_sentence(
        &clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 2_000),
        260,
    );
    json!({
        "tool_name": if tool_name.is_empty() { "tool" } else { tool_name.as_str() },
        "status": if status.is_empty() { if is_error { "error" } else { "ok" } } else { status.as_str() },
        "outcome": if is_error { "error" } else { "ok" },
        "is_error": is_error,
        "reason": result_excerpt,
        "backend": "tool_card",
        "synthetic": true
    })
}

#[cfg(test)]
mod tool_completion_live_status_tests {
    use super::*;

    #[test]
    fn builds_live_status_for_known_tools() {
        let tools = vec![json!({
            "name": "web_search",
            "input": "{\"query\":\"latest stack\"}",
            "result": "ok",
            "is_error": false
        })];
        let enriched =
            enrich_tool_completion_receipt(json!({"completion_state":"reported_findings"}), &tools);
        assert_eq!(
            enriched
                .get("live_tool_status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "Searching internet"
        );
        let steps = enriched
            .get("live_tool_steps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn skips_thought_process_for_live_status() {
        let tools = vec![json!({
            "name": "thought_process",
            "input": "Thinking about next step.",
            "result": "",
            "is_error": false
        })];
        let enriched =
            enrich_tool_completion_receipt(json!({"completion_state":"reported_reason"}), &tools);
        let steps = enriched
            .get("live_tool_steps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(steps.is_empty());
        assert_eq!(
            enriched
                .get("live_tool_status")
                .and_then(Value::as_str)
                .unwrap_or(""),
            ""
        );
    }

    #[test]
    fn builds_terminal_transcript_rows_from_terminal_tools() {
        let rows = tool_terminal_transcript(&[json!({
            "name": "terminal_exec",
            "input": "{\"command\":\"printf 'ok'\",\"cwd\":\"/tmp\"}",
            "result": "ok",
            "is_error": false
        })]);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("command").and_then(Value::as_str),
            Some("printf 'ok'")
        );
        assert_eq!(rows[0].get("output").and_then(Value::as_str), Some("ok"));
        assert_eq!(rows[0].get("cwd").and_then(Value::as_str), Some("/tmp"));
    }

    #[test]
    fn carries_tool_attempt_receipts_into_tool_completion() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "terminal_exec",
                "input": "{\"command\":\"ls\"}",
                "result": "permission denied",
                "is_error": true,
                "tool_attempt_receipt": {
                    "tool_name": "terminal_exec",
                    "status": "blocked",
                    "outcome": "blocked",
                    "reason_code": "caller_not_authorized",
                    "reason": "caller_not_authorized",
                    "backend": "governed_terminal",
                    "required_args": ["command"],
                    "discoverable": true
                }
            })],
        );
        assert_eq!(
            enriched
                .get("tool_attempts")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn carries_hidden_search_artifacts_into_tool_completion_attempts() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "web_search",
                "input": "{\"query\":\"mastra langgraph comparison\"}",
                "result": "low signal",
                "is_error": false,
                "search_results": [
                    {
                        "locator": "https://mastra.ai/docs/overview",
                        "snippet": "Mastra docs overview"
                    }
                ],
                "provider_results": [
                    {
                        "provider": "bing_rss",
                        "summary": "One relevant Mastra result was retained."
                    }
                ],
                "tool_attempt_receipt": {
                    "tool_name": "web_search",
                    "status": "ok",
                    "outcome": "ok"
                }
            })],
        );
        assert_eq!(
            enriched
                .pointer("/tool_attempts/0/search_results/0/locator")
                .and_then(Value::as_str),
            Some("https://mastra.ai/docs/overview")
        );
        assert_eq!(
            enriched
                .pointer("/tool_attempts/0/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
    }

    #[test]
    fn projects_evidence_refs_as_final_package_citations() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_refs": [
                    {
                        "title": "LlamaIndex docs",
                        "locator": "https://docs.llamaindex.ai/",
                        "source_domain": "docs.llamaindex.ai",
                        "snippet": "LlamaIndex focuses on data ingestion and retrieval workflows."
                    }
                ]
            })],
        );
        assert_eq!(
            enriched
                .pointer("/citations/0/title")
                .and_then(Value::as_str),
            Some("LlamaIndex docs")
        );
        let finalization = build_response_finalization_payload(
            "workflow_authored",
            false,
            false,
            &enriched,
            false,
            false,
            false,
            false,
            &json!({}),
            &json!({}),
            &json!({}),
        );
        assert_eq!(
            finalization
                .pointer("/citations/0/locator")
                .and_then(Value::as_str),
            Some("https://docs.llamaindex.ai/")
        );
        assert_eq!(
            finalization
                .pointer("/source_refs/0/source_domain")
                .and_then(Value::as_str),
            Some("docs.llamaindex.ai")
        );
    }

    #[test]
    fn projects_candidate_rows_as_final_package_citations_when_refs_are_thin() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_refs": [
                    {"status": "ok"}
                ],
                "search_results": [
                    {
                        "title": "CrewAI changelog",
                        "url": "https://docs.crewai.com/changelog",
                        "description": "Recent CrewAI releases and production behavior changes."
                    }
                ],
                "provider_results": [
                    {
                        "title": "CrewAI changelog",
                        "url": "https://docs.crewai.com/changelog"
                    }
                ]
            })],
        );
        assert_eq!(
            enriched
                .pointer("/citation_projection_source")
                .and_then(Value::as_str),
            Some("tool_completion_candidate_rows")
        );
        assert_eq!(
            enriched
                .pointer("/citations/0/locator")
                .and_then(Value::as_str),
            Some("https://docs.crewai.com/changelog")
        );
    }

    #[test]
    fn projects_evidence_pack_rows_as_final_package_citations() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [{
                    "title": "Agent platform release notes",
                    "locator": "https://docs.example.test/agents/release-notes",
                    "source_domain": "docs.example.test",
                    "source_kind": "official_docs",
                    "relevant_extract": "The release notes describe agent runtime updates."
                }]
            })],
        );
        assert_eq!(
            enriched
                .pointer("/evidence_refs_used/0/locator")
                .and_then(Value::as_str),
            Some("https://docs.example.test/agents/release-notes")
        );
        assert_eq!(
            enriched
                .pointer("/source_refs/0/source_domain")
                .and_then(Value::as_str),
            Some("docs.example.test")
        );
        assert_eq!(
            enriched
                .pointer("/citation_projection_source")
                .and_then(Value::as_str),
            Some("tool_completion_evidence_refs")
        );
    }

    #[test]
    fn projects_string_evidence_refs_as_source_refs() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_findings"}),
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_refs": [
                    "https://langchain-ai.github.io/langgraph/concepts/durable_execution/"
                ]
            })],
        );
        assert_eq!(
            enriched
                .pointer("/source_refs/0/locator")
                .and_then(Value::as_str),
            Some("https://langchain-ai.github.io/langgraph/concepts/durable_execution/")
        );
    }

    #[test]
    fn finalization_appends_compact_source_grounding_when_answer_lacks_it() {
        let (finalized, report, _) = enforce_user_facing_finalization_contract(
            "Compare Alpha and Beta for production.",
            "Alpha looks steadier for production, while Beta remains more flexible.".to_string(),
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_refs": [
                    {
                        "title": "Alpha docs",
                        "locator": "https://docs.alpha.dev/production",
                        "source_domain": "docs.alpha.dev",
                        "snippet": "Alpha emphasizes production reliability."
                    },
                    {
                        "title": "Beta engineering blog",
                        "locator": "https://engineering.beta.dev/flexibility",
                        "source_domain": "engineering.beta.dev",
                        "snippet": "Beta favors flexibility and experimentation."
                    }
                ]
            })],
        );
        assert!(finalized.contains("Sources used here include"));
        assert!(finalized.contains("docs.alpha.dev"));
        assert_eq!(
            report
                .get("source_grounding_repair_used")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn synthesizes_tool_attempt_receipt_for_receiptless_error_rows() {
        let enriched = enrich_tool_completion_receipt(
            json!({"completion_state":"reported_reason"}),
            &[json!({
                "name": "batch_query",
                "input": "{\"query\":\"compare agent frameworks\"}",
                "result": "",
                "status": "error",
                "is_error": true,
                "provider_results": [
                    {
                        "provider": "web",
                        "query": "compare agent frameworks",
                        "status": "error",
                        "error": "tool_execution_failed"
                    }
                ]
            })],
        );
        assert_eq!(
            enriched
                .pointer("/tool_attempts/0/tool_name")
                .and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            enriched
                .pointer("/tool_attempts/0/synthetic")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            enriched
                .pointer("/tool_attempts/0/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("web")
        );
        assert_eq!(
            enriched
                .pointer("/evidence_refs_used/0/provider")
                .and_then(Value::as_str),
            Some("web")
        );
    }

    #[test]
    fn response_tools_summary_uses_retained_evidence_rows_when_result_is_thin() {
        let summary = response_tools_summary_for_user(
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "result": "Key findings: AutoGen - Microsoft Research: AutoGen is an open-source programming framework for building AI agents.",
                "evidence_refs": [
                    {
                        "title": "LangGraph overview",
                        "snippet": "LangGraph focuses on long-running, stateful agent workflows.",
                        "url": "https://langchain-ai.github.io/langgraph/"
                    },
                    {
                        "title": "CrewAI docs",
                        "snippet": "CrewAI emphasizes role-based multi-agent crews and orchestration.",
                        "url": "https://docs.crewai.com/"
                    },
                    {
                        "title": "OpenHands docs",
                        "snippet": "OpenHands is oriented toward software-development task execution.",
                        "url": "https://docs.all-hands.dev/"
                    }
                ]
            })],
            4,
        );
        assert!(summary.contains("AutoGen"));
        assert!(summary.contains("LangGraph"));
        assert!(summary.contains("CrewAI"));
        assert!(summary.contains("OpenHands"));
    }
}
