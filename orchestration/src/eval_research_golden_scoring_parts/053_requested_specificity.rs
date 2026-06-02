// Layer ownership: orchestration (research eval authority)

fn requested_specificity_satisfaction(
    normalized_prompt: &str,
    response_text: &str,
    normalized_response: &str,
    payload: &Value,
    answer_unit_usefulness: &Value,
    coverage_entities: &[String],
    entity_coverage: f64,
) -> Value {
    let requested_kinds = requested_specificity_kinds(normalized_prompt);
    let requested = !requested_kinds.is_empty();
    let explicit_gap = explicit_specificity_gap_signal(normalized_response);
    let concrete_named_units = concrete_named_answer_units(response_text);
    let prompt_named_entity_units = prompt_named_entity_specificity_units(
        &requested_kinds,
        coverage_entities,
        entity_coverage,
    );
    let evidence_concrete_units = concrete_named_evidence_units(payload, normalized_prompt);
    let min_specific_units = requested_specificity_min_units(&requested_kinds);
    let evidence_specificity_available = !requested || evidence_concrete_units >= min_specific_units;
    let direct_useful_units = answer_unit_usefulness
        .get("direct_useful_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let useful_units_pass = answer_unit_usefulness
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let bounded_substitute_useful = explicit_gap
        && useful_units_pass
        && direct_useful_units >= 3
        && has_tradeoff_or_structure(normalized_response);
    let concrete_answer_present = concrete_named_units >= min_specific_units
        || prompt_named_entity_units >= min_specific_units;
    let pass = !requested || concrete_answer_present || bounded_substitute_useful || !explicit_gap;
    let excellent_ready = !requested || concrete_answer_present || !explicit_gap;
    let failure_boundary = requested_specificity_failure_boundary(
        requested,
        concrete_answer_present,
        evidence_specificity_available,
        explicit_gap,
        bounded_substitute_useful,
    );
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
        "minimum_specific_units": min_specific_units,
        "evidence_concrete_units": evidence_concrete_units,
        "evidence_specificity_available": evidence_specificity_available,
        "concrete_named_units": concrete_named_units,
        "prompt_named_entity_units": prompt_named_entity_units,
        "prompt_named_entity_coverage": entity_coverage,
        "concrete_answer_present": concrete_answer_present,
        "explicit_specificity_gap": explicit_gap,
        "bounded_substitute_useful": bounded_substitute_useful,
        "failure_boundary": failure_boundary,
        "direct_useful_units": direct_useful_units,
        "pass": pass,
        "excellent_ready": excellent_ready,
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "Generic specificity lane. When a prompt asks for concrete examples, named options, dates, comparisons, or a shortlist, an answer that explicitly says the retrieved evidence lacks that specificity can pass as a bounded substitute, but does not earn Excellent unless it actually delivers the requested concrete units."
    })
}

fn prompt_named_entity_specificity_units(
    requested_kinds: &[String],
    coverage_entities: &[String],
    entity_coverage: f64,
) -> u64 {
    if coverage_entities.is_empty() {
        return 0;
    }
    let specificity_by_named_options = requested_kinds.iter().any(|kind| {
        matches!(kind.as_str(), "shortlist_or_options" | "comparison" | "named_examples")
    });
    if !specificity_by_named_options || entity_coverage < 0.75 {
        return 0;
    }
    coverage_entities.len() as u64
}

fn requested_specificity_failure_boundary(
    requested: bool,
    concrete_answer_present: bool,
    evidence_specificity_available: bool,
    explicit_gap: bool,
    bounded_substitute_useful: bool,
) -> &'static str {
    if !requested {
        return "not_requested";
    }
    if concrete_answer_present {
        return "requested_specificity_satisfied";
    }
    if evidence_specificity_available && explicit_gap {
        return "answer_declared_specificity_gap_despite_available_evidence";
    }
    if evidence_specificity_available {
        return "answer_omitted_available_specificity";
    }
    if explicit_gap && bounded_substitute_useful {
        return "evidence_lacks_specificity_bounded_substitute";
    }
    if explicit_gap {
        return "evidence_lacks_specificity_weak_substitute";
    }
    "evidence_lacks_specificity_unlabeled_substitute"
}

fn requested_specificity_kinds(normalized_prompt: &str) -> Vec<String> {
    let mut kinds = Vec::<String>::new();
    if prompt_requests_named_examples(normalized_prompt) {
        kinds.push("named_examples".to_string());
    }
    if prompt_requests_shortlist_or_options(normalized_prompt) {
        kinds.push("shortlist_or_options".to_string());
    }
    if prompt_requests_concrete_comparison(normalized_prompt) {
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
            "give me examples",
            "examples of",
            "actual incidents",
            "actual events",
            "actual cases",
            "actual examples",
            "examples",
            "specific examples",
            "named ",
            "which state",
            "which states",
            "which state laws",
            "which laws",
            "which companies",
            "which products",
            "which models",
            "which tools",
            "which platforms",
            "which services",
            "which places",
            "which restaurants",
            "which schools",
            "which drugs",
            "which treatments",
            "which policies",
            "which cases",
            "which programs",
            "news",
            "headlines",
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
    ) || (contains_any(normalized_prompt, &["notable ", "major "])
        && contains_any(
            normalized_prompt,
            &[
                "breakthrough",
                "breakthroughs",
                "milestone",
                "milestones",
                "development",
                "developments",
                "incident",
                "incidents",
                "event",
                "events",
            ],
        ))
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
            "options",
            "recommendations",
            "pick",
            "picks",
        ],
    )
}

fn prompt_requests_concrete_comparison(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            "versus",
            " vs ",
            "which is better",
            "which one is better",
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

fn concrete_named_evidence_units(payload: &Value, normalized_prompt: &str) -> u64 {
    let mut seen = BTreeSet::<String>::new();
    let mut count = 0_u64;
    for text in evidence_specificity_texts(payload) {
        for unit in answer_text_units(&text) {
            let normalized_unit = normalize_for_compare(&unit);
            if normalized_unit.split_whitespace().count() < 4 {
                continue;
            }
            if seen.insert(normalized_unit)
                && concrete_named_evidence_unit_signal(&unit, normalized_prompt)
            {
                count += 1;
            }
        }
    }
    count
}

fn concrete_named_evidence_unit_signal(unit: &str, normalized_prompt: &str) -> bool {
    has_specific_token_shape_outside_prompt(unit, normalized_prompt)
        || has_multiword_proper_name_outside_prompt(unit, normalized_prompt)
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

fn has_multiword_proper_name_outside_prompt(unit: &str, normalized_prompt: &str) -> bool {
    let mut run = 0_u64;
    let mut pieces = Vec::<String>::new();
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
        let candidate = idx > 0
            && is_capitalized
            && normalized.len() >= 3
            && !generic_specificity_term(&normalized)
            && !prompt_contains_specificity_token(normalized_prompt, &normalized);
        if candidate {
            run += 1;
            pieces.push(normalized);
            if run >= 2 {
                let phrase = pieces
                    .iter()
                    .rev()
                    .take(run as usize)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join(" ");
                return !normalized_prompt.contains(&phrase);
            }
        } else {
            run = 0;
            pieces.clear();
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

fn has_specific_token_shape_outside_prompt(unit: &str, normalized_prompt: &str) -> bool {
    unit.split_whitespace().any(|raw| {
        let cleaned = raw.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.' && ch != '/'
        });
        if cleaned.is_empty() {
            return false;
        }
        let normalized = normalize_research_token(cleaned);
        if generic_specificity_term(&normalized)
            || prompt_contains_specificity_token(normalized_prompt, &normalized)
        {
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

fn prompt_contains_specificity_token(normalized_prompt: &str, token: &str) -> bool {
    if token.len() < 3 {
        return false;
    }
    normalized_prompt
        .split_whitespace()
        .map(normalize_research_token)
        .any(|prompt_token| prompt_token == token)
}

fn evidence_specificity_texts(payload: &Value) -> Vec<String> {
    let mut texts = Vec::<String>::new();
    collect_evidence_specificity_texts(payload, &mut texts);
    texts.sort();
    texts.dedup();
    texts
}

fn collect_evidence_specificity_texts(value: &Value, texts: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_evidence_specificity_texts(item, texts);
            }
        }
        Value::Object(map) => {
            for key in [
                "tools",
                "evidence_pack",
                "evidence_refs",
                "evidence_claims",
                "raw_results",
                "source_refs",
                "citations",
            ] {
                if let Some(child) = map.get(key) {
                    collect_evidence_specificity_texts(child, texts);
                }
            }
            for key in [
                "relevant_extract",
                "snippet",
                "summary",
                "extract",
                "claim",
                "claims",
                "claim_hints",
                "answer_units",
                "materialized_text",
                "page_text",
            ] {
                if let Some(child) = map.get(key) {
                    collect_evidence_specificity_texts(child, texts);
                }
            }
        }
        Value::String(raw) => {
            let cleaned = clean_text(raw, 1_200);
            if cleaned.split_whitespace().count() >= 4 {
                texts.push(cleaned);
            }
        }
        _ => {}
    }
}
