fn direct_tool_intent_from_user_message(message: &str) -> Option<(String, Value)> {
    let trimmed = message.trim();
    if !trimmed.starts_with('/') {
        let lowered = clean_text(trimmed, 400).to_ascii_lowercase();
        if matches!(
            lowered.as_str(),
            "undo" | "undo that" | "undo last turn" | "rewind that" | "rollback that"
        ) {
            return Some(("session_rollback_last_turn".to_string(), json!({})));
        }
        // Conversational turns stay model-first. Even explicit tool syntax in chat is now
        // surfaced as workflow/catalog guidance for the LLM instead of a pre-LLM direct route.
        return None;
    }
    let mut split = trimmed.splitn(2, char::is_whitespace);
    let command = split
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let arg = split.next().map(str::trim).unwrap_or("");
    match command.as_str() {
        "/undo" | "/rewind" | "/rollback" => {
            Some(("session_rollback_last_turn".to_string(), json!({})))
        }
        "/cron" | "/schedule" => cron_tool_request_from_args(arg),
        _ => None,
    }
}

fn response_tools_failure_reason_for_user(response_tools: &[Value], max_items: usize) -> String {
    let limit = max_items.clamp(1, 8);
    let mut lines = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for row in response_tools {
        let raw_name = clean_text(
            row.get("name").and_then(Value::as_str).unwrap_or("tool"),
            80,
        )
        .replace('_', " ");
        let tool_name = if raw_name.is_empty() {
            "tool".to_string()
        } else {
            raw_name
        };
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 1_000);
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 1_200);
        let status_code = row
            .get("status_code")
            .and_then(Value::as_u64)
            .or_else(|| row.get("http_status").and_then(Value::as_u64));
        let blocked = row
            .get("blocked")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || status_code.is_some_and(|code| matches!(code, 401 | 403 | 404 | 422 | 429));
        let failure_reason = if blocked {
            if !error.is_empty() {
                first_sentence(&error, 220)
            } else if !result.is_empty() {
                first_sentence(&result, 220)
            } else {
                "tool execution was blocked".to_string()
            }
        } else if matches!(
            status.as_str(),
            "error" | "failed" | "execution_error" | "timeout" | "no_response"
        ) {
            if !error.is_empty() {
                first_sentence(&error, 220)
            } else if !result.is_empty() {
                first_sentence(&result, 220)
            } else {
                "tool execution failed".to_string()
            }
        } else if matches!(status.as_str(), "low_signal" | "no_results" | "partial_no_results")
            || response_looks_like_tool_ack_without_findings(&result)
            || response_is_no_findings_placeholder(&result)
        {
            if !result.is_empty() {
                first_sentence(&result, 220)
            } else {
                "tool result was too narrow".to_string()
            }
        } else if status_code.map(|value| value >= 500).unwrap_or(false) && !result.is_empty() {
            first_sentence(&result, 220)
        } else if status_code.is_some() && !error.is_empty() {
            first_sentence(&error, 220)
        } else {
            String::new()
        };
        if failure_reason.is_empty() {
            continue;
        }
        let line = format!("{}: {}", tool_name, failure_reason);
        if seen.insert(line.clone()) {
            lines.push(line);
        }
        if lines.len() >= limit {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    clean_text(&format!("Tool failures: {}", lines.join(" | ")), 6_000)
}

fn workspace_analyze_intent_from_message(
    trimmed: &str,
    lowered: &str,
) -> Option<(String, Value)> {
    if lowered.is_empty() {
        return None;
    }
    let asks_ls = lowered == "ls"
        || lowered.starts_with("ls ")
        || lowered.contains(" run ls")
        || lowered.contains("list files")
        || lowered.contains("show files")
        || lowered.contains("directory listing")
        || lowered.contains("folder listing");
    let mentions_workspace = lowered.contains("workspace")
        || lowered.contains("repo")
        || lowered.contains("repository")
        || lowered.contains("project directory")
        || lowered.contains("project folder")
        || lowered.contains("this directory");
    let asks_file_surface = lowered.contains("files")
        || lowered.contains("logs")
        || lowered.contains("directories")
        || lowered.contains("folders")
        || lowered.contains("tree");
    let asks_analysis = lowered.contains("analy")
        || lowered.contains("analyse")
        || lowered.contains("parse")
        || lowered.contains("inspect")
        || lowered.contains("scan")
        || lowered.contains("summarize")
        || lowered.contains("tell me about");
    if !(asks_ls || (mentions_workspace && (asks_file_surface || asks_analysis))) {
        return None;
    }
    let query = clean_text(trimmed, 600);
    if query.is_empty() {
        return None;
    }
    Some(("workspace_analyze".to_string(), json!({ "query": query })))
}

fn message_explicitly_disallows_tool_calls(message: &str) -> bool {
    let lowered = clean_text(message, 400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("dont use a tool")
        || lowered.contains("don't use a tool")
        || lowered.contains("do not use a tool")
        || lowered.contains("dont call a tool")
        || lowered.contains("don't call a tool")
        || lowered.contains("do not call a tool")
        || lowered.contains("dont run tools")
        || lowered.contains("don't run tools")
        || lowered.contains("do not run tools")
        || lowered.contains("without running tools")
        || lowered.contains("without tool")
        || lowered.contains("no tool call")
        || lowered.contains("no tools yet")
        || lowered.contains("dry run only")
        || lowered.contains("just talk to me")
        || lowered.contains("just answer")
}

fn message_is_meta_control_turn(message: &str) -> bool {
    let _ = message;
    false
}

fn message_requests_local_file_mutation(message: &str) -> bool {
    let _ = message;
    false
}

fn message_has_explicit_year_scope(message: &str) -> bool {
    clean_text(message, 400)
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() == 4)
        .filter_map(|token| token.parse::<u32>().ok())
        .any(|year| (1900..=2100).contains(&year))
}

fn message_has_external_information_request_frame(lowered: &str) -> bool {
    lowered.ends_with('?')
        || [
            "what ",
            "what's ",
            "what is ",
            "what are ",
            "who ",
            "when ",
            "where ",
            "which ",
            "how ",
            "give me ",
            "tell me ",
            "show me ",
            "find ",
            "look up ",
            "search ",
            "summarize ",
            "summarise ",
            "list ",
            "compare ",
            "explain ",
            "update me ",
        ]
        .iter()
        .any(|prefix| lowered.starts_with(prefix))
}

fn message_has_external_information_time_cues(message: &str, lowered: &str) -> bool {
    [
        "today",
        "yesterday",
        "tomorrow",
        "this week",
        "this month",
        "this year",
        "right now",
        "as of ",
        "current ",
        "currently ",
        "latest ",
        "recent ",
        "news",
        "update",
        "updates",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
        || message_has_explicit_year_scope(message)
}

fn message_has_external_information_relation_cues(lowered: &str) -> bool {
    [
        "compare",
        "comparison",
        " versus ",
        " vs ",
        "difference between",
        "public sentiment",
        "sentiment",
        "reviews",
        "pricing",
        "benchmark",
        "benchmarks",
        "rank",
        "ranking",
        "landscape",
        "state of",
        "release",
        "releases",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn message_refers_to_local_workspace_or_code(trimmed: &str, lowered: &str) -> bool {
    if workspace_analyze_intent_from_message(trimmed, lowered).is_some() {
        return true;
    }
    if [
        "this system",
        "our system",
        "this workspace",
        "this repo",
        "this repository",
        "this code",
        "read this file",
        "open this file",
        "shell command",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
    {
        return true;
    }
    let tokens = lowered
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<std::collections::HashSet<_>>();
    [
        "workspace",
        "repo",
        "repository",
        "project",
        "codebase",
        "patch",
        "refactor",
        "compile",
        "build",
        "cargo",
        "rust",
        "python",
        "typescript",
        "javascript",
        "function",
        "module",
        "class",
        "crate",
        "test",
        "api",
        "terminal",
    ]
    .iter()
    .any(|needle| tokens.contains(*needle))
}

fn message_requires_information_search(message: &str) -> bool {
    let trimmed = clean_text(message, 2_200);
    if trimmed.is_empty()
        || message_is_tooling_status_check(&trimmed)
        || message_is_meta_control_turn(&trimmed)
        || message_explicitly_disallows_tool_calls(&trimmed)
    {
        return false;
    }
    if natural_web_search_query_from_message(&trimmed).is_some() {
        return true;
    }
    let lowered = trimmed.to_ascii_lowercase();
    if message_refers_to_local_workspace_or_code(&trimmed, &lowered) {
        return false;
    }
    let request_frame = message_has_external_information_request_frame(&lowered);
    let time_cues = message_has_external_information_time_cues(&trimmed, &lowered);
    let relation_cues = message_has_external_information_relation_cues(&lowered);
    time_cues || (request_frame && relation_cues)
}

fn inline_tool_calls_allowed_for_user_message(message: &str) -> bool {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty() {
        return false;
    }
    if message_is_tooling_status_check(&cleaned) {
        return false;
    }
    if message_is_meta_control_turn(&cleaned) {
        return false;
    }
    if message_explicitly_disallows_tool_calls(&cleaned) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let is_explicit_slash_tool_turn = lowered.starts_with("/file")
        || lowered.starts_with("/search")
        || lowered.starts_with("/browse")
        || lowered.starts_with("/batch")
        || lowered.starts_with("/tool");
    is_explicit_slash_tool_turn
}
