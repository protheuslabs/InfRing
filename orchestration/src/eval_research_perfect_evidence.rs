use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "validation/evals/fixtures/research_perfect_evidence_dataset_v1.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/research_perfect_evidence_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/research_perfect_evidence_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_CURRENT.md";

#[derive(Debug, Clone)]
struct CaseReadiness {
    id: String,
    prompt: String,
    category: String,
    posture: String,
    ok: bool,
    blockers: Vec<String>,
    evidence_packets: usize,
    ready_packets: usize,
    source_domains: usize,
    source_kinds: usize,
    claim_hints: usize,
    replay_payload_ready: bool,
}

pub fn run_research_perfect_evidence(args: &[String]) -> i32 {
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let strict = parse_bool_flag(args, "strict", true);

    let dataset = match fs::read_to_string(&cases_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!("research-perfect-evidence: failed to read cases from {cases_path}");
            return 2;
        }
    };

    let cases = dataset
        .get("cases")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let min_cases = dataset
        .get("reliability_thresholds")
        .and_then(|v| v.get("min_cases_for_reliability_claim"))
        .and_then(Value::as_u64)
        .unwrap_or(30) as usize;
    let min_categories = dataset
        .get("reliability_thresholds")
        .and_then(|v| v.get("min_categories_for_reliability_claim"))
        .and_then(Value::as_u64)
        .unwrap_or(8) as usize;

    let mut rows = Vec::new();
    let mut category_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut posture_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut source_domain_set = BTreeSet::new();
    let mut total_packets = 0usize;
    let mut total_ready_packets = 0usize;
    let mut total_claim_hints = 0usize;
    let mut exact_answer_key_cases = 0usize;

    for case in cases {
        let readiness = evaluate_case(case);
        *category_counts
            .entry(readiness.category.clone())
            .or_default() += 1;
        *posture_counts.entry(readiness.posture.clone()).or_default() += 1;
        total_packets += readiness.evidence_packets;
        total_ready_packets += readiness.ready_packets;
        total_claim_hints += readiness.claim_hints;
        if readiness
            .blockers
            .iter()
            .any(|blocker| blocker == "exact_answer_key_present")
        {
            exact_answer_key_cases += 1;
        }
        for item in case
            .get("evidence_pack")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            if let Some(domain) = non_empty_str(item, "source_domain") {
                source_domain_set.insert(domain.to_string());
            }
        }
        rows.push(readiness);
    }

    let passed_cases = rows.iter().filter(|row| row.ok).count();
    let replay_ready_cases = rows.iter().filter(|row| row.replay_payload_ready).count();
    let categories_ready = category_counts.len() >= min_categories;
    let case_volume_ready = cases.len() >= min_cases;
    let all_cases_ready = passed_cases == cases.len() && !cases.is_empty();
    let packet_ready_rate = rate(total_ready_packets, total_packets);
    let case_pass_rate = rate(passed_cases, cases.len());
    let replay_payload_ready_rate = rate(replay_ready_cases, cases.len());
    let ok = all_cases_ready && case_volume_ready && categories_ready;

    let case_rows: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "category": row.category,
                "posture": row.posture,
                "ok": row.ok,
                "blockers": row.blockers,
                "evidence_packets": row.evidence_packets,
                "ready_packets": row.ready_packets,
                "source_domains": row.source_domains,
                "source_kinds": row.source_kinds,
                "claim_hints": row.claim_hints,
                "replay_payload_ready": row.replay_payload_ready,
                "prompt": row.prompt,
            })
        })
        .collect();

    let replay_payload_examples: Vec<Value> =
        cases.iter().take(3).map(build_replay_payload).collect();

    let report = json!({
        "type": "research_perfect_evidence_readiness",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "summary": {
            "cases_total": cases.len(),
            "min_cases_for_reliability_claim": min_cases,
            "case_volume_ready": case_volume_ready,
            "categories_total": category_counts.len(),
            "min_categories_for_reliability_claim": min_categories,
            "categories_ready": categories_ready,
            "passed_cases": passed_cases,
            "case_pass_rate": case_pass_rate,
            "replay_ready_cases": replay_ready_cases,
            "replay_payload_ready_rate": replay_payload_ready_rate,
            "evidence_packets_total": total_packets,
            "ready_evidence_packets": total_ready_packets,
            "evidence_packet_ready_rate": packet_ready_rate,
            "claim_hints_total": total_claim_hints,
            "source_domains_total": source_domain_set.len(),
            "exact_answer_key_cases": exact_answer_key_cases,
            "category_counts": category_counts,
            "posture_counts": posture_counts,
        },
        "dataset": {
            "path": cases_path,
            "dataset_id": str_at(&dataset, "dataset_id"),
            "answer_key_policy": dataset.get("answer_key_policy").cloned().unwrap_or_else(|| json!({})),
        },
        "cases": case_rows,
        "replay_payload_examples": replay_payload_examples,
    });
    let markdown = render_markdown(&report, &rows);

    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("research-perfect-evidence: failed to write one or more outputs");
        return 2;
    }

    print_structured(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

fn evaluate_case(case: &Value) -> CaseReadiness {
    let id = str_at(case, "id");
    let prompt = str_at(case, "prompt");
    let category = str_at(case, "category");
    let posture = str_at(case, "expected_evidence_posture");
    let evidence_pack = case
        .get("evidence_pack")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut blockers = Vec::new();
    let mut source_domains = BTreeSet::new();
    let mut source_kinds = BTreeSet::new();
    let mut ready_packets = 0usize;
    let mut claim_hints = 0usize;

    if id.trim().is_empty() {
        blockers.push("missing_case_id".to_string());
    }
    if prompt.trim().len() < 20 {
        blockers.push("prompt_too_thin".to_string());
    }
    if category.trim().is_empty() {
        blockers.push("missing_category".to_string());
    }
    if forbidden_answer_key_present(case) {
        blockers.push("exact_answer_key_present".to_string());
    }

    let required_packet_count = if posture == "insufficient" { 2 } else { 3 };
    if evidence_pack.len() < required_packet_count {
        blockers.push(format!("evidence_pack_lt_{required_packet_count}"));
    }

    for item in evidence_pack {
        if let Some(domain) = non_empty_str(item, "source_domain") {
            source_domains.insert(domain.to_string());
        }
        if let Some(kind) = non_empty_str(item, "source_kind") {
            source_kinds.insert(kind.to_string());
        }
        let hints = item
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        claim_hints += hints.iter().filter(|hint| text_len(hint) >= 16).count();
        if evidence_item_ready(item) {
            ready_packets += 1;
        }
    }

    if ready_packets < required_packet_count {
        blockers.push(format!("ready_evidence_packets_lt_{required_packet_count}"));
    }
    if posture != "insufficient" && source_domains.len() < 2 {
        blockers.push("source_domain_diversity_lt_2".to_string());
    }
    if source_kinds.len() < 2 {
        blockers.push("source_kind_diversity_lt_2".to_string());
    }
    if posture != "insufficient" && claim_hints < 3 {
        blockers.push("claim_hints_lt_3".to_string());
    }
    let replay_payload_ready = blockers.iter().all(|blocker| {
        blocker != "missing_case_id"
            && blocker != "prompt_too_thin"
            && blocker != "missing_category"
            && blocker != "exact_answer_key_present"
            && !blocker.starts_with("evidence_pack_lt_")
            && !blocker.starts_with("ready_evidence_packets_lt_")
    });

    CaseReadiness {
        id,
        prompt,
        category,
        posture,
        ok: blockers.is_empty(),
        blockers,
        evidence_packets: evidence_pack.len(),
        ready_packets,
        source_domains: source_domains.len(),
        source_kinds: source_kinds.len(),
        claim_hints,
        replay_payload_ready,
    }
}

fn evidence_item_ready(item: &Value) -> bool {
    non_empty_str(item, "id").is_some()
        && non_empty_str(item, "title").is_some()
        && non_empty_str(item, "locator").is_some()
        && non_empty_str(item, "source_domain").is_some()
        && non_empty_str(item, "source_kind").is_some()
        && text_field_len(item, "relevant_extract") >= 120
        && item
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(|hints| hints.iter().any(|hint| text_len(hint) >= 16))
            .unwrap_or(false)
}

fn build_replay_payload(case: &Value) -> Value {
    let case_id = str_at(case, "id");
    let prompt = str_at(case, "prompt");
    let evidence_pack = case
        .get("evidence_pack")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let evidence_refs: Vec<Value> = evidence_pack
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            json!({
                "id": non_empty_str(item, "id").unwrap_or("evidence"),
                "source": non_empty_str(item, "title").unwrap_or("synthetic evidence source"),
                "locator": non_empty_str(item, "locator").unwrap_or("fixture://unknown"),
                "source_domain": non_empty_str(item, "source_domain").unwrap_or("unknown"),
                "rank": idx + 1,
            })
        })
        .collect();
    json!({
        "case_id": case_id,
        "user_prompt": prompt,
        "pending_tool_request": {
            "status": "executed",
            "tool_family": "web_research",
            "tool_name": "batch_query",
            "synthetic_replay": true
        },
        "tool_result_quality": {
            "status": if str_at(case, "expected_evidence_posture") == "insufficient" { "insufficient" } else { "usable" },
            "source": "research_perfect_evidence_dataset_v1"
        },
        "tools": [{
            "name": "batch_query",
            "status": "done",
            "synthetic_replay": true,
            "evidence_refs": evidence_refs,
            "evidence_pack": evidence_pack
        }]
    })
}

fn forbidden_answer_key_present(value: &Value) -> bool {
    let forbidden = [
        "expected_answer",
        "ideal_answer",
        "ideal_response",
        "golden_answer",
        "answer_key",
        "expected_final_answer",
    ];
    match value {
        Value::Object(map) => map.iter().any(|(key, child)| {
            forbidden.contains(&key.as_str()) || forbidden_answer_key_present(child)
        }),
        Value::Array(items) => items.iter().any(forbidden_answer_key_present),
        _ => false,
    }
}

fn render_markdown(report: &Value, rows: &[CaseReadiness]) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = String::new();
    out.push_str("# Research Perfect Evidence Readiness\n\n");
    out.push_str(&format!(
        "- ok: {}\n",
        report.get("ok").and_then(Value::as_bool).unwrap_or(false)
    ));
    out.push_str(&format!(
        "- cases: {} / min {}\n",
        summary
            .get("cases_total")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("min_cases_for_reliability_claim")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "- case_pass_rate: {:.3}\n",
        summary
            .get("case_pass_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- evidence_packet_ready_rate: {:.3}\n",
        summary
            .get("evidence_packet_ready_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- categories: {}\n",
        summary
            .get("categories_total")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str("\n## Blocked Cases\n\n");
    let mut blocked = false;
    for row in rows.iter().filter(|row| !row.ok) {
        blocked = true;
        out.push_str(&format!(
            "- `{}`: {}\n",
            row.id,
            if row.blockers.is_empty() {
                "unknown".to_string()
            } else {
                row.blockers.join(", ")
            }
        ));
    }
    if !blocked {
        out.push_str("- none\n");
    }
    out
}

fn text_len(value: &Value) -> usize {
    value.as_str().map(|text| text.trim().len()).unwrap_or(0)
}

fn text_field_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|text| text.trim().len())
        .unwrap_or(0)
}

fn non_empty_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn str_at(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline_prefix = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline_prefix) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            if let Some(value) = args.get(idx + 1) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    match parse_flag(args, key).as_deref() {
        Some("1" | "true" | "TRUE" | "yes" | "on") => true,
        Some("0" | "false" | "FALSE" | "no" | "off") => false,
        Some(_) | None => default,
    }
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)
}

fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn print_structured(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(_) => println!("{value:?}"),
    }
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_evidence_case_rejects_exact_answer_keys() {
        let case = json!({
            "id": "case",
            "category": "general_other",
            "prompt": "Research a broad topic with outside evidence.",
            "expected_evidence_posture": "answerable",
            "expected_answer": "do not do this",
            "evidence_pack": []
        });
        let readiness = evaluate_case(&case);
        assert!(readiness
            .blockers
            .contains(&"exact_answer_key_present".to_string()));
    }

    #[test]
    fn insufficient_posture_allows_two_gap_sources() {
        let case = json!({
            "id": "case",
            "category": "general_other",
            "prompt": "Research whether a niche product has reliable public benchmarks.",
            "expected_evidence_posture": "insufficient",
            "evidence_pack": [
                evidence_item("one", "official_docs", "The available page names the product and lists marketing features, but it does not include independent benchmark measurements, test conditions, sample sizes, or reproducible methodology. The only useful answer support is that public evidence is thin and any performance ranking should be treated as unsupported."),
                evidence_item("two", "independent_review", "The independent roundup mentions the product as announced but does not publish a hands-on review, measured performance data, support history, pricing stability, or customer reliability evidence. It supports an honest insufficient-evidence answer rather than a confident recommendation.")
            ]
        });
        let readiness = evaluate_case(&case);
        assert!(readiness.ok, "{:?}", readiness.blockers);
    }

    fn evidence_item(id: &str, kind: &str, extract: &str) -> Value {
        json!({
            "id": id,
            "title": format!("Source {id}"),
            "locator": format!("fixture://{id}"),
            "source_domain": format!("{id}.example.test"),
            "source_kind": kind,
            "relevant_extract": extract,
            "claim_hints": ["The answer should stay bounded to what this evidence supports."]
        })
    }
}
