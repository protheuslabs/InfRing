use super::*;

pub(super) fn category_pass_rate_rows(rows: &[Value]) -> Vec<Value> {
    grouped_pass_rate_rows(rows, "category", |row| {
        vec![str_at(row, &["category"], "unknown")]
    })
}

pub(super) fn tag_pass_rate_rows(rows: &[Value]) -> Vec<Value> {
    grouped_pass_rate_rows(rows, "tag", |row| {
        let tags = string_array_at(row, &["tags"]);
        if tags.is_empty() {
            vec!["untagged".to_string()]
        } else {
            tags
        }
    })
}

pub(super) fn default_partial_path(out_path: &str) -> String {
    if let Some(prefix) = out_path.strip_suffix(".json") {
        format!("{prefix}.partial.json")
    } else {
        format!("{out_path}.partial.json")
    }
}

pub(super) fn default_progress_path(out_path: &str) -> String {
    if let Some(prefix) = out_path.strip_suffix(".json") {
        format!("{prefix}.progress.jsonl")
    } else {
        format!("{out_path}.progress.jsonl")
    }
}

pub(super) fn write_research_golden_progress(path: &str, event: Value) {
    if let Err(err) = append_jsonl(path, &[event]) {
        eprintln!("eval_runtime: failed to write research golden progress event: {err}");
    }
}

pub(super) fn write_partial_research_golden_report(
    path: &str,
    status: &str,
    live: bool,
    total_planned_cases: u64,
    rows: &[Value],
    setup_failures: &[String],
    latest_case: Option<&Value>,
) {
    let completed_cases = rows.len() as u64;
    let passed_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["pass"], false))
        .count() as u64;
    let transport_adjusted_passed_cases = rows
        .iter()
        .filter(|row| {
            bool_at(row, &["pass"], false) && !bool_at(row, &["transport_failure"], false)
        })
        .count() as u64;
    let excellent_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["excellent"], false))
        .count() as u64;
    let transport_failures = rows
        .iter()
        .filter(|row| bool_at(row, &["transport_failure"], false))
        .count() as u64;
    let total_score = rows
        .iter()
        .map(|row| u64_at(row, &["score"], 0))
        .fold(0_u64, u64::saturating_add);
    let report = json!({
        "type": "research_golden_partial_eval",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "status": status,
        "mode": if live { "live_dashboard" } else { "offline_responses" },
        "summary": {
            "cases_planned": total_planned_cases,
            "cases_completed": completed_cases,
            "passed_cases": passed_cases,
            "transport_adjusted_passed_cases": transport_adjusted_passed_cases,
            "excellent_cases": excellent_cases,
            "average_score_so_far": ratio(total_score, completed_cases),
            "research_success_rate_so_far": ratio(passed_cases, completed_cases),
            "transport_adjusted_research_success_rate_so_far": ratio(
                transport_adjusted_passed_cases,
                completed_cases.saturating_sub(transport_failures)
            ),
            "excellent_rate_so_far": ratio(excellent_cases, completed_cases),
            "transport_failures": transport_failures,
            "non_transport_cases_so_far": completed_cases.saturating_sub(transport_failures)
        },
        "setup_failures": setup_failures,
        "latest_case": latest_case.cloned(),
        "cases": rows
    });
    if let Err(err) = write_json(path, &report) {
        eprintln!("eval_runtime: failed to write research golden partial report: {err}");
    }
}

pub(super) fn grouped_pass_rate_rows<F>(
    rows: &[Value],
    key_name: &str,
    mut keys_for_row: F,
) -> Vec<Value>
where
    F: FnMut(&Value) -> Vec<String>,
{
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut passes: BTreeMap<String, u64> = BTreeMap::new();
    let mut excellent: BTreeMap<String, u64> = BTreeMap::new();
    for row in rows {
        for key in keys_for_row(row)
            .into_iter()
            .map(|raw| clean_text(&raw, 120))
            .filter(|raw| !raw.is_empty())
        {
            *totals.entry(key.clone()).or_insert(0) += 1;
            if bool_at(row, &["pass"], false) {
                *passes.entry(key.clone()).or_insert(0) += 1;
            }
            if bool_at(row, &["excellent"], false) {
                *excellent.entry(key.clone()).or_insert(0) += 1;
            }
        }
    }
    totals
        .into_iter()
        .map(|(key, total)| {
            let passed = *passes.get(&key).unwrap_or(&0);
            let excellent_count = *excellent.get(&key).unwrap_or(&0);
            let mut row = serde_json::Map::new();
            row.insert(key_name.to_string(), Value::String(key));
            row.insert("passed".to_string(), json!(passed));
            row.insert("excellent".to_string(), json!(excellent_count));
            row.insert("total".to_string(), json!(total));
            row.insert("pass_rate".to_string(), json!(ratio(passed, total)));
            row.insert(
                "excellent_rate".to_string(),
                json!(ratio(excellent_count, total)),
            );
            Value::Object(row)
        })
        .collect()
}
