// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

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
