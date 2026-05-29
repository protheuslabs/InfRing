fn response_matches_prompt_intent(normalized_prompt: &str, normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let asks_comparison = contains_any(
        normalized_prompt,
        &[
            "compare",
            "versus",
            " vs ",
            "tradeoff",
            "tradeoffs",
            "which",
        ],
    );
    if asks_comparison {
        return has_tradeoff_or_structure(normalized_response);
    }
    let asks_explanation = contains_any(
        normalized_prompt,
        &[
            "what",
            "why",
            "how",
            "explain",
            "research",
            "summarize",
            "find",
        ],
    );
    if asks_explanation {
        return has_tradeoff_or_structure(normalized_response)
            || normalized_response.contains("finding")
            || normalized_response.contains("evidence")
            || normalized_response.contains("because");
    }
    true
}

fn response_matches_decision_prompt(normalized_prompt: &str, normalized_response: &str) -> bool {
    let wants_decision = contains_any(
        normalized_prompt,
        &[
            "which",
            "best",
            "recommend",
            "tradeoff",
            "tradeoffs",
            "practical",
            "useful",
            "appropriate",
            "choose",
            "should",
        ],
    );
    !wants_decision || has_recommendation_signal(normalized_response)
}

fn response_has_right_granularity(normalized_response: &str) -> bool {
    let word_count = normalized_response.split_whitespace().count();
    (20..=900).contains(&word_count)
}

fn user_stated_required_entities(
    normalized_prompt: &str,
    required_entities: &[String],
) -> Vec<String> {
    required_entities
        .iter()
        .filter(|entity| required_entity_needs_entity_coverage(entity))
        .filter(|entity| normalized_response_covers_entity(normalized_prompt, entity))
        .cloned()
        .collect()
}

fn required_entity_needs_entity_coverage(entity: &str) -> bool {
    let trimmed = entity.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.chars().any(|ch| ch.is_ascii_digit()) {
        return true;
    }

    let normalized = normalize_for_compare(trimmed);
    let tokens = normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return false;
    }

    let raw_tokens = trimmed
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let distinctive_named_tokens = raw_tokens
        .iter()
        .filter(|token| token_has_distinctive_named_shape(token))
        .count();

    if trimmed.contains(['-', '_', '/', '.']) {
        if tokens.len() == 1 {
            return true;
        }
        if distinctive_named_tokens >= 1 {
            return true;
        }
    }

    if tokens.len() == 1 {
        if distinctive_named_tokens >= 1 {
            return true;
        }
        return !matches!(
            tokens[0],
            "agent"
                | "agents"
                | "agentic"
                | "benchmark"
                | "benchmarks"
                | "browser"
                | "company"
                | "comparison"
                | "credential"
                | "credentials"
                | "database"
                | "deployment"
                | "doc"
                | "docs"
                | "documentation"
                | "evidence"
                | "framework"
                | "frameworks"
                | "inference"
                | "integration"
                | "landscape"
                | "model"
                | "news"
                | "observability"
                | "pricing"
                | "product"
                | "prompt"
                | "provider"
                | "providers"
                | "rag"
                | "release"
                | "releases"
                | "retrieval"
                | "search"
                | "security"
                | "sentiment"
                | "snippet"
                | "snippets"
                | "stack"
                | "tool"
                | "tools"
                | "tradeoff"
                | "tradeoffs"
                | "update"
                | "vector"
                | "workflow"
                | "workflows"
        );
    }

    if distinctive_named_tokens >= 2 {
        return true;
    }

    false
}

fn token_has_distinctive_named_shape(raw: &str) -> bool {
    let cleaned = raw.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if cleaned.is_empty() {
        return false;
    }
    let mut chars = cleaned.chars();
    let first = chars.next().unwrap_or_default();
    let rest = chars.collect::<String>();
    let has_upper = cleaned.chars().any(|ch| ch.is_ascii_uppercase());
    let has_lower = cleaned.chars().any(|ch| ch.is_ascii_lowercase());
    let has_digit = cleaned.chars().any(|ch| ch.is_ascii_digit());
    if has_digit {
        return true;
    }
    if has_upper && has_lower {
        return cleaned.chars().all(|ch| ch.is_ascii_alphanumeric())
            && (cleaned.chars().skip(1).any(|ch| ch.is_ascii_uppercase())
                || (first.is_ascii_uppercase() && rest.chars().all(|ch| ch.is_ascii_lowercase())));
    }
    cleaned.chars().all(|ch| ch.is_ascii_uppercase()) && cleaned.len() >= 2
}

fn entity_supports_derived_initialism_alias(raw: &str) -> bool {
    let tokens = raw
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.len() < 2 {
        return false;
    }
    tokens
        .iter()
        .filter(|token| token_has_distinctive_named_shape(token))
        .count()
        >= 2
}
