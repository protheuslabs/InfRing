
const POLICY_REL: &str = "core/layer0/ops/config/batch_query_policy.json";
const LEGACY_POLICY_REL: &str = "client/runtime/config/batch_query_policy.json";
const RECEIPTS_REL: &str = "client/runtime/local/state/batch_query/receipts.jsonl";

#[derive(Clone, Copy, Debug)]
struct ApertureBudget {
    max_candidates: usize,
    max_evidence: usize,
    max_summary_tokens: usize,
    #[cfg(test)]
    max_query_rewrites: usize,
}

#[derive(Clone, Debug, Serialize)]
struct EvidenceRef {
    source_kind: String,
    title: String,
    locator: String,
    excerpt_hash: String,
    score: f64,
    timestamp: Option<String>,
    permissions: Option<String>,
    confidence: String,
    quality_flags: Vec<String>,
    coverage_facets: Vec<String>,
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

fn read_json_or(path: &Path, fallback: Value) -> Value {
    crate::contract_lane_utils::read_json(path).unwrap_or(fallback)
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("batch_query_create_parent_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("batch_query_encode_policy_failed:{err}"))?;
    fs::write(&tmp, encoded).map_err(|err| format!("batch_query_write_policy_tmp_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("batch_query_rename_policy_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    crate::contract_lane_utils::append_jsonl(path, row)
        .map_err(|err| format!("batch_query_append_receipt_failed:{err}"))
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

fn policy_path(root: &Path) -> PathBuf {
    root.join(POLICY_REL)
}

fn legacy_policy_path(root: &Path) -> PathBuf {
    root.join(LEGACY_POLICY_REL)
}

fn receipts_path(root: &Path) -> PathBuf {
    root.join(RECEIPTS_REL)
}

fn load_policy(root: &Path) -> Value {
    let path = policy_path(root);
    if path.exists() {
        return read_json_or(&path, default_policy());
    }
    let legacy_path = legacy_policy_path(root);
    if legacy_path.exists() {
        return read_json_or(&legacy_path, default_policy());
    }
    let _ = write_json_atomic(&path, &default_policy());
    read_json_or(&path, default_policy())
}

fn aperture_budget(aperture: &str) -> Option<ApertureBudget> {
    match aperture {
        "small" => Some(ApertureBudget {
            max_candidates: 8,
            max_evidence: 2,
            max_summary_tokens: 180,
            #[cfg(test)]
            max_query_rewrites: 0,
        }),
        "medium" => Some(ApertureBudget {
            max_candidates: 20,
            max_evidence: 6,
            max_summary_tokens: 350,
            #[cfg(test)]
            max_query_rewrites: 1,
        }),
        "large" => None,
        _ => None,
    }
}

fn normalize_source(raw: &str) -> String {
    let normalized = clean_text(raw, 40).to_ascii_lowercase();
    if normalized.is_empty() {
        "web".to_string()
    } else {
        normalized
    }
}

fn normalize_aperture(raw: &str) -> String {
    let normalized = clean_text(raw, 20).to_ascii_lowercase();
    if normalized.is_empty() {
        "medium".to_string()
    } else {
        normalized
    }
}

fn enabled_sources(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/enabled_sources")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["web".to_string()])
}

fn allow_large(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/allow_large")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn max_parallel_subqueries(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/max_parallel_subqueries")
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, 16) as usize
}

fn query_timeout(policy: &Value) -> Duration {
    let timeout_ms = policy
        .pointer("/batch_query/query_timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(5000)
        .clamp(500, 60_000);
    Duration::from_millis(timeout_ms)
}

fn page_extraction_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_max_links_per_stage(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/max_links_per_stage")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(0, 10) as usize
}

fn page_extraction_max_total_fetches(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/max_total_fetches")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(0, 40) as usize
}

fn page_extraction_reserved_trusted_primary_fetches(policy: &Value) -> usize {
    let max_total = page_extraction_max_total_fetches(policy);
    let default_reserve = if max_total >= 12 {
        4
    } else if max_total >= 8 {
        2
    } else {
        0
    };
    policy
        .pointer("/batch_query/page_extraction/reserved_trusted_primary_fetches")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(0, max_total as u64) as usize)
        .unwrap_or(default_reserve.min(max_total))
}

fn page_extraction_extract_mode(policy: &Value) -> String {
    let raw = policy
        .pointer("/batch_query/page_extraction/extract_mode")
        .and_then(Value::as_str)
        .unwrap_or("text")
        .trim()
        .to_ascii_lowercase();
    if raw == "markdown" {
        "markdown".to_string()
    } else {
        "text".to_string()
    }
}

fn page_extraction_browser_materialization_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/browser_materialization/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_browser_materialization_timeout_ms(policy: &Value) -> u64 {
    policy
        .pointer("/batch_query/page_extraction/browser_materialization/timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(8_000)
        .clamp(1_000, 45_000)
}

fn page_extraction_browser_materialization_max_response_bytes(policy: &Value) -> u64 {
    policy
        .pointer("/batch_query/page_extraction/browser_materialization/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(200_000)
        .clamp(2_048, 1_000_000)
}

fn page_extraction_min_link_score(policy: &Value) -> f64 {
    policy
        .pointer("/batch_query/page_extraction/min_link_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.08)
        .clamp(-1.0, 1.0)
}

fn page_extraction_min_usable_items_before_skip(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/min_usable_items_before_skip")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(0, 12) as usize
}

fn page_extraction_min_snippet_words_before_skip(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/min_snippet_words_before_skip")
        .and_then(Value::as_u64)
        .unwrap_or(22)
        .clamp(0, 120) as usize
}

fn page_extraction_min_query_overlap_terms_before_skip(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/min_query_overlap_terms_before_skip")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(0, 12) as usize
}

fn page_extraction_candidate_locator_followup_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/candidate_locator_followup/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_candidate_locator_max_per_stage(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/candidate_locator_followup/max_per_stage")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(0, 10) as usize
}

fn page_extraction_hub_discovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/hub_discovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_hub_discovery_max_links(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/hub_discovery/max_links_per_hub")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(0, 6) as usize
}

fn page_extraction_url_hygiene_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/url_hygiene/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_drop_fragment_for_dedupe(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/url_hygiene/drop_fragment_for_dedupe")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_canonical_dedupe_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/url_hygiene/canonical_dedupe_prefer_https_and_non_www")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_require_http_protocol(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/page_extraction/url_hygiene/require_http_protocol")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn page_extraction_excluded_file_extensions(policy: &Value) -> Vec<String> {
    let configured = policy
        .pointer("/batch_query/page_extraction/url_hygiene/excluded_file_extensions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|value| clean_text(value, 32).to_ascii_lowercase())
                .filter(|value| value.starts_with('.') && value.len() > 1)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !configured.is_empty() {
        return configured;
    }
    [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico", ".css", ".js", ".woff",
        ".woff2", ".ttf", ".mp3", ".mp4", ".avi", ".mov", ".zip", ".gz", ".tar", ".dmg",
        ".exe",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn structured_results_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/structured_results/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn structured_results_max_rows_per_stage(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/structured_results/max_rows_per_stage")
        .and_then(Value::as_u64)
        .unwrap_or(12)
        .clamp(0, 40) as usize
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
