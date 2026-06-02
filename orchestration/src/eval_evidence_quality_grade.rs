use serde_json::{json, Value};

use super::eval_research_golden_utils::{
    bool_at, clean_text, f64_at, now_iso_like, str_at, u64_at,
};
use super::eval_web_retrieval_gate_diagnostics::evidence_quality_diagnostics;

const DEFAULT_INPUT_PATH: &str = "core/local/artifacts/web_tooling_golden_current.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/evidence_quality_grade_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/evidence_quality_grade_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVIDENCE_QUALITY_GRADE_CURRENT.md";

pub fn run_evidence_quality_grade(args: &[String]) -> i32 {
    let strict = super::parse_bool_flag(args, "strict", false);
    let input_path =
        super::parse_flag(args, "input").unwrap_or_else(|| DEFAULT_INPUT_PATH.to_string());
    let out_path = super::parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path = super::parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path = super::parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let pass_min = super::parse_flag(args, "pass-min")
        .and_then(|raw| raw.parse::<f64>().ok())
        .unwrap_or(0.95)
        .clamp(0.0, 1.0);

    let input = super::read_json(&input_path);
    let rows = evidence_rows(&input);
    let cases: Vec<Value> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| grade_evidence_row(row, index))
        .collect();
    let summary = evidence_summary(&cases, pass_min);
    let ok = bool_at(&summary, &["ok"], false);
    let report = json!({
        "type": "evidence_quality_grade",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "mode": "offline_replay",
        "source": {
            "input": input_path
        },
        "summary": summary,
        "cases": cases
    });
    let markdown = evidence_markdown_report(&report);
    let write_ok = super::write_json(&out_path, &report).is_ok()
        && super::write_json(&out_latest_path, &report).is_ok()
        && super::write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more evidence-quality grade outputs");
        return 2;
    }
    super::print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn evidence_rows(input: &Value) -> Vec<Value> {
    input
        .get("cases")
        .or_else(|| input.get("rows"))
        .or_else(|| input.get("evidence_cases"))
        .and_then(Value::as_array)
        .cloned()
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec![input.clone()])
}

fn grade_evidence_row(row: &Value, index: usize) -> Value {
    let case_id = row_id(row, index);
    let (payload, payload_source) = evidence_payload(row);
    let retrieval_quality = evidence_retrieval_quality(row, &payload);
    let recomputed = evidence_quality_diagnostics(&payload, &retrieval_quality);
    let embedded = row
        .pointer("/web_tool_gate_diagnostics/evidence_quality")
        .or_else(|| row.pointer("/evidence_quality"))
        .cloned();
    let (effective, effective_source) =
        effective_evidence_quality(&recomputed, embedded.as_ref(), payload_source);
    let ready = readiness_pass(&effective);
    json!({
        "case_id": case_id,
        "ready": ready,
        "payload_source": payload_source,
        "effective_evidence_quality_source": effective_source,
        "embedded_evidence_quality_present": embedded.is_some(),
        "retrieval_status": str_at(&retrieval_quality, &["status"], ""),
        "retrieval_usable_evidence": bool_at(&retrieval_quality, &["usable_evidence"], false),
        "effective_evidence_quality": effective,
        "recomputed_evidence_quality": recomputed,
        "embedded_evidence_quality": embedded.unwrap_or(Value::Null)
    })
}

fn row_id(row: &Value, index: usize) -> String {
    let id = str_at(row, &["case_id"], "");
    if !id.is_empty() {
        return id;
    }
    let id = str_at(row, &["id"], "");
    if !id.is_empty() {
        return id;
    }
    format!("evidence_case_{:03}", index + 1)
}

fn evidence_payload(row: &Value) -> (Value, &'static str) {
    for (field, label) in [
        ("evidence_payload", "evidence_payload"),
        ("payload", "payload"),
        ("synthetic_payload", "synthetic_payload"),
        ("direct_tool_payload", "direct_tool_payload"),
        ("direct_tool_payload_sample", "direct_tool_payload_sample"),
        ("tooling_payload", "tooling_payload"),
        ("tool_payload", "tool_payload"),
        ("raw_payload", "raw_payload"),
    ] {
        if let Some(value) = row.get(field) {
            return (value.clone(), label);
        }
    }
    if let Some(value) = row.pointer("/tool_pipeline/raw_payload") {
        return (value.clone(), "tool_pipeline.raw_payload");
    }
    if let Some(value) = row.pointer("/response_finalization/tool_completion/tool_attempts/0") {
        return (
            value.clone(),
            "response_finalization.tool_completion.tool_attempts.0",
        );
    }
    (row.clone(), "row")
}

fn evidence_retrieval_quality(row: &Value, payload: &Value) -> Value {
    row.get("retrieval_quality")
        .or_else(|| row.get("web_tooling_retrieval_quality"))
        .or_else(|| payload.get("retrieval_quality"))
        .or_else(|| payload.pointer("/tool_pipeline/retrieval_quality"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn effective_evidence_quality(
    recomputed: &Value,
    embedded: Option<&Value>,
    payload_source: &str,
) -> (Value, &'static str) {
    let recomputed_has_observed_rows = u64_at(recomputed, &["row_sample_count"], 0) > 0
        || u64_at(recomputed, &["source_domain_count"], 0) > 0
        || bool_at(recomputed, &["evidence_pack_quality_present"], false);
    if payload_source == "row" && embedded.is_some() && !recomputed_has_observed_rows {
        return (embedded.cloned().unwrap_or_else(|| json!({})), "embedded");
    }
    (recomputed.clone(), "recomputed")
}

fn readiness_pass(evidence_quality: &Value) -> bool {
    bool_at(evidence_quality, &["source_quality_ready"], false)
        && bool_at(evidence_quality, &["claim_quality_ready"], false)
        && bool_at(evidence_quality, &["citation_renderability_ready"], false)
        && bool_at(evidence_quality, &["answerability_ready"], false)
        && bool_at(evidence_quality, &["evidence_packet_contract_ready"], false)
}

fn evidence_summary(cases: &[Value], pass_min: f64) -> Value {
    let total = cases.len() as u64;
    let ready = cases
        .iter()
        .filter(|case| bool_at(case, &["ready"], false))
        .count() as u64;
    let source_ready = count_ready(cases, "source_quality_ready");
    let source_authority_ready = count_ready(cases, "source_authority_ready");
    let source_authority_sensitive = count_ready(cases, "source_authority_sensitive");
    let claim_ready = count_ready(cases, "claim_quality_ready");
    let citation_ready = count_ready(cases, "citation_renderability_ready");
    let answerability_ready = count_ready(cases, "answerability_ready");
    let packet_ready = count_ready(cases, "evidence_packet_contract_ready");
    let bounded_ready = count_ready(cases, "bounded_answerability_ready");
    let pass_rate = ratio(ready, total);
    let weakest_gates = weakest_rows(
        total,
        &[
            ("source_quality_ready", source_ready),
            ("source_authority_ready", source_authority_ready),
            ("claim_quality_ready", claim_ready),
            ("citation_renderability_ready", citation_ready),
            ("answerability_ready", answerability_ready),
            ("evidence_packet_contract_ready", packet_ready),
            ("bounded_answerability_ready", bounded_ready),
        ],
    );
    json!({
        "cases": total,
        "ready_cases": ready,
        "pass_rate": pass_rate,
        "pass_min": pass_min,
        "ok": pass_rate >= pass_min,
        "readiness_rates": {
            "source_quality_ready": ratio(source_ready, total),
            "source_authority_sensitive": ratio(source_authority_sensitive, total),
            "source_authority_ready": ratio(source_authority_ready, total),
            "claim_quality_ready": ratio(claim_ready, total),
            "citation_renderability_ready": ratio(citation_ready, total),
            "answerability_ready": ratio(answerability_ready, total),
            "evidence_packet_contract_ready": ratio(packet_ready, total),
            "bounded_answerability_ready": ratio(bounded_ready, total)
        },
        "averages": {
            "evidence_item_count": average_u64(cases, "evidence_item_count"),
            "claim_count": average_u64(cases, "claim_count"),
            "source_domain_count": average_u64(cases, "source_domain_count"),
            "authority_grade_source_domain_count": average_u64(
                cases,
                "authority_grade_source_domain_count"
            ),
            "clean_evidence_rate": average_f64(cases, "clean_evidence_rate"),
            "low_quality_evidence_rate": average_f64(cases, "low_quality_evidence_rate"),
            "concrete_claim_rate": average_f64(cases, "concrete_claim_rate"),
            "citation_ready_claim_rate": average_f64(cases, "citation_ready_claim_rate"),
            "handoff_claim_count": average_u64(cases, "handoff_claim_count"),
            "handoff_concrete_claim_rate": average_f64(cases, "handoff_concrete_claim_rate"),
            "handoff_low_quality_claim_rate": average_f64(
                cases,
                "handoff_low_quality_claim_rate"
            ),
            "handoff_citation_ready_claim_rate": average_f64(
                cases,
                "handoff_citation_ready_claim_rate"
            )
        },
        "weakest_gates": weakest_gates,
        "plain_english": {
            "purpose": "Offline, provider-agnostic evidence-quality replay. This grades the evidence packet itself without calling search APIs, stealth browser, or final synthesis.",
            "ready_cases": "Cases where source quality, claim quality, citation renderability, answerability, and packet contract all pass.",
            "bounded_answerability_ready": "Stricter read on whether the packet has enough diverse, covered, citable evidence for a bounded answer.",
            "source_authority_ready": "For source-sensitive requests, the packet has enough authority-grade source diversity to support a bounded answer."
        }
    })
}

fn count_ready(cases: &[Value], field: &str) -> u64 {
    cases
        .iter()
        .filter(|case| {
            case.pointer(&format!("/effective_evidence_quality/{field}"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count() as u64
}

fn weakest_rows(total: u64, gates: &[(&str, u64)]) -> Vec<Value> {
    let mut rows: Vec<Value> = gates
        .iter()
        .map(|(gate, passed)| {
            json!({
                "gate": gate,
                "passed": passed,
                "total": total,
                "pass_rate": ratio(*passed, total)
            })
        })
        .collect();
    rows.sort_by(|left, right| {
        f64_at(left, &["pass_rate"], 0.0)
            .partial_cmp(&f64_at(right, &["pass_rate"], 0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn average_u64(cases: &[Value], field: &str) -> f64 {
    if cases.is_empty() {
        return 0.0;
    }
    let total: u64 = cases
        .iter()
        .map(|case| u64_at(case, &["effective_evidence_quality", field], 0))
        .sum();
    total as f64 / cases.len() as f64
}

fn average_f64(cases: &[Value], field: &str) -> f64 {
    if cases.is_empty() {
        return 0.0;
    }
    cases
        .iter()
        .map(|case| f64_at(case, &["effective_evidence_quality", field], 0.0))
        .sum::<f64>()
        / cases.len() as f64
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn evidence_markdown_report(report: &Value) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = String::new();
    out.push_str("# Evidence Quality Grade\n\n");
    out.push_str(&format!("- ok: `{}`\n", bool_at(report, &["ok"], false)));
    out.push_str(&format!(
        "- ready cases: `{}/{}`\n",
        u64_at(summary, &["ready_cases"], 0),
        u64_at(summary, &["cases"], 0)
    ));
    out.push_str(&format!(
        "- pass rate: `{:.3}`\n\n",
        f64_at(summary, &["pass_rate"], 0.0)
    ));
    out.push_str("## Handoff Quality\n\n");
    out.push_str(&format!(
        "- avg promoted claims/case: `{:.3}`\n",
        f64_at(summary, &["averages", "handoff_claim_count"], 0.0)
    ));
    out.push_str(&format!(
        "- promoted concrete claim rate: `{:.3}`\n",
        f64_at(summary, &["averages", "handoff_concrete_claim_rate"], 0.0)
    ));
    out.push_str(&format!(
        "- promoted low-quality claim rate: `{:.3}`\n",
        f64_at(
            summary,
            &["averages", "handoff_low_quality_claim_rate"],
            0.0
        )
    ));
    out.push_str(&format!(
        "- promoted citation-ready claim rate: `{:.3}`\n\n",
        f64_at(
            summary,
            &["averages", "handoff_citation_ready_claim_rate"],
            0.0
        )
    ));
    out.push_str("## Weakest Gates\n\n");
    for row in summary
        .get("weakest_gates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push_str(&format!(
            "- `{}`: `{}/{}` (`{:.3}`)\n",
            clean_text(&str_at(row, &["gate"], ""), 120),
            u64_at(row, &["passed"], 0),
            u64_at(row, &["total"], 0),
            f64_at(row, &["pass_rate"], 0.0)
        ));
    }
    out.push_str("\n## Cases\n\n");
    for case in report
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push_str(&format!(
            "- `{}`: ready=`{}` source=`{}` claim=`{}` handoff_clean=`{}` citation=`{}` answerable=`{}` packet=`{}`\n",
            clean_text(&str_at(case, &["case_id"], ""), 160),
            bool_at(case, &["ready"], false),
            bool_at(
                case,
                &["effective_evidence_quality", "source_quality_ready"],
                false
            ),
            bool_at(
                case,
                &["effective_evidence_quality", "claim_quality_ready"],
                false
            ),
            bool_at(
                case,
                &[
                    "effective_evidence_quality",
                    "handoff_claim_quality_ready"
                ],
                false
            ),
            bool_at(
                case,
                &["effective_evidence_quality", "citation_renderability_ready"],
                false
            ),
            bool_at(
                case,
                &["effective_evidence_quality", "answerability_ready"],
                false
            ),
            bool_at(
                case,
                &[
                    "effective_evidence_quality",
                    "evidence_packet_contract_ready"
                ],
                false
            )
        ));
    }
    out
}
