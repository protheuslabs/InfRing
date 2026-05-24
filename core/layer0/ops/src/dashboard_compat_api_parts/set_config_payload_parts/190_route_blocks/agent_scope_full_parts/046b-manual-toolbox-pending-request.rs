// Layer ownership: core/layer0/ops (authoritative)

fn workflow_has_manual_toolbox_candidate_menu(workflow: &Value) -> bool {
    workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        == Some("first_gate_pending_llm_tool_choice")
        || workflow
            .get("system_events")
            .and_then(Value::as_array)
            .map(|events| {
                events.iter().any(|event| {
                    event
                        .get("kind")
                        .or_else(|| event.get("name"))
                        .or_else(|| event.get("type"))
                        .and_then(Value::as_str)
                        == Some("manual_toolbox_candidate_menu")
                })
            })
            .unwrap_or(false)
}

fn record_manual_toolbox_pending_request(workflow: &mut Value, response_text: &str, message: &str) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    let pending_request = manual_toolbox_pending_request_from_response(response_text, message);
    let Some(pending_request) = pending_request else {
        return;
    };
    record_manual_toolbox_pending_request_value(workflow, pending_request);
}

fn record_manual_toolbox_pending_request_value(workflow: &mut Value, mut pending_request: Value) {
    if workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
    {
        return;
    }
    if let Some((tool_name, input)) = pending_request
        .get("tool_name")
        .and_then(Value::as_str)
        .zip(pending_request.get("input").cloned())
    {
        if let Ok(repaired_input) =
            crate::infring_tooling_core_v1_bridge::repair_and_validate_args(tool_name, &input)
        {
            pending_request["input"] = repaired_input;
        }
    }
    workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
    workflow["response"] = Value::String(String::new());
    workflow["visible_response_source"] = Value::String("json_private_tool_request".to_string());
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("first_gate_pending_tool_confirmation".to_string());
    if let Some(events) = workflow
        .get_mut("system_events")
        .and_then(Value::as_array_mut)
    {
        events.push(turn_workflow_event(
            "manual_toolbox_pending_tool_request",
            pending_request,
        ));
    }
}

fn manual_toolbox_pending_request_from_latent_candidates(
    latent_tool_candidates: &Value,
    message: &str,
) -> Option<Value> {
    let candidates = latent_tool_candidates.as_array()?;
    let valid = candidates
        .iter()
        .filter_map(|candidate| {
            let workflow_only = candidate
                .get("workflow_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !workflow_only {
                return None;
            }
            let family_key = clean_text(
                candidate
                    .get("selected_tool_family")
                    .or_else(|| candidate.get("tool_family"))
                    .or_else(|| candidate.get("family"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let tool_key = clean_text(
                candidate
                    .get("selected_tool_key")
                    .or_else(|| candidate.get("tool_key"))
                    .or_else(|| candidate.get("tool_name"))
                    .or_else(|| candidate.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            let tool_label = clean_text(
                candidate
                    .get("selected_tool_label")
                    .or_else(|| candidate.get("label"))
                    .or_else(|| candidate.get("tool_label"))
                    .and_then(Value::as_str)
                    .unwrap_or(&tool_key),
                120,
            );
            let input = candidate
                .get("input")
                .or_else(|| candidate.get("request_payload"))
                .or_else(|| candidate.get("proposed_input"))
                .cloned()
                .filter(Value::is_object)?;
            manual_toolbox_pending_request_from_parts(
                &family_key,
                &tool_key,
                &tool_label,
                input,
                message,
            )
            .map(|mut pending| {
                pending["source"] = Value::String("latent_candidate_recovery".to_string());
                pending["recovery_contract"] = Value::String(
                    "single_valid_workflow_only_candidate_after_private_gate_failure_or_terminal_invariant_recovery".to_string(),
                );
                pending
            })
        })
        .collect::<Vec<_>>();
    if valid.len() == 1 {
        valid.into_iter().next()
    } else {
        None
    }
}

fn workflow_repair_recovered_request_payload(
    family_key: &str,
    tool_key: &str,
    input: Value,
    message: &str,
) -> Value {
    // Raw-message recovery must stay mechanical and contract-driven. Do not
    // infer entities, aliases, facets, comparison modes, or temporal classes
    // from user prose in Rust.
    workflow_reconcile_relative_time_window_payload(family_key, tool_key, input, message)
}

fn workflow_reconcile_relative_time_window_payload(
    family_key: &str,
    tool_key: &str,
    mut input: Value,
    message: &str,
) -> Value {
    if !workflow_is_web_query_tool(family_key, tool_key)
        || !workflow_message_has_relative_time_window(message)
    {
        return input;
    }
    let Some((current_year, current_month)) = workflow_runtime_temporal_context() else {
        return input;
    };
    let Some(input_object) = input.as_object_mut() else {
        return input;
    };
    let mut changed = false;
    for key in ["query", "queries", "keywords"] {
        if let Some(value) = input_object.get_mut(key) {
            changed |= workflow_reconcile_relative_time_window_value(
                value,
                message,
                current_year,
                current_month,
            );
        }
    }
    if changed {
        input_object.insert(
            "query_metadata_policy".to_string(),
            json!({
                "temporal_coherence": {
                    "status": "reconciled_relative_time_window",
                    "runtime_year": current_year,
                    "runtime_month": current_month,
                    "rule": "replace_unrequested_stale_year_or_month_year_in_query_fields"
                }
            }),
        );
    }
    input
}

fn workflow_is_web_query_tool(family_key: &str, tool_key: &str) -> bool {
    let family = normalized_workflow_token(family_key);
    let tool = normalized_workflow_token(tool_key);
    family == "web research" && (tool == "batch query" || tool == "web search")
}

fn workflow_message_has_relative_time_window(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    [
        "this month",
        "this week",
        "today",
        "yesterday",
        "tomorrow",
        "latest",
        "news today",
        "news from this week",
        "as of now",
        "right now",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn workflow_runtime_temporal_context() -> Option<(i32, &'static str)> {
    let now = crate::now_iso();
    let year = now.get(0..4)?.parse::<i32>().ok()?;
    let month = match now.get(5..7)? {
        "01" => "January",
        "02" => "February",
        "03" => "March",
        "04" => "April",
        "05" => "May",
        "06" => "June",
        "07" => "July",
        "08" => "August",
        "09" => "September",
        "10" => "October",
        "11" => "November",
        "12" => "December",
        _ => return None,
    };
    Some((year, month))
}

fn workflow_reconcile_relative_time_window_value(
    value: &mut Value,
    message: &str,
    current_year: i32,
    current_month: &str,
) -> bool {
    match value {
        Value::String(raw) => {
            let repaired = workflow_reconcile_relative_time_window_text(
                raw,
                message,
                current_year,
                current_month,
            );
            if repaired != *raw {
                *raw = repaired;
                true
            } else {
                false
            }
        }
        Value::Array(rows) => {
            let mut changed = false;
            for row in rows.iter_mut() {
                changed |= workflow_reconcile_relative_time_window_value(
                    row,
                    message,
                    current_year,
                    current_month,
                );
            }
            changed
        }
        _ => false,
    }
}

fn workflow_reconcile_relative_time_window_text(
    raw: &str,
    message: &str,
    current_year: i32,
    current_month: &str,
) -> String {
    let mut out = raw.to_string();
    let message_lower = message.to_ascii_lowercase();
    for stale_year in (current_year.saturating_sub(3))..current_year {
        let stale_year_text = stale_year.to_string();
        if message_lower.contains(&stale_year_text) {
            continue;
        }
        let current_year_text = current_year.to_string();
        for month in workflow_month_names() {
            let lower_month = month.to_ascii_lowercase();
            for month_variant in [*month, lower_month.as_str()] {
                let stale_month_year = format!("{month_variant} {stale_year_text}");
                if out.contains(&stale_month_year) {
                    out = out.replace(
                        &stale_month_year,
                        &format!("{current_month} {current_year_text}"),
                    );
                }
            }
        }
        out = workflow_replace_unrequested_year_token(&out, stale_year, current_year);
    }
    clean_text(&out, 1_200)
}

fn workflow_replace_unrequested_year_token(raw: &str, stale_year: i32, current_year: i32) -> String {
    let stale = stale_year.to_string();
    let current = current_year.to_string();
    let mut out = String::with_capacity(raw.len());
    let mut token = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch);
        } else {
            if token == stale {
                out.push_str(&current);
            } else {
                out.push_str(&token);
            }
            token.clear();
            out.push(ch);
        }
    }
    if token == stale {
        out.push_str(&current);
    } else {
        out.push_str(&token);
    }
    out
}

fn workflow_month_names() -> &'static [&'static str] {
    &[
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
}

fn workflow_batch_query_recovery_repair_policy(family_key: &str) -> Option<Value> {
    let contract = workflow_tool_menu_contract_for_family(family_key);
    let policy = contract.pointer(
        "/latent_candidate_recovery_contract/request_contract_repair/batch_query",
    )?;
    if policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Some(policy.clone())
    } else {
        None
    }
}

fn workflow_recovery_policy_terms(policy: &Value, field: &str, limit: usize) -> Vec<String> {
    policy
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, 80))
                .filter(|raw| !raw.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_json_array_is_empty(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
}

fn workflow_recovery_request_needs_metadata(
    query: &str,
    message: &str,
    entities: &[String],
) -> bool {
    let lowered = clean_text(&format!("{query} {message}"), 2_400).to_ascii_lowercase();
    entities.len() > 1
        || [
            "research",
            "compare",
            "comparison",
            " versus ",
            " vs ",
            "rank",
            "ranking",
            "evaluate",
            "assessment",
            "assess",
            "best",
            "defensible",
            "recommend",
            "selection",
            "landscape",
            "benchmark",
            "benchmarks",
            "current",
            "currently",
            "recent",
            "latest",
            "right now",
            "as of",
            "maturity",
            "risk",
            "security",
            "marketing",
            "production",
            "strength",
            "weak",
            "tradeoff",
            "tradeoffs",
        ]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn workflow_recovery_entity_terms(query: &str, message: &str) -> Vec<String> {
    let raw = clean_text(&format!("{query} {message}"), 2_400);
    let mut out = Vec::<String>::new();
    let mut current = Vec::<String>::new();
    for raw_token in raw.split_whitespace() {
        let entity_boundary = workflow_recovery_entity_boundary_after(raw_token);
        let token = workflow_recovery_clean_token(raw_token);
        if token.is_empty() {
            workflow_recovery_flush_entity_phrase(&mut out, &mut current);
            continue;
        }
        if workflow_recovery_token_looks_like_entity(&token) {
            current.push(token);
            if entity_boundary {
                workflow_recovery_flush_entity_phrase(&mut out, &mut current);
            }
            continue;
        }
        workflow_recovery_flush_entity_phrase(&mut out, &mut current);
    }
    workflow_recovery_flush_entity_phrase(&mut out, &mut current);
    workflow_recovery_dedupe_limit(out, 8)
}

fn workflow_recovery_clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        ch.is_ascii_punctuation() && !matches!(ch, '-' | '_' | '/' | '+')
    })
    .chars()
    .filter(|ch| !ch.is_control())
    .collect::<String>()
}

fn workflow_recovery_entity_boundary_after(raw: &str) -> bool {
    raw.chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .map(|ch| matches!(ch, ',' | ';' | ':' | '.' | '?' | '!' | ')' | ']' | '}'))
        .unwrap_or(false)
}

fn workflow_recovery_token_looks_like_entity(token: &str) -> bool {
    if workflow_recovery_entity_stopword(token) {
        return false;
    }
    let letters = token.chars().filter(|ch| ch.is_alphabetic()).count();
    if letters < 2 {
        return false;
    }
    let uppercase = token.chars().filter(|ch| ch.is_uppercase()).count();
    token
        .chars()
        .next()
        .map(|ch| ch.is_uppercase())
        .unwrap_or(false)
        || uppercase >= 2
        || token.contains('.')
}

fn workflow_recovery_entity_stopword(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "and"
            | "as"
            | "april"
            | "august"
            | "best"
            | "current"
            | "december"
            | "for"
            | "from"
            | "give"
            | "i"
            | "if"
            | "in"
            | "january"
            | "july"
            | "june"
            | "look"
            | "march"
            | "may"
            | "me"
            | "november"
            | "october"
            | "of"
            | "on"
            | "or"
            | "practical"
            | "recommendation"
            | "recent"
            | "research"
            | "search"
            | "september"
            | "summarize"
            | "summary"
            | "tell"
            | "the"
            | "to"
            | "up"
            | "tradeoff"
            | "tradeoffs"
            | "use"
            | "what"
            | "which"
            | "with"
    )
}

fn workflow_recovery_flush_entity_phrase(out: &mut Vec<String>, current: &mut Vec<String>) {
    if current.is_empty() {
        return;
    }
    let phrase = clean_text(&current.join(" "), 120);
    current.clear();
    if !phrase.is_empty() {
        out.push(phrase);
    }
}

fn workflow_recovery_facet_terms(
    query: &str,
    message: &str,
    generic_facet_terms: &[String],
) -> Vec<String> {
    let lowered = clean_text(&format!("{query} {message}"), 2_400).to_ascii_lowercase();
    let mut out = Vec::<String>::new();
    for term in generic_facet_terms {
        if lowered.contains(&term.to_ascii_lowercase()) {
            out.push(term.clone());
        }
    }
    for raw_token in lowered.split(|ch: char| !ch.is_alphanumeric() && ch != '-') {
        let token = clean_text(raw_token, 80);
        if token.len() < 4 || workflow_recovery_facet_stopword(&token) {
            continue;
        }
        out.push(token);
    }
    workflow_recovery_dedupe_limit(out, 10)
}

fn workflow_recovery_facet_stopword(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "after"
            | "also"
            | "and"
            | "are"
            | "based"
            | "between"
            | "could"
            | "from"
            | "give"
            | "have"
            | "into"
            | "right"
            | "search"
            | "some"
            | "that"
            | "their"
            | "there"
            | "this"
            | "what"
            | "when"
            | "where"
            | "which"
            | "with"
            | "would"
    )
}

fn workflow_recovery_query_lanes(
    query: &str,
    entities: &[String],
    facets: &[String],
    source_class_terms: &[String],
) -> Vec<String> {
    let mut lanes = vec![clean_text(query, 400)];
    let facet_suffix = clean_text(&facets.iter().take(3).cloned().collect::<Vec<_>>().join(" "), 120);
    if !entities.is_empty() {
        for entity in entities.iter().take(5) {
            let suffix = if facet_suffix.is_empty() {
                source_class_terms
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "primary sources".to_string())
            } else {
                facet_suffix.clone()
            };
            lanes.push(clean_text(&format!("{entity} {suffix}"), 400));
        }
    } else {
        for source_term in source_class_terms.iter().take(3) {
            lanes.push(clean_text(&format!("{query} {source_term}"), 400));
        }
    }
    if let Some(source_term) = source_class_terms.get(1) {
        lanes.push(clean_text(&format!("{query} {source_term}"), 400));
    }
    workflow_recovery_dedupe_limit(lanes, 8)
}

fn workflow_recovery_keywords(
    entities: &[String],
    facets: &[String],
    source_class_terms: &[String],
) -> Vec<String> {
    let mut terms = Vec::<String>::new();
    terms.extend(entities.iter().cloned());
    terms.extend(facets.iter().take(8).cloned());
    terms.extend(source_class_terms.iter().take(3).cloned());
    workflow_recovery_dedupe_limit(terms, 16)
}

fn workflow_recovery_aliases(entities: &[String]) -> Vec<String> {
    let mut aliases = Vec::<String>::new();
    for entity in entities {
        for alias in workflow_recovery_parenthetical_aliases(entity) {
            aliases.push(alias);
        }
        if let Some(alias) = workflow_recovery_initialism_alias(entity) {
            aliases.push(alias);
        }
    }
    workflow_recovery_dedupe_limit(aliases, 16)
}

fn workflow_recovery_parenthetical_aliases(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut rest = raw;
    while let Some(open_idx) = rest.find('(') {
        let after_open = &rest[open_idx + 1..];
        let Some(close_idx) = after_open.find(')') else {
            break;
        };
        let alias = clean_text(&after_open[..close_idx], 80);
        if workflow_recovery_alias_term_allowed(&alias) {
            out.push(alias);
        }
        rest = &after_open[close_idx + 1..];
    }
    out
}

fn workflow_recovery_initialism_alias(raw: &str) -> Option<String> {
    let tokens = raw
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter(|token| !workflow_recovery_entity_stopword(token))
        .collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    let alias = tokens
        .iter()
        .filter_map(|token| token.chars().next())
        .collect::<String>()
        .to_ascii_uppercase();
    if workflow_recovery_alias_term_allowed(&alias) && alias.chars().count() >= 3 {
        Some(alias)
    } else {
        None
    }
}

fn workflow_recovery_alias_term_allowed(raw: &str) -> bool {
    let alnum_count = raw.chars().filter(|ch| ch.is_ascii_alphanumeric()).count();
    alnum_count >= 2
        && raw
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch.is_whitespace())
}

fn workflow_recovery_required_coverage(
    existing: Option<&Value>,
    entities: &[String],
    facets: &[String],
) -> Option<Value> {
    let mut value = existing.cloned().unwrap_or_else(|| json!({}));
    let map = value.as_object_mut()?;
    if workflow_json_array_is_empty(map.get("entities")) && !entities.is_empty() {
        map.insert("entities".to_string(), json!(entities));
    }
    if workflow_json_array_is_empty(map.get("facets")) && !facets.is_empty() {
        map.insert("facets".to_string(), json!(facets));
    }
    Some(value)
}

fn workflow_recovery_dedupe_limit(values: Vec<String>, limit: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for value in values {
        let cleaned = clean_text(&value, 160);
        if cleaned.is_empty() {
            continue;
        }
        let normalized = cleaned.to_ascii_lowercase();
        if out
            .iter()
            .any(|existing| existing.to_ascii_lowercase() == normalized)
        {
            continue;
        }
        out.push(cleaned);
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn workflow_tool_family_prompt_context(
    previous_category_key: &str,
    previous_category_label: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let families = contract
        .get("tool_family_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let family_menu_json = serde_json::to_string(&families).unwrap_or_else(|_| "[]".to_string());
    contract
        .get("llm_tool_family_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace(
                        "{previous_category_key}",
                        &clean_text(previous_category_key, 120),
                    )
                    .replace(
                        "{previous_category_label}",
                        &clean_text(previous_category_label, 120),
                    )
                    .replace("{tool_family_menu_json}", &family_menu_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn workflow_tool_selection_prompt_context(family_key: &str, family_label: &str) -> String {
    let family_key = clean_text(family_key, 120);
    let family_label = clean_text(family_label, 120);
    let contract = workflow_tool_menu_contract_for_family(&family_key);
    let tools = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&family_key))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tools_json = serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string());
    let allowed_tool_keys_json = tools
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(workflow_option_key)
                .filter(|key| !key.is_empty())
                .collect::<Vec<_>>()
        })
        .map(|keys| serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "[]".to_string());
    contract
        .get("llm_tool_selection_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_family_key}", &family_key)
                    .replace("{selected_family_label}", &family_label)
                    .replace("{selected_tool_keys_json}", &allowed_tool_keys_json)
                    .replace("{selected_tool_menu_json}", &tools_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn workflow_tool_payload_prompt_context(
    family_key: &str,
    tool_key: &str,
    tool_label: &str,
) -> String {
    let family_key = clean_text(family_key, 120);
    let tool_key = clean_text(tool_key, 120);
    let tool_label = clean_text(tool_label, 120);
    let contract = workflow_tool_menu_contract_for_family(&family_key);
    let tool = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
        .unwrap_or_else(|| json!({}));
    let request_format_json =
        serde_json::to_string(tool.get("request_format").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "null".to_string());
    let request_example_json =
        serde_json::to_string(tool.get("request_example").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "null".to_string());
    contract
        .get("llm_tool_payload_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_family_key}", &family_key)
                    .replace("{selected_tool_key}", &tool_key)
                    .replace("{selected_tool_label}", &tool_label)
                    .replace("{selected_tool_request_format_json}", &request_format_json)
                    .replace(
                        "{selected_tool_request_example_json}",
                        &request_example_json,
                    ),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn manual_toolbox_private_gate_max_attempts() -> u64 {
    let contract = default_workflow_tool_menu_contract();
    let base_gate_count = contract
        .get("gate_order")
        .and_then(Value::as_array)
        .and_then(|gates| {
            gates
                .iter()
                .position(|gate| gate.as_str() == Some("gate_4_request_payload_input"))
                .map(|idx| idx as u64 + 1)
        })
        .unwrap_or(4);
    let retry_limit = contract
        .get("private_gate_retry_limit")
        .or_else(|| contract.get("workflow_retry_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(4);
    base_gate_count.saturating_add(retry_limit).clamp(4, 8)
}

fn workflow_private_gate_retry_prompt_context(
    current_gate_id: &str,
    message: &str,
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
) -> String {
    let contract = default_workflow_tool_menu_contract();
    let fallback = "INTERNAL RETRY — output is never shown to the user. The previous response for `{current_gate_id}` was rejected with reason `{last_reject_reason}`. Previous excerpt: {last_invalid_excerpt}. If the excerpt is empty, treat it as an empty response. Re-read the current gate system instruction and output only the exact JSON artifact required by that gate. Do not answer the user directly, do not write prose, and do not include markdown.";
    let template = contract
        .get("private_gate_retry_instruction")
        .and_then(Value::as_str)
        .unwrap_or(fallback);
    let excerpt = if last_invalid_excerpt.trim().is_empty() {
        "(empty response)"
    } else {
        last_invalid_excerpt
    };
    clean_text(
        &format!(
            "{}\n\nContext-only user message. Do not answer it directly. Use it only to produce the artifact required for the current workflow gate:\n{}",
            template
                .replace("{current_gate_id}", &clean_text(current_gate_id, 120))
                .replace(
                    "{last_reject_reason}",
                    &clean_text(last_reject_reason, 160)
                )
                .replace("{last_invalid_excerpt}", &clean_text(excerpt, 320)),
            message
        ),
        8_000,
    )
}

fn workflow_tool_family_selection_from_response(response: &str) -> Option<(String, String)> {
    let contract = default_workflow_tool_menu_contract();
    let token = workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_tool_request_string_field(&request, &contract, "tool_family")
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "family"))
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "tool_family_key")
                })
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "selected_tool_family")
                })
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "category"))
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "gate"))
        })
        .unwrap_or_else(|| clean_text(response, 240));
    let family_key = workflow_family_key_for_selection(&contract, &token);
    if family_key.is_empty() {
        return None;
    }
    contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .and_then(|families| {
            families.iter().find_map(|family| {
                (workflow_option_key(family) == family_key)
                    .then(|| (family_key.clone(), workflow_option_label(family)))
            })
        })
}

fn workflow_tool_selection_from_response(
    family_key: &str,
    response: &str,
) -> Option<(String, String)> {
    let contract = workflow_tool_menu_contract_for_family(family_key);
    let token = workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_tool_request_string_field(&request, &contract, "tool")
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "selected_tool"))
                .or_else(|| workflow_tool_request_string_field(&request, &contract, "tool_key"))
                .or_else(|| {
                    workflow_tool_request_string_field(&request, &contract, "selected_tool_key")
                })
        })
        .unwrap_or_else(|| clean_text(response, 240));
    let tool_key = workflow_tool_key_for_selection(&contract, family_key, &token);
    if tool_key.is_empty() {
        return None;
    }
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter().find_map(|tool| {
                (workflow_option_key(tool) == tool_key)
                    .then(|| (tool_key.clone(), workflow_option_label(tool)))
            })
        })
}

fn workflow_selected_tool_request_format_keys(family_key: &str, tool_key: &str) -> Vec<String> {
    workflow_tool_menu_contract_for_family(family_key)
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
        .and_then(|tool| tool.get("request_format").cloned())
        .and_then(|format| format.as_object().cloned())
        .map(|format| {
            format
                .keys()
                .map(|key| normalized_workflow_token(key))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_payload_object_matches_selected_tool(
    value: &Value,
    family_key: &str,
    tool_key: &str,
) -> bool {
    let Some(payload) = value.as_object() else {
        return false;
    };
    let expected_keys = workflow_selected_tool_request_format_keys(family_key, tool_key);
    if expected_keys.is_empty() {
        return false;
    }
    let reserved_keys = [
        "gate",
        "tool",
        "tool_name",
        "selected_tool",
        "selected_tool_name",
        "selected_tool_key",
        "tool_family",
        "selected_tool_family",
        "category",
        "final_answer",
        "message",
        "response",
        "content",
        "visible_response",
    ]
    .into_iter()
    .map(normalized_workflow_token)
    .collect::<Vec<_>>();
    let payload_keys = payload
        .keys()
        .map(|key| normalized_workflow_token(key))
        .collect::<Vec<_>>();
    !payload_keys
        .iter()
        .any(|key| reserved_keys.iter().any(|reserved| reserved == key))
        && expected_keys
            .iter()
            .any(|expected| payload_keys.iter().any(|key| key == expected))
}

fn workflow_request_payload_from_json_response(
    request: &Value,
    family_key: &str,
    tool_key: &str,
) -> Option<Value> {
    workflow_tool_request_object_field(
        request,
        &default_workflow_tool_menu_contract(),
        "request_payload",
    )
    .and_then(|value| workflow_tool_request_payload_from_json_value(&value))
    .or_else(|| {
        workflow_payload_object_matches_selected_tool(request, family_key, tool_key)
            .then(|| request.clone())
    })
}

fn workflow_request_payload_from_response(
    family_key: &str,
    tool_key: &str,
    response: &str,
) -> Option<Value> {
    workflow_structured_gate_submission(response)
        .and_then(|request| {
            workflow_request_payload_from_json_response(&request, family_key, tool_key)
        })
        .or_else(|| {
            manual_toolbox_payload_json(response).and_then(|request| {
                workflow_request_payload_from_json_response(&request, family_key, tool_key)
            })
        })
        .filter(Value::is_object)
}

fn workflow_tool_menu_contract_from_response_workflow(response_workflow: &Value) -> Value {
    let contract = response_workflow
        .pointer("/selected_workflow/tool_menu_interface_contract")
        .cloned()
        .unwrap_or_else(default_workflow_tool_menu_contract);
    let family_key = workflow_selected_tool_family_key_from_workflow(response_workflow);
    if !family_key.is_empty() {
        let family_contract = workflow_tool_menu_contract_for_family(&family_key);
        let has_family = family_contract
            .get("tool_menu_by_family")
            .and_then(Value::as_object)
            .and_then(|families| families.get(&family_key))
            .is_some();
        if has_family {
            return family_contract;
        }
    }
    contract
}

fn workflow_contract_has_family(contract: &Value, family_key: &str) -> bool {
    let family_key = clean_text(family_key, 120);
    !family_key.is_empty()
        && contract
            .get("tool_menu_by_family")
            .and_then(Value::as_object)
            .and_then(|families| families.get(&family_key))
            .is_some()
}

fn workflow_selected_work_category_as_family(response_workflow: &Value) -> String {
    let category_key = clean_text(
        response_workflow
            .pointer("/tool_gate/selected_work_category")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if category_key.is_empty() || category_key == "none" || category_key == "unselected" {
        return String::new();
    }
    let selected_contract = response_workflow
        .pointer("/selected_workflow/tool_menu_interface_contract")
        .cloned()
        .unwrap_or_else(default_workflow_tool_menu_contract);
    if workflow_contract_has_family(&selected_contract, &category_key)
        || workflow_contract_has_family(&workflow_tool_menu_contract_for_family(&category_key), &category_key)
    {
        category_key
    } else {
        String::new()
    }
}

fn workflow_selected_tool_family_key_from_workflow(response_workflow: &Value) -> String {
    let family_key = clean_text(
        response_workflow
            .pointer("/tool_gate/selected_tool_family")
            .or_else(|| {
                response_workflow.pointer("/manual_toolbox_pending_tool_request/selected_tool_family")
            })
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if family_key.is_empty() || family_key == "none" || family_key == "unselected" {
        workflow_selected_work_category_as_family(response_workflow)
    } else {
        family_key
    }
}

fn workflow_selected_tool_key_from_workflow(response_workflow: &Value) -> String {
    clean_text(
        response_workflow
            .pointer("/tool_gate/selected_tool")
            .or_else(|| response_workflow.pointer("/tool_gate/selected_tool_key"))
            .or_else(|| {
                response_workflow.pointer("/manual_toolbox_pending_tool_request/selected_tool_key")
            })
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
}

fn workflow_tool_contract_from_contract(
    contract: &Value,
    family_key: &str,
    tool_key: &str,
) -> Option<Value> {
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools.iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .cloned()
        })
}

fn workflow_default_raw_message_recovery_tool_key(contract: &Value, family_key: &str) -> String {
    let Some(tools) = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
    else {
        return String::new();
    };
    if let Some(tool) = tools.iter().find(|tool| {
        tool.get("recovery_default_for_raw_message")
            .and_then(Value::as_bool)
            == Some(true)
    }) {
        return workflow_option_key(tool);
    }
    if tools.len() == 1 {
        return workflow_option_key(&tools[0]);
    }
    String::new()
}

fn workflow_sanitize_request_format_for_raw_message(value: &Value) -> Option<Value> {
    match value {
        Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (key, child) in map {
                let normalized_key = normalized_workflow_token(key);
                if matches!(
                    normalized_key.as_str(),
                    "queries"
                        | "keywords"
                        | "required coverage"
                        | "aliases"
                        | "negative terms"
                        | "query metadata policy"
                ) {
                    continue;
                }
                if let Some(sanitized_child) =
                    workflow_sanitize_request_format_for_raw_message(child)
                {
                    sanitized.insert(key.clone(), sanitized_child);
                }
            }
            Some(Value::Object(sanitized))
        }
        Value::Array(_) => Some(Value::Array(Vec::new())),
        Value::String(raw) => {
            let cleaned = clean_text(raw, 240);
            if cleaned.starts_with('<') && cleaned.ends_with('>') {
                None
            } else {
                Some(Value::String(cleaned))
            }
        }
        Value::Null => None,
        _ => Some(value.clone()),
    }
}

fn workflow_raw_message_binding_value(bind_field: &str, message: &str) -> Option<Value> {
    let normalized = normalized_workflow_token(bind_field);
    if normalized == "url" {
        let url = message
            .split_whitespace()
            .find(|token| token.starts_with("https://") || token.starts_with("http://"))
            .map(|token| token.trim_matches(|ch: char| ",.;:!?)]}\"'".contains(ch)))
            .map(|raw| clean_text(raw, 1_200))
            .filter(|raw| !raw.is_empty())?;
        return Some(Value::String(url));
    }
    let query = clean_text(message, 1_200);
    if query.is_empty() {
        None
    } else {
        Some(Value::String(query))
    }
}

fn workflow_request_payload_from_raw_message_contract(
    contract: &Value,
    family_key: &str,
    tool_key: &str,
    message: &str,
) -> Option<Value> {
    let tool = workflow_tool_contract_from_contract(contract, family_key, tool_key)?;
    let raw_message_contract = tool
        .get("raw_message_fallback_contract")
        .filter(|value| value.is_object())?;
    if raw_message_contract
        .get("enabled")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return None;
    }
    let bind_field = clean_text(
        raw_message_contract
            .get("bind_user_message_to")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if bind_field.is_empty() {
        return None;
    }
    let mut payload = workflow_sanitize_request_format_for_raw_message(
        tool.get("request_format").unwrap_or(&Value::Null),
    )?;
    let binding_value = workflow_raw_message_binding_value(&bind_field, message)?;
    payload.as_object_mut()?.insert(bind_field, binding_value);
    Some(payload)
}

fn workflow_pending_request_from_selected_tool_contract(
    response_workflow: &Value,
    message: &str,
) -> Option<Value> {
    let family_key = workflow_selected_tool_family_key_from_workflow(response_workflow);
    if family_key.is_empty() || family_key == "none" || family_key == "unselected" {
        return None;
    }
    let contract = workflow_tool_menu_contract_from_response_workflow(response_workflow);
    let mut tool_key = workflow_selected_tool_key_from_workflow(response_workflow);
    if tool_key.is_empty() {
        tool_key = workflow_default_raw_message_recovery_tool_key(&contract, &family_key);
    }
    if tool_key.is_empty() {
        return None;
    }
    let tool = workflow_tool_contract_from_contract(&contract, &family_key, &tool_key)?;
    let tool_label = workflow_option_label(&tool);
    let input =
        workflow_request_payload_from_raw_message_contract(&contract, &family_key, &tool_key, message)?;
    let mut pending = manual_toolbox_pending_request_from_parts(
        &family_key,
        &tool_key,
        &tool_label,
        input,
        message,
    )?;
    pending["source"] = Value::String("workflow_selected_tool_contract_recovery".to_string());
    pending["recovery_contract"] = Value::String(
        "workflow_selected_tool_contract_plus_raw_user_message".to_string(),
    );
    Some(pending)
}

fn manual_toolbox_pending_request_from_parts(
    family_key: &str,
    tool_key: &str,
    tool_label: &str,
    input: Value,
    message: &str,
) -> Option<Value> {
    let tool_name = canonical_manual_toolbox_tool_name(family_key, tool_key);
    if tool_name.is_empty() || !input.is_object() {
        return None;
    }
    let input = workflow_repair_recovered_request_payload(family_key, tool_key, input, message);
    let input = workflow_apply_declared_request_literal_defaults(family_key, tool_key, input);
    let receipt_binding = crate::deterministic_receipt_hash(&json!({
        "type": "manual_toolbox_pending_tool_request",
        "tool_name": tool_name,
        "input": input,
        "message": clean_text(message, 600)
    }));
    Some(json!({
        "status": "pending_confirmation",
        "source": "split_manual_toolbox_gates",
        "tool_name": tool_name,
        "tool_key": clean_text(tool_key, 120),
        "selected_tool_key": clean_text(tool_key, 120),
        "selected_tool_family": clean_text(family_key, 120),
        "selected_tool_label": clean_text(tool_label, 120),
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn manual_toolbox_active_gate_id(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "gate_1_work_category_menu"
    } else if family_key.is_empty() {
        "gate_2_tool_family_menu"
    } else if tool_key.is_empty() {
        "gate_3_tool_menu"
    } else {
        "gate_4_request_payload_input"
    }
}

fn workflow_declared_request_format_for_tool(family_key: &str, tool_key: &str) -> Option<Value> {
    workflow_tool_menu_contract_for_family(family_key)
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(family_key))
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| workflow_option_key(tool) == tool_key)
                .and_then(|tool| tool.get("request_format").cloned())
        })
}

fn workflow_request_format_literal_default(value: &Value) -> Option<String> {
    let raw = value.as_str()?;
    if raw.starts_with('<') && raw.ends_with('>') {
        None
    } else {
        Some(clean_text(raw, 240))
    }
    .filter(|value| !value.is_empty())
}

fn workflow_apply_declared_request_literal_defaults(
    family_key: &str,
    tool_key: &str,
    mut input: Value,
) -> Value {
    let Some(format) = workflow_declared_request_format_for_tool(family_key, tool_key) else {
        return input;
    };
    let Some(format_object) = format.as_object() else {
        return input;
    };
    let Some(input_object) = input.as_object_mut() else {
        return input;
    };
    for (key, format_value) in format_object {
        if input_object.contains_key(key) {
            continue;
        }
        if let Some(default_value) = workflow_request_format_literal_default(format_value) {
            input_object.insert(key.clone(), Value::String(default_value));
        }
    }
    input
}

fn manual_toolbox_pending_direct_response_path(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "first_gate_pending_llm_tool_choice"
    } else if family_key.is_empty() {
        "gate_2_pending_llm_tool_family"
    } else if tool_key.is_empty() {
        "gate_3_pending_llm_tool_choice"
    } else {
        "gate_4_pending_llm_tool_request"
    }
}

fn manual_toolbox_pending_stage_status(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "first_gate_pending_tool_choice"
    } else if family_key.is_empty() {
        "gate_2_pending_tool_family_selection"
    } else if tool_key.is_empty() {
        "gate_3_pending_tool_selection"
    } else {
        "gate_4_pending_request_payload"
    }
}

fn manual_toolbox_invalid_reject_reason(
    category_key: &str,
    family_key: &str,
    tool_key: &str,
) -> &'static str {
    if category_key.is_empty() {
        "tool_category_without_selection_diagnostic_only"
    } else if family_key.is_empty() {
        "tool_family_without_selection_diagnostic_only"
    } else if tool_key.is_empty() {
        "tool_without_selection_diagnostic_only"
    } else {
        "tool_request_without_payload_submission_diagnostic_only"
    }
}

#[cfg(test)]
mod manual_toolbox_pending_request_tests {
    use super::*;

    #[test]
    fn recovered_payload_repair_does_not_infer_metadata_from_message() {
        let payload = workflow_repair_recovered_request_payload(
            "web_research",
            "batch_query",
            serde_json::json!({
                "source": "web",
                "query": "what is the public sentiment on xvacume versus yvacume?",
                "aperture": "medium"
            }),
            "what is the public sentiment on xvacume versus yvacume?",
        );
        assert_eq!(
            payload,
            json!({
                "source": "web",
                "query": "what is the public sentiment on xvacume versus yvacume?",
                "aperture": "medium"
            })
        );
    }

    #[test]
    fn recovered_payload_repair_reconciles_relative_time_window_years() {
        let (current_year, current_month) =
            workflow_runtime_temporal_context().expect("runtime temporal context");
        let stale_year = current_year - 1;
        let payload = workflow_repair_recovered_request_payload(
            "web_research",
            "batch_query",
            serde_json::json!({
                "source": "web",
                "query": "Research current shipping and logistics disruptions this month.",
                "queries": [
                    format!("shipping logistics disruptions February {stale_year} news"),
                    format!("air cargo demand supply chain bottlenecks {stale_year}")
                ],
                "keywords": [
                    format!("January {stale_year}"),
                    format!("February {stale_year}")
                ],
                "aperture": "medium"
            }),
            "Research current shipping and logistics disruptions this month.",
        );

        let rendered = payload.to_string();
        assert!(
            !rendered.contains(&stale_year.to_string()),
            "{rendered}"
        );
        assert!(
            rendered.contains(&current_year.to_string()),
            "{rendered}"
        );
        assert!(
            rendered.contains(current_month),
            "{rendered}"
        );
        assert_eq!(
            payload
                .pointer("/query_metadata_policy/temporal_coherence/status")
                .and_then(Value::as_str),
            Some("reconciled_relative_time_window")
        );
    }

    #[test]
    fn recovered_payload_repair_preserves_user_supplied_year() {
        let payload = workflow_repair_recovered_request_payload(
            "web_research",
            "batch_query",
            serde_json::json!({
                "source": "web",
                "query": "Research shipping disruptions this month compared with 2025.",
                "queries": ["shipping disruptions 2025 comparison"],
                "aperture": "medium"
            }),
            "Research shipping disruptions this month compared with 2025.",
        );

        assert!(
            payload.to_string().contains("2025"),
            "{payload}"
        );
        assert!(
            payload.pointer("/query_metadata_policy").is_none(),
            "{payload}"
        );
    }

    #[test]
    fn selected_web_research_tool_contract_recovers_raw_message_query() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": default_workflow_tool_menu_contract()
            },
            "tool_gate": {
                "selected_tool_family": "web_research",
                "selected_tool": "web_search"
            }
        });
        let pending = workflow_pending_request_from_selected_tool_contract(
            &workflow,
            "what is the public sentiment on xvacume versus yvacume?",
        )
        .expect("pending request");
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("what is the public sentiment on xvacume versus yvacume?")
        );
        assert_eq!(
            pending.pointer("/input/aperture").and_then(Value::as_str),
            Some("medium")
        );
        assert!(pending.pointer("/input/keywords").is_none(), "{pending:?}");
        assert!(pending.pointer("/input/required_coverage").is_none(), "{pending:?}");
        assert!(pending.pointer("/input/query_metadata_policy").is_none(), "{pending:?}");
        assert_eq!(
            pending.get("source").and_then(Value::as_str),
            Some("workflow_selected_tool_contract_recovery")
        );
    }

    #[test]
    fn selected_web_research_family_uses_batch_query_as_contract_default_tool() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": default_workflow_tool_menu_contract()
            },
            "tool_gate": {
                "selected_tool_family": "web_research"
            }
        });
        let pending = workflow_pending_request_from_selected_tool_contract(
            &workflow,
            "what is the news today",
        )
        .expect("pending request");
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("what is the news today")
        );
        assert_eq!(
            pending.pointer("/input/source").and_then(Value::as_str),
            Some("web")
        );
        assert!(pending.pointer("/input/queries").is_none(), "{pending:?}");
        assert!(pending.pointer("/input/keywords").is_none(), "{pending:?}");
        assert!(pending.pointer("/input/required_coverage").is_none(), "{pending:?}");
    }

    #[test]
    fn selected_batch_query_payload_gets_declared_source_default() {
        let pending = manual_toolbox_pending_request_from_parts(
            "web_research",
            "batch_query",
            "Research query pack",
            json!({
                "query": "compare current public sentiment for xvacume and yvacume",
                "aperture": "medium"
            }),
            "compare current public sentiment for xvacume and yvacume",
        )
        .expect("pending request");

        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare current public sentiment for xvacume and yvacume")
        );
        assert_eq!(
            pending.pointer("/input/source").and_then(Value::as_str),
            Some("web")
        );
        assert!(pending.pointer("/input/queries").is_none(), "{pending:?}");
    }

    #[test]
    fn selected_work_category_can_recover_delegated_web_family_default_tool() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": default_workflow_tool_menu_contract()
            },
            "tool_gate": {
                "selected_work_category": "web_research"
            }
        });
        let pending = workflow_pending_request_from_selected_tool_contract(
            &workflow,
            "compare current public sentiment for xvacume and yvacume",
        )
        .expect("pending request");
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            pending.get("selected_tool_family").and_then(Value::as_str),
            Some("web_research")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare current public sentiment for xvacume and yvacume")
        );
        assert_eq!(
            pending.pointer("/input/source").and_then(Value::as_str),
            Some("web")
        );
    }
}
