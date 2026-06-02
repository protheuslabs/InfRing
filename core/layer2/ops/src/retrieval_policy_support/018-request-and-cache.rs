// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BatchQueryKeywordPack {
    keywords: Vec<String>,
    entities: Vec<String>,
    facets: Vec<String>,
    aliases: Vec<String>,
    negative_terms: Vec<String>,
    metadata_authority: String,
}

impl BatchQueryKeywordPack {
    fn is_empty(&self) -> bool {
        self.keywords.is_empty()
            && self.entities.is_empty()
            && self.facets.is_empty()
            && self.aliases.is_empty()
            && self.negative_terms.is_empty()
    }
}

fn contains_antibot_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing",
        "error-lite@duckduckgo.com",
        "anomaly-modal",
        "captcha",
        "access to this page has been denied",
        "are you a robot",
        "confirm you are a human",
        "press & hold to confirm you are a human",
        "press and hold to confirm you are a human",
        "not a bot",
        "not a robot",
        "robot check",
        "detected unusual activity",
        "unusual activity from your computer network",
        "click the box below",
        "verify you are human",
        "checking your browser before accessing",
        "cf-challenge",
        "cloudflare ray id",
        "just a moment...",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn contains_web_junk_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if contains_antibot_marker(&lowered) {
        return true;
    }
    [
        "please enable javascript",
        "enable javascript and cookies",
        "access denied",
        "copy markdown",
        "copy as markdown",
        "403 forbidden",
        "login required",
        "open in chatgpt",
        "open in claude",
        "open in cursor",
        "subscribe to continue",
        "please log in to continue",
        "this content is not available in your region",
        "view as markdown",
        "we use cookies to improve your experience",
        "manage your cookie preferences",
        "copyright zendesk, inc",
        "use of this source code is governed",
        ":root { --",
        "window.__",
        "document.addeventlistener",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_domain_list_noise(text: &str) -> bool {
    let cleaned = clean_text(text, 1_600);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 16);
    if domains.len() < 3 {
        return false;
    }
    let words = cleaned.split_whitespace().count();
    words <= (domains.len() * 3 + 10)
}

fn query_recovery_policy_strings(policy: &Value, pointer: &str, max_len: usize) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, max_len))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn query_recovery_policy_strings_with_default(
    policy: &Value,
    pointer: &str,
    max_len: usize,
) -> Vec<String> {
    let configured = query_recovery_policy_strings(policy, pointer, max_len);
    if configured.is_empty() {
        query_recovery_policy_strings(&default_policy(), pointer, max_len)
    } else {
        configured
    }
}

fn broad_current_research_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn broad_current_research_recovery_markers(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/broad_current_research/intent_markers",
        80,
    )
    .into_iter()
    .map(|row| row.to_ascii_lowercase())
    .collect()
}

fn query_looks_like_broad_current_research(policy: &Value, query: &str) -> bool {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() || !current_web_intent(&cleaned) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains('"')
        || lowered.contains('`')
    {
        return false;
    }
    let word_count = cleaned.split_whitespace().count();
    let broad_marker = broad_current_research_recovery_markers(policy)
        .iter()
        .any(|marker| lowered.contains(marker));
    broad_marker || word_count <= 8
}

fn facet_aware_evidence_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/coverage_aware_evidence/enabled")
        .and_then(Value::as_bool)
        .or_else(|| {
            policy
                .pointer("/batch_query/coverage_aware_query_planning/coverage_buckets/enabled")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn facet_aware_max_facets(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_aware_evidence/max_facets")
        .or_else(|| {
            policy.pointer("/batch_query/coverage_aware_query_planning/budget/default_max_lanes")
        })
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, budget.max_candidates.clamp(1, 16) as u64) as usize
}

fn facet_aware_min_terms(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/coverage_aware_evidence/min_facet_terms")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 6) as usize
}

fn coverage_gap_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/coverage_gap_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn coverage_gap_recovery_min_usable_evidence(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_gap_recovery/min_usable_evidence")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_evidence.max(1) as u64) as usize
}

fn coverage_gap_recovery_min_covered_facets(
    policy: &Value,
    facet_count: usize,
    budget: ApertureBudget,
) -> usize {
    if facet_count == 0 {
        return 0;
    }
    let configured_ratio = policy
        .pointer("/batch_query/coverage_gap_recovery/min_covered_facet_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let ratio_target = ((facet_count as f64) * configured_ratio).ceil() as usize;
    let configured_min = policy
        .pointer("/batch_query/coverage_gap_recovery/min_covered_facets")
        .and_then(Value::as_u64)
        .unwrap_or(2) as usize;
    ratio_target
        .max(configured_min)
        .min(facet_count)
        .min(budget.max_evidence.max(1))
}

fn coverage_gap_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_gap_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn coverage_gap_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/coverage_gap_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 320))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "{facet} source-backed evidence".to_string(),
                "{facet} primary or official source".to_string(),
                "{facet} independent analysis evidence".to_string(),
                "{facet} examples reports data".to_string(),
            ]
        })
}

fn claim_gap_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/claim_gap_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn claim_gap_recovery_min_materialized_evidence(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/claim_gap_recovery/min_materialized_evidence")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, budget.max_evidence.max(1) as u64) as usize
}

fn claim_gap_recovery_min_claim_hints(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/claim_gap_recovery/min_claim_hints")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, (budget.max_evidence.max(1) * 2) as u64) as usize
}

fn claim_gap_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/claim_gap_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn claim_gap_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/claim_gap_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 320))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "{query} detailed findings".to_string(),
                "{query} source-backed evidence".to_string(),
            ]
        })
}
