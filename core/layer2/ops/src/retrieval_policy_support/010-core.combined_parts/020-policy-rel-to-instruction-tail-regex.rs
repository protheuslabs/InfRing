// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

#[derive(Clone, Copy, Debug)]
struct ApertureBudget {
    max_candidates: usize,
    max_evidence: usize,
    #[cfg(test)]
    max_query_rewrites: usize,
}

#[derive(Clone, Debug)]
struct Candidate {
    source_kind: String,
    title: String,
    locator: String,
    snippet: String,
    excerpt_hash: String,
    timestamp: Option<String>,
    permissions: Option<String>,
    status_code: i64,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(Some(raw), max_len.max(1))
}

fn trim_words(raw: &str, max_words: usize) -> String {
    if max_words == 0 {
        return String::new();
    }
    raw.split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_policy() -> Value {
    json!({
        "version": "v1",
        "batch_query": {
            "enabled_sources": ["web"],
            "allow_large": false,
            "max_parallel_subqueries": 4,
            "query_timeout_ms": 5000,
            "cache": {
                "mode": "enabled",
                "ttl_success_seconds": 1800,
                "ttl_no_results_seconds": 120,
                "max_entries": 240
            },
            "page_extraction": {
                "enabled": true,
                "extract_mode": "text",
                "max_links_per_stage": 3,
                "max_total_fetches": 16,
                "min_link_score": 0.0,
                "min_usable_items_before_skip": 2,
                "min_snippet_words_before_skip": 22,
                "min_query_overlap_terms_before_skip": 2,
                "trigger": "low_thin_or_coverage_weak_candidates",
                "browser_materialization": {
                    "enabled": true,
                    "timeout_ms": 8000,
                    "max_response_bytes": 200000,
                    "only_when_static_unusable": true
                },
                "candidate_locator_followup": {
                    "enabled": true,
                    "max_per_stage": 3,
                    "selection": "merge_structured_result_locators_with_payload_links_when_candidates_are_thin_or_coverage_is_weak"
                },
                "hub_discovery": {
                    "enabled": true,
                    "max_links_per_hub": 2,
                    "mode": "fetch_relevant_directory_pages_only_to_discover_article_like_links"
                },
                "url_hygiene": {
                    "enabled": true,
                    "drop_fragment_for_dedupe": true,
                    "canonical_dedupe_prefer_https_and_non_www": true,
                    "require_http_protocol": true,
                    "excluded_file_extensions": [
                        ".png",
                        ".jpg",
                        ".jpeg",
                        ".gif",
                        ".webp",
                        ".svg",
                        ".ico",
                        ".css",
                        ".js",
                        ".woff",
                        ".woff2",
                        ".ttf",
                        ".mp3",
                        ".mp4",
                        ".avi",
                        ".mov",
                        ".zip",
                        ".gz",
                        ".tar",
                        ".dmg",
                        ".exe"
                    ]
                }
            },
            "structured_results": {
                "enabled": true,
                "max_rows_per_stage": 12
            },
            "evidence_pack": {
                "enabled": true,
                "max_items": 6,
                "max_snippet_words": 72,
                "source_class_rules": [
                    {
                        "class": "public_institution",
                        "host_suffixes": [".gov"]
                    },
                    {
                        "class": "scholarly_or_research",
                        "host_suffixes": [".edu"],
                        "host_contains": ["arxiv.", "doi."],
                        "path_contains": ["/paper", "/publication", "/journal"],
                        "title_contains": ["paper", "study", "journal", "arxiv"],
                        "snippet_contains": ["peer-reviewed", "preprint", "published in"]
                    },
                    {
                        "class": "documentation_or_reference",
                        "path_contains": ["/docs", "/documentation", "/reference", "/manual", "/guide"],
                        "title_contains": ["documentation", "reference", "manual", "guide", "tutorial", "how to", "build ", "building "]
                    },
                    {
                        "class": "news_or_current",
                        "path_contains": ["/news", "/press", "/release", "/releases", "/blog", "/announcements"],
                        "title_contains": ["announces", "announced", "introducing", "launches", "launched", "raises", "released", "release"]
                    },
                    {
                        "class": "independent_analysis",
                        "title_contains": ["analysis", "review", "comparison", " vs ", "best ", "ranked", "benchmark", "benchmarks", "risk", "risks", "tradeoff", "tradeoffs"]
                    },
                    {
                        "class": "repository_or_dataset",
                        "host_contains": ["github.", "gitlab."],
                        "path_contains": ["/repo", "/repository", "/dataset", "/datasets"]
                    },
                    {
                        "class": "community_or_forum",
                        "host_contains": ["forum.", "reddit.", "quora."],
                        "path_contains": ["/forum", "/community", "/discussion"]
                    }
                ]
            },
            "coverage_aware_evidence": {
                "enabled": true,
                "max_facets": 8,
                "min_facet_terms": 2,
                "record_coverage": true
            },
            "coverage_gap_recovery": {
                "enabled": true,
                "max_queries": 4,
                "min_usable_evidence": 3,
                "min_covered_facets": 3,
                "min_covered_facet_ratio": 1.0,
                "templates": [
                    "{entities} {facet} official documentation",
                    "{entities} {facet} primary source evidence",
                    "{entities} {facet} independent analysis evidence",
                    "{entities} {facet} examples reports data",
                    "{query} {facet} source-backed evidence"
                ]
            },
            "claim_gap_recovery": {
                "enabled": true,
                "max_queries": 3,
                "min_materialized_evidence": 2,
                "min_claim_hints": 3,
                "templates": [
                    "{entities} {facet} detailed findings",
                    "{entities} {facet} source-backed evidence",
                    "{query} detailed findings",
                    "{query} source-backed evidence"
                ]
            },
            "quality_gate": {
                "enabled": true,
                "provider_recovery": {
                    "enabled": true,
                    "max_providers": 6,
                    "providers": [
                        "tavily",
                        "exa",
                        "brave",
                        "serperdev",
                        "google_news_rss",
                        "duckduckgo_lite",
                        "bing_rss"
                    ],
                    "current_intent_providers": [
                        "tavily",
                        "exa",
                        "brave",
                        "serperdev",
                        "google_news_rss",
                        "bing_rss",
                        "duckduckgo_lite"
                    ],
                    "official_source_providers": [
                        "tavily",
                        "exa",
                        "brave",
                        "serperdev",
                        "browser_serp",
                        "duckduckgo_lite",
                        "duckduckgo"
                    ]
                }
            },
            "query_recovery": {
                "broad_current_research": {
                    "enabled": true,
                    "max_queries": 4,
                    "intent_markers": [
                        "breakthrough",
                        "breakthroughs",
                        "changes",
                        "current state",
                        "developments",
                        "landscape",
                        "news",
                        "overview",
                        "some ",
                        "state of",
                        "trend",
                        "trends",
                        "what are",
                        "what were"
                    ],
                    "templates": [
                        "{query}",
                        "{query} latest",
                        "{query} recent report",
                        "{query} source-backed evidence"
                    ]
                },
                "general_research": {
                    "enabled": true,
                    "max_queries": 5,
                    "intent_markers": [
                        "assess",
                        "avoid",
                        "benchmark",
                        "best ",
                        "choose",
                        "compare",
                        "comparison",
                        "current state",
                        "ecosystem",
                        "evaluate",
                        "evaluation",
                        "fit ",
                        "landscape",
                        "limitation",
                        "limitations",
                        "mature",
                        "maturity",
                        "production",
                        "recommend",
                        "recommendation",
                        "reliability",
                        "risk",
                        "risks",
                        "security",
                        "strength",
                        "strengths",
                        "versus",
                        " vs ",
                        "weakness",
                        "weaknesses",
                        "which "
                    ],
                    "templates": [
                        "{query}",
                        "{query} source-backed evidence",
                        "{query} independent analysis",
                        "{query} reports data",
                        "{query} limitations risks"
                    ]
                }
            }
        }
    })
}

#[cfg(test)]
fn exact_match_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r#""[^"]+""#).expect("quoted"),
            Regex::new(r"https?://\S+").expect("url"),
            Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("email"),
            Regex::new(r"\b[a-fA-F0-9]{8,}\b").expect("hex-id"),
            Regex::new(r"[/~][A-Za-z0-9._/\-]+").expect("path"),
            Regex::new(r"[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*").expect("symbol"),
        ]
    })
}

#[cfg(test)]
fn is_exact_match_pattern(query: &str) -> bool {
    exact_match_regexes().iter().any(|re| re.is_match(query))
}

fn instruction_frame_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:verify|check|test|research(?:ing)?|find(?:\s+out)?|report|return|provide|show|summarize|compare|assess|evaluate|investigate|answer|brief(?:ing)?|cite|prioritize|group|give)\b",
        )
        .expect("instruction-frame")
    })
}

fn instruction_tail_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:verify|check|test|research(?:ing)?|find(?:\s+out)?|report|return|provide|show|summarize|compare|assess|evaluate|investigate|answer|brief(?:ing)?|cite|prioritize|group|give)\b.{0,120}?\b(?:by|about|on)\b\s+(.+)$",
        )
        .expect("instruction-tail")
    })
}
