use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::super::eval_research_golden_utils::{
    clean_text, normalize_for_compare, read_json, str_at, string_array_at,
};

pub(super) fn request_pack_for_case(
    case: &Value,
    report_request: Option<&Value>,
    default_tool: &str,
) -> Value {
    if let Some(request) = report_request.filter(report_request_usable) {
        let input = request.get("input").cloned().unwrap_or_else(|| json!({}));
        let input = repair_report_request_input_if_polluted(&input, case);
        return json!({
            "request_pack_source": "research_report_pending_tool_request",
            "tool_name": str_at(request, &["tool_name"], default_tool),
            "input": input
        });
    }
    if let Some(request) = case.get("tooling_request").and_then(Value::as_object) {
        return json!({
            "request_pack_source": "case_tooling_request",
            "tool_name": request
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or(default_tool),
            "input": Value::Object(request.clone())
        });
    }
    if let Some(prompt) = tooling_setup_prompt(case) {
        return request_pack_from_prompt(
            &prompt,
            case,
            default_tool,
            "case_web_tooling_setup_prompt",
            "tooling_setup_prompt_request",
        );
    }
    let prompt = str_at(case, &["prompt"], "").to_string();
    request_pack_from_prompt(
        &prompt,
        case,
        default_tool,
        "derived_minimal_prompt_request",
        "derived_prompt_request",
    )
}

fn repair_report_request_input_if_polluted(input: &Value, case: &Value) -> Value {
    if !query_pack_has_instruction_scaffold_pollution(input) {
        return input.clone();
    }
    let raw_prompt = input
        .get("query")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| str_at(case, &["prompt"], ""));
    let prompt = clean_text(&raw_prompt, 2_000);
    if prompt.is_empty() {
        return input.clone();
    }
    let required_entities = string_array_at(case, &["required_entities"]);
    let explicit_facets = string_array_at(case, &["required_facets"]);
    let (coverage_entities, coverage_facets) =
        partition_required_coverage_terms(&prompt, &required_entities, &explicit_facets);
    let keywords = derived_keywords(&prompt, &required_entities, &coverage_facets);
    let queries = derived_queries(&prompt, &coverage_entities, &coverage_facets, &keywords);
    let mut repaired = input.clone();
    if !repaired.is_object() {
        repaired = json!({});
    }
    if let Some(obj) = repaired.as_object_mut() {
        obj.insert("query".to_string(), json!(prompt));
        obj.insert("queries".to_string(), json!(queries));
        obj.insert("keywords".to_string(), json!(keywords));
        obj.insert(
            "required_coverage".to_string(),
            json!({
                "entities": coverage_entities,
                "facets": coverage_facets
            }),
        );
        let policy = obj
            .entry("query_metadata_policy")
            .or_insert_with(|| json!({}));
        if !policy.is_object() {
            *policy = json!({});
        }
        if let Some(policy_obj) = policy.as_object_mut() {
            policy_obj.insert(
                "eval_request_pack_repair".to_string(),
                json!({
                    "status": "repaired_instruction_scaffold_pollution",
                    "source": "case_prompt_and_case_metadata"
                }),
            );
        }
    }
    repaired
}

fn query_pack_has_instruction_scaffold_pollution(input: &Value) -> bool {
    metadata_terms_at(input, "/required_coverage/entities")
        .iter()
        .any(|term| metadata_term_looks_like_instruction_scaffold(term))
        || metadata_terms_at(input, "/required_coverage/facets")
            .iter()
            .filter(|term| metadata_term_looks_like_instruction_scaffold(term))
            .count()
            >= 3
        || input
            .get("queries")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .any(query_lane_looks_instruction_polluted)
            })
            .unwrap_or(false)
}

fn metadata_terms_at(input: &Value, pointer: &str) -> Vec<String> {
    input
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 160))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn metadata_term_looks_like_instruction_scaffold(term: &str) -> bool {
    let normalized = normalize_for_compare(term);
    if normalized.is_empty() {
        return false;
    }
    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(
                *token,
                "research"
                    | "focus"
                    | "focused"
                    | "focusing"
                    | "biggest"
                    | "major"
                    | "current"
                    | "latest"
                    | "recent"
                    | "month"
                    | "week"
                    | "year"
                    | "development"
                    | "developments"
                    | "move"
                    | "moves"
                    | "matter"
                    | "matters"
                    | "generic"
                    | "chatter"
                    | "official"
                    | "documentation"
                    | "primary"
                    | "source"
                    | "sources"
                    | "release"
                    | "notes"
                    | "announcements"
            )
        })
}

fn query_lane_looks_instruction_polluted(raw: &str) -> bool {
    let normalized = normalize_for_compare(raw);
    let tokens = normalized.split_whitespace().take(4).collect::<Vec<_>>();
    if tokens.len() >= 2
        && tokens[0] == tokens[1]
        && metadata_term_looks_like_instruction_scaffold(tokens[0])
    {
        return true;
    }
    tokens
        .first()
        .map(|token| metadata_term_looks_like_instruction_scaffold(token))
        .unwrap_or(false)
        && normalized.contains(" official ")
}

fn request_pack_from_prompt(
    prompt: &str,
    case: &Value,
    default_tool: &str,
    request_pack_source: &str,
    classification: &str,
) -> Value {
    let required_entities = string_array_at(case, &["required_entities"]);
    let explicit_facets = string_array_at(case, &["required_facets"]);
    let (coverage_entities, coverage_facets) =
        partition_required_coverage_terms(prompt, &required_entities, &explicit_facets);
    let keywords = derived_keywords(prompt, &required_entities, &coverage_facets);
    let queries = derived_queries(prompt, &coverage_entities, &coverage_facets, &keywords);
    json!({
        "request_pack_source": request_pack_source,
        "tool_name": default_tool,
        "input": {
            "source": "web",
            "query": prompt,
            "queries": queries,
            "keywords": keywords,
            "required_coverage": {
                "entities": coverage_entities,
                "facets": coverage_facets
            },
            "aperture": "medium",
            "query_metadata_policy": {
                "classification": classification
            }
        }
    })
}

fn tooling_setup_prompt(case: &Value) -> Option<String> {
    let prompt = case
        .pointer("/web_tooling_setup/prompt")
        .and_then(Value::as_str)
        .unwrap_or("");
    let cleaned = clean_text(prompt, 2_000);
    (!cleaned.is_empty()).then_some(cleaned)
}

fn derived_keywords(
    prompt: &str,
    required_entities: &[String],
    coverage_facets: &[String],
) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for entity in required_entities {
        let cleaned = clean_text(entity, 160);
        if !cleaned.is_empty() && !out.iter().any(|current| current == &cleaned) {
            out.push(cleaned);
        }
    }
    for facet in coverage_facets {
        let cleaned = clean_text(facet, 160);
        if !cleaned.is_empty() && !out.iter().any(|current| current == &cleaned) {
            out.push(cleaned);
        }
    }
    let normalized = normalize_for_compare(prompt);
    for token in normalized.split_whitespace() {
        let cleaned = derived_keyword_token(token);
        if cleaned.len() < 4 {
            continue;
        }
        if matches!(
            cleaned.as_str(),
            "with"
                | "that"
                | "from"
                | "into"
                | "give"
                | "what"
                | "when"
                | "where"
                | "which"
                | "would"
                | "about"
                | "using"
                | "research"
                | "compare"
                | "practical"
                | "tradeoffs"
        ) {
            continue;
        }
        if !out
            .iter()
            .any(|current| normalize_for_compare(current) == cleaned)
        {
            out.push(cleaned);
        }
        if out.len() >= 12 {
            break;
        }
    }
    out
}

fn partition_required_coverage_terms(
    prompt: &str,
    required_entities: &[String],
    required_facets: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut entities = Vec::<String>::new();
    let mut facets = Vec::<String>::new();
    for term in required_facets {
        let cleaned = clean_text(term, 160);
        if !cleaned.is_empty() && !facets.iter().any(|current| current == &cleaned) {
            facets.push(cleaned);
        }
    }
    for term in required_entities {
        let cleaned = clean_text(term, 160);
        if cleaned.is_empty() {
            continue;
        }
        if looks_like_named_subject(&cleaned, prompt) {
            if !entities.iter().any(|current| current == &cleaned) {
                entities.push(cleaned);
            }
        } else if !facets.iter().any(|current| current == &cleaned) {
            facets.push(cleaned);
        }
    }
    (entities, facets)
}

fn looks_like_named_subject(term: &str, prompt: &str) -> bool {
    if looks_like_broad_or_temporal_facet(term) {
        return false;
    }
    if term.split_whitespace().count() == 1
        && term.chars().all(|ch| !ch.is_ascii_lowercase())
        && term.chars().any(|ch| ch.is_ascii_uppercase())
    {
        return true;
    }
    if term
        .chars()
        .any(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return true;
    }
    if term.contains('/') || term.contains('.') || term.contains('-') {
        return true;
    }
    prompt.contains(term)
        && term.split_whitespace().any(|token| {
            token
                .chars()
                .next()
                .map(|ch| ch.is_ascii_uppercase())
                .unwrap_or(false)
        })
}

fn looks_like_broad_or_temporal_facet(term: &str) -> bool {
    let normalized = normalize_for_compare(term);
    if normalized.is_empty() {
        return false;
    }
    [
        "this week",
        "this month",
        "this year",
        "last week",
        "last month",
        "current",
        "recent",
        "news",
        "landscape",
        "sentiment",
        "market",
        "rates",
        "trends",
        "developments",
        "breakthrough",
        "breakthroughs",
        "adoption",
        "ecosystem",
        "crowding",
        "defenses",
        "teams",
        "builds",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
        || normalized
            .split_whitespace()
            .any(|token| token.len() == 4 && token.chars().all(|ch| ch.is_ascii_digit()))
}

fn looks_like_temporal_scope_facet(term: &str) -> bool {
    let normalized = normalize_for_compare(term);
    if normalized.is_empty() {
        return false;
    }
    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    if tokens
        .iter()
        .all(|token| token.len() == 4 && token.chars().all(|ch| ch.is_ascii_digit()))
    {
        return true;
    }
    let temporal_tokens = [
        "current",
        "latest",
        "recent",
        "today",
        "week",
        "month",
        "year",
        "this",
        "last",
        "may",
        "january",
        "february",
        "march",
        "april",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    tokens.iter().all(|token| {
        temporal_tokens.contains(token)
            || (token.len() == 4 && token.chars().all(|ch| ch.is_ascii_digit()))
    })
}

fn exact_subject_query_term(raw: &str) -> String {
    let cleaned = clean_text(raw, 160).replace('"', "");
    if cleaned.is_empty() {
        String::new()
    } else if cleaned.split_whitespace().count() > 1 {
        format!("\"{cleaned}\"")
    } else {
        cleaned
    }
}

fn text_contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn prompt_or_keywords_look_like_local_stay_research(prompt: &str, keywords: &[String]) -> bool {
    let mut haystack = normalize_for_compare(prompt);
    for keyword in keywords {
        haystack.push(' ');
        haystack.push_str(&normalize_for_compare(keyword));
    }
    let local_stay_signal = text_contains_any(
        &haystack,
        &[
            "neighborhood",
            "neighborhoods",
            "where to stay",
            "stay",
            "walkability",
            "transit",
            "museum",
            "museums",
        ],
    );
    let travel_signal = text_contains_any(
        &haystack,
        &[
            "family",
            "friendly",
            "visitor",
            "visit",
            "travel",
            "hotel",
            "hotels",
            "food",
            "restaurant",
            "restaurants",
        ],
    );
    local_stay_signal && travel_signal
}

fn local_stay_query_tail(entity: &str, keywords: &[String]) -> String {
    let entity_key = normalize_for_compare(entity);
    let mut pieces = Vec::<String>::new();
    for keyword in keywords {
        let cleaned = clean_text(keyword, 80);
        let key = normalize_for_compare(&cleaned);
        if cleaned.is_empty() || key == entity_key {
            continue;
        }
        pieces.push(cleaned);
        if pieces.len() >= 4 {
            break;
        }
    }
    clean_text(&pieces.join(" "), 220)
}

fn push_local_stay_entity_queries(out: &mut Vec<String>, entity: &str, keywords: &[String]) {
    let subject = exact_subject_query_term(entity);
    if subject.is_empty() {
        return;
    }
    let tail = local_stay_query_tail(entity, keywords);
    if !tail.is_empty() {
        push_unique_query(out, format!("{subject} {tail} travel guide comparison"));
    }
    push_unique_query(out, format!("{subject} where to stay guide"));
    push_unique_query(out, format!("{subject} neighborhood guide"));
}

fn push_unique_query(out: &mut Vec<String>, raw: String) {
    let cleaned = clean_text(&raw, 600);
    if cleaned.is_empty() || out.iter().any(|current| current == &cleaned) {
        return;
    }
    out.push(cleaned);
}

fn derived_queries(
    prompt: &str,
    coverage_entities: &[String],
    coverage_facets: &[String],
    keywords: &[String],
) -> Vec<String> {
    let mut queries = vec![clean_text(prompt, 600)];
    let local_stay_research = prompt_or_keywords_look_like_local_stay_research(prompt, keywords);
    if coverage_entities.len() >= 2 {
        let mut pieces = coverage_entities
            .iter()
            .take(4)
            .map(|entity| exact_subject_query_term(entity))
            .filter(|entity| !entity.is_empty())
            .collect::<Vec<_>>();
        pieces.extend(
            coverage_facets
                .iter()
                .filter(|facet| !looks_like_broad_or_temporal_facet(facet))
                .take(2)
                .map(|facet| clean_text(facet, 160))
                .filter(|facet| !facet.is_empty()),
        );
        if pieces.len() >= 2 {
            push_unique_query(&mut queries, format!("{} comparison", pieces.join(" ")));
            push_unique_query(
                &mut queries,
                format!("{} independent comparison", pieces.join(" ")),
            );
            push_unique_query(&mut queries, format!("{} reviews", pieces.join(" ")));
            push_unique_query(&mut queries, pieces.join(" "));
        }
    }
    let entity_lane_limit = if coverage_entities.len() >= 2 { 3 } else { 2 };
    for entity in coverage_entities.iter().take(entity_lane_limit) {
        if local_stay_research {
            push_local_stay_entity_queries(&mut queries, entity, keywords);
            continue;
        }
        let subject = exact_subject_query_term(entity);
        if subject.is_empty() {
            continue;
        }
        push_unique_query(&mut queries, format!("{subject} official site"));
        push_unique_query(&mut queries, format!("{subject} official documentation"));
    }
    if queries.len() >= 12 {
        queries.truncate(12);
        return queries;
    }
    let has_topical_facet = coverage_facets
        .iter()
        .any(|facet| !looks_like_temporal_scope_facet(facet));
    for facet in coverage_facets.iter().take(2) {
        let facet = clean_text(facet, 160);
        if facet.is_empty() {
            continue;
        }
        if has_topical_facet && looks_like_temporal_scope_facet(&facet) {
            continue;
        }
        push_unique_query(&mut queries, format!("{facet} source-backed evidence"));
        push_unique_query(&mut queries, format!("{facet} recent developments"));
        push_unique_query(&mut queries, format!("{facet} independent analysis"));
        if broad_sentiment_prompt(prompt, &facet) {
            push_unique_query(
                &mut queries,
                format!("{facet} public sentiment user reports"),
            );
        }
    }
    queries
}

fn broad_sentiment_prompt(prompt: &str, facet: &str) -> bool {
    let joined = format!(
        "{} {}",
        normalize_for_compare(prompt),
        normalize_for_compare(facet)
    );
    ["sentiment", "saying", "reviews", "complaints", "praise"]
        .iter()
        .any(|needle| joined.contains(needle))
}

fn derived_keyword_token(raw: &str) -> String {
    let normalized = normalize_for_compare(raw);
    let trimmed = normalized
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .to_string();
    clean_text(&trimmed, 64)
}

pub(super) fn load_request_pack_index(path: &str) -> BTreeMap<String, Value> {
    let report = read_json(path);
    report
        .get("cases")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let case_id = str_at(row, &["case_id"], "");
                    if case_id.is_empty() {
                        return None;
                    }
                    let request = row
                        .pointer("/response_diagnostics/pending_tool_request")
                        .or_else(|| {
                            row.pointer(
                                "/turn_sequence/initial_response_diagnostics/pending_tool_request",
                            )
                        })
                        .filter(report_request_usable)
                        .cloned()?;
                    Some((case_id, request))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn report_request_usable(request: &&Value) -> bool {
    let Some(request_obj) = request.as_object() else {
        return false;
    };
    if let Some(input_obj) = request.get("input").and_then(Value::as_object) {
        if !input_obj.is_empty() {
            return true;
        }
    }
    request_obj.contains_key("query")
        || request_obj.contains_key("queries")
        || request_obj.contains_key("url")
        || request_obj.contains_key("locator")
}
