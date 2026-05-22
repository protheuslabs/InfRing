pub(super) fn record_web_retrieval_gate_counts(
    diagnostics: &Value,
    total_counts: &mut BTreeMap<String, u64>,
    pass_counts: &mut BTreeMap<String, u64>,
) {
    let Some(gates) = diagnostics.get("gates").and_then(Value::as_array) else {
        return;
    };
    for gate in gates {
        let Some(name) = gate.get("gate").and_then(Value::as_str) else {
            continue;
        };
        *total_counts.entry(name.to_string()).or_insert(0) += 1;
        if gate.get("status").and_then(Value::as_str) == Some("pass") {
            *pass_counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

pub(super) fn web_tooling_measurement_eligible_case(
    case: &Value,
    payload: &Value,
    retrieval_quality: &Value,
) -> bool {
    web_tooling_measurement_exclusion_reason_case(case, payload, retrieval_quality).is_none()
}

pub(super) fn web_tooling_measurement_exclusion_reason_case(
    case: &Value,
    payload: &Value,
    retrieval_quality: &Value,
) -> Option<&'static str> {
    if payload_is_transport_failure(payload) {
        return Some("transport_failure");
    }
    if unseeded_post_tool_synthesis_case(case, payload, retrieval_quality) {
        return Some("post_tool_context_not_seeded");
    }
    None
}

pub(super) fn web_retrieval_gate_rate_rows(
    total_counts: &BTreeMap<String, u64>,
    pass_counts: &BTreeMap<String, u64>,
) -> Vec<Value> {
    total_counts
        .iter()
        .map(|(gate, total)| {
            let passed = *pass_counts.get(gate).unwrap_or(&0);
            json!({
                "gate": gate,
                "passed": passed,
                "total": total,
                "pass_rate": ratio(passed, *total),
                "boundary": web_failure_boundary(gate)
            })
        })
        .collect()
}

pub(super) fn web_retrieval_gate_metric_rows(rows: &[Value], gate_rates: &[Value]) -> Vec<Value> {
    let measured_rows = web_tooling_measured_rows(rows);
    let measured_cases = measured_rows.len() as u64;
    let mut metrics = BTreeMap::<String, WebGateMetric>::new();
    for gate_rate in gate_rates {
        if let Some(gate) = gate_rate.get("gate").and_then(Value::as_str) {
            metrics.entry(gate.to_string()).or_default();
        }
    }

    for row in measured_rows {
        let first_failed_gate = row
            .pointer("/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str)
            .unwrap_or("");
        let Some(gates) = row
            .pointer("/web_tool_gate_diagnostics/gates")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for gate in gates {
            let Some(name) = gate.get("gate").and_then(Value::as_str) else {
                continue;
            };
            let metric = metrics.entry(name.to_string()).or_default();
            let artifact_present = gate
                .get("artifact_present")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let passed = gate.get("status").and_then(Value::as_str) == Some("pass");
            metric.total = metric.total.saturating_add(1);
            if artifact_present {
                metric.artifact_present = metric.artifact_present.saturating_add(1);
            } else {
                metric.artifact_missing = metric.artifact_missing.saturating_add(1);
            }
            if passed {
                metric.passed = metric.passed.saturating_add(1);
            } else {
                metric.failed = metric.failed.saturating_add(1);
                if artifact_present {
                    metric.artifact_present_failures =
                        metric.artifact_present_failures.saturating_add(1);
                } else {
                    metric.artifact_missing_failures =
                        metric.artifact_missing_failures.saturating_add(1);
                }
            }
            if first_failed_gate == name {
                metric.first_failure_count = metric.first_failure_count.saturating_add(1);
            }
        }
    }

    metrics
        .into_iter()
        .map(|(gate, metric)| {
            let pass_rate = ratio(metric.passed, metric.total);
            let fail_rate = ratio(metric.failed, metric.total);
            json!({
                "gate": gate,
                "boundary": web_failure_boundary(&gate),
                "measured_cases": measured_cases,
                "target_pass_rate": WEB_GATE_TARGET_PASS_RATE,
                "ok": pass_rate >= WEB_GATE_TARGET_PASS_RATE,
                "total": metric.total,
                "passed": metric.passed,
                "failed": metric.failed,
                "pass_rate": pass_rate,
                "fail_rate": fail_rate,
                "artifact_present": metric.artifact_present,
                "artifact_missing": metric.artifact_missing,
                "artifact_present_rate": ratio(metric.artifact_present, metric.total),
                "artifact_missing_rate": ratio(metric.artifact_missing, metric.total),
                "artifact_present_failures": metric.artifact_present_failures,
                "artifact_missing_failures": metric.artifact_missing_failures,
                "first_failure_count": metric.first_failure_count,
                "first_failure_rate": ratio(metric.first_failure_count, measured_cases)
            })
        })
        .collect()
}

