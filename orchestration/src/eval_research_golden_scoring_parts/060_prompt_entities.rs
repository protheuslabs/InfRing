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

    if trimmed
        .chars()
        .any(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return true;
    }

    if trimmed.contains(['-', '_', '/', '.']) {
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

    if tokens.len() == 1 {
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

    !tokens.iter().all(|token| {
        matches!(
            *token,
            "agent"
                | "agents"
                | "agentic"
                | "best"
                | "benchmark"
                | "benchmarks"
                | "browser"
                | "company"
                | "comparison"
                | "credential"
                | "credentials"
                | "current"
                | "database"
                | "deployment"
                | "enterprise"
                | "framework"
                | "frameworks"
                | "injection"
                | "integration"
                | "landscape"
                | "latest"
                | "model"
                | "news"
                | "observability"
                | "prompt"
                | "provider"
                | "providers"
                | "public"
                | "rag"
                | "recent"
                | "release"
                | "releases"
                | "research"
                | "retrieval"
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
        )
    })
}
