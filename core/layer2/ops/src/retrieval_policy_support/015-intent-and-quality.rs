fn is_framework_catalog_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let ranking_marker = [
        "top ",
        "best ",
        "leading ",
        "popular ",
        "ranking",
        "rankings",
        "landscape",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    let explicit_catalog_marker = ["best ", "leading ", "popular ", "ranking", "rankings", "landscape"]
        .iter()
        .any(|marker| lowered.contains(marker));
    let framework_marker = [
        "agent framework",
        "agent frameworks",
        "agentic framework",
        "agentic frameworks",
        "framework",
        "frameworks",
        "agents sdk",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    let benchmark_marker = [
        "benchmark",
        "benchmarks",
        "performance metric",
        "performance metrics",
        "latency",
        "throughput",
        "success rate",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    if benchmark_marker && !explicit_catalog_marker {
        return false;
    }
    ranking_marker && framework_marker
}

#[cfg(test)]
fn canonical_framework_catalog_focus(query: &str) -> Option<String> {
    if !is_framework_catalog_intent(query) {
        return None;
    }
    let tokens = clean_text(query, 600)
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter(|token| token.len() > 2 || token.eq_ignore_ascii_case("ai"))
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| {
            !matches!(
                token.as_str(),
                "top" | "best" | "leading" | "popular" | "ranking" | "rankings" | "landscape"
            )
        })
        .collect::<Vec<_>>();
    let focus = clean_text(&tokens.join(" "), 600);
    if focus.contains("framework") {
        Some(focus)
    } else {
        None
    }
}

#[cfg(test)]
fn preferred_query_rewrite(base: &str) -> String {
    if looks_like_instructional_query(base) {
        return normalize_instructional_query(base)
            .unwrap_or_else(|| clean_text(&format!("{base} overview"), 600));
    }
    if let Some(focus) = canonical_framework_catalog_focus(base) {
        return clean_text(&format!("{focus} landscape"), 600);
    }
    clean_text(&format!("{base} overview"), 600)
}

fn is_local_subject_comparison_query(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let deictic_local_subject = [
        "this system",
        "this workspace",
        "this stack",
        "this platform",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    deictic_local_subject && is_benchmark_or_comparison_intent(query)
}

fn framework_name_hits(text: &str) -> usize {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    [
        "langgraph",
        "openai agents sdk",
        "autogen",
        "crewai",
        "llamaindex",
        "semantic kernel",
        "haystack",
        "mastra",
        "smolagents",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count()
}

fn framework_names_in_text(text: &str) -> Vec<&'static str> {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    [
        ("langgraph", "LangGraph"),
        ("openai agents sdk", "OpenAI Agents SDK"),
        ("autogen", "AutoGen"),
        ("crewai", "CrewAI"),
        ("llamaindex", "LlamaIndex"),
        ("semantic kernel", "Semantic Kernel"),
        ("haystack", "Haystack"),
        ("mastra", "Mastra"),
        ("smolagents", "smolagents"),
    ]
    .iter()
    .filter_map(|(needle, label)| lowered.contains(needle).then_some(*label))
    .collect::<Vec<_>>()
}

fn looks_like_framework_catalog_text(text: &str) -> bool {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if framework_name_hits(&lowered) >= 2 {
        return true;
    }
    lowered.contains("agent frameworks such as")
        || lowered.contains("popular agent frameworks")
        || lowered.contains("top agent frameworks")
        || lowered.contains("agentic frameworks")
}

fn looks_like_framework_overview_text(text: &str) -> bool {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    if lowered.is_empty() || framework_name_hits(&lowered) < 1 {
        return false;
    }
    lowered.contains("framework")
        || lowered.contains("agent")
        || lowered.contains("sdk")
        || lowered.contains("workflow")
        || lowered.contains("orchestration")
}

fn looks_like_competitive_programming_dump(text: &str) -> bool {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "csdn.net",
        "acm",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_hits >= 3
}

fn candidate_needs_link_fetch(query: &str, policy: &Value, candidate: &Candidate) -> bool {
    let snippet = clean_text(&candidate.snippet, 1_600);
    if snippet.is_empty() {
        return true;
    }
    if looks_like_low_signal_search_summary(&snippet)
        || looks_like_source_only_snippet(&snippet)
        || contains_web_junk_marker(&snippet)
    {
        return true;
    }
    if looks_like_competitive_programming_dump(&format!("{} {}", candidate.title, snippet)) {
        return true;
    }
    let min_words = page_extraction_min_snippet_words_before_skip(policy);
    if min_words > 0 && snippet.split_whitespace().count() < min_words {
        return true;
    }
    let min_overlap = page_extraction_min_query_overlap_terms_before_skip(policy);
    if min_overlap > 0 && query_overlap_terms(query, candidate) < min_overlap {
        return true;
    }
    if is_framework_catalog_intent(query) {
        let combined = format!("{} {}", candidate.title, snippet);
        if framework_name_hits(&combined) < 2 {
            return true;
        }
        if snippet.split_whitespace().count() < 18 {
            return true;
        }
    }
    false
}

fn framework_catalog_fallback_insights(
    actionable_ranked: &[(Candidate, f64)],
    max_items: usize,
) -> Vec<String> {
    let mut insights = Vec::<String>::new();
    let mut seen_frameworks = HashSet::<String>::new();
    for (candidate, _) in actionable_ranked {
        let framework_names = framework_names_in_text(&format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ));
        if framework_names.is_empty() {
            continue;
        }
        let domain = candidate_domain_hint(candidate);
        let snippet = trim_words(&clean_text(&candidate.snippet, 220), 18);
        for framework in framework_names {
            let framework_key = framework.to_ascii_lowercase();
            if !seen_frameworks.insert(framework_key) {
                continue;
            }
            let insight = if !snippet.is_empty() && looks_like_framework_overview_text(&format!("{framework} {snippet}")) {
                if domain == "source" {
                    format!("{framework}: {snippet}")
                } else {
                    format!("{framework} ({domain}): {snippet}")
                }
            } else if domain == "source" {
                format!("{framework}: framework overview result")
            } else {
                format!("{framework}: overview result from {domain}")
            };
            insights.push(trim_words(&insight, 24));
            if insights.len() >= max_items.max(1) {
                return insights;
            }
        }
    }
    insights
}

fn framework_catalog_source_adjustment(candidate: &Candidate) -> f64 {
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    let combined = format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator);
    let combined_lowered = combined.to_ascii_lowercase();
    if looks_like_competitive_programming_dump(&combined) {
        return -0.45;
    }
    if domain.contains("reddit.com") || domain.contains("zhihu.com") || domain.contains("quora.com")
    {
        return -0.28;
    }
    if domain.contains("medium.com") || domain.contains("dev.to") {
        return -0.12;
    }
    if domain.contains("support.microsoft.com")
        || combined_lowered.contains("contact microsoft support")
        || combined_lowered.contains("/contactus")
        || (combined_lowered.contains("support")
            && !combined_lowered.contains("agent")
            && framework_name_hits(&combined_lowered) == 0)
    {
        return -0.4;
    }
    if domain.contains("langgraph.com.cn")
        || domain.contains("crewai.org.cn")
        || domain.ends_with(".org.cn")
        || domain.ends_with(".com.cn")
    {
        return -0.18;
    }
    if domain.contains("langchain.com")
        || domain.contains("openai.com")
        || domain.contains("openai.github.io")
        || domain.contains("crewai.com")
        || domain.contains("huggingface.co")
        || domain.contains("microsoft.github.io")
    {
        return 0.2;
    }
    if domain.contains("github.com") {
        if combined_lowered.contains("microsoft/autogen") || combined_lowered.contains("autogen") {
            return 0.18;
        }
        if framework_name_hits(&combined) >= 1 {
            return 0.12;
        }
    }
    0.0
}
