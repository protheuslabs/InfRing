// Layer ownership: orchestration (research eval authority)

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(*needle))
}

fn has_limitation_signal(normalized: &str) -> bool {
    [
        "limited",
        "limitation",
        "uncertain",
        "caveat",
        "sparse",
        "weak",
        "insufficient",
        "gap",
        "gaps",
        "missing",
        "unknown",
        "not enough",
        "low signal",
        "low-signal",
        "off topic",
        "off-topic",
        "no substantive",
        "not clear",
        "does not establish",
        "doesn't establish",
        "does not support",
        "doesn't support",
        "as of",
        "current",
        "verify",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn has_tradeoff_or_structure(normalized: &str) -> bool {
    [
        "tradeoff",
        "trade-off",
        "compare",
        "comparison",
        "criteria",
        "dimension",
        "versus",
        "vs",
        "differ",
        "different",
        "option",
        "choice",
        "better for",
        "stronger for",
        "weaker for",
        "safer for",
        "whereas",
        "rather than",
        "at the same time",
        "not uncritical",
        "tension",
        "tensions",
        "competing",
        "strength",
        "weakness",
        "finding",
        "source-backed",
        "evidence supports",
        "evidence shows",
        "what the evidence",
        "risk",
        "concern",
        "boundary",
        "evaluation plan",
        "plan",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn has_recommendation_signal(normalized: &str) -> bool {
    [
        "recommend",
        "best for",
        "use ",
        "choose",
        "should",
        "default",
        "pragmatic",
        "priority",
        "priorities",
        "prioritize",
        "prioritise",
        "favor",
        "focus on",
        "risk-reduction",
        "what you can do",
        "next step",
        "takeaway",
        "bottom line",
        "balanced view",
        "practical view",
        "watch for",
        "wait for",
        "plan",
        "treat",
        "avoid",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn normal_prose_signal(response_text: &str) -> bool {
    let trimmed = response_text.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('{')
        && !trimmed.starts_with('[')
        && trimmed.split_whitespace().count() >= 8
}

fn visible_response_text_for_grading(payload: &Value) -> String {
    for pointer in [
        "/response_finalization/final_response/text",
        "/response_finalization/finalized_output",
        "/response_finalization/final_output",
        "/response_workflow/final_llm_response/text",
    ] {
        let candidate = payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(sanitize_visible_response_text_for_grading)
            .unwrap_or_default();
        if !candidate.is_empty() {
            return candidate;
        }
    }
    sanitize_visible_response_text_for_grading(&assistant_text(payload))
}

fn sanitize_visible_response_text_for_grading(response_text: &str) -> String {
    let cleaned = clean_text(response_text, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&cleaned) {
        for key in [
            "final_answer",
            "answer",
            "visible_response",
            "final_response",
            "message",
            "text",
            "response",
            "content",
        ] {
            let candidate = clean_text(map.get(key).and_then(Value::as_str).unwrap_or(""), 12_000);
            if !candidate.is_empty() {
                return sanitize_visible_response_text_for_grading(&candidate);
            }
        }
    }
    strip_internal_evidence_posture_disclosure_for_grading(
        &strip_internal_evidence_posture_prefix_for_grading(&cleaned),
    )
}

fn strip_internal_evidence_posture_prefix_for_grading(response_text: &str) -> String {
    let cleaned = clean_text(response_text, 12_000);
    let trimmed = cleaned
        .trim_start()
        .trim_start_matches(|ch: char| ch == '*' || ch == '`' || ch == '_' || ch.is_whitespace());
    for posture in [
        "supported_answer",
        "bounded_partial_answer",
        "evidence_insufficient_answer",
    ] {
        let lowered_trimmed = trimmed.to_ascii_lowercase();
        let Some(_) = lowered_trimmed.strip_prefix(posture) else {
            continue;
        };
        let after_posture = &trimmed[posture.len()..];
        let after_posture = after_posture.trim_start_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, ':' | '-' | '.' | ';' | '*' | '`' | '_')
        });
        return clean_text(after_posture.trim_start(), 12_000);
    }
    cleaned
}

fn strip_internal_evidence_posture_disclosure_for_grading(response_text: &str) -> String {
    let mut cleaned = clean_text(response_text, 12_000);
    for posture in [
        "supported_answer",
        "bounded_partial_answer",
        "evidence_insufficient_answer",
    ] {
        for posture_variant in internal_evidence_posture_label_variants_for_grading(posture) {
            for marker in [
                format!("**Posture: `{posture_variant}`** — "),
                format!("**Posture: `{posture_variant}`** - "),
                format!("**Posture: `{posture_variant}`** "),
                format!("**Posture:** `{posture_variant}` — "),
                format!("**Posture:** `{posture_variant}` - "),
                format!("**Posture:** `{posture_variant}` "),
                format!("Posture: `{posture_variant}` — "),
                format!("Posture: `{posture_variant}` - "),
                format!("Posture: `{posture_variant}` "),
                format!("Posture: {posture_variant} — "),
                format!("Posture: {posture_variant} - "),
                format!("Posture: {posture_variant} "),
                format!("**Outcome posture: `{posture_variant}`** — "),
                format!("**Outcome posture: `{posture_variant}`** - "),
                format!("**Outcome posture: `{posture_variant}`** "),
                format!("**Outcome posture:** `{posture_variant}` — "),
                format!("**Outcome posture:** `{posture_variant}` - "),
                format!("**Outcome posture:** `{posture_variant}` "),
                format!("Outcome posture: `{posture_variant}` — "),
                format!("Outcome posture: `{posture_variant}` - "),
                format!("Outcome posture: `{posture_variant}` "),
                format!("Outcome posture: {posture_variant} — "),
                format!("Outcome posture: {posture_variant} - "),
                format!("Outcome posture: {posture_variant} "),
                format!("**Outcome posture: {posture_variant}** "),
            ] {
                cleaned = cleaned.replace(&marker, "");
            }
        }
    }
    clean_text(&cleaned, 12_000)
}

fn internal_evidence_posture_label_variants_for_grading(posture: &str) -> Vec<String> {
    let mut variants = vec![posture.to_string(), posture.to_ascii_uppercase()];
    if let Some(first) = posture.chars().next() {
        let rest = &posture[first.len_utf8()..];
        variants.push(format!("{}{}", first.to_ascii_uppercase(), rest));
    }
    variants.sort();
    variants.dedup();
    variants
}

fn response_looks_truncated_or_incomplete(response_text: &str) -> bool {
    let trimmed = response_text.trim();
    if trimmed.is_empty() {
        return false;
    }
    let tail = trimmed
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(trimmed);
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
    let dangling_connector = normalize_for_compare(tail)
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

fn entity_coverage(normalized_response: &str, required_entities: &[String]) -> f64 {
    if required_entities.is_empty() {
        return 1.0;
    }
    let covered = required_entities
        .iter()
        .filter(|entity| normalized_response_covers_entity(normalized_response, entity))
        .count() as u64;
    ratio(covered, required_entities.len() as u64)
}

fn normalized_response_covers_entity(normalized_response: &str, entity: &str) -> bool {
    let aliases = entity_coverage_aliases(entity);
    aliases
        .iter()
        .any(|alias| normalized_response_covers_entity_alias(normalized_response, alias))
}

fn normalized_response_covers_entity_alias(normalized_response: &str, alias: &str) -> bool {
    let normalized_alias = normalize_for_compare(alias);
    if normalized_alias.is_empty() {
        return false;
    }
    if normalized_term_present(normalized_response, &normalized_alias) {
        return true;
    }
    if normalized_term_present(
        normalized_response,
        &simple_plural_variant(&normalized_alias),
    ) || normalized_term_present(
        normalized_response,
        &simple_singular_variant(&normalized_alias),
    ) {
        return true;
    }
    let tokens = normalized_alias
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| token_or_simple_variant_present(normalized_response, token))
}

fn entity_coverage_aliases(entity: &str) -> Vec<String> {
    let mut aliases = Vec::<String>::new();
    push_unique_alias(&mut aliases, entity);
    for alias in common_entity_aliases(entity) {
        push_unique_alias(&mut aliases, &alias);
    }
    for alias in generic_entity_aliases(entity) {
        push_unique_alias(&mut aliases, &alias);
    }
    for alias in explicit_parenthetical_aliases(entity) {
        push_unique_alias(&mut aliases, &alias);
    }
    if let Some(acronym) = derived_initialism_alias(entity) {
        push_unique_alias(&mut aliases, &acronym);
    }
    aliases
}

fn common_entity_aliases(entity: &str) -> Vec<String> {
    let normalized = normalize_for_compare(entity);
    match normalized.as_str() {
        "europe" => vec!["EU".to_string(), "European Union".to_string(), "European".to_string()],
        "european union" => vec!["EU".to_string(), "Europe".to_string(), "European".to_string()],
        "united states" | "u s" | "us" | "usa" => {
            vec![
                "US".to_string(),
                "U.S.".to_string(),
                "America".to_string(),
                "DEA".to_string(),
                "HHS".to_string(),
                "FDA".to_string(),
                "CDC".to_string(),
                "CMS".to_string(),
                "DOJ".to_string(),
                "Ryan Haight Act".to_string(),
                "FedRAMP".to_string(),
                "federal court".to_string(),
                "federal courts".to_string(),
                "state law".to_string(),
                "state laws".to_string(),
                "state bill".to_string(),
                "state bills".to_string(),
                "state legislature".to_string(),
                "state legislatures".to_string(),
            ]
        }
        "united kingdom" => vec!["UK".to_string(), "Britain".to_string(), "Great Britain".to_string()],
        _ => Vec::new(),
    }
}

fn generic_entity_aliases(entity: &str) -> Vec<String> {
    let mut aliases = Vec::<String>::new();
    let mut seeds = vec![entity.to_string()];
    let normalized = normalize_for_compare(entity);
    if normalized.contains("state-level") {
        seeds.push(entity.replace("state-level", "state"));
        seeds.push(entity.replace("State-level", "State"));
    }
    aliases.extend(seeds.iter().skip(1).cloned());
    for (from, replacements) in [
        ("regulation", &["legislation", "law", "rules", "governance"][..]),
        (
            "regulations",
            &["legislation", "laws", "rules", "governance"][..],
        ),
        (
            "regulatory",
            &["legislative", "legal", "governance"][..],
        ),
    ] {
        for seed in &seeds {
            if normalize_for_compare(seed).contains(from) {
                for replacement in replacements {
                    aliases.push(replace_word_case_insensitive(seed, from, replacement));
                }
            }
        }
    }
    aliases
}

fn replace_word_case_insensitive(raw: &str, needle: &str, replacement: &str) -> String {
    raw.split_whitespace()
        .map(|part| {
            let trimmed = part.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
            if trimmed.eq_ignore_ascii_case(needle) {
                part.replacen(trimmed, replacement, 1)
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn coverage_entity_aliases(coverage_entities: &[String]) -> Value {
    Value::Object(
        coverage_entities
            .iter()
            .map(|entity| {
                (
                    entity.clone(),
                    json!(entity_coverage_aliases(entity)
                        .into_iter()
                        .filter(
                            |alias| normalize_for_compare(alias) != normalize_for_compare(entity)
                        )
                        .collect::<Vec<_>>()),
                )
            })
            .collect(),
    )
}

fn push_unique_alias(aliases: &mut Vec<String>, raw: &str) {
    let cleaned = clean_text(raw, 120);
    if cleaned.is_empty() {
        return;
    }
    let normalized = normalize_for_compare(&cleaned);
    if aliases
        .iter()
        .any(|existing| normalize_for_compare(existing) == normalized)
    {
        return;
    }
    aliases.push(cleaned);
}

fn explicit_parenthetical_aliases(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut rest = raw;
    while let Some(open_idx) = rest.find('(') {
        let after_open = &rest[open_idx + 1..];
        let Some(close_idx) = after_open.find(')') else {
            break;
        };
        let alias = clean_text(&after_open[..close_idx], 40);
        if alias
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch.is_whitespace())
            && alias
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .count()
                >= 2
        {
            out.push(alias);
        }
        rest = &after_open[close_idx + 1..];
    }
    out
}

fn derived_initialism_alias(raw: &str) -> Option<String> {
    if !entity_supports_derived_initialism_alias(raw) {
        return None;
    }
    let tokens = raw
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter(|token| !entity_initialism_stopword(token))
        .collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    let acronym = tokens
        .iter()
        .filter_map(|token| token.chars().next())
        .collect::<String>()
        .to_ascii_uppercase();
    let len = acronym.chars().count();
    if (3..=8).contains(&len) {
        Some(acronym)
    } else {
        None
    }
}

fn entity_initialism_stopword(raw: &str) -> bool {
    matches!(
        normalize_for_compare(raw).as_str(),
        "a" | "an"
            | "and"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "in"
            | "of"
            | "on"
            | "or"
            | "the"
            | "to"
            | "vs"
            | "with"
    )
}

fn normalized_term_present(normalized_response: &str, normalized_term: &str) -> bool {
    if normalized_term.is_empty() {
        return false;
    }
    if normalized_term.split_whitespace().count() > 1 {
        return normalized_response.contains(normalized_term);
    }
    if normalized_term.len() <= 4 {
        return normalized_response
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|token| token == normalized_term);
    }
    normalized_response.contains(normalized_term)
}

fn token_or_simple_variant_present(normalized_response: &str, token: &str) -> bool {
    normalized_term_present(normalized_response, token)
        || normalized_term_present(normalized_response, &simple_plural_variant(token))
        || normalized_term_present(normalized_response, &simple_singular_variant(token))
}

fn simple_plural_variant(value: &str) -> String {
    if value.ends_with('s') {
        value.to_string()
    } else {
        format!("{value}s")
    }
}

fn simple_singular_variant(value: &str) -> String {
    value.strip_suffix('s').unwrap_or(value).to_string()
}

fn raw_tool_payload_leak(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    [
        "pending_tool_request",
        "response_workflow",
        "request_payload",
        "tool_attempts",
        "tool_receipt",
        "receipt_binding",
        "selected_tool_family",
        "\"tool_name\"",
        "\"tool_key\"",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn internal_workflow_leak(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    [
        "gate_1",
        "gate_2",
        "gate_3",
        "gate_4",
        "web_gate_",
        "web_tooling_gates",
        "workflow_trace",
        "workflow_state",
        "finalization_outcome",
        "visible_response_source",
        "llm_gate_instruction",
        "supported_answer",
        "bounded_partial_answer",
        "evidence_insufficient_answer",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn tool_choice_as_final_response(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    normalized.starts_with("yes. tool")
        || normalized.starts_with("tool family")
        || normalized.starts_with("tool:")
        || normalized.contains("request payload:")
        || normalized.contains("selected tool:")
}

fn unsupported_claim_signal(case: &Value, response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    if normalized.is_empty() {
        return false;
    }
    let asks_best = normalize_for_compare(&str_at(case, &["prompt"], "")).contains("best");
    let has_universal_best = normalized.contains("the best")
        || normalized.contains("clear winner")
        || normalized.contains("always use");
    asks_best
        && has_universal_best
        && !has_limitation_signal(&normalized)
        && !has_tradeoff_or_structure(&normalized)
}

fn outside_evidence_used_for_decision_signal(normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let outside_evidence_marker = contains_any(
        normalized_response,
        &[
            "not source backed in this turn",
            "not source-backed in this turn",
            "not supported by retrieved evidence",
            "not supported by the retrieved evidence",
            "outside retrieved evidence",
            "outside the retrieved evidence",
            "general knowledge",
            "prior knowledge",
            "training knowledge",
            "historically lies",
            "known for",
        ],
    );
    if !outside_evidence_marker {
        return false;
    }
    let explicitly_not_decision_basis = contains_any(
        normalized_response,
        &[
            "not enough to recommend",
            "cannot recommend",
            "can't recommend",
            "no source backed basis to choose",
            "no source-backed basis to choose",
            "no source backed basis to recommend",
            "no source-backed basis to recommend",
            "do not use this as a recommendation",
            "do not use it as a recommendation",
            "should not be used as a recommendation",
        ],
    );
    !explicitly_not_decision_basis && has_recommendation_signal(normalized_response)
}
