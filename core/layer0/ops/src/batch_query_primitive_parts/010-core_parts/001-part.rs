fn looks_like_empty_duckduckgo_instant_shell(decoded: &Value) -> bool {
    let Some(obj) = decoded.as_object() else {
        return false;
    };
    let metadata_keys = [
        "Abstract",
        "AbstractSource",
        "AbstractText",
        "AbstractURL",
        "Answer",
        "AnswerType",
        "Definition",
        "DefinitionSource",
        "DefinitionURL",
        "Heading",
        "RelatedTopics",
        "Results",
        "Type",
    ];
    let metadata_hits = metadata_keys
        .iter()
        .filter(|key| obj.contains_key(**key))
        .count();
    if metadata_hits < 5 {
        return false;
    }
    let has_usable_primary_text = ["AbstractText", "Answer", "Definition", "Heading"]
        .iter()
        .any(|key| {
            clean_text(
                obj.get(*key).and_then(Value::as_str).unwrap_or(""),
                400,
            )
            .len()
                > 1
        });
    if has_usable_primary_text {
        return false;
    }
    let has_related_topics = obj
        .get("RelatedTopics")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if has_related_topics {
        return false;
    }
    let has_results = obj
        .get("Results")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    !has_results
}

fn looks_like_truncated_duckduckgo_instant_shell(text: &str) -> bool {
    let lowered = clean_text(text, 3_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let empty_markers = [
        "\"abstract\":\"\"",
        "\"abstracttext\":\"\"",
        "\"answer\":\"\"",
        "\"definition\":\"\"",
        "\"heading\":\"\"",
        "\"entity\":\"\"",
        "\"relatedtopics\":[]",
        "\"results\":[]",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    empty_markers >= 4
}

fn looks_like_source_only_snippet(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if lowered.starts_with("potential sources:")
        || lowered.starts_with("candidate sources:")
        || lowered.starts_with("found sources:")
    {
        let domain_hits = extract_domains_from_text(&lowered, 8).len();
        let word_count = lowered.split_whitespace().count();
        if domain_hits > 0 && word_count <= 28 {
            return true;
        }
    }
    false
}

fn is_benchmark_or_comparison_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "benchmark",
        "benchmarks",
        "compare",
        "comparison",
        "competitor",
        "competitors",
        "versus",
        " vs ",
        "performance metrics",
        "latency",
        "throughput",
        "success rate",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn comparison_entities_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bcompare\s+([a-z0-9._-]+(?:\s+[a-z0-9._-]+){0,3})\s+(?:to|with|against|vs\.?|versus)\s+([a-z0-9._-]+(?:\s+[a-z0-9._-]+){0,3})",
        )
        .expect("comparison-entities")
    })
}

fn normalize_entity_phrase(raw: &str) -> String {
    let phrase = clean_text(raw, 120)
        .split_whitespace()
        .take_while(|token| {
            !matches!(
                token.to_ascii_lowercase().as_str(),
                "about" | "across" | "by" | "for" | "on" | "regarding"
            )
        })
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    clean_text(&phrase, 120)
}

fn comparison_entities_from_query(query: &str) -> Vec<String> {
    let resolved = resolve_deictic_framework_reference(query);
    if !is_benchmark_or_comparison_intent(&resolved) {
        return Vec::new();
    }
    let lowered = resolved.to_ascii_lowercase();
    if let Some(caps) = comparison_entities_regex().captures(&lowered) {
        let mut rows = Vec::new();
        if let Some(left) = caps.get(1) {
            let entity = normalize_entity_phrase(left.as_str());
            if !entity.is_empty() {
                rows.push(entity);
            }
        }
        if let Some(right) = caps.get(2) {
            let entity = normalize_entity_phrase(right.as_str());
            if !entity.is_empty() && !rows.iter().any(|row| row == &entity) {
                rows.push(entity);
            }
        }
        if rows.len() >= 2 {
            return rows;
        }
    }
    let mut entities = Vec::<String>::new();
    for known in [
        "infring",
        "openclaw",
        "langgraph",
        "autogen",
        "crewai",
        "haystack",
        "llamaindex",
        "aider",
    ] {
        if lowered.contains(known) {
            entities.push(known.to_string());
        }
    }
    entities.sort();
    entities.dedup();
    entities
}

fn metric_number_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b\d+(?:\.\d+)?\s*(?:%|ms|s|sec|seconds|minutes|x|qps|tps|ops/?sec|tokens/?s)\b",
        )
        .expect("metric-number")
    })
}

fn looks_like_metric_rich_text(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if metric_number_regex().is_match(&lowered) {
        return true;
    }
    let metric_term_hits = [
        "latency",
        "throughput",
        "accuracy",
        "precision",
        "recall",
        "f1",
        "ops/sec",
        "tokens/s",
        "qps",
        "memory",
        "cpu",
        "cost",
        "benchmark",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    metric_term_hits >= 2
}

fn looks_like_definition_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "dictionary",
        "definition",
        "meaning",
        "thesaurus",
        "merriam-webster",
        "dictionary.com",
        "cambridge.org/dictionary",
        "collinsdictionary",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_comparison_noise_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    let low_quality_domain = [
        "wordreference.com",
        "forum.wordreference.com",
        "wiktionary.org",
        "grammar",
        "english usage",
        "merriam-webster",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    let noisy_compare_form = lowered.contains("compare [a with b]")
        || lowered.contains("compare a with b")
        || lowered.contains("vs compare")
        || lowered.contains("wordreference forums");
    low_quality_domain || noisy_compare_form
}

fn query_asks_for_word_meaning(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "definition of",
        "meaning of",
        "define ",
        "dictionary",
        "what does",
        "what is the meaning",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn query_asks_for_shopping_or_products(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "buy ",
        "price",
        "pricing",
        "deal",
        "discount",
        "where can i buy",
        "shopping",
        "retailer",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn query_asks_for_music_or_lyrics(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "lyrics", "song", "album", "music", "artist", "track", "chords",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_shopping_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "bestbuy.",
        "best buy",
        "add to cart",
        "shopping cart",
        "free shipping",
        "coupon",
        "store pickup",
        "shop now",
        "product reviews",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_lyrics_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "lyrics",
        "song lyrics",
        "genius.com",
        "azlyrics",
        "musixmatch",
        "chords",
        "official audio",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_off_intent_noise_candidate(query: &str, candidate: &Candidate) -> bool {
    (looks_like_definition_candidate(candidate) && !query_asks_for_word_meaning(query))
        || (looks_like_shopping_candidate(candidate) && !query_asks_for_shopping_or_products(query))
        || (looks_like_lyrics_candidate(candidate) && !query_asks_for_music_or_lyrics(query))
}

fn candidate_title_for_relevance(candidate: &Candidate) -> String {
    if candidate
        .title
        .to_ascii_lowercase()
        .starts_with("web result from ")
    {
        String::new()
    } else {
        candidate.title.clone()
    }
}

fn candidate_relevance_text(candidate: &Candidate) -> String {
    format!(
        "{} {} {}",
        candidate_title_for_relevance(candidate),
        candidate.snippet,
        candidate.locator
    )
}

fn is_relevance_stop_token(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "any"
            | "are"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "how"
            | "in"
            | "into"
            | "is"
            | "it"
            | "its"
            | "of"
            | "on"
            | "or"
            | "our"
            | "the"
            | "their"
            | "them"
            | "this"
            | "those"
            | "to"
            | "try"
            | "was"
            | "we"
            | "were"
            | "with"
            | "you"
            | "your"
    )
}

fn tokenize_relevance(raw: &str, cap: usize) -> HashSet<String> {
    let mut out = HashSet::<String>::new();
    for token in clean_text(raw, 4_800)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        let normalized = token.trim();
        if normalized.len() < 3 || is_relevance_stop_token(normalized) {
            continue;
        }
        out.insert(normalized.to_string());
        if out.len() >= cap.max(1) {
            break;
        }
    }
    out
}

fn looks_like_portal_noise_candidate(candidate: &Candidate) -> bool {
    let lowered = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    [
        "login page",
        "log in",
        "sign in",
        "forgot password",
        "mychart",
        "watch live",
        "home news sport business",
        "create account",
        "manage account",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn candidate_passes_relevance_gate(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return true;
    }
    let candidate_relevance = candidate_relevance_text(candidate);
    let candidate_tokens = tokenize_relevance(&candidate_relevance, 120);
    if candidate_tokens.is_empty() {
        return false;
    }
    let overlap = query_tokens.intersection(&candidate_tokens).count();
    let query_has_distinctive_terms = query_has_distinctive_relevance_terms(query);
    let broad_current_article_evidence = current_web_intent(query)
        && !query_has_distinctive_terms
        && segment_has_current_signal(&candidate_relevance)
        && page_extraction_link_has_article_like_path(&candidate.locator)
        && content_rich_text(&candidate.snippet)
        && !looks_like_link_directory_or_aggregator_shell(&candidate.snippet);
    if is_framework_catalog_intent(query) && overlap == 0 {
        let combined = candidate_relevance.clone();
        let domain = candidate_domain_hint(candidate);
        if framework_name_hits(&combined) >= 1
            && looks_like_framework_overview_text(&combined)
            && framework_official_domain(&domain)
        {
            return true;
        }
    }
    if overlap == 0 {
        if broad_current_article_evidence {
            return true;
        }
        return false;
    }
    let overlap_ratio = overlap as f64 / query_tokens.len() as f64;
    if benchmark_intent {
        if overlap < 2 && overlap_ratio < 0.22 && !looks_like_metric_rich_text(&candidate.snippet) {
            return false;
        }
        if looks_like_portal_noise_candidate(candidate) && overlap < 3 {
            return false;
        }
        return true;
    }
    if looks_like_portal_noise_candidate(candidate) && overlap < 2 && overlap_ratio < 0.25 {
        return false;
    }
    true
}

fn candidate_mentions_entity(candidate: &Candidate, entity: &str) -> bool {
    let needle = clean_text(entity, 80).to_ascii_lowercase();
    if needle.is_empty() {
        return false;
    }
    let haystack = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    haystack.contains(&needle)
}
