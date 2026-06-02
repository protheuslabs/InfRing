fn web_evidence_quality_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut scan = EvidenceQualityScan::default();
    scan.request_terms = evidence_quality_request_terms(payload);
    scan_evidence_quality(payload, "payload", &mut scan, 0);
    scan_evidence_quality(retrieval_quality, "retrieval_quality", &mut scan, 0);
    scan.source_domains.sort_unstable();
    scan.source_domains.dedup();
    scan.authority_grade_source_domains.sort_unstable();
    scan.authority_grade_source_domains.dedup();
    scan.low_quality_flags.sort_unstable();
    scan.low_quality_flags.dedup();
    scan.malformed_evidence_samples.sort_unstable();
    scan.malformed_evidence_samples.dedup();
    scan.malformed_evidence_samples.truncate(8);
    scan.malformed_citation_title_samples.sort_unstable();
    scan.malformed_citation_title_samples.dedup();
    scan.malformed_citation_title_samples.truncate(8);
    scan.evidence_packet_missing_fields.sort_unstable();
    scan.evidence_packet_missing_fields.dedup();
    scan.refs.sort_unstable();
    scan.refs.dedup();
    scan.sample_rows.sort_by(|left, right| {
        let left_ready = left
            .get("packet_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let right_ready = right
            .get("packet_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        left_ready.cmp(&right_ready)
    });
    scan.sample_rows.truncate(8);

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
        .filter(|row| row.is_object())
        .or_else(|| payload.get("evidence_pack_quality"))
        .or_else(|| payload.pointer("/tools/0/evidence_pack_quality"))
        .or_else(|| payload.pointer("/tools/0/tool_pipeline/raw_payload/evidence_pack_quality"))
        .or_else(|| {
            payload.pointer(
                "/response_finalization/tool_completion/tool_attempts/0/evidence_pack_quality",
            )
        })
        .unwrap_or(&Value::Null);
    let quality_pack_present = quality_pack.is_object();
    let pack_status = str_at(quality_pack, &["status"], "");
    let pack_usable_count = u64_at(quality_pack, &["usable_count"], 0);
    let pack_content_rich_count = u64_at(quality_pack, &["content_rich_item_count"], 0);
    let pack_claim_hint_count = u64_at(quality_pack, &["claim_hint_count"], 0);
    let pack_low_confidence_count = u64_at(quality_pack, &["low_confidence_count"], 0);
    let pack_candidate_only_count = u64_at(quality_pack, &["candidate_only_count"], 0);
    let pack_source_domain_count = u64_at(quality_pack, &["source_domain_count"], 0);
    let pack_missing_facet_count = u64_at(quality_pack, &["missing_facet_count"], 0);
    let pack_weak_facet_count = u64_at(quality_pack, &["weak_facet_count"], 0);
    let pack_covered_facet_count = u64_at(quality_pack, &["covered_facet_count"], 0);
    let pack_total_facet_count = u64_at(quality_pack, &["total_facet_count"], 0);
    let pack_covered_facet_ratio = quality_pack
        .get("covered_facet_ratio")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| {
            if pack_total_facet_count == 0 {
                1.0
            } else {
                pack_covered_facet_count as f64 / pack_total_facet_count as f64
            }
        });
    let pack_min_usable_items = u64_at(quality_pack, &["thresholds", "min_usable_items"], 1).max(1);
    let pack_min_source_domains =
        u64_at(quality_pack, &["thresholds", "min_source_domains"], 1).max(1);
    let pack_min_covered_facets = if pack_total_facet_count == 0 {
        0
    } else {
        pack_total_facet_count.min(2)
    };
    let pack_source_thresholds_met = !quality_pack_present
        || (pack_usable_count >= pack_min_usable_items
            && pack_source_domain_count >= pack_min_source_domains
            && pack_content_rich_count > 0
            && pack_low_confidence_count < pack_usable_count
            && pack_candidate_only_count < pack_usable_count);
    let pack_coverage_thresholds_met = quality_pack
        .get("coverage_thresholds_met")
        .and_then(Value::as_bool)
        .unwrap_or(
            !quality_pack_present || (pack_missing_facet_count == 0 && pack_weak_facet_count == 0),
        );
    let pack_status_allows_answerability = !quality_pack_present || pack_status == "usable";

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
    let handoff_claim_count = scan.handoff_claim_count;
    let handoff_concrete_claim_count = scan.handoff_concrete_claim_count;
    let handoff_low_quality_claim_count = scan.handoff_low_quality_claim_count;
    let handoff_citation_ready_claim_count = scan.handoff_citation_ready_claim_count;
    let clean_evidence_rate = ratio(clean_evidence_count, evidence_item_count);
    let low_quality_evidence_rate = ratio(low_quality_evidence_count, evidence_item_count);
    let malformed_evidence_item_rate =
        ratio(scan.malformed_evidence_item_count, evidence_item_count);
    let malformed_evidence_clean = scan.malformed_evidence_fragment_count == 0;
    let citation_titles_clean = scan.malformed_citation_title_count == 0;
    let concrete_claim_rate = ratio(concrete_claim_count, claim_count);
    let low_quality_claim_rate = ratio(scan.low_quality_claim_count, claim_count);
    let citation_ready_claim_rate = ratio(citation_ready_claim_count, claim_count);
    let handoff_concrete_claim_rate = ratio(handoff_concrete_claim_count, handoff_claim_count);
    let handoff_low_quality_claim_rate =
        ratio(handoff_low_quality_claim_count, handoff_claim_count);
    let handoff_citation_ready_claim_rate =
        ratio(handoff_citation_ready_claim_count, handoff_claim_count);
    let handoff_claim_quality_ready = handoff_claim_count > 0
        && handoff_concrete_claim_count > 0
        && handoff_low_quality_claim_count == 0
        && handoff_concrete_claim_rate >= 0.5;
    let source_authority_sensitive =
        source_authority_sensitive_request(payload, &scan.request_terms);
    let authority_grade_source_domain_count = scan.authority_grade_source_domains.len() as u64;
    let source_authority_ready =
        !source_authority_sensitive || authority_grade_source_domain_count >= 2;
    let source_quality_threshold_met = clean_evidence_rate >= 0.5;
    let low_quality_flags_block_source_quality =
        low_quality_flags_block_source_quality(&scan.low_quality_flags);
    let clean_diverse_source_quality = clean_evidence_count >= 3
        && scan.source_domains.len() >= 2
        && low_quality_evidence_rate <= 0.25
        && !low_quality_flags_block_source_quality;
    let evidence_packet_item_count = scan
        .evidence_packet_item_count
        .max(scan.evidence_packet_ready_count);
    let evidence_packet_ready_rate =
        ratio(scan.evidence_packet_ready_count, evidence_packet_item_count);
    let evidence_packet_contract_ready =
        scan.evidence_packet_ready_count > 0 && evidence_packet_ready_rate >= 0.5;
    let source_quality_pack_observed_packet_ready =
        evidence_packet_item_count == 0 || evidence_packet_ready_rate >= 0.5;
    let observed_row_source_thresholds_met = clean_evidence_count >= pack_min_usable_items
        && (scan.source_domains.len() as u64) >= pack_min_source_domains
        && low_quality_evidence_count < evidence_item_count;
    let source_quality_pack_thresholds_met =
        pack_source_thresholds_met || observed_row_source_thresholds_met;
    let source_quality_pack_ready = quality_pack_present
        && pack_status == "usable"
        && pack_source_thresholds_met
        && pack_coverage_thresholds_met
        && pack_usable_count >= pack_min_usable_items
        && pack_source_domain_count >= pack_min_source_domains
        && pack_content_rich_count > 0
        && pack_low_confidence_count < pack_usable_count
        && pack_candidate_only_count < pack_usable_count
        && source_quality_pack_observed_packet_ready;

    let observed_source_quality_ready = evidence_item_count > 0
        && clean_evidence_count > 0
        && (scan.citation_ready_evidence_item_count > 0
            || !scan.source_domains.is_empty()
            || retrieval_usable)
        && low_quality_evidence_count < evidence_item_count
        && (source_quality_threshold_met || clean_diverse_source_quality)
        && source_quality_pack_thresholds_met;
    let source_quality_ready = observed_source_quality_ready || source_quality_pack_ready;
    let claim_quality_ready = claim_count > 0
        && concrete_claim_count > 0
        && scan.low_quality_claim_count < claim_count
        && concrete_claim_rate >= 0.35
        && low_quality_claim_rate < 0.75;
    let citation_renderability_ready = citation_ready_claim_count > 0
        || (scan.citation_ready_evidence_item_count > 0
            && (claim_hint_count > 0 || direct_claim_count > 0 || pack_claim_hint_count > 0));
    let bounded_answerability_ready = source_quality_ready
        && claim_quality_ready
        && citation_renderability_ready
        && evidence_packet_contract_ready
        && source_authority_ready
        && pack_source_thresholds_met
        && pack_usable_count >= pack_min_usable_items
        && pack_source_domain_count >= pack_min_source_domains
        && pack_covered_facet_count >= pack_min_covered_facets
        && pack_covered_facet_ratio >= 0.5
        && pack_missing_facet_count <= 1
        && pack_weak_facet_count <= pack_covered_facet_count
        && concrete_claim_count >= 2
        && citation_ready_claim_count >= 2
        && pack_candidate_only_count == 0
        && pack_low_confidence_count < pack_usable_count;
    let answerability_ready = source_quality_ready
        && claim_quality_ready
        && citation_renderability_ready
        && source_authority_ready
        && ((pack_coverage_thresholds_met && pack_status_allows_answerability)
            || bounded_answerability_ready);
    let pack_thresholds = json!({
        "min_usable_items": pack_min_usable_items,
        "min_source_domains": pack_min_source_domains,
        "min_covered_facets_for_bounded_answerability": pack_min_covered_facets
    });
    let evidence_packet_contract = json!({
        "schema_version": 1,
        "ready": evidence_packet_contract_ready,
        "ready_item_count": scan.evidence_packet_ready_count,
        "item_count": evidence_packet_item_count,
        "ready_rate": evidence_packet_ready_rate,
        "missing_fields": scan.evidence_packet_missing_fields,
        "required_field_groups": [
            "source_identity",
            "source_identity_consistency",
            "source_type",
            "relevant_extract",
            "claim_hints",
            "why_relevant_to_query",
            "query_relevance_alignment"
        ],
        "note": "Generic EvidencePacket contract: each answerable packet should carry source identity, source type, an extract, concrete claim material, and a query-relevance explanation that aligns with durable request terms. Dates are optional when unavailable."
    });
    let row_sample_status = if scan.sample_rows.is_empty() && evidence_item_count > 0 {
        "aggregate_evidence_without_row_samples"
    } else if scan.sample_rows.is_empty() {
        "no_evidence_rows_observed"
    } else {
        "row_samples_observed"
    };

    let mut out = serde_json::Map::new();
    out.insert("schema_version".to_string(), json!(1));
    out.insert(
        "source_quality_ready".to_string(),
        json!(source_quality_ready),
    );
    out.insert(
        "source_authority_sensitive".to_string(),
        json!(source_authority_sensitive),
    );
    out.insert(
        "source_authority_ready".to_string(),
        json!(source_authority_ready),
    );
    out.insert(
        "claim_quality_ready".to_string(),
        json!(claim_quality_ready),
    );
    out.insert(
        "citation_renderability_ready".to_string(),
        json!(citation_renderability_ready),
    );
    out.insert(
        "evidence_packet_contract_ready".to_string(),
        json!(evidence_packet_contract_ready),
    );
    out.insert(
        "answerability_ready".to_string(),
        json!(answerability_ready),
    );
    out.insert(
        "bounded_answerability_ready".to_string(),
        json!(bounded_answerability_ready),
    );
    out.insert(
        "evidence_item_count".to_string(),
        json!(evidence_item_count),
    );
    out.insert(
        "clean_evidence_item_count".to_string(),
        json!(clean_evidence_count),
    );
    out.insert(
        "low_quality_evidence_item_count".to_string(),
        json!(low_quality_evidence_count),
    );
    out.insert(
        "clean_evidence_rate".to_string(),
        json!(clean_evidence_rate),
    );
    out.insert(
        "low_quality_evidence_rate".to_string(),
        json!(low_quality_evidence_rate),
    );
    out.insert(
        "malformed_evidence_clean".to_string(),
        json!(malformed_evidence_clean),
    );
    out.insert(
        "malformed_evidence_fragment_count".to_string(),
        json!(scan.malformed_evidence_fragment_count),
    );
    out.insert(
        "malformed_evidence_item_count".to_string(),
        json!(scan.malformed_evidence_item_count),
    );
    out.insert(
        "malformed_evidence_item_rate".to_string(),
        json!(malformed_evidence_item_rate),
    );
    out.insert(
        "malformed_evidence_samples".to_string(),
        json!(scan.malformed_evidence_samples),
    );
    out.insert(
        "citation_titles_clean".to_string(),
        json!(citation_titles_clean),
    );
    out.insert(
        "malformed_citation_title_count".to_string(),
        json!(scan.malformed_citation_title_count),
    );
    out.insert(
        "malformed_citation_title_samples".to_string(),
        json!(scan.malformed_citation_title_samples),
    );
    out.insert(
        "source_quality_threshold_met".to_string(),
        json!(source_quality_threshold_met),
    );
    out.insert(
        "clean_diverse_source_quality".to_string(),
        json!(clean_diverse_source_quality),
    );
    out.insert(
        "low_quality_flags_block_source_quality".to_string(),
        json!(low_quality_flags_block_source_quality),
    );
    out.insert(
        "evidence_pack_quality_status".to_string(),
        json!(pack_status),
    );
    out.insert(
        "evidence_pack_quality_present".to_string(),
        json!(quality_pack_present),
    );
    out.insert(
        "pack_source_thresholds_met".to_string(),
        json!(pack_source_thresholds_met),
    );
    out.insert(
        "observed_row_source_thresholds_met".to_string(),
        json!(observed_row_source_thresholds_met),
    );
    out.insert(
        "source_quality_pack_thresholds_met".to_string(),
        json!(source_quality_pack_thresholds_met),
    );
    out.insert(
        "source_quality_pack_ready".to_string(),
        json!(source_quality_pack_ready),
    );
    out.insert(
        "source_quality_pack_observed_packet_ready".to_string(),
        json!(source_quality_pack_observed_packet_ready),
    );
    out.insert(
        "observed_source_quality_ready".to_string(),
        json!(observed_source_quality_ready),
    );
    out.insert(
        "pack_coverage_thresholds_met".to_string(),
        json!(pack_coverage_thresholds_met),
    );
    out.insert(
        "pack_status_allows_answerability".to_string(),
        json!(pack_status_allows_answerability),
    );
    out.insert("pack_usable_count".to_string(), json!(pack_usable_count));
    out.insert(
        "pack_content_rich_item_count".to_string(),
        json!(pack_content_rich_count),
    );
    out.insert(
        "pack_source_domain_count".to_string(),
        json!(pack_source_domain_count),
    );
    out.insert(
        "pack_missing_facet_count".to_string(),
        json!(pack_missing_facet_count),
    );
    out.insert(
        "pack_weak_facet_count".to_string(),
        json!(pack_weak_facet_count),
    );
    out.insert(
        "pack_covered_facet_count".to_string(),
        json!(pack_covered_facet_count),
    );
    out.insert(
        "pack_total_facet_count".to_string(),
        json!(pack_total_facet_count),
    );
    out.insert(
        "pack_covered_facet_ratio".to_string(),
        json!(pack_covered_facet_ratio),
    );
    out.insert("pack_thresholds".to_string(), pack_thresholds);
    out.insert("claim_count".to_string(), json!(claim_count));
    out.insert(
        "concrete_claim_count".to_string(),
        json!(concrete_claim_count),
    );
    out.insert(
        "low_quality_claim_count".to_string(),
        json!(scan.low_quality_claim_count),
    );
    out.insert(
        "citation_ready_claim_count".to_string(),
        json!(citation_ready_claim_count),
    );
    out.insert(
        "handoff_claim_count".to_string(),
        json!(handoff_claim_count),
    );
    out.insert(
        "handoff_concrete_claim_count".to_string(),
        json!(handoff_concrete_claim_count),
    );
    out.insert(
        "handoff_low_quality_claim_count".to_string(),
        json!(handoff_low_quality_claim_count),
    );
    out.insert(
        "handoff_citation_ready_claim_count".to_string(),
        json!(handoff_citation_ready_claim_count),
    );
    out.insert(
        "citation_ready_evidence_item_count".to_string(),
        json!(scan.citation_ready_evidence_item_count),
    );
    out.insert(
        "authority_grade_evidence_item_count".to_string(),
        json!(scan.authority_grade_evidence_item_count),
    );
    out.insert(
        "concrete_claim_rate".to_string(),
        json!(concrete_claim_rate),
    );
    out.insert(
        "low_quality_claim_rate".to_string(),
        json!(low_quality_claim_rate),
    );
    out.insert(
        "citation_ready_claim_rate".to_string(),
        json!(citation_ready_claim_rate),
    );
    out.insert(
        "handoff_claim_quality_ready".to_string(),
        json!(handoff_claim_quality_ready),
    );
    out.insert(
        "handoff_concrete_claim_rate".to_string(),
        json!(handoff_concrete_claim_rate),
    );
    out.insert(
        "handoff_low_quality_claim_rate".to_string(),
        json!(handoff_low_quality_claim_rate),
    );
    out.insert(
        "handoff_citation_ready_claim_rate".to_string(),
        json!(handoff_citation_ready_claim_rate),
    );
    out.insert(
        "source_domain_count".to_string(),
        json!(scan.source_domains.len() as u64),
    );
    out.insert("source_domains".to_string(), json!(scan.source_domains));
    out.insert(
        "authority_grade_source_domain_count".to_string(),
        json!(authority_grade_source_domain_count),
    );
    out.insert(
        "authority_grade_source_domains".to_string(),
        json!(scan.authority_grade_source_domains),
    );
    out.insert(
        "low_quality_flags".to_string(),
        json!(scan.low_quality_flags),
    );
    out.insert("request_terms".to_string(), json!(scan.request_terms));
    out.insert(
        "evidence_packet_contract".to_string(),
        evidence_packet_contract,
    );
    out.insert("row_sample_status".to_string(), json!(row_sample_status));
    out.insert(
        "row_sample_count".to_string(),
        json!(scan.sample_rows.len() as u64),
    );
    out.insert("sample_rows".to_string(), Value::Array(scan.sample_rows));
    out.insert("artifact_refs".to_string(), json!(scan.refs));
    out.insert(
        "note".to_string(),
        json!("Generic evidence-quality readout: checks whether packaged evidence has clean source-backed material, concrete claim text, and citation renderability without assuming the query domain."),
    );
    Value::Object(out)
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

fn low_quality_flags_block_source_quality(flags: &[String]) -> bool {
    flags.iter().any(|flag| {
        matches!(
            normalize_for_compare(flag).as_str(),
            "not usable evidence"
                | "not usable"
                | "thin query overlap"
                | "low trust source"
                | "low confidence raw"
                | "candidate only"
                | "candidate only row"
                | "provider degraded"
                | "materialization failed"
        )
    })
}

#[derive(Default)]
struct EvidenceQualityScan {
    evidence_item_count: u64,
    clean_evidence_item_count: u64,
    low_quality_evidence_item_count: u64,
    malformed_evidence_fragment_count: u64,
    malformed_evidence_item_count: u64,
    malformed_citation_title_count: u64,
    citation_ready_evidence_item_count: u64,
    authority_grade_evidence_item_count: u64,
    claim_count: u64,
    concrete_claim_count: u64,
    low_quality_claim_count: u64,
    citation_ready_claim_count: u64,
    handoff_claim_count: u64,
    handoff_concrete_claim_count: u64,
    handoff_low_quality_claim_count: u64,
    handoff_citation_ready_claim_count: u64,
    evidence_packet_item_count: u64,
    evidence_packet_ready_count: u64,
    evidence_packet_missing_fields: Vec<String>,
    request_terms: Vec<String>,
    source_domains: Vec<String>,
    authority_grade_source_domains: Vec<String>,
    low_quality_flags: Vec<String>,
    malformed_evidence_samples: Vec<String>,
    malformed_citation_title_samples: Vec<String>,
    refs: Vec<String>,
    sample_rows: Vec<Value>,
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
    if let Some(title) = malformed_citation_title_for_object(map) {
        scan.malformed_citation_title_count = scan.malformed_citation_title_count.saturating_add(1);
        push_malformed_citation_title_sample(scan, &title);
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
    if !low_quality && evidence_object_authority_grade(map) {
        scan.authority_grade_evidence_item_count =
            scan.authority_grade_evidence_item_count.saturating_add(1);
        if let Some(domain) = source_domain_value(map) {
            push_unique_case_insensitive(&mut scan.authority_grade_source_domains, &domain);
        }
    }

    if evidence_packet_contract_path(path) {
        scan.evidence_packet_item_count = scan.evidence_packet_item_count.saturating_add(1);
        let packet_missing_fields = evidence_packet_missing_fields(map, &scan.request_terms);
        if packet_missing_fields.is_empty() && !low_quality {
            scan.evidence_packet_ready_count = scan.evidence_packet_ready_count.saturating_add(1);
        } else {
            for field in packet_missing_fields {
                push_unique_case_insensitive(&mut scan.evidence_packet_missing_fields, field);
            }
        }
    }

    push_evidence_quality_sample(scan, map, path, low_quality);

    let handoff_claim_path = evidence_claim_handoff_path(path);
    for claim in evidence_object_claim_strings(map) {
        scan.claim_count = scan.claim_count.saturating_add(1);
        if handoff_claim_path {
            scan.handoff_claim_count = scan.handoff_claim_count.saturating_add(1);
        }
        let low_claim = claim_text_low_quality(&claim) || claim_text_malformed_fragment(&claim);
        if low_claim {
            scan.low_quality_claim_count = scan.low_quality_claim_count.saturating_add(1);
            if handoff_claim_path {
                scan.handoff_low_quality_claim_count =
                    scan.handoff_low_quality_claim_count.saturating_add(1);
            }
        }
        if !low_claim && claim_text_concrete(&claim) {
            scan.concrete_claim_count = scan.concrete_claim_count.saturating_add(1);
            if handoff_claim_path {
                scan.handoff_concrete_claim_count =
                    scan.handoff_concrete_claim_count.saturating_add(1);
            }
            if citation_ready {
                scan.citation_ready_claim_count = scan.citation_ready_claim_count.saturating_add(1);
                if handoff_claim_path {
                    scan.handoff_citation_ready_claim_count =
                        scan.handoff_citation_ready_claim_count.saturating_add(1);
                }
            }
        }
    }
}

fn evidence_claim_handoff_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    normalized.contains("evidence claims") || normalized.contains("evidence claim")
}

fn push_evidence_quality_sample(
    scan: &mut EvidenceQualityScan,
    map: &serde_json::Map<String, Value>,
    path: &str,
    low_quality: bool,
) {
    if scan.sample_rows.len() >= 16 {
        return;
    }
    let missing_fields = if evidence_packet_contract_path(path) {
        evidence_packet_missing_fields(map, &scan.request_terms)
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let packet_ready = missing_fields.is_empty() && !low_quality;
    let claims = evidence_object_claim_strings(map)
        .into_iter()
        .take(3)
        .map(|claim| clean_text(&claim, 180))
        .filter(|claim| !claim.is_empty())
        .collect::<Vec<_>>();
    let content_preview = evidence_object_content_strings(map)
        .into_iter()
        .find_map(|raw| {
            let cleaned = clean_text(&raw, 260);
            (!cleaned.is_empty()).then_some(cleaned)
        })
        .unwrap_or_default();
    let sample = json!({
        "path": clean_text(path, 240),
        "packet_ready": packet_ready,
        "low_quality": low_quality,
        "missing_fields": missing_fields,
        "source_domain": source_domain_value(map).unwrap_or_default(),
        "source_type": first_string_at(map, &["source_type", "source_kind", "source_class"], 80),
        "title": first_string_at(map, &["title", "source_title", "source_ref"], 180),
        "locator": first_string_at(map, &["locator", "url", "source_url", "link", "source_locator"], 220),
        "content_preview": content_preview,
        "malformed_fragments": malformed_fragment_samples_for_object(map),
        "claim_hints": claims,
        "quality_flags": string_array_at(map, &["quality_flags", "flags"], 6, 80),
        "confidence": first_string_at(map, &["confidence", "status"], 80),
        "materialization_quality": first_string_at(map, &["materialization_quality"], 80)
    });
    scan.sample_rows.push(sample);
}

fn first_string_at(map: &serde_json::Map<String, Value>, keys: &[&str], max_len: usize) -> String {
    keys.iter()
        .find_map(|key| {
            let cleaned = clean_text(map.get(*key).and_then(Value::as_str).unwrap_or(""), max_len);
            (!cleaned.is_empty()).then_some(cleaned)
        })
        .unwrap_or_default()
}

fn string_array_at(
    map: &serde_json::Map<String, Value>,
    keys: &[&str],
    limit: usize,
    max_len: usize,
) -> Vec<String> {
    keys.iter()
        .find_map(|key| map.get(*key).and_then(Value::as_array))
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, max_len))
                .filter(|raw| !raw.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn evidence_packet_contract_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    normalized.contains("evidence pack")
        || normalized.contains("synthesis candidates")
        || normalized.contains("source candidates")
}

fn evidence_quality_request_terms(payload: &Value) -> Vec<String> {
    let mut raw_terms = Vec::<String>::new();
    for pointer in [
        "/query",
        "/effective_query",
        "/queries",
        "/keywords",
        "/required_coverage/entities",
        "/required_coverage/facets",
        "/query_metadata/keywords",
        "/query_metadata/required_coverage/entities",
        "/query_metadata/required_coverage/facets",
        "/submitted_query_plan/primary_query",
        "/submitted_query_plan/queries",
        "/submitted_query_plan/keywords",
        "/pending_tool_request/input/query",
        "/pending_tool_request/input/queries",
        "/pending_tool_request/input/keywords",
        "/pending_tool_request/input/required_coverage/entities",
        "/pending_tool_request/input/required_coverage/facets",
        "/tooling_request/query",
        "/tooling_request/queries",
        "/tooling_request/keywords",
        "/tooling_request/required_coverage/entities",
        "/tooling_request/required_coverage/facets",
    ] {
        collect_value_strings(
            payload.pointer(pointer).unwrap_or(&Value::Null),
            &mut raw_terms,
        );
    }
    let mut out = Vec::<String>::new();
    for raw in raw_terms {
        for token in normalize_for_compare(&raw).split_whitespace() {
            let term = evidence_quality_request_term_stem(token);
            if term.len() < 3 && term != "ai" {
                continue;
            }
            if evidence_quality_request_stop_term(&term) {
                continue;
            }
            if !out.iter().any(|existing| existing == &term) {
                out.push(term);
            }
            if out.len() >= 24 {
                return out;
            }
        }
    }
    out
}

fn evidence_quality_request_term_stem(raw: &str) -> String {
    let mut value = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return value;
    }
    for suffix in ["ing", "ed", "es", "s"] {
        if value.len() > suffix.len() + 3 && value.ends_with(suffix) {
            value.truncate(value.len() - suffix.len());
            break;
        }
    }
    value
}

fn evidence_quality_request_stop_term(term: &str) -> bool {
    matches!(
        term,
        "about"
            | "after"
            | "also"
            | "answer"
            | "back"
            | "backed"
            | "best"
            | "brief"
            | "broad"
            | "broadly"
            | "but"
            | "cite"
            | "cited"
            | "citable"
            | "citation"
            | "compare"
            | "current"
            | "currently"
            | "different"
            | "early"
            | "evidence"
            | "explain"
            | "find"
            | "from"
            | "give"
            | "global"
            | "headline"
            | "headlines"
            | "how"
            | "include"
            | "including"
            | "landscape"
            | "latest"
            | "look"
            | "major"
            | "more"
            | "most"
            | "need"
            | "news"
            | "notable"
            | "official"
            | "overview"
            | "preliminary"
            | "primary"
            | "recent"
            | "recommend"
            | "recommendation"
            | "report"
            | "reported"
            | "result"
            | "results"
            | "search"
            | "separate"
            | "some"
            | "source"
            | "sources"
            | "summarize"
            | "tell"
            | "that"
            | "the"
            | "their"
            | "there"
            | "these"
            | "this"
            | "today"
            | "tomorrow"
            | "trust"
            | "update"
            | "using"
            | "what"
            | "when"
            | "where"
            | "which"
            | "while"
            | "with"
            | "would"
            | "yesterday"
    )
}

fn source_authority_sensitive_request(payload: &Value, request_terms: &[String]) -> bool {
    let mut values = Vec::<String>::new();
    for pointer in [
        "/category",
        "/tags",
        "/query",
        "/effective_query",
        "/keywords",
        "/query_metadata/tags",
        "/query_metadata/category",
        "/query_metadata/keywords",
        "/query_metadata/required_coverage/entities",
        "/query_metadata/required_coverage/facets",
        "/submitted_query_plan/primary_query",
        "/submitted_query_plan/queries",
        "/submitted_query_plan/keywords",
        "/pending_tool_request/input/query",
        "/pending_tool_request/input/queries",
        "/pending_tool_request/input/keywords",
        "/pending_tool_request/input/required_coverage/entities",
        "/pending_tool_request/input/required_coverage/facets",
        "/tooling_request/query",
        "/tooling_request/queries",
        "/tooling_request/keywords",
        "/tooling_request/required_coverage/entities",
        "/tooling_request/required_coverage/facets",
    ] {
        collect_value_strings(
            payload.pointer(pointer).unwrap_or(&Value::Null),
            &mut values,
        );
    }
    values.extend(request_terms.iter().cloned());
    let normalized = format!(" {} ", normalize_for_compare(&values.join(" ")));
    [
        " source sensitive ",
        " health medical ",
        " medical ",
        " health ",
        " clinical ",
        " clinician ",
        " patient ",
        " medicine ",
        " medication ",
        " drug ",
        " vaccine ",
        " therapy ",
        " treatment ",
        " diagnosis ",
        " disease ",
        " symptom ",
        " supplement ",
        " supplementation ",
        " prescribing ",
        " blood pressure ",
        " telehealth ",
        " public health ",
        " long covid ",
        " migraine ",
        " hormone ",
        " menopause ",
        " adhd ",
        " rsv ",
        " glp ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn evidence_packet_request_aligned(
    map: &serde_json::Map<String, Value>,
    request_terms: &[String],
) -> bool {
    if request_terms.is_empty() {
        return true;
    }
    let mut values = Vec::<String>::new();
    for key in [
        "title",
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "description",
        "relevant_extract",
        "support_snippet",
        "claim",
        "claims",
        "claim_hint",
        "claim_hints",
        "why_relevant_to_query",
        "coverage_facets",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    if values.is_empty() {
        return false;
    }
    let combined = normalize_for_compare(&values.join(" "));
    let matched = request_terms
        .iter()
        .filter(|term| evidence_packet_text_supports_request_term(&combined, term))
        .count();
    matched >= 1
}

fn evidence_packet_text_supports_request_term(normalized_text: &str, term: &str) -> bool {
    normalized_text
        .split_whitespace()
        .map(evidence_quality_request_term_stem)
        .any(|text_term| text_term == term)
        || (term.len() >= 7
            && normalized_text
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .collect::<String>()
                .contains(term))
}

fn evidence_packet_missing_fields(
    map: &serde_json::Map<String, Value>,
    request_terms: &[String],
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !evidence_packet_source_identity_ready(map) {
        missing.push("source_identity");
    }
    if !evidence_packet_source_identity_consistent(map) {
        missing.push("source_identity_consistency");
    }
    if !evidence_packet_source_type_ready(map) {
        missing.push("source_type");
    }
    if !evidence_packet_relevant_extract_ready(map) {
        missing.push("relevant_extract");
    }
    if !evidence_packet_claim_hints_ready(map) {
        missing.push("claim_hints");
    }
    if !evidence_packet_relevance_reason_ready(map, request_terms) {
        missing.push("why_relevant_to_query");
    }
    if !evidence_packet_request_aligned(map, request_terms) {
        missing.push("query_relevance_alignment");
    }
    missing
}

fn evidence_packet_source_identity_ready(map: &serde_json::Map<String, Value>) -> bool {
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
    let title_or_domain_present = ["title", "source_title", "source_ref"]
        .iter()
        .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
        || source_domain_value(map).is_some();
    locator_present && title_or_domain_present
}

fn evidence_packet_source_identity_consistent(map: &serde_json::Map<String, Value>) -> bool {
    let domain = source_domain_value(map).unwrap_or_default();
    if domain.is_empty() {
        return true;
    }
    let title = first_string_at(map, &["title", "source_title", "source_ref"], 180);
    if !source_title_is_generic_for_domain(&title, &domain) {
        return true;
    }
    let Some(publisher) = evidence_packet_publisher_menu_signature(map) else {
        return true;
    };
    source_identity_names_match(&publisher, &domain)
}

fn source_title_is_generic_for_domain(title: &str, domain: &str) -> bool {
    let normalized_title = normalize_for_compare(title);
    if normalized_title.is_empty() {
        return true;
    }
    let normalized_domain = normalize_for_compare(domain);
    normalized_title.starts_with("web result from ")
        || normalized_title.starts_with("search result from ")
        || normalized_title.starts_with("result from ")
        || normalized_title == normalized_domain
}

fn evidence_packet_publisher_menu_signature(
    map: &serde_json::Map<String, Value>,
) -> Option<String> {
    evidence_object_content_strings(map)
        .into_iter()
        .find_map(|raw| publisher_menu_signature_from_text(&raw))
}

fn publisher_menu_signature_from_text(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 500);
    let lowered = cleaned.to_ascii_lowercase();
    let marker_index = lowered.find("/ menu")?;
    let prefix = &cleaned[..marker_index];
    let segment = prefix
        .rsplit(['.', '!', '?', '|', '\n', '\r'])
        .next()
        .unwrap_or(prefix);
    let candidate = clean_text(
        segment.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != ' '),
        120,
    );
    let words = word_count(&candidate);
    let has_letter = candidate.chars().any(|ch| ch.is_ascii_alphabetic());
    let has_name_shape = candidate
        .split_whitespace()
        .any(|word| word.chars().next().map(char::is_uppercase).unwrap_or(false));
    (has_letter && has_name_shape && (1..=6).contains(&words)).then_some(candidate)
}

fn source_identity_names_match(publisher: &str, domain: &str) -> bool {
    let publisher_compact = compact_identity_name(publisher);
    let domain_compact = compact_identity_name_without_tld(domain);
    if publisher_compact.len() < 4 || domain_compact.len() < 4 {
        return true;
    }
    domain_compact.contains(&publisher_compact) || publisher_compact.contains(&domain_compact)
}

fn compact_identity_name(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn compact_identity_name_without_tld(raw: &str) -> String {
    let mut parts = raw
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or(raw)
        .split('.')
        .filter(|part| {
            !matches!(
                part.to_ascii_lowercase().as_str(),
                "www" | "com" | "org" | "net" | "gov" | "edu" | "co" | "uk" | "io" | "ai" | "dev"
            )
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        parts.push(raw.to_string());
    }
    compact_identity_name(&parts.join(""))
}

fn evidence_packet_source_type_ready(map: &serde_json::Map<String, Value>) -> bool {
    ["source_type", "source_kind", "source_class"]
        .iter()
        .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
}

fn evidence_packet_relevant_extract_ready(map: &serde_json::Map<String, Value>) -> bool {
    let mut values = evidence_object_content_strings(map);
    collect_value_strings(
        map.get("relevant_extract").unwrap_or(&Value::Null),
        &mut values,
    );
    values
        .iter()
        .any(|raw| !content_text_low_quality(raw) && word_count(raw) >= 8)
}

fn evidence_packet_claim_hints_ready(map: &serde_json::Map<String, Value>) -> bool {
    evidence_object_claim_strings(map)
        .iter()
        .any(|claim| !claim_text_low_quality(claim) && claim_text_concrete(claim))
}

fn evidence_packet_relevance_reason_ready(
    map: &serde_json::Map<String, Value>,
    request_terms: &[String],
) -> bool {
    let direct_reason = [
        "why_relevant_to_query",
        "relevance_reason",
        "selection_reason",
        "coverage_reason",
    ]
    .iter()
    .filter_map(|key| map.get(*key).and_then(Value::as_str))
    .any(|raw| {
        let cleaned = clean_text(raw, 300);
        !cleaned.is_empty() && !content_text_low_quality(&cleaned) && word_count(&cleaned) >= 4
    });
    (direct_reason && evidence_packet_request_aligned(map, request_terms))
        || map
            .get("coverage_facets")
            .map(value_has_content)
            .unwrap_or(false)
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

fn evidence_object_authority_grade(map: &serde_json::Map<String, Value>) -> bool {
    let mut values = Vec::<String>::new();
    for key in [
        "source_type",
        "source_kind",
        "source_class",
        "class",
        "source_quality",
        "materialization_quality",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    let normalized = format!(" {} ", normalize_for_compare(&values.join(" ")));
    let authority_class = [
        " official ",
        " primary ",
        " public institution ",
        " government ",
        " scholarly ",
        " research ",
        " journal ",
        " peer reviewed ",
        " peer reviewed ",
        " guideline ",
        " medical guidance ",
        " clinical ",
        " registry ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    authority_class
        || source_domain_value(map)
            .map(|domain| source_domain_authority_grade(&domain))
            .unwrap_or(false)
}

fn source_domain_authority_grade(domain: &str) -> bool {
    let host = domain
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or(domain)
        .to_ascii_lowercase();
    host.ends_with(".gov")
        || host.ends_with(".edu")
        || host == "who.int"
        || host.ends_with(".who.int")
        || host == "cochrane.org"
        || host.ends_with(".cochrane.org")
        || host == "doi.org"
        || host.ends_with(".doi.org")
        || host.contains("pubmed.ncbi.nlm.nih.gov")
        || host.contains("pmc.ncbi.nlm.nih.gov")
        || host.contains("ncbi.nlm.nih.gov")
        || host.contains("nih.gov")
        || host.contains("medlineplus.gov")
        || host.contains("cdc.gov")
        || host.contains("fda.gov")
        || host.contains("clinicaltrials.gov")
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
    if evidence_packet_contract_path(path)
        && !evidence_packet_request_aligned(map, &scan.request_terms)
    {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "query_relevance_not_aligned");
    }
    if evidence_packet_contract_path(path) && !evidence_packet_source_identity_consistent(map) {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "source_identity_mismatch");
    }
    let content_strings = evidence_object_content_strings(map);
    let mut malformed_item = false;
    for raw in &content_strings {
        if evidence_text_malformed_fragment(raw) {
            malformed_item = true;
            scan.malformed_evidence_fragment_count =
                scan.malformed_evidence_fragment_count.saturating_add(1);
            push_unique_case_insensitive(&mut scan.low_quality_flags, "malformed_content_text");
            push_malformed_evidence_sample(scan, raw);
        }
    }
    if !content_strings.is_empty()
        && content_strings
            .iter()
            .all(|raw| content_text_low_quality(raw))
    {
        low_quality = true;
        push_unique_case_insensitive(
            &mut scan.low_quality_flags,
            "boilerplate_or_source_only_text",
        );
    }
    let claim_strings = evidence_object_claim_strings(map);
    for claim in &claim_strings {
        if claim_text_malformed_fragment(claim) {
            malformed_item = true;
            scan.malformed_evidence_fragment_count =
                scan.malformed_evidence_fragment_count.saturating_add(1);
            push_unique_case_insensitive(&mut scan.low_quality_flags, "malformed_claim_text");
            push_malformed_evidence_sample(scan, claim);
        }
    }
    if malformed_item {
        low_quality = true;
        scan.malformed_evidence_item_count = scan.malformed_evidence_item_count.saturating_add(1);
    }
    if !claim_strings.is_empty()
        && claim_strings.iter().all(|claim| {
            claim_text_low_quality(claim)
                || claim_text_malformed_fragment(claim)
                || !claim_text_concrete(claim)
        })
    {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "low_quality_claim_text");
    }
    low_quality
}

fn push_malformed_evidence_sample(scan: &mut EvidenceQualityScan, raw: &str) {
    if scan.malformed_evidence_samples.len() >= 16 {
        return;
    }
    let cleaned = clean_text(raw, 220);
    if cleaned.is_empty() {
        return;
    }
    push_unique_case_insensitive(&mut scan.malformed_evidence_samples, &cleaned);
}

fn malformed_citation_title_for_object(map: &serde_json::Map<String, Value>) -> Option<String> {
    let title = first_string_at(map, &["title", "source_title"], 220);
    if title.is_empty() {
        return None;
    }
    let domain = source_domain_value(map).unwrap_or_default();
    if source_title_is_generic_for_domain(&title, &domain) {
        return None;
    }
    citation_title_malformed_fragment(&title).then_some(title)
}

fn push_malformed_citation_title_sample(scan: &mut EvidenceQualityScan, raw: &str) {
    if scan.malformed_citation_title_samples.len() >= 16 {
        return;
    }
    let cleaned = clean_text(raw, 220);
    if cleaned.is_empty() {
        return;
    }
    push_unique_case_insensitive(&mut scan.malformed_citation_title_samples, &cleaned);
}

fn malformed_fragment_samples_for_object(map: &serde_json::Map<String, Value>) -> Vec<String> {
    evidence_object_content_strings(map)
        .into_iter()
        .chain(evidence_object_claim_strings(map))
        .filter(|raw| evidence_text_malformed_fragment(raw) || claim_text_malformed_fragment(raw))
        .map(|raw| clean_text(&raw, 180))
        .filter(|raw| !raw.is_empty())
        .take(3)
        .collect()
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
        "relevant_extract",
        "support_snippet",
        "raw_content_excerpt",
        "snippet_preview",
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
    if text_looks_like_headline_or_dateline_shell(&cleaned) {
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

fn evidence_text_malformed_fragment(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    if cleaned.is_empty() {
        return false;
    }
    text_has_stitched_title_tail(&cleaned) || text_looks_like_page_chrome_fragment(&cleaned)
}

fn claim_text_malformed_fragment(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    if cleaned.is_empty() {
        return false;
    }
    evidence_text_malformed_fragment(&cleaned)
        || text_looks_like_dangling_claim_fragment(&cleaned)
        || text_looks_like_title_shell_fragment(&cleaned)
}

fn citation_title_malformed_fragment(raw: &str) -> bool {
    let cleaned = clean_text(raw, 220);
    if cleaned.is_empty() {
        return false;
    }
    text_looks_like_page_chrome_fragment(&cleaned)
        || text_looks_like_dangling_claim_fragment(&cleaned)
        || source_title_starts_with_page_debris_lead(&cleaned)
        || source_title_starts_with_lowercase_fragment_before_title_shell(&cleaned)
}

fn source_title_starts_with_page_debris_lead(raw: &str) -> bool {
    let cleaned = clean_text(raw, 220);
    let words = cleaned.split_whitespace().collect::<Vec<_>>();
    if words.len() < 4 {
        return false;
    }
    let first = words[0]
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    let second = words[1]
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    let first_is_debris = matches!(
        first.as_str(),
        "browse"
            | "click"
            | "continue"
            | "listen"
            | "menu"
            | "more"
            | "open"
            | "read"
            | "see"
            | "view"
            | "visit"
            | "watch"
    );
    if !first_is_debris {
        return false;
    }
    let tail_start = if matches!(second.as_str(), "as" | "more" | "now" | "the" | "this") {
        2
    } else {
        1
    };
    if words.len() <= tail_start + 2 {
        return false;
    }
    tail_looks_like_title_shell(&words[tail_start..].join(" "))
}

fn source_title_starts_with_lowercase_fragment_before_title_shell(raw: &str) -> bool {
    let cleaned = clean_text(raw, 220);
    let words = cleaned.split_whitespace().collect::<Vec<_>>();
    if words.len() < 5 || words.len() > 18 {
        return false;
    }
    for lead_len in 1..=3.min(words.len().saturating_sub(3)) {
        let lead_is_fragment = words[..lead_len].iter().all(|word| {
            let token = word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-');
            !token.is_empty()
                && token.len() <= 18
                && token.chars().all(|ch| ch.is_ascii_lowercase() || ch == '-')
        });
        if !lead_is_fragment {
            continue;
        }
        if tail_looks_like_title_shell(&words[lead_len..].join(" ")) {
            return true;
        }
    }
    false
}

fn claim_text_low_quality(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = normalize_for_compare(&cleaned);
    if cleaned.len() < 28 || word_count(&cleaned) < 4 {
        return true;
    }
    if text_looks_like_headline_or_dateline_shell(&cleaned)
        || text_looks_like_question_headline(&cleaned)
        || text_looks_like_title_byline_shell(&cleaned)
        || text_looks_like_teaser_or_projection_shell(&cleaned)
        || text_looks_like_dangling_claim_fragment(&cleaned)
    {
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
        "look out for ",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn text_looks_like_page_chrome_fragment(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = format!(" {} ", normalize_for_compare(&cleaned));
    let has_breadcrumb = cleaned.contains('>')
        && (normalized.contains(" home ")
            || normalized.contains(" categories ")
            || normalized.contains(" city guides ")
            || normalized.contains(" blog "));
    let has_promotional_chrome = normalized.contains(" save up to ")
        && normalized.contains(" home ")
        && (normalized.contains(" guide ") || normalized.contains(" guides "));
    has_breadcrumb || has_promotional_chrome
}

fn text_looks_like_title_shell_fragment(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let words = word_count(&cleaned);
    if !(3..=16).contains(&words) {
        return false;
    }
    let normalized = normalize_for_compare(&cleaned);
    let starts_like_title = cleaned
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
        || [
            "best ",
            "top ",
            "where to ",
            "how to ",
            "what to ",
            "guide to ",
            "complete guide ",
            "ultimate guide ",
        ]
        .iter()
        .any(|prefix| normalized.starts_with(prefix));
    let lacks_sentence_verb = ![
        " is ",
        " are ",
        " was ",
        " were ",
        " has ",
        " have ",
        " had ",
        " says ",
        " said ",
        " shows ",
        " found ",
        " reports ",
        " reported ",
        " announced ",
        " released ",
        " provides ",
        " offers ",
        " supports ",
        " requires ",
    ]
    .iter()
    .any(|marker| format!(" {normalized} ").contains(marker));
    tail_looks_like_title_shell(&cleaned) && (starts_like_title || lacks_sentence_verb)
}

fn text_looks_like_title_byline_shell(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = format!(" {} ", normalize_for_compare(&cleaned));
    let words = word_count(&cleaned);
    if words > 58 || !normalized.contains(" by ") {
        return false;
    }
    let has_month = [
        " january ",
        " february ",
        " march ",
        " april ",
        " may ",
        " june ",
        " july ",
        " august ",
        " september ",
        " october ",
        " november ",
        " december ",
        " jan ",
        " feb ",
        " mar ",
        " apr ",
        " jun ",
        " jul ",
        " aug ",
        " sep ",
        " oct ",
        " nov ",
        " dec ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    let has_dateline_or_media_marker = normalized.contains(" gmt ")
        || normalized.contains(" utc ")
        || normalized.contains(" est ")
        || normalized.contains(" edt ")
        || normalized.contains(" pst ")
        || normalized.contains(" pdt ")
        || normalized.contains(" getty images ")
        || normalized.contains(" published ");
    has_month || has_dateline_or_media_marker
}

fn text_looks_like_teaser_or_projection_shell(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = format!(" {} ", normalize_for_compare(&cleaned));
    let trimmed = normalized.trim_start();
    let editorial_teaser = [
        "look out for ",
        "watch for ",
        "what to expect ",
        "things to watch ",
        "would like to see ",
        "i d like to see ",
        "id like to see ",
    ]
    .iter()
    .any(|marker| trimmed.starts_with(marker) || normalized.contains(marker));
    if editorial_teaser {
        return true;
    }
    let projection_signal = [
        " will become available ",
        " will be available ",
        " expected to become ",
        " expected to be ",
        " set to become ",
        " set to be ",
        " could become ",
        " could lead to ",
        " may become ",
        " may lead to ",
        " future timeline ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    if !projection_signal {
        return false;
    }
    let attributed_or_reported = [
        " according to ",
        " predicts ",
        " forecast ",
        " projects ",
        " estimates ",
        " expects ",
        " reported ",
        " announced ",
        " released ",
        " granted ",
        " approved ",
        " found ",
        " showed ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    !attributed_or_reported
}

fn text_looks_like_dangling_claim_fragment(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let trimmed = cleaned.trim_start();
    if trimmed.is_empty() {
        return true;
    }
    let Some(first) = trimmed.chars().next() else {
        return true;
    };
    if matches!(first, ',' | ';' | ':' | '.' | '!' | '?' | ')' | ']' | '}') {
        return true;
    }
    let first_token = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    if matches!(
        first_token.as_str(),
        "and"
            | "or"
            | "but"
            | "because"
            | "while"
            | "although"
            | "though"
            | "which"
            | "who"
            | "that"
            | "than"
            | "including"
            | "with"
            | "without"
            | "featuring"
            | "from"
            | "into"
            | "onto"
            | "over"
            | "representing"
            | "under"
            | "through"
    ) {
        return true;
    }
    let last = trimmed
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if last.len() == 1
        && last
            .chars()
            .next()
            .map(|ch| ch.is_ascii_lowercase())
            .unwrap_or(false)
    {
        return true;
    }
    if text_has_stitched_title_tail(&cleaned) {
        return true;
    }
    let normalized = normalize_for_compare(&cleaned);
    [
        " based on data",
        " based on evidence",
        " based on findings",
        " including",
        " for example",
        " showed that",
        " according to",
    ]
    .iter()
    .any(|tail| normalized.ends_with(tail))
}

fn text_has_stitched_title_tail(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let lowered = format!(" {} ", cleaned.to_ascii_lowercase());
    if lowered.contains(" vs ") && lowered.trim_end().ends_with(" comparison") {
        return true;
    }
    [
        " doesn't ",
        " doesnt ",
        " doesn’t ",
        " does not ",
        " isn't ",
        " isnt ",
        " isn’t ",
        " is not ",
    ]
    .iter()
    .any(|marker| {
        lowered.find(marker).map_or(false, |index| {
            let tail = clean_text(&cleaned[index + marker.len() - 1..], 220);
            tail.split_whitespace().count() <= 8
                && (tail.contains(" vs ")
                    || tail.contains(" Comparison")
                    || tail.contains(" comparison")
                    || tail
                        .chars()
                        .find(|ch| ch.is_ascii_alphabetic())
                        .map(|ch| ch.is_ascii_uppercase())
                        .unwrap_or(false))
        })
    }) || stitched_terminal_title_shell(&cleaned)
}

fn stitched_terminal_title_shell(raw: &str) -> bool {
    let words = raw.split_whitespace().collect::<Vec<_>>();
    if words.len() < 6 {
        return false;
    }
    let max_tail = words.len().min(10);
    for tail_len in 3..=max_tail {
        let start = words.len().saturating_sub(tail_len);
        if start < 3 {
            continue;
        }
        let tail = words[start..].join(" ");
        if !tail_looks_like_title_shell(&tail) {
            continue;
        }
        let prefix = words[..start].join(" ");
        let prefix_words = word_count(&prefix);
        if prefix_words < 4 {
            continue;
        }
        let previous_token = words[start.saturating_sub(1)]
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
            .to_string();
        let previous_ends_lowercase = previous_token
            .chars()
            .rev()
            .find(|ch| ch.is_ascii_alphabetic())
            .map(|ch| ch.is_ascii_lowercase())
            .unwrap_or(false);
        let prefix_has_sentence_delimiter = prefix.trim_end().ends_with(['.', '!', '?']);
        if previous_ends_lowercase || prefix_has_sentence_delimiter {
            return true;
        }
    }
    false
}

fn tail_looks_like_title_shell(raw: &str) -> bool {
    let cleaned = clean_text(raw, 220);
    let normalized = format!(" {} ", normalize_for_compare(&cleaned));
    let words = word_count(&cleaned);
    if !(3..=12).contains(&words) {
        return false;
    }
    let has_title_marker = [
        " best ",
        " top ",
        " guide ",
        " guides ",
        " comparison ",
        " compared ",
        " alternatives ",
        " review ",
        " reviews ",
        " where to ",
        " how to ",
        " what to ",
        " things to ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    if !has_title_marker {
        return false;
    }
    let titleish_tokens = cleaned
        .split_whitespace()
        .filter(|token| {
            let token = token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
            token
                .chars()
                .next()
                .map(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
                .unwrap_or(false)
        })
        .count();
    titleish_tokens >= 2
}

fn text_looks_like_headline_or_dateline_shell(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = normalize_for_compare(&cleaned);
    let words = word_count(&cleaned);
    let has_published_marker = normalized.contains(" published ")
        || normalized.contains(" published:")
        || normalized.starts_with("published ");
    let has_source_marker = normalized.contains(" source ")
        || normalized.contains(" source:")
        || normalized.starts_with("source ");
    let has_wire_time_marker = normalized.contains(" gmt ")
        || normalized.contains(" utc ")
        || normalized.contains(" via google news");
    if has_published_marker && (has_source_marker || has_wire_time_marker) && words <= 42 {
        return true;
    }
    if normalized.contains(" via google news") && words <= 32 {
        return true;
    }
    false
}

fn text_looks_like_question_headline(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let words = word_count(&cleaned);
    if words > 22 {
        return false;
    }
    let raw_lowered = cleaned.to_ascii_lowercase();
    let starts_interrogative = [
        "what", "why", "how", "when", "where", "who", "can", "could", "is", "are", "will", "should",
    ]
    .iter()
    .any(|prefix| raw_lowered.starts_with(prefix));
    if cleaned.contains('?') && starts_interrogative {
        return true;
    }
    let normalized = normalize_for_compare(&cleaned);
    let normalized_interrogative = [
        "what ", "what s ", "whats ", "what is ", "why ", "how ", "when ", "where ", "who ",
        "can ", "could ", "is ", "are ", "will ", "should ",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix));
    if cleaned.contains('?') {
        return normalized_interrogative;
    }
    let has_factual_anchor = cleaned.chars().any(|ch| ch.is_ascii_digit())
        || [
            " announced ",
            " approved ",
            " found ",
            " granted ",
            " reported ",
            " showed ",
            " shows ",
        ]
        .iter()
        .any(|marker| format!(" {normalized} ").contains(marker));
    words <= 12 && normalized_interrogative && !has_factual_anchor
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
