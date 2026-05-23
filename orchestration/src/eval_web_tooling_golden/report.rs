use serde_json::{json, Value};

use super::super::eval_research_golden_utils::{str_at, u64_at};

pub(super) fn tooling_markdown_report(report: &Value) -> String {
    let summary = report.get("summary").cloned().unwrap_or_else(|| json!({}));
    let web = report
        .pointer("/measurement_split/web_tooling")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let top_layer = str_at(&web, &["operator_metrics", "top_layer"], "unknown");
    let top_failure = str_at(
        &web,
        &["operator_metrics", "top_first_failure", "name"],
        "unknown",
    );
    let mut output = format!(
        "# Web Tooling Golden\n\n- mode: {}\n- cases: {}\n- measured_cases: {}\n- success_rate: {:.3}\n- transport_adjusted_success_rate: {:.3}\n- measurement_adjusted_success_rate: {:.3}\n- transport_failures: {}\n- top_layer: {}\n- top_first_failure: {}\n",
        str_at(report, &["mode"], "unknown"),
        u64_at(&summary, &["cases"], 0),
        u64_at(&summary, &["measured_cases"], 0),
        summary
            .get("success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        summary
            .get("transport_adjusted_success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        summary
            .get("measurement_adjusted_success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        u64_at(&summary, &["transport_failures"], 0),
        top_layer,
        top_failure
    );
    output.push_str("\n## Case Readout\n\n");
    for case in report
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let case_id = str_at(case, &["case_id"], "unknown_case");
        let first_failed_gate = str_at(
            case,
            &["web_tool_gate_diagnostics", "first_failed_gate"],
            "none",
        );
        let boundary = str_at(
            case,
            &["web_tool_gate_diagnostics", "inferred_failure_boundary"],
            "unknown",
        );
        let retrieval_status = str_at(
            case,
            &["web_tool_gate_diagnostics", "retrieval_status"],
            "unknown",
        );
        let candidate_count = u64_at(case, &["web_tool_gate_diagnostics", "candidate_count"], 0);
        let evidence_count = u64_at(case, &["web_tool_gate_diagnostics", "evidence_count"], 0);
        let content_rich_candidate_count = u64_at(
            case,
            &["web_tool_gate_diagnostics", "content_rich_candidate_count"],
            0,
        );
        let transport_error = str_at(
            case,
            &["direct_tool_payload_diagnostics", "transport_error"],
            "",
        );
        let stderr = str_at(case, &["direct_tool_payload_diagnostics", "stderr"], "");
        output.push_str(&format!(
            "- `{case_id}`: first_failed=`{first_failed_gate}`, boundary=`{boundary}`, status=`{retrieval_status}`, candidates={candidate_count}, evidence={evidence_count}, content_rich={content_rich_candidate_count}"
        ));
        if !transport_error.is_empty() {
            output.push_str(&format!(", transport_error=`{transport_error}`"));
        }
        if !stderr.is_empty() {
            output.push_str(&format!(
                ", stderr=\"{}\"",
                clean_markdown_inline(&stderr, 180)
            ));
        }
        output.push('\n');
    }
    output
}

fn clean_markdown_inline(raw: &str, max_chars: usize) -> String {
    let mut out = raw.replace('\n', " ");
    if out.chars().count() > max_chars {
        out = out.chars().take(max_chars).collect::<String>();
        out.push_str("...");
    }
    out
}
