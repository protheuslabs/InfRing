use infring_orchestration_v1::contracts::{
    EvalCalibrationSnapshot, EvalQualityGateHistory, EvalQualityGatePolicy, EvalQualitySignalMode,
    EvalQualitySignalSnapshot,
};
use infring_orchestration_v1::eval::{
    evaluate_judge_human_agreement, evaluate_quality_gate, EvalJudgeHumanAgreementPolicy,
    EvalJudgeHumanComparableRow, EvalVerdict,
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../eval_action_economy_guard.rs"]
mod eval_action_economy_guard;
#[path = "../eval_agent_feedback.rs"]
mod eval_agent_feedback;
#[path = "../eval_agent_self_diagnosis.rs"]
mod eval_agent_self_diagnosis;
#[path = "../eval_authority_calibration.rs"]
mod eval_authority_calibration;
#[path = "../eval_calibration_stats.rs"]
mod eval_calibration_stats;
#[path = "../eval_contamination_guard.rs"]
mod eval_contamination_guard;
#[path = "../eval_evidence_quality_grade.rs"]
mod eval_evidence_quality_grade;
#[path = "../eval_feedback_router.rs"]
mod eval_feedback_router;
#[path = "../eval_final_runtime.rs"]
mod eval_final_runtime;
#[path = "../eval_grader_hacking.rs"]
mod eval_grader_hacking;
#[path = "../eval_issue_authority.rs"]
mod eval_issue_authority;
#[path = "../eval_issue_runtime.rs"]
mod eval_issue_runtime;
#[path = "../eval_learning_loop.rs"]
mod eval_learning_loop;
#[path = "../eval_learning_loop_policy.rs"]
mod eval_learning_loop_policy;
#[path = "../eval_learning_loop_policy_version.rs"]
mod eval_learning_loop_policy_version;
#[path = "../eval_learning_loop_review.rs"]
mod eval_learning_loop_review;
#[path = "../eval_learning_loop_rsi_handoff.rs"]
mod eval_learning_loop_rsi_handoff;
#[path = "../eval_lifecycle_runtime.rs"]
mod eval_lifecycle_runtime;
#[path = "../eval_metamorphic_guard.rs"]
mod eval_metamorphic_guard;
#[path = "../eval_multiturn_simulation.rs"]
mod eval_multiturn_simulation;
#[path = "../eval_production_workflow_guard.rs"]
mod eval_production_workflow_guard;
#[path = "../eval_research_gate_diagnostics.rs"]
mod eval_research_gate_diagnostics;
#[path = "../eval_research_golden.rs"]
mod eval_research_golden;
#[path = "../eval_research_golden_report.rs"]
mod eval_research_golden_report;
#[path = "../eval_research_golden_scoring.rs"]
mod eval_research_golden_scoring;
#[path = "../eval_research_golden_utils.rs"]
mod eval_research_golden_utils;
#[path = "../eval_research_perfect_evidence.rs"]
mod eval_research_perfect_evidence;
#[path = "../eval_rsi_promotion_guard.rs"]
mod eval_rsi_promotion_guard;
#[path = "../eval_synthetic_user_chat_harness.rs"]
mod eval_synthetic_user_chat_harness;
#[path = "../eval_trace_localization.rs"]
mod eval_trace_localization;
#[path = "../eval_trajectory_scoring.rs"]
mod eval_trajectory_scoring;
#[path = "../eval_web_retrieval_gate_diagnostics.rs"]
mod eval_web_retrieval_gate_diagnostics;
#[path = "../eval_web_tooling_golden/mod.rs"]
mod eval_web_tooling_golden;

const DEFAULT_QUALITY_PATH: &str = "artifacts/eval_quality_metrics_latest.json";
const DEFAULT_MONITOR_PATH: &str = "local/state/ops/eval_agent_chat_monitor/latest.json";
const DEFAULT_JUDGE_HUMAN_PATH: &str = "artifacts/eval_judge_human_agreement_latest.json";
const DEFAULT_HISTORY_PATH: &str = "local/state/ops/eval_quality_gate_v1/history.json";
const DEFAULT_QUALITY_OUT_PATH: &str = "core/local/artifacts/eval_quality_gate_v1_current.json";
const DEFAULT_QUALITY_OUT_LATEST_PATH: &str = "artifacts/eval_quality_gate_v1_latest.json";
const DEFAULT_QUALITY_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_QUALITY_GATE_V1_CURRENT.md";

const DEFAULT_FEEDBACK_PATH: &str =
    "local/state/ops/eval_agent_chat_monitor/reviewer_feedback.jsonl";
const DEFAULT_THRESHOLDS_PATH: &str = "validation/evals/fixtures/eval_quality_thresholds.json";
const DEFAULT_JUDGE_OUT_PATH: &str = "core/local/artifacts/eval_judge_human_agreement_current.json";
const DEFAULT_JUDGE_OUT_LATEST_PATH: &str = "artifacts/eval_judge_human_agreement_latest.json";
const DEFAULT_JUDGE_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_JUDGE_HUMAN_AGREEMENT_CURRENT.md";
const DEFAULT_REVIEWER_OUT_PATH: &str =
    "core/local/artifacts/eval_reviewer_feedback_weekly_current.json";
const DEFAULT_REVIEWER_OUT_LATEST_PATH: &str =
    "artifacts/eval_reviewer_feedback_weekly_latest.json";
const DEFAULT_REVIEWER_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_REVIEWER_FEEDBACK_WEEKLY_CURRENT.md";

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
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
    let Some(raw) = parse_flag(args, key) else {
        return default;
    };
    match raw.trim() {
        "1" | "true" | "TRUE" | "yes" | "on" => true,
        "0" | "false" | "FALSE" | "no" | "off" => false,
        _ => default,
    }
}

fn parse_u64_flag(args: &[String], key: &str, default: u64) -> u64 {
    parse_flag(args, key)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_f64_from_path(value: &Value, path: &[&str], default: f64) -> f64 {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default,
        };
    }
    cursor.as_f64().unwrap_or(default)
}

fn parse_u64_from_path(value: &Value, path: &[&str], default: u64) -> u64 {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default,
        };
    }
    cursor
        .as_u64()
        .or_else(|| {
            cursor
                .as_i64()
                .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
        })
        .unwrap_or(default)
}

fn parse_bool_from_path(value: &Value, path: &[&str], default: bool) -> bool {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default,
        };
    }
    cursor.as_bool().unwrap_or(default)
}

fn parse_string_from_path(value: &Value, path: &[&str], default: &str) -> String {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default.to_string(),
        };
    }
    cursor.as_str().unwrap_or(default).to_string()
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn read_jsonl(path: &str) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            serde_json::from_str::<Value>(trimmed).ok()
        })
        .collect()
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, format!("{payload}\n"))
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn print_structured(report: &Value) {
    if let Ok(serialized) = serde_json::to_string(report) {
        let _ = writeln!(io::stdout(), "{serialized}");
    }
}

fn as_eval_mode(raw: &str) -> EvalQualitySignalMode {
    match raw.trim().to_lowercase().as_str() {
        "scored" => EvalQualitySignalMode::Scored,
        "insufficient_signal" => EvalQualitySignalMode::InsufficientSignal,
        _ => EvalQualitySignalMode::Unknown,
    }
}

fn verdict_from_row(row: &Value, keys: &[&str]) -> Option<EvalVerdict> {
    for key in keys {
        if let Some(raw) = row.get(*key).and_then(|v| v.as_str()) {
            if let Some(verdict) = EvalVerdict::from_any(raw) {
                return Some(verdict);
            }
        }
    }
    None
}

fn feedback_category_from_row(row: &Value) -> String {
    for key in [
        "reviewer_outcome",
        "reviewer_status",
        "feedback_outcome",
        "verdict",
        "human_verdict",
        "human_label",
        "label",
    ] {
        let raw = row
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let category = match raw.as_str() {
            "accepted" | "accept" | "approved" | "correct" | "true_positive" | "tp" => "accepted",
            "rejected" | "reject" | "incorrect" | "false_positive" | "fp" => "rejected",
            "partial" | "partially_accepted" | "partially_correct" | "mixed" => "partial",
            "missed" | "false_negative" | "fn" => "missed",
            _ => "",
        };
        if !category.is_empty() {
            return category.to_string();
        }
    }
    "unknown".to_string()
}

fn string_field(row: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(raw) = row.get(*key).and_then(|v| v.as_str()) {
            if !raw.trim().is_empty() {
                return raw.trim().to_string();
            }
        }
    }
    String::new()
}

fn run_reviewer_feedback(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let feedback_path =
        parse_flag(args, "feedback").unwrap_or_else(|| DEFAULT_FEEDBACK_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_REVIEWER_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_REVIEWER_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_REVIEWER_MARKDOWN_PATH.to_string());
    let window_days = parse_u64_flag(args, "window-days", 7);
    let now_iso = now_iso_like();
    let rows = read_jsonl(&feedback_path);
    let previous = read_json(&out_latest_path);

    let mut accepted = 0_u64;
    let mut rejected = 0_u64;
    let mut partial = 0_u64;
    let mut missed = 0_u64;
    let mut unknown = 0_u64;
    let mut severity_checked = 0_u64;
    let mut severity_matches = 0_u64;
    let mut normalized_rows = Vec::new();

    for row in &rows {
        if !row.is_object() {
            unknown = unknown.saturating_add(1);
            continue;
        }
        let category = feedback_category_from_row(row);
        match category.as_str() {
            "accepted" => accepted = accepted.saturating_add(1),
            "rejected" => rejected = rejected.saturating_add(1),
            "partial" => partial = partial.saturating_add(1),
            "missed" => missed = missed.saturating_add(1),
            _ => unknown = unknown.saturating_add(1),
        }
        let judge_severity =
            string_field(row, &["severity", "judge_severity", "predicted_severity"]);
        let reviewer_severity = string_field(row, &["reviewer_severity", "human_severity"]);
        let severity_match = if judge_severity.is_empty() || reviewer_severity.is_empty() {
            None
        } else {
            severity_checked = severity_checked.saturating_add(1);
            let matches = judge_severity.eq_ignore_ascii_case(&reviewer_severity);
            if matches {
                severity_matches = severity_matches.saturating_add(1);
            }
            Some(matches)
        };
        normalized_rows.push(json!({
            "ts": string_field(row, &["ts", "timestamp"]),
            "issue_id": string_field(row, &["issue_id", "id"]),
            "reviewer_outcome": category,
            "judge_verdict": string_field(row, &["judge_verdict", "llm_verdict", "model_verdict", "predicted_verdict"]),
            "human_verdict": string_field(row, &["human_verdict", "reviewer_verdict", "verdict"]),
            "severity": judge_severity,
            "reviewer_severity": reviewer_severity,
            "severity_match": severity_match,
            "note": string_field(row, &["note", "comment", "rationale"])
        }));
    }

    let predicted_positive = accepted.saturating_add(rejected).saturating_add(partial);
    let actual_positive = accepted.saturating_add(partial).saturating_add(missed);
    let precision = if predicted_positive == 0 {
        0.0
    } else {
        accepted as f64 / predicted_positive as f64
    };
    let recall = if actual_positive == 0 {
        0.0
    } else {
        accepted as f64 / actual_positive as f64
    };
    let severity_calibration = if severity_checked == 0 {
        0.0
    } else {
        severity_matches as f64 / severity_checked as f64
    };
    let previous_precision = parse_f64_from_path(
        &previous,
        &["summary", "calibration", "precision"],
        precision,
    );
    let previous_recall =
        parse_f64_from_path(&previous, &["summary", "calibration", "recall"], recall);
    let previous_severity = parse_f64_from_path(
        &previous,
        &["summary", "calibration", "severity_calibration"],
        severity_calibration,
    );
    let rows_len = rows.len() as u64;
    let status = if rows.is_empty() {
        "awaiting_feedback"
    } else if unknown > 0 {
        "needs_triage"
    } else {
        "active"
    };
    let checks = vec![
        json!({
            "id": "feedback_ingestion_contract",
            "ok": Path::new(&feedback_path).exists() || rows.is_empty(),
            "detail": format!("rows={};path={}", rows_len, feedback_path),
        }),
        json!({
            "id": "reviewer_outcome_category_contract",
            "ok": unknown == 0,
            "detail": format!(
                "accepted={};rejected={};partial={};missed={};unknown={}",
                accepted, rejected, partial, missed, unknown
            ),
        }),
        json!({
            "id": "reviewer_calibration_metrics_contract",
            "ok": true,
            "detail": format!(
                "precision={:.3};recall={:.3};false_positive={};false_negative={};severity_calibration={:.3}",
                precision, recall, rejected, missed, severity_calibration
            ),
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    let report = json!({
        "type": "eval_reviewer_feedback_weekly_report",
        "schema_version": 2,
        "generated_at": now_iso,
        "ok": ok,
        "checks": checks,
        "summary": {
            "window_days": window_days,
            "feedback_rows": rows_len,
            "status": status,
            "verdict_counts": {
                "accepted": accepted,
                "rejected": rejected,
                "partial": partial,
                "missed": missed,
                "unknown": unknown
            },
            "calibration": {
                "precision": precision,
                "recall": recall,
                "false_positives": rejected,
                "false_negatives": missed,
                "partial_findings": partial,
                "severity_calibration": severity_calibration,
                "severity_checked": severity_checked,
                "precision_delta_vs_previous": precision - previous_precision,
                "recall_delta_vs_previous": recall - previous_recall,
                "severity_calibration_delta_vs_previous": severity_calibration - previous_severity
            }
        },
        "feedback_rows": normalized_rows,
        "sources": {
            "feedback": feedback_path
        }
    });
    let markdown = format!(
        "# Eval Reviewer Feedback Weekly Report (Current)\n\n- generated_at: {}\n- ok: {}\n- status: {}\n- feedback_rows: {}\n- accepted: {}\n- rejected: {}\n- partial: {}\n- missed: {}\n- precision: {:.3}\n- recall: {:.3}\n- false_positives: {}\n- false_negatives: {}\n- severity_calibration: {:.3}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        status,
        rows_len,
        accepted,
        rejected,
        partial,
        missed,
        precision,
        recall,
        rejected,
        missed,
        severity_calibration
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write reviewer feedback outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn run_quality_gate(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let quality_path =
        parse_flag(args, "quality").unwrap_or_else(|| DEFAULT_QUALITY_PATH.to_string());
    let monitor_path =
        parse_flag(args, "monitor").unwrap_or_else(|| DEFAULT_MONITOR_PATH.to_string());
    let judge_human_path =
        parse_flag(args, "judge-human").unwrap_or_else(|| DEFAULT_JUDGE_HUMAN_PATH.to_string());
    let history_path =
        parse_flag(args, "history").unwrap_or_else(|| DEFAULT_HISTORY_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_QUALITY_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_QUALITY_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_QUALITY_MARKDOWN_PATH.to_string());
    let required_consecutive_passes = parse_u64_flag(args, "required-consecutive-passes", 3).max(1);
    let now_iso = now_iso_like();

    let quality = read_json(&quality_path);
    let monitor = read_json(&monitor_path);
    let judge_human = read_json(&judge_human_path);
    let history = read_json(&history_path);

    let evaluation_mode_raw =
        parse_string_from_path(&quality, &["metrics", "evaluation_mode"], "unknown");
    let signal_snapshot = EvalQualitySignalSnapshot {
        quality_ok: parse_bool_from_path(&quality, &["ok"], false),
        monitor_ok: parse_bool_from_path(&monitor, &["ok"], false),
        evaluation_mode: as_eval_mode(&evaluation_mode_raw),
        predicted_non_info_samples: parse_u64_from_path(
            &quality,
            &["metrics", "predicted_non_info_samples"],
            0,
        ),
        minimum_eval_samples: parse_u64_from_path(
            &quality,
            &["metrics", "minimum_eval_samples"],
            0,
        ),
    };
    let calibration_snapshot = EvalCalibrationSnapshot {
        calibration_ready: parse_bool_from_path(
            &judge_human,
            &["summary", "calibration_ready"],
            false,
        ),
        status: parse_string_from_path(&judge_human, &["summary", "status"], "missing"),
        agreement_rate: parse_f64_from_path(&judge_human, &["summary", "agreement_rate"], 0.0),
        agreement_min: parse_f64_from_path(&judge_human, &["summary", "agreement_min"], 0.0),
        comparable_samples: parse_u64_from_path(
            &judge_human,
            &["summary", "comparable_samples"],
            0,
        ),
        minimum_samples: parse_u64_from_path(&judge_human, &["summary", "minimum_samples"], 0),
    };
    let history_snapshot = EvalQualityGateHistory {
        consecutive_passes: parse_u64_from_path(&history, &["consecutive_passes"], 0),
    };
    let policy = EvalQualityGatePolicy {
        required_consecutive_passes,
    };
    let gate_state = evaluate_quality_gate(
        &signal_snapshot,
        &calibration_snapshot,
        &history_snapshot,
        &policy,
    );
    let sample_deficit = signal_snapshot
        .minimum_eval_samples
        .saturating_sub(signal_snapshot.predicted_non_info_samples);
    let block_reasons = gate_block_reasons(&signal_snapshot, &calibration_snapshot, &gate_state);
    let block_reason_text = block_reasons.join(", ");
    let required_actions: Vec<String> = block_reasons
        .iter()
        .map(|reason| match reason.as_str() {
            "quality_signal_insufficient" => format!(
                "collect at least {} additional non-informational eval samples and rerun eval quality metrics",
                sample_deficit
            ),
            "consecutive_clean_passes_below_required" => format!(
                "run {} more clean eval quality gate pass(es) before allowing autonomous escalation",
                gate_state.remaining_to_unlock
            ),
            "quality_thresholds_failed" => {
                "repair eval quality threshold violations before using eval as a release gate".to_string()
            }
            "monitor_not_ok" => {
                "repair eval chat monitor ingestion before trusting eval issue synthesis".to_string()
            }
            "judge_human_calibration_not_ready" => {
                "collect or repair judge-human calibration samples before promoting eval autonomy".to_string()
            }
            _ => format!("review eval quality gate block reason: {reason}"),
        })
        .collect();
    let required_action_text = if required_actions.is_empty() {
        "none".to_string()
    } else {
        required_actions.join("; ")
    };

    let checks = vec![
        json!({
            "id": "quality_artifact_present",
            "ok": Path::new(&quality_path).exists(),
            "detail": quality_path,
        }),
        json!({
            "id": "monitor_artifact_present",
            "ok": Path::new(&monitor_path).exists(),
            "detail": monitor_path,
        }),
        json!({
            "id": "judge_human_artifact_present",
            "ok": Path::new(&judge_human_path).exists(),
            "detail": judge_human_path,
        }),
        json!({
            "id": "quality_threshold_contract",
            "ok": signal_snapshot.quality_ok,
            "detail": format!("quality_ok={}", signal_snapshot.quality_ok),
        }),
        json!({
            "id": "monitor_contract",
            "ok": signal_snapshot.monitor_ok,
            "detail": format!("monitor_ok={}", signal_snapshot.monitor_ok),
        }),
        json!({
            "id": "quality_signal_ready_contract",
            "ok": gate_state.quality_signal_sufficient,
            "detail": format!(
                "evaluation_mode={};predicted_non_info={};minimum_eval_samples={}",
                evaluation_mode_raw,
                signal_snapshot.predicted_non_info_samples,
                signal_snapshot.minimum_eval_samples
            ),
        }),
        json!({
            "id": "judge_human_calibration_contract",
            "ok": calibration_snapshot.calibration_ready,
            "detail": format!(
                "status={};agreement_rate={:.3};comparable_samples={}",
                calibration_snapshot.status,
                calibration_snapshot.agreement_rate,
                calibration_snapshot.comparable_samples
            ),
        }),
        json!({
            "id": "consecutive_pass_tracking_contract",
            "ok": true,
            "detail": format!(
                "consecutive={};required={};remaining={};soft_blocked={}",
                gate_state.consecutive_passes,
                gate_state.required_consecutive_passes,
                gate_state.remaining_to_unlock,
                gate_state.soft_blocked
            ),
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    let release_gate_status = if gate_state.autonomous_escalation_allowed {
        "autonomous_escalation_allowed"
    } else if gate_state.current_pass {
        "pass_collecting_consecutive_history"
    } else {
        "blocked"
    };
    let issue_candidate = if ok {
        json!(null)
    } else {
        json!({
            "type": "eval_quality_gate_issue_candidate",
            "schema_version": 1,
            "generated_at": now_iso.clone(),
            "status": "candidate",
            "source": "eval_quality_gate_v1",
            "fingerprint": format!("eval_quality_gate_v1:{}", block_reasons.join("|")),
            "dedupe_key": format!("eval_quality_gate_v1:{}", block_reasons.join("|")),
            "owner": "orchestration/eval",
            "route_to": "eval_issue_backlog",
            "title": "Eval quality gate is not ready for autonomous escalation",
            "severity": if gate_state.current_pass { "medium" } else { "high" },
            "labels": ["eval", "release-gate", "rsi-readiness"],
            "impact": "eval cannot safely drive RSI promotion or autonomous escalation until this gate unlocks",
            "block_reasons": block_reasons.clone(),
            "required_actions": required_actions.clone(),
            "source_artifacts": [quality_path.clone(), monitor_path.clone(), judge_human_path.clone()],
            "triage": {
                "state": "ready_for_issue_synthesis",
                "safe_to_auto_file_issue": true,
                "safe_to_auto_apply_patch": false,
                "requires_eval_authority_receipt_to_close": true
            },
            "automation_policy": {
                "mode": "proposal_only",
                "requires_operator_or_eval_authority_receipt_before_apply": true,
                "autonomous_escalation_allowed": false
            },
            "quality_evaluation_mode": evaluation_mode_raw,
            "sample_deficit": sample_deficit,
            "remaining_to_unlock": gate_state.remaining_to_unlock,
            "acceptance_criteria": [
                "quality_signal_sufficient is true",
                "judge-human calibration is ready",
                "required consecutive clean passes have been reached",
                "autonomous_escalation_allowed is true"
            ]
        })
    };

    let report = json!({
        "type": "eval_quality_gate_v1",
        "schema_version": 2,
        "generated_at": now_iso,
        "ok": ok,
        "checks": checks,
        "block_reasons": block_reasons.clone(),
        "summary": {
            "quality_ok": signal_snapshot.quality_ok,
            "monitor_ok": signal_snapshot.monitor_ok,
            "quality_signal_sufficient": gate_state.quality_signal_sufficient,
            "quality_evaluation_mode": evaluation_mode_raw,
            "predicted_non_info_samples": signal_snapshot.predicted_non_info_samples,
            "minimum_eval_samples": signal_snapshot.minimum_eval_samples,
            "sample_deficit": sample_deficit,
            "calibration_ready": calibration_snapshot.calibration_ready,
            "calibration_status": calibration_snapshot.status,
            "calibration_agreement_rate": calibration_snapshot.agreement_rate,
            "calibration_comparable_samples": calibration_snapshot.comparable_samples,
            "current_pass": gate_state.current_pass,
            "consecutive_passes": gate_state.consecutive_passes,
            "required_consecutive_passes": gate_state.required_consecutive_passes,
            "autonomous_escalation_allowed": gate_state.autonomous_escalation_allowed,
            "rsi_promotion_blocked": !gate_state.autonomous_escalation_allowed,
            "remaining_to_unlock": gate_state.remaining_to_unlock,
            "required_actions": required_actions.clone(),
        },
        "operator_summary": {
            "status": release_gate_status,
            "blocker_count": block_reasons.len(),
            "primary_blocker": block_reasons.first().cloned().unwrap_or_else(|| "none".to_string()),
            "block_reasons": block_reasons.clone(),
            "required_actions": required_actions.clone(),
            "issue_candidate_ready": !ok
        },
        "issue_candidate": issue_candidate,
        "issue_candidate_contract": {
            "candidate_schema_version": 1,
            "safe_to_auto_file_issue": true,
            "safe_to_auto_apply_patch": false,
            "authority_receipt_required_to_close": true
        },
        "release_gate_contract": {
            "pass_fail_authoritative": true,
            "autonomous_escalation_requires_quality_signal": true,
            "autonomous_escalation_requires_calibration": true,
            "autonomous_escalation_requires_consecutive_passes": true
        },
        "unlock_contract": {
            "remaining_to_unlock": gate_state.remaining_to_unlock,
            "sample_deficit": sample_deficit,
            "required_actions": required_actions.clone(),
            "issue_candidate_ready": !ok,
            "operator_can_unlock_by_following_required_actions": true
        },
        "sources": {
            "quality": quality_path,
            "monitor": monitor_path,
            "judge_human": judge_human_path,
            "history": history_path,
        }
    });

    let history_out = json!({
        "type": "eval_quality_gate_v1_history",
        "updated_at": now_iso,
        "consecutive_passes": gate_state.consecutive_passes,
        "required_consecutive_passes": gate_state.required_consecutive_passes,
        "autonomous_escalation_allowed": gate_state.autonomous_escalation_allowed,
        "last_soft_blocked": gate_state.soft_blocked,
        "last_result_ok": ok,
        "last_block_reasons": block_reasons.clone(),
        "last_required_actions": required_actions.clone(),
        "last_sample_deficit": sample_deficit,
    });
    let markdown = format!(
        "# Eval Quality Gate v1 (Current)\n\n- generated_at: {}\n- ok: {}\n- quality_ok: {}\n- monitor_ok: {}\n- quality_signal_sufficient: {}\n- quality_evaluation_mode: {}\n- predicted_non_info_samples: {}\n- minimum_eval_samples: {}\n- sample_deficit: {}\n- calibration_ready: {}\n- calibration_status: {}\n- consecutive_passes: {}\n- required_consecutive_passes: {}\n- autonomous_escalation_allowed: {}\n- block_reasons: {}\n- required_actions: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        signal_snapshot.quality_ok,
        signal_snapshot.monitor_ok,
        gate_state.quality_signal_sufficient,
        evaluation_mode_raw,
        signal_snapshot.predicted_non_info_samples,
        signal_snapshot.minimum_eval_samples,
        sample_deficit,
        calibration_snapshot.calibration_ready,
        calibration_snapshot.status,
        gate_state.consecutive_passes,
        gate_state.required_consecutive_passes,
        gate_state.autonomous_escalation_allowed,
        block_reason_text,
        required_action_text
    );

    let write_ok = write_json(&history_path, &history_out).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more quality-gate outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn gate_block_reasons(
    signal_snapshot: &EvalQualitySignalSnapshot,
    calibration_snapshot: &EvalCalibrationSnapshot,
    gate_state: &infring_orchestration_v1::contracts::EvalQualityGateState,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !signal_snapshot.quality_ok {
        reasons.push("quality_metrics_not_passing".to_string());
    }
    if !signal_snapshot.monitor_ok {
        reasons.push("eval_monitor_not_passing".to_string());
    }
    if !gate_state.quality_signal_sufficient {
        reasons.push("quality_signal_insufficient".to_string());
    }
    if !calibration_snapshot.calibration_ready {
        reasons.push("human_calibration_not_ready".to_string());
    }
    if gate_state.remaining_to_unlock > 0 {
        reasons.push("consecutive_clean_passes_below_required".to_string());
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn run_judge_human_agreement(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let feedback_path =
        parse_flag(args, "feedback").unwrap_or_else(|| DEFAULT_FEEDBACK_PATH.to_string());
    let thresholds_path =
        parse_flag(args, "thresholds").unwrap_or_else(|| DEFAULT_THRESHOLDS_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_JUDGE_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_JUDGE_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_JUDGE_MARKDOWN_PATH.to_string());
    let window_days = parse_u64_flag(args, "window-days", 7);
    let now_iso = now_iso_like();

    let thresholds = read_json(&thresholds_path);
    let minimum_samples =
        parse_u64_from_path(&thresholds, &["global", "judge_human_min_samples"], 5);
    let agreement_min =
        parse_f64_from_path(&thresholds, &["global", "judge_human_agreement_min"], 0.7);
    let calibration_ci_max_half_width = parse_f64_from_path(
        &thresholds,
        &["global", "judge_human_ci_max_half_width"],
        0.45,
    );
    let calibration_sensitivity_min =
        parse_f64_from_path(&thresholds, &["global", "judge_human_sensitivity_min"], 0.6);
    let calibration_specificity_min =
        parse_f64_from_path(&thresholds, &["global", "judge_human_specificity_min"], 0.6);
    let policy = EvalJudgeHumanAgreementPolicy {
        minimum_samples,
        agreement_min,
    };

    let rows = read_jsonl(&feedback_path);
    let mut malformed_rows = 0_u64;
    let mut comparable_rows = Vec::new();
    for row in rows.iter() {
        if !row.is_object() {
            malformed_rows = malformed_rows.saturating_add(1);
            continue;
        }
        let human = verdict_from_row(
            row,
            &[
                "human_verdict",
                "reviewer_verdict",
                "verdict",
                "human_label",
                "label",
            ],
        );
        let judge = verdict_from_row(
            row,
            &[
                "judge_verdict",
                "llm_verdict",
                "model_verdict",
                "predicted_verdict",
                "auto_verdict",
                "system_verdict",
            ],
        );
        let (Some(human_verdict), Some(judge_verdict)) = (human, judge) else {
            continue;
        };
        comparable_rows.push(EvalJudgeHumanComparableRow {
            ts: row
                .get("ts")
                .and_then(|v| v.as_str())
                .or_else(|| row.get("timestamp").and_then(|v| v.as_str()))
                .map(|v| v.to_string()),
            issue_id: row
                .get("issue_id")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
            human_verdict,
            judge_verdict,
            note: row
                .get("note")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
        });
    }

    let result = evaluate_judge_human_agreement(&comparable_rows, &policy);
    let calibration_stats = eval_calibration_stats::judge_calibration_stats(
        &comparable_rows,
        eval_calibration_stats::EvalCalibrationStatsPolicy {
            minimum_samples,
            max_ci_half_width: calibration_ci_max_half_width,
            sensitivity_min: calibration_sensitivity_min,
            specificity_min: calibration_specificity_min,
        },
    );
    let calibration_promotion_ready = parse_bool_from_path(
        &calibration_stats,
        &["adaptive_sample_policy", "promotion_ready"],
        false,
    );
    let calibration_promotion_blocked = parse_bool_from_path(
        &calibration_stats,
        &["adaptive_sample_policy", "promotion_blocked"],
        true,
    );
    let widest_ci_half_width = parse_f64_from_path(
        &calibration_stats,
        &["confidence_intervals", "widest_wilson_95_half_width"],
        1.0,
    );
    let checks = vec![
        json!({
            "id": "feedback_rows_present",
            "ok": Path::new(&feedback_path).exists(),
            "detail": feedback_path,
        }),
        json!({
            "id": "thresholds_present",
            "ok": Path::new(&thresholds_path).exists(),
            "detail": thresholds_path,
        }),
        json!({
            "id": "judge_human_signal_coverage_contract",
            "ok": true,
            "detail": format!(
                "status={};comparable_samples={};minimum_samples={};window_days={}",
                result.summary.status, result.summary.comparable_samples, result.summary.minimum_samples, window_days
            ),
        }),
        json!({
            "id": "judge_human_agreement_threshold_contract",
            "ok": if result.summary.status == "insufficient_signal" { true } else { result.summary.agreement_rate >= result.summary.agreement_min },
            "detail": format!(
                "agreement_rate={:.3};agreement_min={:.3}",
                result.summary.agreement_rate, result.summary.agreement_min
            ),
        }),
        json!({
            "id": "judge_human_sensitivity_specificity_contract",
            "ok": calibration_stats.get("confusion").is_some(),
            "detail": format!(
                "sensitivity={:.3};sensitivity_min={:.3};specificity={:.3};specificity_min={:.3}",
                parse_f64_from_path(&calibration_stats, &["rates", "sensitivity"], 0.0),
                calibration_sensitivity_min,
                parse_f64_from_path(&calibration_stats, &["rates", "specificity"], 0.0),
                calibration_specificity_min
            ),
        }),
        json!({
            "id": "judge_human_confidence_interval_contract",
            "ok": calibration_stats.get("confidence_intervals").is_some(),
            "detail": format!(
                "widest_wilson_95_half_width={:.3};max_ci_half_width={:.3}",
                widest_ci_half_width, calibration_ci_max_half_width
            ),
        }),
        json!({
            "id": "judge_human_adaptive_sample_policy_contract",
            "ok": calibration_stats.get("adaptive_sample_policy").is_some(),
            "detail": format!(
                "promotion_ready={};promotion_blocked={}",
                calibration_promotion_ready, calibration_promotion_blocked
            ),
        }),
        json!({
            "id": "feedback_shape_contract",
            "ok": malformed_rows == 0,
            "detail": format!("rows={};malformed={}", rows.len(), malformed_rows),
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

    let report = json!({
        "type": "eval_judge_human_agreement_guard",
        "schema_version": 2,
        "generated_at": now_iso,
        "ok": ok,
        "checks": checks,
        "summary": {
            "window_days": window_days,
            "feedback_rows_total": rows.len(),
            "feedback_rows_window": rows.len(),
            "comparable_samples": result.summary.comparable_samples,
            "minimum_samples": result.summary.minimum_samples,
            "agreement_rate": result.summary.agreement_rate,
            "agreement_min": result.summary.agreement_min,
            "calibration_ready": result.summary.calibration_ready,
            "calibration_promotion_ready": calibration_promotion_ready,
            "statistical_promotion_blocked": calibration_promotion_blocked,
            "widest_wilson_95_half_width": widest_ci_half_width,
            "status": result.summary.status,
            "pair_counts": result.summary.pair_counts,
        },
        "calibration_statistics": calibration_stats,
        "comparisons": comparable_rows,
        "sources": {
            "feedback": feedback_path,
            "thresholds": thresholds_path,
        }
    });
    let markdown = format!(
        "# Eval Judge-Human Agreement Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- status: {}\n- agreement_rate: {:.3}\n- agreement_min: {:.3}\n- comparable_samples: {}\n- minimum_samples: {}\n- calibration_ready: {}\n- calibration_promotion_ready: {}\n- statistical_promotion_blocked: {}\n- widest_wilson_95_half_width: {:.3}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        result.summary.status,
        result.summary.agreement_rate,
        result.summary.agreement_min,
        result.summary.comparable_samples,
        result.summary.minimum_samples,
        result.summary.calibration_ready,
        calibration_promotion_ready,
        calibration_promotion_blocked,
        widest_ci_half_width
    );

    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more judge-human outputs");
        return 2;
    }

    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn usage() {
    eprintln!(
        "usage: cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- <reviewer-feedback|quality-gate|judge-human-agreement|authority-calibration|feedback-router|agent-feedback|agent-self-diagnosis|issue-authority|grader-hacking-guard|trace-localization-guard|trajectory-scoring-guard|multiturn-simulation-guard|synthetic-user-chat-harness|research-golden|research-perfect-evidence|evidence-quality-grade|web-tooling-golden|misty-live-health-gate|contamination-guard|action-economy-guard|production-workflow-guard|learning-loop-ingest|learning-loop-issues|learning-loop-review|learning-loop-policy|learning-loop-version|learning-loop-rsi-handoff|metamorphic-guard|rsi-promotion-ladder|issue-drafts|replay|fix-verification|issue-lifecycle|rsi-escalation|phase-trace-persist|adversarial-routing|workflow-selection|runtime-ownership> [--strict=0|1] [args...]"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((command, tail)) = args.split_first() else {
        usage();
        return ExitCode::from(2);
    };
    let status = match command.as_str() {
        "reviewer-feedback" => run_reviewer_feedback(tail),
        "quality-gate" => run_quality_gate(tail),
        "judge-human-agreement" => run_judge_human_agreement(tail),
        "authority-calibration" => eval_authority_calibration::run_eval_authority_calibration(tail),
        "feedback-router" => eval_feedback_router::run_eval_feedback_router(tail),
        "agent-feedback" => eval_agent_feedback::run_eval_agent_feedback(tail),
        "agent-self-diagnosis" => eval_agent_self_diagnosis::run_eval_agent_self_diagnosis(tail),
        "issue-authority" => eval_issue_authority::run_eval_issue_authority(tail),
        "grader-hacking-guard" => eval_grader_hacking::run_grader_hacking_guard(tail),
        "trace-localization-guard" => eval_trace_localization::run_trace_localization_guard(tail),
        "trajectory-scoring-guard" => eval_trajectory_scoring::run_trajectory_scoring_guard(tail),
        "multiturn-simulation-guard" => {
            eval_multiturn_simulation::run_multiturn_simulation_guard(tail)
        }
        "synthetic-user-chat-harness" => {
            eval_synthetic_user_chat_harness::run_synthetic_user_chat_harness(tail)
        }
        "research-golden" => eval_research_golden::run_research_golden(tail),
        "research-perfect-evidence" => {
            eval_research_perfect_evidence::run_research_perfect_evidence(tail)
        }
        "evidence-quality-grade" => eval_evidence_quality_grade::run_evidence_quality_grade(tail),
        "web-tooling-golden" => eval_web_tooling_golden::run_web_tooling_golden(tail),
        "misty-live-health-gate" => {
            eval_synthetic_user_chat_harness::run_misty_live_health_gate(tail)
        }
        "contamination-guard" => eval_contamination_guard::run_contamination_guard(tail),
        "action-economy-guard" => eval_action_economy_guard::run_action_economy_guard(tail),
        "production-workflow-guard" => {
            eval_production_workflow_guard::run_production_workflow_guard(tail)
        }
        "learning-loop-ingest" => eval_learning_loop::run_eval_learning_loop_ingest(tail),
        "learning-loop-issues" => eval_learning_loop::run_eval_learning_loop_issue_candidates(tail),
        "learning-loop-review" => eval_learning_loop_review::run_eval_learning_loop_review(tail),
        "learning-loop-policy" => eval_learning_loop_policy::run_eval_learning_loop_policy(tail),
        "learning-loop-version" => {
            eval_learning_loop_policy_version::run_eval_learning_loop_policy_version(tail)
        }
        "learning-loop-rsi-handoff" => {
            eval_learning_loop_rsi_handoff::run_eval_learning_loop_rsi_handoff(tail)
        }
        "metamorphic-guard" => eval_metamorphic_guard::run_metamorphic_guard(tail),
        "rsi-promotion-ladder" => eval_rsi_promotion_guard::run_rsi_promotion_ladder_guard(tail),
        "issue-drafts" => eval_issue_runtime::run_issue_drafts(tail),
        "replay" => eval_issue_runtime::run_replay(tail),
        "fix-verification" => eval_lifecycle_runtime::run_fix_verification(tail),
        "issue-lifecycle" => eval_lifecycle_runtime::run_issue_lifecycle(tail),
        "rsi-escalation" => eval_lifecycle_runtime::run_rsi_escalation(tail),
        "phase-trace-persist" => eval_final_runtime::run_phase_trace_persist(tail),
        "adversarial-routing" => eval_final_runtime::run_adversarial_routing(tail),
        "workflow-selection" => eval_final_runtime::run_workflow_selection(tail),
        "runtime-ownership" => eval_final_runtime::run_runtime_ownership(tail),
        _ => {
            usage();
            2
        }
    };
    if status == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(status as u8)
    }
}
