use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeCodingUsefulWorkSeedReport {
    pub harness_kind: String,
    pub ok: bool,
    pub batch_root: String,
    pub seed_started_at_unix_ms: Option<u128>,
    pub attempt_count: usize,
    pub jobs: Vec<NativeCodingUsefulWorkJob>,
    pub failures: Vec<String>,
    pub operator_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeCodingUsefulWorkJob {
    pub attempt_id: String,
    pub task_id: String,
    pub package: String,
    pub run_root: String,
    pub project_root: String,
    pub prompt_path: String,
    pub validation_command: String,
    pub semantic_probe_command: Vec<String>,
    pub baseline_test_count: usize,
    pub expected_symbols: Vec<String>,
    pub seed_completed_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeCodingUsefulWorkJudgeReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub timing: NativeCodingUsefulWorkTimingSummary,
    pub attempts: Vec<NativeCodingUsefulWorkAttemptJudge>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeCodingUsefulWorkAttemptJudge {
    pub attempt_id: String,
    pub task_id: String,
    pub ok: bool,
    pub timing: NativeCodingUsefulWorkAttemptTiming,
    pub checks: Vec<NativeCodingUsefulWorkCheck>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeCodingUsefulWorkTimingSummary {
    pub batch_started_at_unix_ms: Option<u128>,
    pub judged_at_unix_ms: u128,
    pub batch_elapsed_ms: Option<u128>,
    pub first_attempt_completed_at_unix_ms: Option<u128>,
    pub last_attempt_completed_at_unix_ms: Option<u128>,
    pub completion_span_ms: Option<u128>,
    pub average_attempt_elapsed_ms: Option<u128>,
    pub average_time_to_first_mutation_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeCodingUsefulWorkAttemptTiming {
    pub completed_at_unix_ms: Option<u128>,
    pub elapsed_ms_since_batch_start: Option<u128>,
    pub first_mutation_unix_ms: Option<u128>,
    pub time_to_first_mutation_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeCodingUsefulWorkCheck {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct UsefulWorkCase {
    task_id: &'static str,
    package: &'static str,
    baseline_test_count: usize,
    expected_symbols: &'static [&'static str],
    prompt_goal: &'static str,
    semantic_probe: &'static str,
    seed: fn(&Path) -> Result<(), String>,
}

const LEVEL2_MAX_WORKER_RUNTIME_MS: u128 = 180_000;

const CASES: &[UsefulWorkCase] = &[
    UsefulWorkCase {
        task_id: "existing_behavior_patch",
        package: "pricing_core",
        baseline_test_count: 2,
        expected_symbols: &["DiscountPolicy", "discounted_quote_total"],
        prompt_goal: "Add a prompt-faithful discount feature to the existing pricing package. Read the local files first, add a DiscountPolicy model that can be constructed as DiscountPolicy(rate=0.20), add a discounted_quote_total helper where discounted_quote_total(100, DiscountPolicy(rate=0.20)) returns 88.0 after discount plus existing tax, preserve quote_total behavior, add regression tests, run validation, and report receipt-backed changes.",
        semantic_probe: "from pricing_core.pricing import DiscountPolicy, discounted_quote_total\nassert discounted_quote_total(100, DiscountPolicy(rate=0.20)) == 88.0\n",
        seed: seed_pricing_core,
    },
    UsefulWorkCase {
        task_id: "multi_requirement_vertical_slice",
        package: "task_router",
        baseline_test_count: 2,
        expected_symbols: &["TaskAuditLog", "route_and_record", "counts_by_status"],
        prompt_goal: "Extend the existing task router with a multi-file audit slice. Read the local package first, add TaskAuditLog, route_and_record, counts_by_status, preserve route_task behavior, and keep route_and_record importable from task_router.router because it wraps the existing router behavior. TaskAuditLog should be importable from task_router.audit and expose counts_by_status. Add regression tests, run validation, and report receipt-backed changes.",
        semantic_probe: "from task_router.router import route_and_record\nfrom task_router.audit import TaskAuditLog\nlog = TaskAuditLog()\nroute_and_record({'id': 'a', 'status': 'urgent'}, log)\nassert log.counts_by_status()['urgent'] == 1\n",
        seed: seed_task_router,
    },
    UsefulWorkCase {
        task_id: "validation_repair_ready_csv_loader",
        package: "csv_loader",
        baseline_test_count: 2,
        expected_symbols: &["OrderImportResult", "parse_orders_csv", "invalid_rows"],
        prompt_goal: "Implement a real CSV import slice in the existing loader. Read the local files first, add OrderImportResult and parse_orders_csv. parse_orders_csv must accept CSV text with id,total headers, return orders as plain dictionaries with id preserved and total parsed as a float, and return invalid_rows as 1-based data row numbers for rows whose total is not numeric. Add regression tests for valid and invalid rows, run validation, repair any failures from command output, and report receipt-backed changes.",
        semantic_probe: "from csv_loader.loader import parse_orders_csv\nresult = parse_orders_csv('id,total\\na,12.5\\nb,nope\\n')\nassert result.orders == [{'id': 'a', 'total': 12.5}]\nassert result.invalid_rows == [2]\n",
        seed: seed_csv_loader,
    },
    UsefulWorkCase {
        task_id: "no_fake_success_existing_tests",
        package: "feature_flags",
        baseline_test_count: 2,
        expected_symbols: &["FlagDecision", "resolve_flag", "environment_overrides"],
        prompt_goal: "Do not treat the existing tests as completion. Read the local feature flag package, add FlagDecision, resolve_flag, and environment_overrides behavior. resolve_flag must return a FlagDecision value object with enabled and source attributes; when environment overrides win, source should be environment_overrides. Add new tests that exercise the added behavior, run validation, and report receipt-backed changes.",
        semantic_probe: "from feature_flags.flags import FlagDecision, resolve_flag\ndecision = resolve_flag('new-dashboard', {'new-dashboard': False}, {'new-dashboard': True})\nassert isinstance(decision, FlagDecision)\nassert decision.enabled is True and decision.source == 'environment_overrides'\n",
        seed: seed_feature_flags,
    },
];

pub fn seed_native_coding_useful_work_batch(
    attempt_count: usize,
) -> NativeCodingUsefulWorkSeedReport {
    let seed_started_at_unix_ms = millis_now();
    let count = attempt_count.max(1);
    let batch_root = std::env::temp_dir().join(format!(
        "native-coding-useful-work-batch-{}-{}",
        std::process::id(),
        seed_started_at_unix_ms
    ));
    let prompts_root = batch_root.join("prompts");
    let mut jobs = Vec::new();
    let mut failures = Vec::new();

    if let Err(error) = fs::create_dir_all(&prompts_root) {
        failures.push(format!("create_prompts_root_failed:{error}"));
    }

    for ordinal in 0..count {
        let case = &CASES[ordinal % CASES.len()];
        match seed_case(ordinal + 1, case, &batch_root, &prompts_root) {
            Ok(job) => jobs.push(job),
            Err(error) => failures.push(error),
        }
    }

    let report = NativeCodingUsefulWorkSeedReport {
        harness_kind: "native_coding_useful_work_eval_v1_seed".to_string(),
        ok: failures.is_empty() && jobs.len() == count,
        batch_root: batch_root.display().to_string(),
        seed_started_at_unix_ms: Some(seed_started_at_unix_ms),
        attempt_count: jobs.len(),
        jobs,
        failures,
        operator_next_action: "run one Infring-native coding worker per prompt, save stdout to <batch_root>/agent_outputs/<attempt_id>.json, then run judge".to_string(),
    };
    let _ = write_file(
        &batch_root.join("jobs.json"),
        &serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()),
    );
    report
}

pub fn judge_native_coding_useful_work_batch(
    batch_root: &Path,
) -> NativeCodingUsefulWorkJudgeReport {
    let jobs_path = batch_root.join("jobs.json");
    let seed_report = fs::read_to_string(&jobs_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<NativeCodingUsefulWorkSeedReport>(&raw).ok());
    let batch_started_at_unix_ms = seed_report
        .as_ref()
        .and_then(|report| report.seed_started_at_unix_ms)
        .or_else(|| file_modified_unix_ms(&jobs_path));
    let mut failures = Vec::new();
    let jobs = match seed_report {
        Some(report) => report.jobs,
        None => {
            failures.push(format!("jobs_json_unreadable:{}", jobs_path.display()));
            Vec::new()
        }
    };

    let attempts = jobs
        .iter()
        .map(|job| judge_job(job, batch_root, batch_started_at_unix_ms))
        .collect::<Vec<_>>();
    for attempt in &attempts {
        if !attempt.ok {
            failures.extend(
                attempt
                    .failures
                    .iter()
                    .map(|failure| format!("{}:{failure}", attempt.attempt_id)),
            );
        }
    }
    let pass_count = attempts.iter().filter(|attempt| attempt.ok).count();
    let fail_count = attempts.len().saturating_sub(pass_count);
    let judged_at_unix_ms = millis_now();
    let timing = summarize_timing(batch_started_at_unix_ms, judged_at_unix_ms, &attempts);
    NativeCodingUsefulWorkJudgeReport {
        harness_kind: "native_coding_useful_work_eval_v1_judge",
        ok: failures.is_empty() && !attempts.is_empty(),
        batch_root: batch_root.display().to_string(),
        attempt_count: attempts.len(),
        pass_count,
        fail_count,
        timing,
        attempts,
        failures,
    }
}

fn seed_case(
    ordinal: usize,
    case: &UsefulWorkCase,
    batch_root: &Path,
    prompts_root: &Path,
) -> Result<NativeCodingUsefulWorkJob, String> {
    let attempt_id = format!("attempt_{ordinal:02}_{}", case.task_id);
    let run_root = batch_root.join(&attempt_id);
    let project_root = run_root.join("project");
    (case.seed)(&project_root)?;
    let validation = run_command(
        &project_root,
        &[
            "sh",
            "-c",
            "PYTHONPATH=src python3 -m unittest discover -s tests",
        ],
    );
    if !validation.ok {
        return Err(format!(
            "{attempt_id}:seed_validation_failed:{}",
            validation.detail
        ));
    }
    let semantic_probe_dir = project_root.join(".infring");
    fs::create_dir_all(&semantic_probe_dir).map_err(|error| {
        format!(
            "{attempt_id}:semantic_probe_dir_create_failed:{}:{error}",
            semantic_probe_dir.display()
        )
    })?;
    write_file(
        &semantic_probe_dir.join("semantic_probe.py"),
        case.semantic_probe,
    )?;
    let prompt_path = prompts_root.join(format!("{attempt_id}.txt"));
    write_file(&prompt_path, &worker_prompt(&project_root, case))?;
    let seed_completed_at_unix_ms = millis_now();
    Ok(NativeCodingUsefulWorkJob {
        attempt_id,
        task_id: case.task_id.to_string(),
        package: case.package.to_string(),
        run_root: run_root.display().to_string(),
        project_root: project_root.display().to_string(),
        prompt_path: prompt_path.display().to_string(),
        validation_command: "PYTHONPATH=src python3 -m unittest discover -s tests".to_string(),
        semantic_probe_command: vec![
            "python3".to_string(),
            "-c".to_string(),
            case.semantic_probe.to_string(),
        ],
        baseline_test_count: case.baseline_test_count,
        expected_symbols: case
            .expected_symbols
            .iter()
            .map(|item| item.to_string())
            .collect(),
        seed_completed_at_unix_ms: Some(seed_completed_at_unix_ms),
    })
}

fn worker_prompt(project_root: &Path, case: &UsefulWorkCase) -> String {
    format!(
        "You are running Native Coding Useful-Work Eval v1. Use the Infring native coding workflow and tools, not a simulated Codex worker. Your write ownership is limited to this project root: {project_root}\n\nTask: {goal}\n\nRules:\n1. Current local files are authoritative. Read relevant existing files before modifying behavior.\n2. This is an implementation task, so baseline validation passing before mutation is not completion evidence.\n3. Add or patch source and tests so the requested behavior exists semantically.\n4. Treat user-named functions, classes, attributes, constructor arguments, and input shapes as public API requirements. Put public helpers in the existing module that owns the adjacent preserved behavior, or re-export them there, unless the prompt explicitly names a new module.\n5. Regression tests must import and call the same public API shape requested by the task, not only an internal helper or alternate module.\n6. Run this validation command from project root: PYTHONPATH=src python3 -m unittest discover -s tests\n7. Run this semantic probe command from project root after validation: PYTHONPATH=src python3 .infring/semantic_probe.py\n8. If validation or the semantic probe fails, use the command output as repair input and patch the code/tests.\n9. Final response must list changed files, validation command/result, semantic probe result, caveats, and receipt-backed evidence.\n10. Do not commit anything.\n",
        project_root = project_root.display(),
        goal = case.prompt_goal,
    )
}

fn judge_job(
    job: &NativeCodingUsefulWorkJob,
    batch_root: &Path,
    batch_started_at_unix_ms: Option<u128>,
) -> NativeCodingUsefulWorkAttemptJudge {
    let project_root = PathBuf::from(&job.project_root);
    let mut checks = Vec::new();
    let mut failures = Vec::new();

    let validation = run_command(
        &project_root,
        &["sh", "-c", job.validation_command.as_str()],
    );
    push_check(
        &mut checks,
        "validation_passes_after_worker",
        validation.ok,
        validation.detail.clone(),
    );

    let observed_test_count = extract_unittest_count(&validation.detail).unwrap_or(0);
    push_check(
        &mut checks,
        "new_regression_tests_exercised",
        observed_test_count > job.baseline_test_count,
        format!(
            "observed_test_count={observed_test_count} baseline_test_count={}",
            job.baseline_test_count
        ),
    );

    let combined = read_python_project_text(&project_root, &job.package);
    let missing_symbols = job
        .expected_symbols
        .iter()
        .filter(|symbol| !combined.contains(symbol.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    push_check(
        &mut checks,
        "expected_symbols_present",
        missing_symbols.is_empty(),
        format!("missing_symbols={}", missing_symbols.join(",")),
    );

    let semantic_probe = run_command(
        &project_root,
        &job.semantic_probe_command
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    push_check(
        &mut checks,
        "semantic_probe_passes",
        semantic_probe.ok,
        semantic_probe.detail,
    );

    let source_mutation = source_or_test_modified_after_seed(job);
    push_check(
        &mut checks,
        "source_or_test_mutated_after_seed",
        source_mutation.0,
        source_mutation.1,
    );

    let receipt_evidence = native_receipt_evidence(batch_root, job);
    push_check(
        &mut checks,
        "native_receipts_observed",
        receipt_evidence.has_native_receipts,
        receipt_evidence.detail.clone(),
    );
    push_check(
        &mut checks,
        "mutation_receipt_observed",
        receipt_evidence.has_mutation_receipt,
        receipt_evidence.detail.clone(),
    );
    push_check(
        &mut checks,
        "validation_receipt_observed_or_command_verified",
        receipt_evidence.has_validation_receipt || validation.ok,
        receipt_evidence.detail.clone(),
    );
    push_check(
        &mut checks,
        "final_answer_reports_changed_files",
        receipt_evidence.has_changed_file_summary,
        receipt_evidence.final_answer_detail.clone(),
    );
    push_check(
        &mut checks,
        "final_answer_reports_validation",
        receipt_evidence.has_validation_summary,
        receipt_evidence.final_answer_detail,
    );
    let worker_runtime_ok = receipt_evidence
        .worker_runtime_ms
        .map(|runtime_ms| runtime_ms <= LEVEL2_MAX_WORKER_RUNTIME_MS)
        .unwrap_or(false);
    push_check(
        &mut checks,
        "worker_runtime_within_level2_budget",
        worker_runtime_ok,
        format!(
            "worker_runtime_ms={:?} max_worker_runtime_ms={}",
            receipt_evidence.worker_runtime_ms, LEVEL2_MAX_WORKER_RUNTIME_MS
        ),
    );

    for check in &checks {
        if !check.ok {
            failures.push(check.id.to_string());
        }
    }
    let timing = attempt_timing(
        job,
        batch_root,
        batch_started_at_unix_ms,
        receipt_evidence.first_mutation_ms,
    );
    NativeCodingUsefulWorkAttemptJudge {
        attempt_id: job.attempt_id.clone(),
        task_id: job.task_id.clone(),
        ok: failures.is_empty(),
        timing,
        checks,
        failures,
    }
}

#[derive(Debug, Clone)]
struct CommandResult {
    ok: bool,
    detail: String,
}

fn run_command(cwd: &Path, command: &[&str]) -> CommandResult {
    if command.is_empty() {
        return CommandResult {
            ok: false,
            detail: "empty_command".to_string(),
        };
    }
    match Command::new(command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .env("PYTHONPATH", "src")
        .output()
    {
        Ok(output) => CommandResult {
            ok: output.status.success(),
            detail: format!(
                "exit={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        },
        Err(error) => CommandResult {
            ok: false,
            detail: format!("spawn_failed:{error}"),
        },
    }
}

fn push_check(
    checks: &mut Vec<NativeCodingUsefulWorkCheck>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    checks.push(NativeCodingUsefulWorkCheck { id, ok, detail });
}

fn extract_unittest_count(detail: &str) -> Option<usize> {
    let marker = "Ran ";
    let index = detail.find(marker)? + marker.len();
    let tail = &detail[index..];
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<usize>().ok()
}

fn read_python_project_text(project_root: &Path, package: &str) -> String {
    let mut combined = String::new();
    for root in [
        project_root.join(format!("src/{package}")),
        project_root.join("tests"),
    ] {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("py") {
                    combined.push_str(&fs::read_to_string(path).unwrap_or_default());
                    combined.push('\n');
                }
            }
        }
    }
    combined
}

fn source_or_test_modified_after_seed(job: &NativeCodingUsefulWorkJob) -> (bool, String) {
    let Some(seed_completed) = job.seed_completed_at_unix_ms else {
        return (false, "seed_completed_at_unix_ms_missing".to_string());
    };
    let project_root = PathBuf::from(&job.project_root);
    let mut newest = None;
    let mut modified_paths = Vec::new();
    for root in [project_root.join("src"), project_root.join("tests")] {
        collect_modified_py_files(&root, seed_completed, &mut newest, &mut modified_paths);
    }
    (
        !modified_paths.is_empty(),
        format!(
            "seed_completed_at_unix_ms={seed_completed} newest_py_mtime_ms={:?} modified_paths={}",
            newest,
            modified_paths.join(",")
        ),
    )
}

fn collect_modified_py_files(
    root: &Path,
    seed_completed: u128,
    newest: &mut Option<u128>,
    modified_paths: &mut Vec<String>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_modified_py_files(&path, seed_completed, newest, modified_paths);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        let mtime = file_modified_unix_ms(&path).unwrap_or(0);
        *newest = Some(newest.map(|current| current.max(mtime)).unwrap_or(mtime));
        if mtime > seed_completed {
            modified_paths.push(path.display().to_string());
        }
    }
}

#[derive(Debug, Clone)]
struct ReceiptEvidence {
    has_native_receipts: bool,
    has_mutation_receipt: bool,
    has_validation_receipt: bool,
    has_changed_file_summary: bool,
    has_validation_summary: bool,
    first_mutation_ms: Option<u128>,
    worker_runtime_ms: Option<u128>,
    detail: String,
    final_answer_detail: String,
}

fn native_receipt_evidence(batch_root: &Path, job: &NativeCodingUsefulWorkJob) -> ReceiptEvidence {
    let output_path = batch_root
        .join("agent_outputs")
        .join(format!("{}.json", job.attempt_id));
    let journal_path = PathBuf::from(&job.project_root)
        .join(".infring")
        .join("native_run_journal.json");
    let raw = fs::read_to_string(&output_path).unwrap_or_default();
    let journal_raw = fs::read_to_string(&journal_path).unwrap_or_default();
    let raw_lower = format!("{raw}\n{journal_raw}").to_ascii_lowercase();
    let output = serde_json::from_str::<serde_json::Value>(&raw).ok();
    let journal = serde_json::from_str::<serde_json::Value>(&journal_raw).ok();
    let journal_receipts = journal
        .as_ref()
        .and_then(|value| value.get("native_tool_receipts"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let has_native_receipts = raw.contains("native_tool_receipts")
        || raw.contains("tool_receipts")
        || !journal_receipts.is_empty();
    let has_structured_mutation_receipt = journal_receipts.iter().any(|receipt| {
        receipt.get("status").and_then(serde_json::Value::as_str) == Some("ok")
            && matches!(
                receipt.get("tool_name").and_then(serde_json::Value::as_str),
                Some("file_write" | "file_patch")
            )
            && receipt
                .get("result")
                .and_then(|result| result.get("path"))
                .and_then(serde_json::Value::as_str)
                .is_some()
    });
    let has_mutation_receipt = has_structured_mutation_receipt;
    let has_validation_receipt = journal_receipts.iter().any(|receipt| {
        receipt.get("status").and_then(serde_json::Value::as_str) == Some("ok")
            && receipt.get("tool_name").and_then(serde_json::Value::as_str) == Some("command_run")
    }) || (raw.contains("command_run") && raw.contains("unittest"));
    let has_changed_file_summary = journal
        .as_ref()
        .and_then(|value| value.get("changed_files"))
        .and_then(serde_json::Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
        || (raw_lower.contains("changed") && raw_lower.contains(".py"));
    let has_validation_summary = journal
        .as_ref()
        .and_then(|value| value.get("validation_receipts"))
        .and_then(serde_json::Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
        || raw_lower.contains("validation")
        || raw_lower.contains("test");
    let worker_runtime_ms = output
        .as_ref()
        .and_then(|value| value.get("receipt"))
        .and_then(|receipt| receipt.get("phase_latency_ms"))
        .and_then(|latency| latency.get("total"))
        .and_then(serde_json::Value::as_u64)
        .map(u128::from)
        .or_else(|| {
            output
                .as_ref()
                .and_then(|value| value.get("trace_summary"))
                .and_then(|trace| trace.get("phase_latency_ms"))
                .and_then(|latency| latency.get("total"))
                .and_then(serde_json::Value::as_u64)
                .map(u128::from)
        });
    ReceiptEvidence {
        has_native_receipts,
        has_mutation_receipt,
        has_validation_receipt,
        has_changed_file_summary,
        has_validation_summary,
        first_mutation_ms: first_project_mutation_mtime(job),
        worker_runtime_ms,
        detail: format!(
            "output_path={} journal_path={} has_native_receipts={has_native_receipts} has_mutation_receipt={has_mutation_receipt} has_validation_receipt={has_validation_receipt} worker_runtime_ms={worker_runtime_ms:?}",
            output_path.display(),
            journal_path.display()
        ),
        final_answer_detail: format!(
            "output_path={} journal_path={} has_changed_file_summary={has_changed_file_summary} has_validation_summary={has_validation_summary}",
            output_path.display(),
            journal_path.display()
        ),
    }
}

fn first_project_mutation_mtime(job: &NativeCodingUsefulWorkJob) -> Option<u128> {
    let seed_completed = job.seed_completed_at_unix_ms?;
    let project_root = PathBuf::from(&job.project_root);
    let mut first: Option<u128> = None;
    for root in [project_root.join("src"), project_root.join("tests")] {
        collect_first_modified_py_file(&root, seed_completed, &mut first);
    }
    first
}

fn collect_first_modified_py_file(root: &Path, seed_completed: u128, first: &mut Option<u128>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_first_modified_py_file(&path, seed_completed, first);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        let mtime = file_modified_unix_ms(&path).unwrap_or(0);
        if mtime > seed_completed {
            *first = Some(first.map(|current| current.min(mtime)).unwrap_or(mtime));
        }
    }
}

fn attempt_timing(
    job: &NativeCodingUsefulWorkJob,
    batch_root: &Path,
    batch_started_at_unix_ms: Option<u128>,
    first_mutation_unix_ms: Option<u128>,
) -> NativeCodingUsefulWorkAttemptTiming {
    let output_path = batch_root
        .join("agent_outputs")
        .join(format!("{}.json", job.attempt_id));
    let completed_at_unix_ms = newest_project_mtime(&PathBuf::from(&job.project_root))
        .max(file_modified_unix_ms(&output_path));
    NativeCodingUsefulWorkAttemptTiming {
        completed_at_unix_ms,
        elapsed_ms_since_batch_start: batch_started_at_unix_ms.and_then(|start| {
            completed_at_unix_ms.map(|completed| completed.saturating_sub(start))
        }),
        first_mutation_unix_ms,
        time_to_first_mutation_ms: job
            .seed_completed_at_unix_ms
            .and_then(|seed| first_mutation_unix_ms.map(|first| first.saturating_sub(seed))),
    }
}

fn summarize_timing(
    batch_started_at_unix_ms: Option<u128>,
    judged_at_unix_ms: u128,
    attempts: &[NativeCodingUsefulWorkAttemptJudge],
) -> NativeCodingUsefulWorkTimingSummary {
    let completed = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.completed_at_unix_ms)
        .collect::<Vec<_>>();
    let elapsed = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.elapsed_ms_since_batch_start)
        .collect::<Vec<_>>();
    let first_mutation_elapsed = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.time_to_first_mutation_ms)
        .collect::<Vec<_>>();
    NativeCodingUsefulWorkTimingSummary {
        batch_started_at_unix_ms,
        judged_at_unix_ms,
        batch_elapsed_ms: batch_started_at_unix_ms
            .map(|start| judged_at_unix_ms.saturating_sub(start)),
        first_attempt_completed_at_unix_ms: completed.iter().copied().min(),
        last_attempt_completed_at_unix_ms: completed.iter().copied().max(),
        completion_span_ms: completed.iter().copied().min().and_then(|first| {
            completed
                .iter()
                .copied()
                .max()
                .map(|last| last.saturating_sub(first))
        }),
        average_attempt_elapsed_ms: average_u128(&elapsed),
        average_time_to_first_mutation_ms: average_u128(&first_mutation_elapsed),
    }
}

fn average_u128(values: &[u128]) -> Option<u128> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<u128>() / values.len() as u128)
    }
}

fn newest_project_mtime(project_root: &Path) -> Option<u128> {
    let mut newest: Option<u128> = None;
    for root in [project_root.join("src"), project_root.join("tests")] {
        newest = newest.max(newest_mtime_recursive(&root));
    }
    newest
}

fn newest_mtime_recursive(root: &Path) -> Option<u128> {
    let mut newest = file_modified_unix_ms(root);
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let candidate = if entry.path().is_dir() {
                newest_mtime_recursive(&entry.path())
            } else {
                file_modified_unix_ms(&entry.path())
            };
            newest = newest.max(candidate);
        }
    }
    newest
}

fn millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn file_modified_unix_ms(path: &Path) -> Option<u128> {
    path.metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create_parent_failed:{}:{error}", parent.display()))?;
    }
    fs::write(path, content)
        .map_err(|error| format!("write_file_failed:{}:{error}", path.display()))
}

fn seed_pricing_core(project_root: &Path) -> Result<(), String> {
    write_common_package(project_root, "pricing_core")?;
    write_file(
        &project_root.join("src/pricing_core/pricing.py"),
        "def quote_total(subtotal):\n    return round(subtotal * 1.10, 2)\n",
    )?;
    write_file(
        &project_root.join("tests/test_pricing.py"),
        "import unittest\nfrom pricing_core.pricing import quote_total\n\nclass PricingTests(unittest.TestCase):\n    def test_quote_total_applies_tax(self):\n        self.assertEqual(quote_total(100), 110.0)\n\n    def test_quote_total_rounds(self):\n        self.assertEqual(quote_total(10.05), 11.06)\n\nif __name__ == '__main__':\n    unittest.main()\n",
    )
}

fn seed_task_router(project_root: &Path) -> Result<(), String> {
    write_common_package(project_root, "task_router")?;
    write_file(
        &project_root.join("src/task_router/router.py"),
        "def route_task(task):\n    status = task.get('status', 'normal')\n    if status == 'urgent':\n        return 'oncall'\n    return 'backlog'\n",
    )?;
    write_file(
        &project_root.join("tests/test_router.py"),
        "import unittest\nfrom task_router.router import route_task\n\nclass RouterTests(unittest.TestCase):\n    def test_urgent_routes_oncall(self):\n        self.assertEqual(route_task({'status': 'urgent'}), 'oncall')\n\n    def test_normal_routes_backlog(self):\n        self.assertEqual(route_task({'status': 'normal'}), 'backlog')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    )
}

fn seed_csv_loader(project_root: &Path) -> Result<(), String> {
    write_common_package(project_root, "csv_loader")?;
    write_file(
        &project_root.join("src/csv_loader/loader.py"),
        "def normalize_order_id(raw):\n    return raw.strip().lower()\n",
    )?;
    write_file(
        &project_root.join("tests/test_loader.py"),
        "import unittest\nfrom csv_loader.loader import normalize_order_id\n\nclass LoaderTests(unittest.TestCase):\n    def test_normalize_order_id_strips(self):\n        self.assertEqual(normalize_order_id(' A-1 '), 'a-1')\n\n    def test_normalize_order_id_lowercases(self):\n        self.assertEqual(normalize_order_id('B-2'), 'b-2')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    )
}

fn seed_feature_flags(project_root: &Path) -> Result<(), String> {
    write_common_package(project_root, "feature_flags")?;
    write_file(
        &project_root.join("src/feature_flags/flags.py"),
        "def is_enabled(name, defaults):\n    return bool(defaults.get(name, False))\n",
    )?;
    write_file(
        &project_root.join("tests/test_flags.py"),
        "import unittest\nfrom feature_flags.flags import is_enabled\n\nclass FlagTests(unittest.TestCase):\n    def test_default_true(self):\n        self.assertTrue(is_enabled('alpha', {'alpha': True}))\n\n    def test_missing_false(self):\n        self.assertFalse(is_enabled('missing', {}))\n\nif __name__ == '__main__':\n    unittest.main()\n",
    )
}

fn write_common_package(project_root: &Path, package: &str) -> Result<(), String> {
    write_file(&project_root.join(format!("src/{package}/__init__.py")), "")?;
    write_file(&project_root.join("tests/__init__.py"), "")?;
    write_file(
        &project_root.join("README.md"),
        &format!("# {package}\n\nNative Coding Useful-Work Eval fixture.\n"),
    )?;
    write_file(
        &project_root.join("pyproject.toml"),
        &format!(
            "[project]\nname = \"{package}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.10\"\n"
        ),
    )
}

#[allow(dead_code)]
fn _json_debug(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or_else(|_| json!({}))
}
