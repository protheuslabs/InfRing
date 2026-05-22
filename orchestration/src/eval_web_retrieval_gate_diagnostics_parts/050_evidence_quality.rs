fn web_evidence_quality_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut scan = EvidenceQualityScan::default();
    scan_evidence_quality(payload, "payload", &mut scan, 0);
    scan_evidence_quality(retrieval_quality, "retrieval_quality", &mut scan, 0);
    scan.source_domains.sort_unstable();
    scan.source_domains.dedup();
    scan.low_quality_flags.sort_unstable();
    scan.low_quality_flags.dedup();
    scan.refs.sort_unstable();
    scan.refs.dedup();

    let retrieval_usable = bool_at(retrieval_quality, &["usable_evidence"], false);
    let evidence_count = u64_at(retrieval_quality, &["evidence_count"], 0);
    let claim_hint_count = u64_at(retrieval_quality, &["claim_hint_count"], 0);
    let direct_claim_count = u64_at(
        retrieval_quality,
        &["classification_inputs", "direct_evidence_claim_count"],
        0,
    );
    let quality_pack = retrieval_quality
        .get("evidence_pack_quality")
        .unwrap_or(&Value::Null);
    let pack_usable_count = u64_at(quality_pack, &["usable_count"], 0);
    let pack_content_rich_count = u64_at(quality_pack, &["content_rich_item_count"], 0);
    let pack_claim_hint_count = u64_at(quality_pack, &["claim_hint_count"], 0);
    let pack_low_confidence_count = u64_at(quality_pack, &["low_confidence_count"], 0);
    let pack_candidate_only_count = u64_at(quality_pack, &["candidate_only_count"], 0);

    let evidence_item_count = scan
        .evidence_item_count
        .max(evidence_count)
        .max(pack_usable_count);
    let clean_evidence_count = scan
        .clean_evidence_item_count
        .max(pack_usable_count)
        .max(pack_content_rich_count.min(evidence_item_count));
    let low_quality_evidence_count = scan
        .low_quality_evidence_item_count
        .max(pack_low_confidence_count.saturating_add(pack_candidate_only_count));
    let claim_count = scan
        .claim_count
        .max(claim_hint_count)
        .max(direct_claim_count);
    let concrete_claim_count = scan.concrete_claim_count.max(direct_claim_count);
    let citation_ready_claim_count = scan.citation_ready_claim_count;
    let clean_evidence_rate = ratio(clean_evidence_count, evidence_item_count);
    let low_quality_evidence_rate = ratio(low_quality_evidence_count, evidence_item_count);
    let concrete_claim_rate = ratio(concrete_claim_count, claim_count);
    let low_quality_claim_rate = ratio(scan.low_quality_claim_count, claim_count);
    let citation_ready_claim_rate = ratio(citation_ready_claim_count, claim_count);

    let source_quality_ready = evidence_item_count > 0
        && clean_evidence_count > 0
        && (scan.citation_ready_evidence_item_count > 0
            || !scan.source_domains.is_empty()
            || retrieval_usable)
        && low_quality_evidence_count < evidence_item_count
        && clean_evidence_rate >= 0.5;
    let claim_quality_ready = claim_count > 0
        && concrete_claim_count > 0
        && scan.low_quality_claim_count < claim_count
        && concrete_claim_rate >= 0.35
        && low_quality_claim_rate < 0.75;
    let citation_renderability_ready = citation_ready_claim_count > 0
        || (scan.citation_ready_evidence_item_count > 0
            && (claim_hint_count > 0 || direct_claim_count > 0 || pack_claim_hint_count > 0));
    let answerability_ready =
        source_quality_ready && claim_quality_ready && citation_renderability_ready;

    json!({
        "schema_version": 1,
        "source_quality_ready": source_quality_ready,
        "claim_quality_ready": claim_quality_ready,
        "citation_renderability_ready": citation_renderability_ready,
        "answerability_ready": answerability_ready,
        "evidence_item_count": evidence_item_count,
        "clean_evidence_item_count": clean_evidence_count,
        "low_quality_evidence_item_count": low_quality_evidence_count,
        "clean_evidence_rate": clean_evidence_rate,
        "low_quality_evidence_rate": low_quality_evidence_rate,
        "claim_count": claim_count,
        "concrete_claim_count": concrete_claim_count,
        "low_quality_claim_count": scan.low_quality_claim_count,
        "citation_ready_claim_count": citation_ready_claim_count,
        "citation_ready_evidence_item_count": scan.citation_ready_evidence_item_count,
        "concrete_claim_rate": concrete_claim_rate,
        "low_quality_claim_rate": low_quality_claim_rate,
        "citation_ready_claim_rate": citation_ready_claim_rate,
        "source_domain_count": scan.source_domains.len() as u64,
        "source_domains": scan.source_domains,
        "low_quality_flags": scan.low_quality_flags,
        "artifact_refs": scan.refs,
        "note": "Generic evidence-quality readout: checks whether packaged evidence has clean source-backed material, concrete claim text, and citation renderability without assuming the query domain."
    })
}

fn evidence_quality_refs(evidence_quality: &Value) -> Vec<String> {
    evidence_quality
        .get("artifact_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "evidence_pack".to_string(),
                "evidence_refs".to_string(),
                "evidence_claims".to_string(),
                "evidence_pack_quality".to_string(),
                "retrieval_quality.classification_inputs".to_string(),
            ]
        })
}

#[derive(Default)]
struct EvidenceQualityScan {
    evidence_item_count: u64,
    clean_evidence_item_count: u64,
    low_quality_evidence_item_count: u64,
    citation_ready_evidence_item_count: u64,
    claim_count: u64,
    concrete_claim_count: u64,
    low_quality_claim_count: u64,
    citation_ready_claim_count: u64,
    source_domains: Vec<String>,
    low_quality_flags: Vec<String>,
    refs: Vec<String>,
}

fn scan_evidence_quality(value: &Value, path: &str, scan: &mut EvidenceQualityScan, depth: usize) {
    if depth > 10 || evidence_quality_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_evidence_quality(row, &format!("{path}.{index}"), scan, depth + 1);
            }
        }
        Value::Object(map) => {
            let evidence_context = evidence_quality_context_path(path);
            if evidence_context && object_looks_like_evidence(map) {
                analyze_evidence_quality_object(map, path, scan);
            }
            for (key, child) in map {
                scan_evidence_quality(child, &format!("{path}.{key}"), scan, depth + 1);
            }
        }
    }
}

fn evidence_quality_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "pending tool request",
        "query metadata",
        "gate diagnostics",
        "operator metrics",
        "plain english",
        "blocker taxonomy",
        "recommended next capability",
        "capability contract",
        "tooling cd",
        "workflow cd",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn evidence_quality_context_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "evidence pack",
        "evidence refs",
        "evidence ref",
        "evidence claims",
        "evidence claim",
        "findings",
        "source candidates",
        "synthesis candidates",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn analyze_evidence_quality_object(
    map: &serde_json::Map<String, Value>,
    path: &str,
    scan: &mut EvidenceQualityScan,
) {
    scan.evidence_item_count = scan.evidence_item_count.saturating_add(1);
    scan.refs.push(path.to_string());

    if let Some(domain) = source_domain_value(map) {
        push_unique_case_insensitive(&mut scan.source_domains, &domain);
    }

    let citation_ready = evidence_object_citation_ready(map);
    if citation_ready {
        scan.citation_ready_evidence_item_count =
            scan.citation_ready_evidence_item_count.saturating_add(1);
    }

    let low_quality = evidence_object_low_quality(map, path, scan);
    if low_quality {
        scan.low_quality_evidence_item_count =
            scan.low_quality_evidence_item_count.saturating_add(1);
    } else if evidence_object_has_clean_content(map) {
        scan.clean_evidence_item_count = scan.clean_evidence_item_count.saturating_add(1);
    }

    for claim in evidence_object_claim_strings(map) {
        scan.claim_count = scan.claim_count.saturating_add(1);
        let low_claim = claim_text_low_quality(&claim);
        if low_claim {
            scan.low_quality_claim_count = scan.low_quality_claim_count.saturating_add(1);
        }
        if !low_claim && claim_text_concrete(&claim) {
            scan.concrete_claim_count = scan.concrete_claim_count.saturating_add(1);
            if citation_ready {
                scan.citation_ready_claim_count = scan.citation_ready_claim_count.saturating_add(1);
            }
        }
    }
}

fn evidence_object_citation_ready(map: &serde_json::Map<String, Value>) -> bool {
    let locator_present = [
        "locator",
        "url",
        "source_url",
        "link",
        "source_locator",
        "citation",
    ]
    .iter()
    .any(|key| map.get(*key).map(value_has_content).unwrap_or(false));
    let title_or_ref_present = ["title", "source_title", "source_ref"]
        .iter()
        .any(|key| map.get(*key).map(value_has_content).unwrap_or(false));
    locator_present || (title_or_ref_present && source_domain_value(map).is_some())
}

fn evidence_object_has_clean_content(map: &serde_json::Map<String, Value>) -> bool {
    evidence_object_content_strings(map)
        .iter()
        .any(|raw| !content_text_low_quality(raw) && word_count(raw) >= 6)
}

fn evidence_object_low_quality(
    map: &serde_json::Map<String, Value>,
    path: &str,
    scan: &mut EvidenceQualityScan,
) -> bool {
    let mut low_quality = false;
    for flag in evidence_object_flag_strings(map) {
        if low_quality_flag_text(&flag) {
            low_quality = true;
            push_unique_case_insensitive(&mut scan.low_quality_flags, &flag);
        }
    }
    if let Some(confidence) = ["confidence", "score", "quality_score"]
        .iter()
        .find_map(|key| map.get(*key).and_then(Value::as_f64))
        .filter(|score| *score < 0.35)
    {
        low_quality = true;
        push_unique_case_insensitive(
            &mut scan.low_quality_flags,
            &format!("low_numeric_confidence_{confidence:.2}"),
        );
    }
    if map
        .get("counts_as_usable_evidence")
        .and_then(Value::as_bool)
        == Some(false)
    {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "not_usable_evidence");
    }
    if map.get("usable").and_then(Value::as_bool) == Some(false)
        && evidence_quality_context_path(path)
    {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "not_usable");
    }
    for raw in evidence_object_content_strings(map) {
        if content_text_low_quality(&raw) {
            low_quality = true;
            push_unique_case_insensitive(
                &mut scan.low_quality_flags,
                "boilerplate_or_source_only_text",
            );
        }
    }
    low_quality
}

fn evidence_object_flag_strings(map: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "quality_flags",
        "flags",
        "limitations",
        "failure_reasons",
        "rejection_reasons",
        "status",
        "materialization_quality",
        "result_quality",
        "source_quality",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    values
}

fn evidence_object_content_strings(map: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "description",
        "result",
        "body",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    values
}

fn evidence_object_claim_strings(map: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "claim",
        "claims",
        "claim_hint",
        "claim_hints",
        "evidence_claims",
        "findings",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    values
        .into_iter()
        .map(|raw| clean_text(&raw, 500))
        .filter(|raw| !raw.is_empty())
        .collect()
}

fn collect_value_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(raw) => {
            let cleaned = clean_text(raw, 500);
            if !cleaned.is_empty() {
                out.push(cleaned);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                collect_value_strings(row, out);
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                collect_value_strings(child, out);
            }
        }
        _ => {}
    }
}

fn low_quality_flag_text(raw: &str) -> bool {
    let normalized = normalize_for_compare(raw);
    [
        "low trust",
        "low_trust",
        "low confidence",
        "low_confidence",
        "low relevance",
        "low_relevance",
        "off topic",
        "off_topic",
        "thin",
        "title only",
        "title_only",
        "source only",
        "source_only",
        "candidate only",
        "candidate_only",
        "search row only",
        "search_row_only",
        "boilerplate",
        "script dump",
        "script_dump",
        "style dump",
        "style_dump",
        "html dump",
        "materialization failed",
        "materialization_failed",
        "content too thin",
        "content_too_thin",
        "link directory",
        "link_directory",
        "aggregator shell",
        "aggregator_shell",
        "social video shell",
        "social_video_shell",
        "affiliate",
        "privacy policy",
        "terms of service",
        "cookie banner",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn content_text_low_quality(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = normalize_for_compare(&cleaned);
    if cleaned.len() < 40 || word_count(&cleaned) < 5 {
        return true;
    }
    if urlish_or_source_label(&normalized) {
        return true;
    }
    [
        "click here",
        "subscribe",
        "sign up",
        "privacy policy",
        "terms of service",
        "cookie",
        "javascript",
        "function ",
        "loading",
        "enable javascript",
        "checking your browser",
        "verify you are human",
        "here s what i found",
        "from web retrieval",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn claim_text_low_quality(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = normalize_for_compare(&cleaned);
    if cleaned.len() < 28 || word_count(&cleaned) < 4 {
        return true;
    }
    if urlish_or_source_label(&normalized) {
        return true;
    }
    if normalized.starts_with("source ")
        || normalized.starts_with("sources ")
        || normalized.starts_with("web search ")
        || normalized.starts_with("from web retrieval")
    {
        return true;
    }
    [
        "click here",
        "subscribe",
        "sign up",
        "privacy policy",
        "terms of service",
        "cookie",
        "javascript",
        "loading",
        "current news latest news photos videos",
        "here s what i found",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn claim_text_concrete(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    if claim_text_low_quality(&cleaned) {
        return false;
    }
    let normalized = normalize_for_compare(&cleaned);
    let words = word_count(&cleaned);
    let has_specific_marker = cleaned.chars().any(|ch| ch.is_ascii_digit())
        || cleaned.chars().any(|ch| ch.is_ascii_uppercase())
        || normalized.contains(" compared ")
        || normalized.contains(" announced ")
        || normalized.contains(" released ")
        || normalized.contains(" reported ")
        || normalized.contains(" found ")
        || normalized.contains(" shows ")
        || normalized.contains(" offers ")
        || normalized.contains(" supports ")
        || normalized.contains(" requires ")
        || normalized.contains(" costs ")
        || normalized.contains(" changed ")
        || normalized.contains(" increased ")
        || normalized.contains(" decreased ");
    words >= 7 || (words >= 5 && has_specific_marker)
}

fn urlish_or_source_label(normalized: &str) -> bool {
    normalized.starts_with("http ")
        || normalized.starts_with("https ")
        || normalized.starts_with("www ")
        || normalized.starts_with("source ")
        || normalized.starts_with("sources ")
        || normalized.ends_with(" source")
        || normalized.contains(" com ")
        || normalized.contains(" org ")
        || normalized.contains(" net ")
        || normalized.contains(" via google news")
}

fn word_count(raw: &str) -> usize {
    raw.split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_ascii_alphanumeric()))
        .count()
}
