
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

fn is_official_source_query_lane(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "official site",
        "official documentation",
        "official source",
        "official sources",
        "primary source evidence",
        "project sources",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn entityish_query_token(raw: &str) -> Option<String> {
    let cleaned = raw
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.')
        .trim();
    if cleaned.len() < 2 {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if matches!(
        lowered.as_str(),
        "a"
            | "an"
            | "and"
            | "as"
            | "at"
            | "avoid"
            | "based"
            | "browser"
            | "compare"
            | "current"
            | "focus"
            | "for"
            | "give"
            | "how"
            | "in"
            | "is"
            | "it"
            | "most"
            | "of"
            | "on"
            | "or"
            | "production"
            | "qa"
            | "repeatable"
            | "research"
            | "right"
            | "should"
            | "summarize"
            | "task"
            | "team"
            | "the"
            | "to"
            | "use"
            | "what"
            | "which"
            | "workflow"
            | "workflows"
    ) {
        return None;
    }
    let alpha_count = cleaned.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    let all_upper = alpha_count >= 2
        && cleaned
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .all(|ch| ch.is_ascii_uppercase());
    let has_inner_upper = cleaned.chars().skip(1).any(|ch| ch.is_ascii_uppercase());
    let has_titlecase = cleaned
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false);
    let has_digit = cleaned.chars().any(|ch| ch.is_ascii_digit());
    let has_symbolic_shape = cleaned.contains('-') || cleaned.contains('.');
    if !(all_upper || has_inner_upper || has_titlecase || has_digit || has_symbolic_shape) {
        return None;
    }
    Some(clean_text(cleaned, 120))
}

fn push_unique_subject_phrase(
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
    raw: &str,
) {
    let cleaned = clean_text(raw, 160);
    if cleaned.is_empty() {
        return;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if seen.insert(lowered) {
        out.push(cleaned);
    }
}

fn query_subject_phrases(query: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for quoted in query.split('"').skip(1).step_by(2) {
        let cleaned = clean_text(quoted, 160);
        if cleaned.is_empty() {
            continue;
        }
        let token_count = cleaned.split_whitespace().count();
        if token_count == 0 || token_count > 4 {
            continue;
        }
        push_unique_subject_phrase(&mut out, &mut seen, &cleaned);
    }
    let tokens = query
        .split_whitespace()
        .filter_map(entityish_query_token)
        .collect::<Vec<_>>();
    let mut current = Vec::<String>::new();
    let flush_current = |current: &mut Vec<String>,
                         out: &mut Vec<String>,
                         seen: &mut HashSet<String>| {
        if current.is_empty() {
            return;
        }
        let phrase = current.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
        push_unique_subject_phrase(out, seen, &phrase);
        if current.len() == 1 {
            push_unique_subject_phrase(out, seen, &current[0]);
        }
        current.clear();
    };
    for token in tokens {
        let continuation = current.last().map(|prior| {
            prior.chars()
                .last()
                .map(|ch| ch != '.')
                .unwrap_or(true)
        });
        if continuation.unwrap_or(true) && current.len() < 4 {
            current.push(token);
            continue;
        }
        flush_current(&mut current, &mut out, &mut seen);
        current.push(token);
    }
    flush_current(&mut current, &mut out, &mut seen);
    out
}

fn query_subject_phrase_matches_candidate(query: &str, candidate: &Candidate) -> bool {
    let haystack = candidate_relevance_text(candidate).to_ascii_lowercase();
    if haystack.is_empty() {
        return false;
    }
    query_subject_phrases(query)
        .iter()
        .map(|phrase| phrase.to_ascii_lowercase())
        .any(|phrase| haystack.contains(&phrase))
}

fn official_lane_subject_tokens(query: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for phrase in query_subject_phrases(query) {
        for piece in phrase
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .map(str::trim)
        {
            let lowered = piece.to_ascii_lowercase();
            if official_lane_subject_token_allowed(&lowered) && seen.insert(lowered.clone()) {
                out.push(lowered);
            }
        }
        let compact = compact_alnum(&phrase);
        if official_lane_subject_token_allowed(&compact) && seen.insert(compact.clone()) {
            out.push(compact);
        }
    }
    for token in clean_text(query, 800)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
    {
        if official_lane_subject_token_allowed(token) && seen.insert(token.to_string()) {
            out.push(token.to_string());
        }
    }
    out
}

fn official_lane_subject_token_allowed(token: &str) -> bool {
    if token.len() < 3 || is_relevance_stop_token(token) || is_weak_relevance_token(token) {
        return false;
    }
    !matches!(
        token,
        "doc"
            | "docs"
            | "documentation"
            | "homepage"
            | "home"
            | "official"
            | "primary"
            | "project"
            | "projects"
            | "reference"
            | "site"
            | "sites"
            | "source"
            | "sources"
            | "website"
            | "websites"
    )
}

fn compact_alnum(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>()
}

fn official_lane_direct_subject_source_signal(query: &str, candidate: &Candidate) -> bool {
    if !is_official_source_query_lane(query) {
        return false;
    }
    let subject_tokens = official_lane_subject_tokens(query);
    if subject_tokens.is_empty() {
        return false;
    }
    let locator = clean_text(&candidate.locator, 2_200).to_ascii_lowercase();
    if !(locator.starts_with("https://") || locator.starts_with("http://")) {
        return false;
    }
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    if domain.is_empty() || domain == "source" || is_search_engine_domain(&domain) {
        return false;
    }
    let locator_compact = compact_alnum(&locator);
    let domain_compact = compact_alnum(&domain);
    let trusted_path_source = source_trust_adjustment(candidate) >= 0.15
        || domain.starts_with("docs.")
        || domain.contains("developer.")
        || domain.contains("github.com")
        || locator.contains("/docs")
        || locator.contains("/documentation")
        || locator.contains("/reference")
        || locator.contains("/api");
    subject_tokens.iter().any(|token| {
        let compact = compact_alnum(token);
        if compact.len() < 3 {
            return false;
        }
        domain_compact.contains(&compact)
            || (trusted_path_source && locator_compact.contains(&compact))
    })
}

fn candidate_has_trusted_primary_source_signal(query: &str, candidate: &Candidate) -> bool {
    let combined = candidate_relevance_text(candidate);
    let locator = clean_text(&candidate.locator, 2_200).to_ascii_lowercase();
    let domain = candidate_domain_hint(candidate);
    let domain_lower = domain.to_ascii_lowercase();
    let snippet_words = clean_text(&candidate.snippet, 1_800).split_whitespace().count();
    let (overlap, distinctive_overlap, query_len) = query_overlap_profile(query, candidate);
    let subject_phrase_match = query_subject_phrase_matches_candidate(query, candidate);
    let direct_official_subject_source =
        official_lane_direct_subject_source_signal(query, candidate);
    let official_lane_domain_candidate = is_official_source_query_lane(query)
        && !domain_lower.is_empty()
        && domain_lower != "source"
        && !is_search_engine_domain(&domain_lower)
        && (200..400).contains(&candidate.status_code)
        && (locator.starts_with("https://") || locator.starts_with("http://"))
        && direct_official_subject_source
        && (distinctive_overlap >= 1 || overlap >= 2 || subject_phrase_match);
    let trusted_source = source_trust_adjustment(candidate) >= 0.15
        || framework_official_domain(&domain)
        || official_lane_domain_candidate
        || domain.to_ascii_lowercase().starts_with("docs.")
        || locator.contains("/docs")
        || locator.contains("/documentation")
        || locator.contains("/reference")
        || locator.contains("/api");
    if !trusted_source {
        return false;
    }
    let overview_signal = looks_like_framework_overview_text(&combined)
        || looks_like_framework_catalog_text(&combined)
        || locator.contains("/docs")
        || locator.contains("/documentation")
        || locator.contains("/reference")
        || locator.contains("/api");
    distinctive_overlap >= 1
        || subject_phrase_match
        || (overlap >= 2 && snippet_words >= 8)
        || (overlap >= 1
            && overview_signal
            && (query_len <= 8 || snippet_words >= 12))
}

fn candidate_has_trusted_official_source_signal(query: &str, candidate: &Candidate) -> bool {
    is_official_source_query_lane(query)
        && candidate_has_trusted_primary_source_signal(query, candidate)
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
            | "what"
            | "which"
            | "who"
            | "when"
            | "where"
            | "why"
            | "some"
            | "give"
            | "tell"
            | "show"
            | "find"
            | "about"
            | "compare"
            | "comparison"
            | "versus"
            | "with"
            | "you"
            | "your"
            | "verify"
            | "report"
            | "top"
            | "benchmark"
            | "benchmarks"
            | "metric"
            | "metrics"
            | "performance"
    )
}

fn tokenize_relevance(raw: &str, cap: usize) -> HashSet<String> {
    let mut out = HashSet::<String>::new();
    for token in clean_text(raw, 4_800)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
    {
        let normalized = token.trim();
        let canonical = canonicalize_relevance_token(normalized);
        if canonical.len() < 3 || is_relevance_stop_token(&canonical) {
            continue;
        }
        out.insert(canonical);
        if out.len() >= cap.max(1) {
            break;
        }
    }
    out
}

fn canonicalize_relevance_token(token: &str) -> String {
    if token.len() <= 4 || token.chars().all(|ch| ch.is_ascii_digit()) {
        return token.to_string();
    }
    if token.ends_with("ies") && token.len() > 5 {
        return format!("{}y", &token[..token.len() - 3]);
    }
    if ["ches", "shes", "sses", "xes", "zes", "oes"]
        .iter()
        .any(|suffix| token.ends_with(suffix))
        && token.len() > 5
    {
        return token[..token.len() - 2].to_string();
    }
    if token.ends_with('s')
        && token.len() > 4
        && !token.ends_with("ss")
        && !token.ends_with("us")
        && !token.ends_with("is")
        && !token.ends_with("ws")
        && !token.ends_with("ics")
    {
        return token[..token.len() - 1].to_string();
    }
    token.to_string()
}

fn is_weak_relevance_token(token: &str) -> bool {
    if token.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }
    matches!(
        token,
        "current"
            | "latest"
            | "recent"
            | "model"
            | "official"
            | "primary"
            | "source"
            | "sources"
            | "evidence"
            | "backed"
            | "verified"
            | "verification"
            | "reporting"
            | "publication"
            | "publications"
            | "published"
            | "peer"
            | "reviewed"
            | "detailed"
            | "brief"
            | "briefing"
            | "concise"
            | "prioritize"
            | "prioritise"
            | "cite"
            | "cites"
            | "cited"
            | "citation"
            | "citations"
            | "group"
            | "grouped"
            | "theme"
            | "themes"
            | "practical"
            | "recommendation"
            | "recommendations"
            | "caveat"
            | "caveats"
            | "finding"
            | "findings"
            | "overview"
            | "guide"
            | "general"
            | "information"
            | "world"
            | "global"
            | "today"
            | "week"
            | "month"
            | "year"
            | "daily"
            | "weekly"
            | "biggest"
            | "major"
            | "important"
            | "broad"
            | "broadly"
            | "different"
            | "multiple"
            | "various"
            | "breaking"
            | "headline"
            | "headlines"
            | "story"
            | "stories"
            | "development"
            | "developments"
            | "update"
            | "updates"
            | "landscape"
            | "online"
            | "web"
            | "news"
            | "research"
            | "report"
            | "reported"
            | "reports"
            | "science"
            | "scientific"
            | "breakthrough"
            | "breakthroughs"
            | "advance"
            | "advances"
            | "field"
            | "fields"
    )
}

fn query_has_distinctive_relevance_terms(query: &str) -> bool {
    tokenize_relevance(query, 40)
        .iter()
        .any(|token| !is_weak_relevance_token(token))
}

fn query_overlap_profile(query: &str, candidate: &Candidate) -> (usize, usize, usize) {
    let query_tokens = tokenize_relevance(query, 40);
    if query_tokens.is_empty() {
        return (0, 0, 0);
    }
    let candidate_tokens = tokenize_relevance(&candidate_relevance_text(candidate), 120);
    if candidate_tokens.is_empty() {
        return (0, 0, query_tokens.len());
    }
    let overlap = query_tokens
        .iter()
        .filter(|token| candidate_tokens.contains(token.as_str()))
        .count();
    let distinctive_overlap = query_tokens
        .iter()
        .filter(|token| !is_weak_relevance_token(token))
        .filter(|token| candidate_tokens.contains(token.as_str()))
        .count();
    (overlap, distinctive_overlap, query_tokens.len())
}

fn has_only_weak_query_overlap(query: &str, candidate: &Candidate) -> bool {
    let (overlap, distinctive_overlap, query_len) = query_overlap_profile(query, candidate);
    query_has_distinctive_relevance_terms(query)
        && query_len >= 2
        && overlap > 0
        && distinctive_overlap == 0
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

fn candidate_has_substantive_page_enriched_bridge(query: &str, candidate: &Candidate) -> bool {
    let source_kind = candidate.source_kind.to_ascii_lowercase();
    let permissions = candidate.permissions.as_deref().unwrap_or("").to_ascii_lowercase();
    let page_enriched =
        source_kind.contains("page_enriched") || permissions.contains("page_enriched");
    if !page_enriched {
        return false;
    }
    let combined = candidate_relevance_text(candidate);
    if combined.is_empty()
        || looks_like_link_directory_or_aggregator_shell(&combined)
        || looks_like_off_intent_noise_candidate(query, candidate)
        || looks_like_portal_noise_candidate(candidate)
    {
        return false;
    }
    let content_rich = content_rich_text(&candidate.snippet) || content_rich_text(&combined);
    if !content_rich {
        return false;
    }
    let trusted_or_article = candidate_has_trusted_primary_source_signal(query, candidate)
        || candidate_has_trusted_official_source_signal(query, candidate)
        || source_trust_adjustment(candidate) >= 0.1
        || page_extraction_link_has_article_like_path(&candidate.locator);
    if !trusted_or_article {
        return false;
    }
    query_subject_phrase_matches_candidate(query, candidate)
        || (current_web_intent(query) && segment_has_current_signal(&combined))
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
    let (overlap, distinctive_overlap, query_len) = query_overlap_profile(query, candidate);
    let query_has_distinctive_terms = query_has_distinctive_relevance_terms(query);
    let trusted_primary_source = candidate_has_trusted_primary_source_signal(query, candidate);
    let page_enriched_context_bridge =
        candidate_has_substantive_page_enriched_bridge(query, candidate);
    let broad_current_article_evidence = current_web_intent(query)
        && !query_has_distinctive_terms
        && segment_has_current_signal(&candidate_relevance)
        && (page_extraction_link_has_article_like_path(&candidate.locator)
            || source_trust_adjustment(candidate) >= 0.1)
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
        if broad_current_article_evidence || page_enriched_context_bridge {
            return true;
        }
        return false;
    }
    if query_has_distinctive_terms
        && query_len >= 3
        && distinctive_overlap == 0
        && !page_enriched_context_bridge
    {
        return false;
    }
    let overlap_ratio = overlap as f64 / query_len as f64;
    if benchmark_intent {
        if overlap < 2
            && overlap_ratio < 0.22
            && !looks_like_metric_rich_text(&candidate.snippet)
            && !trusted_primary_source
        {
            return false;
        }
        if query_has_distinctive_terms
            && query_len >= 3
            && overlap < 2
            && distinctive_overlap < 1
            && !trusted_primary_source
        {
            return false;
        }
        if looks_like_portal_noise_candidate(candidate) && overlap < 3 {
            return false;
        }
        return true;
    }
    if query_has_distinctive_terms
        && query_len >= 3
        && overlap < 2
        && !trusted_primary_source
        && !page_enriched_context_bridge
    {
        return false;
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

fn extract_metric_focused_fragment(text: &str) -> String {
    let cleaned = clean_text(text, 1_200)
        .replace("[...]", ". ")
        .replace("[\u{2026}]", ". ")
        .replace('\u{2026}', ". ")
        .replace('\u{2022}', ". ")
        .replace(" # ", ". ")
        .replace(" - ", ". ");
    if cleaned.is_empty() {
        return String::new();
    }
    let segments = cleaned
        .split(['.', ';', '\n', '|'])
        .map(|segment| clean_text(segment, 400))
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    for (index, segment_clean) in segments.iter().enumerate() {
        if looks_like_metric_rich_text(&segment_clean) {
            let previous = (0..index)
                .rev()
                .filter_map(|previous_index| segments.get(previous_index))
                .find(|previous_segment| {
                    let lowered = previous_segment.to_ascii_lowercase();
                    previous_segment.split_whitespace().count() >= 2
                        && !looks_like_metric_rich_text(previous_segment)
                        && !lowered.starts_with("http")
                        && !lowered.starts_with("www")
                        && !lowered.contains(" | ")
                });
            if segment_clean.split_whitespace().count() >= 5 {
                return segment_clean.clone();
            }
            if let Some(previous) = previous {
                let combined = clean_text(&format!("{previous} - {segment_clean}"), 420);
                if combined.split_whitespace().count() >= 5 {
                    return combined;
                }
            }
            return segment_clean.clone();
        }
    }
    cleaned
}
