// Layer ownership: orchestration (research eval authority)

fn requested_specificity_satisfaction(
    normalized_prompt: &str,
    response_text: &str,
    normalized_response: &str,
    answer_unit_usefulness: &Value,
) -> Value {
    let requested_kinds = requested_specificity_kinds(normalized_prompt);
    let requested = !requested_kinds.is_empty();
    let explicit_gap = explicit_specificity_gap_signal(normalized_response);
    let concrete_named_units = concrete_named_answer_units(response_text);
    let direct_useful_units = answer_unit_usefulness
        .get("direct_useful_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let useful_units_pass = answer_unit_usefulness
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let bounded_substitute_useful =
        explicit_gap && useful_units_pass && direct_useful_units >= 3 && has_tradeoff_or_structure(normalized_response);
    let concrete_answer_present = concrete_named_units >= requested_specificity_min_units(&requested_kinds);
    let pass = !requested || concrete_answer_present || bounded_substitute_useful || !explicit_gap;
    let excellent_ready = !requested || concrete_answer_present || !explicit_gap;
    let mut blockers = Vec::<String>::new();
    if requested && explicit_gap && !concrete_answer_present {
        blockers.push("requested_specificity_not_delivered".to_string());
    }
    if requested && !pass {
        blockers.push("requested_specificity_unsatisfied".to_string());
    }
    json!({
        "schema_version": 1,
        "lane_id": "requested_specificity_satisfaction_v1",
        "requested": requested,
        "requested_kinds": requested_kinds,
        "concrete_named_units": concrete_named_units,
        "concrete_answer_present": concrete_answer_present,
        "explicit_specificity_gap": explicit_gap,
        "bounded_substitute_useful": bounded_substitute_useful,
        "direct_useful_units": direct_useful_units,
        "pass": pass,
        "excellent_ready": excellent_ready,
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "Generic specificity lane. When a prompt asks for concrete examples, named options, dates, comparisons, or a shortlist, an answer that explicitly says the retrieved evidence lacks that specificity can pass as a bounded substitute, but does not earn Excellent unless it actually delivers the requested concrete units."
    })
}

fn requested_specificity_kinds(normalized_prompt: &str) -> Vec<String> {
    let mut kinds = Vec::<String>::new();
    if prompt_requests_named_examples(normalized_prompt) {
        kinds.push("named_examples".to_string());
    }
    if prompt_requests_shortlist_or_options(normalized_prompt) {
        kinds.push("shortlist_or_options".to_string());
    }
    if prompt_requests_comparison(normalized_prompt) {
        kinds.push("comparison".to_string());
    }
    if prompt_requests_dates_or_numbers(normalized_prompt) {
        kinds.push("dates_or_numbers".to_string());
    }
    kinds.sort();
    kinds.dedup();
    kinds
}

fn prompt_requests_named_examples(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            "what are some",
            "some ",
            "examples",
            "which ",
            "who ",
            "where ",
            "notable ",
            "specific ",
            "named ",
            "news",
            "headlines",
            "developments",
            "breakthroughs",
            "milestones",
            "incidents",
            "events",
            "products",
            "models",
            "companies",
            "tools",
            "platforms",
            "services",
            "places",
            "restaurants",
            "schools",
            "drugs",
            "treatments",
            "policies",
            "cases",
        ],
    )
}

fn prompt_requests_shortlist_or_options(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            "shortlist",
            "short list",
            "list ",
            "rank ",
            "ranking",
            "top ",
            "best ",
            "options",
            "recommend",
            "recommendations",
            "pick",
            "picks",
            "choose",
        ],
    )
}

fn prompt_requests_comparison(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            "compare",
            "comparison",
            "versus",
            " vs ",
            "better",
            "worse",
            "stronger",
            "weaker",
            "tradeoff",
            "trade-off",
            "between ",
        ],
    )
}

fn prompt_requests_dates_or_numbers(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            "date",
            "dates",
            "when ",
            "timeline",
            "deadline",
            "price",
            "pricing",
            "cost",
            "rate",
            "rates",
            "number",
            "numbers",
            "how many",
            "how much",
            "battery life",
            "published",
            "released",
        ],
    )
}

fn requested_specificity_min_units(kinds: &[String]) -> u64 {
    if kinds.iter().any(|kind| kind == "comparison") {
        return 2;
    }
    if kinds.iter().any(|kind| kind == "shortlist_or_options") {
        return 2;
    }
    if kinds.iter().any(|kind| kind == "named_examples") {
        return 2;
    }
    1
}

fn explicit_specificity_gap_signal(normalized_response: &str) -> bool {
    contains_any(
        normalized_response,
        &[
            "no specific",
            "not include specific",
            "does not include specific",
            "doesn t include specific",
            "doesn't include specific",
            "do not name",
            "does not name",
            "doesn t name",
            "doesn't name",
            "not name specific",
            "without retrieved specifics",
            "retrieved material does not specify",
            "retrieved material doesn t specify",
            "retrieved material doesn't specify",
            "available evidence does not specify",
            "source material does not provide",
            "does not provide named",
            "doesn t provide named",
            "doesn't provide named",
            "lacks named",
            "lacks specific",
            "does not list",
            "doesn t list",
            "doesn't list",
            "cannot provide a source-backed",
            "cannot provide a source backed",
            "can not provide a source-backed",
            "can not provide a source backed",
            "available context consists solely",
            "context consists solely",
        ],
    )
}

fn concrete_named_answer_units(response_text: &str) -> u64 {
    answer_text_units(response_text)
        .iter()
        .filter(|unit| concrete_named_answer_unit_signal(unit))
        .count() as u64
}

fn concrete_named_answer_unit_signal(unit: &str) -> bool {
    let normalized_unit = normalize_for_compare(unit);
    if answer_unit_is_hedged_or_gap(&normalized_unit) {
        return false;
    }
    has_specific_token_shape(unit) || has_multiword_proper_name(unit)
}

fn generic_specificity_term(term: &str) -> bool {
    matches!(
        term,
        "anc"
            | "api"
            | "apis"
            | "app"
            | "apps"
            | "bluetooth"
            | "case"
            | "docs"
            | "github"
            | "google"
            | "model"
            | "models"
            | "source"
            | "sources"
            | "web"
    )
}

fn has_multiword_proper_name(unit: &str) -> bool {
    let mut run = 0_u64;
    for (idx, raw) in unit.split_whitespace().enumerate() {
        let cleaned = raw.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        if cleaned.is_empty() {
            continue;
        }
        let is_capitalized = cleaned
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false);
        let normalized = normalize_research_token(cleaned);
        let candidate =
            idx > 0 && is_capitalized && normalized.len() >= 3 && !generic_specificity_term(&normalized);
        if candidate {
            run += 1;
            if run >= 2 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn has_specific_token_shape(unit: &str) -> bool {
    unit.split_whitespace().any(|raw| {
        let cleaned = raw.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.' && ch != '/'
        });
        if cleaned.is_empty() {
            return false;
        }
        let normalized = normalize_research_token(cleaned);
        if generic_specificity_term(&normalized) {
            return false;
        }
        if token_looks_domain_like(cleaned) {
            return true;
        }
        if cleaned.chars().any(|ch| ch.is_ascii_digit()) {
            return true;
        }
        let letters = cleaned
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .collect::<Vec<_>>();
        let uppercase_letters = letters.iter().filter(|ch| ch.is_ascii_uppercase()).count();
        let is_acronym = letters.len() >= 2
            && uppercase_letters >= 2
            && uppercase_letters * 2 >= letters.len();
        let has_internal_capital = letters.iter().skip(1).any(|ch| ch.is_ascii_uppercase());
        is_acronym || has_internal_capital
    })
}
