// Layer ownership: eval/observability (ForgeCode-derived medium software analysis).
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct ForgeLevel4SoftwareAnalysisReport {
    pub harness_kind: &'static str,
    pub analysis_target_kind: &'static str,
    pub candidate_root: String,
    pub ok: bool,
    pub decision: &'static str,
    pub dimensions: Vec<ForgeLevel4SoftwareAnalysisDimension>,
    pub architecture_boundary_violations: Vec<String>,
    pub behavior_gaps: Vec<String>,
    pub integration_gaps: Vec<String>,
    pub repair_loop_gaps: Vec<String>,
    pub evidence_gaps: Vec<String>,
    pub operator_next_action: &'static str,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeLevel4SoftwareAnalysisDimension {
    pub dimension: &'static str,
    pub status: &'static str,
    pub checks: Vec<ForgeLevel4SoftwareAnalysisCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeLevel4SoftwareAnalysisCheck {
    pub id: &'static str,
    pub ok: bool,
    pub severity: &'static str,
    pub detail: String,
}

pub fn forge_level4_software_analysis_fixture_report() -> ForgeLevel4SoftwareAnalysisReport {
    let root = fixture_root_path();
    let mut seed_failures = Vec::new();
    if let Err(error) = fs::create_dir_all(&root) {
        seed_failures.push(format!("fixture_root_create_failed:{error}"));
    }
    seed_level4_fixture(&root, &mut seed_failures);
    let mut report =
        forge_level4_software_analysis_report_for_path_kind(&root, "generated_fixture");
    report.failures.extend(seed_failures);
    report.ok = report.ok && report.failures.is_empty();
    report
}

pub fn forge_level4_software_analysis_report_for_path(
    candidate_root: &Path,
) -> ForgeLevel4SoftwareAnalysisReport {
    forge_level4_software_analysis_report_for_path_kind(candidate_root, "provided_candidate")
}

fn forge_level4_software_analysis_report_for_path_kind(
    candidate_root: &Path,
    analysis_target_kind: &'static str,
) -> ForgeLevel4SoftwareAnalysisReport {
    let root = candidate_root.to_path_buf();
    let domain_files = collect_rs_files(&root.join("src/domain"));
    let app_files = collect_rs_files(&root.join("src/app"));
    let persistence_files = collect_rs_files(&root.join("src/persistence"));
    let interface_files = collect_rs_files(&root.join("src/interface"));
    let test_files = collect_rs_files(&root.join("tests"));

    let domain_text = combined_text(&domain_files);
    let app_text = combined_text(&app_files);
    let persistence_text = combined_text(&persistence_files);
    let interface_text = combined_text(&interface_files);
    let test_text = combined_text(&test_files);
    let module_text = [
        read_optional(&root.join("src/lib.rs")),
        read_optional(&root.join("src/main.rs")),
    ]
    .join("\n");

    let architecture_checks = vec![
        check(
            "domain_layer_present",
            !domain_files.is_empty(),
            "error",
            format!("domain_files={}", display_paths(&domain_files).join(",")),
        ),
        check(
            "app_layer_present",
            !app_files.is_empty(),
            "error",
            format!("app_files={}", display_paths(&app_files).join(",")),
        ),
        check(
            "persistence_adapter_present",
            !persistence_files.is_empty(),
            "error",
            format!(
                "persistence_files={}",
                display_paths(&persistence_files).join(",")
            ),
        ),
        check(
            "interface_or_cli_layer_present",
            !interface_files.is_empty(),
            "error",
            format!("interface_files={}", display_paths(&interface_files).join(",")),
        ),
        check(
            "domain_has_no_persistence_or_cli_leak",
            !contains_any(
                &domain_text,
                &[
                    "std::fs",
                    "File::",
                    "PathBuf",
                    "serde_json",
                    "clap::",
                    "structopt",
                    "crate::persistence",
                    "crate::interface",
                    "crate::cli",
                ],
            ),
            "error",
            "domain layer should not import filesystem, JSON persistence, CLI, or adapter modules"
                .to_string(),
        ),
        check(
            "persistence_adapter_depends_on_domain",
            persistence_text.contains("crate::domain"),
            "error",
            "persistence adapter should serialize/deserialize domain values through an adapter boundary"
                .to_string(),
        ),
        check(
            "interface_depends_on_app_or_domain",
            interface_text.contains("crate::app") || interface_text.contains("crate::domain"),
            "error",
            "interface layer should route to app/domain APIs instead of standing alone".to_string(),
        ),
    ];

    let behavior_checks = vec![
        check(
            "behavior_tests_present",
            !test_files.is_empty(),
            "error",
            format!("test_files={}", display_paths(&test_files).join(",")),
        ),
        check(
            "create_or_add_flow_covered",
            contains_any(&test_text, &["create", "add_task", "add task", "Add"]),
            "error",
            "tests should cover creating or adding a task".to_string(),
        ),
        check(
            "list_flow_covered",
            contains_any(&test_text, &["list", "List"]),
            "error",
            "tests should cover listing tasks".to_string(),
        ),
        check(
            "update_or_complete_flow_covered",
            contains_any(&test_text, &["complete", "update", "Complete", "Update"]),
            "error",
            "tests should cover update or completion behavior".to_string(),
        ),
        check(
            "persistence_roundtrip_covered",
            contains_any(
                &test_text,
                &["roundtrip", "round_trip", "save", "load", "persist"],
            ),
            "error",
            "tests should cover persistence round-trip behavior".to_string(),
        ),
        check(
            "validation_receipt_present_and_passing",
            validation_receipt_present_and_passing(&root),
            "error",
            "validation receipt should exist and indicate current passing validation".to_string(),
        ),
    ];

    let integration_checks = vec![
        check(
            "root_module_wires_domain",
            module_text.contains("mod domain") || module_text.contains("pub mod domain"),
            "error",
            "src/lib.rs or src/main.rs should wire the domain module".to_string(),
        ),
        check(
            "root_module_wires_app",
            module_text.contains("mod app") || module_text.contains("pub mod app"),
            "error",
            "src/lib.rs or src/main.rs should wire the app module".to_string(),
        ),
        check(
            "root_module_wires_persistence",
            module_text.contains("mod persistence") || module_text.contains("pub mod persistence"),
            "error",
            "src/lib.rs or src/main.rs should wire the persistence module".to_string(),
        ),
        check(
            "root_module_wires_interface",
            module_text.contains("mod interface") || module_text.contains("pub mod interface"),
            "error",
            "src/lib.rs or src/main.rs should wire the interface module".to_string(),
        ),
        check(
            "app_layer_integrates_domain",
            app_text.contains("crate::domain"),
            "error",
            "app layer should call into the domain layer".to_string(),
        ),
        check(
            "app_layer_integrates_persistence",
            app_text.contains("crate::persistence"),
            "error",
            "app layer should call into the persistence adapter".to_string(),
        ),
    ];

    let repair_checks = vec![
        check(
            "repair_policy_present",
            repair_policy_path(&root).is_some(),
            "error",
            "workflow_artifacts/repair_policy.json or receipts/repair_policy.json should exist"
                .to_string(),
        ),
        check(
            "repair_policy_has_bounded_attempts",
            repair_attempt_budget_ok(&root),
            "error",
            "repair policy should set max_repair_attempts <= 3".to_string(),
        ),
        check(
            "repair_policy_declares_allowed_scope",
            repair_policy_text(&root)
                .map(|text| {
                    contains_any(&text, &["allowed_scope", "allowed_files", "allowed_paths"])
                })
                .unwrap_or(false),
            "error",
            "repair policy should constrain allowed repair scope".to_string(),
        ),
        check(
            "repair_policy_declares_stop_conditions",
            repair_policy_text(&root)
                .map(|text| contains_any(&text, &["stop_condition", "stop_conditions", "stop_on"]))
                .unwrap_or(false),
            "error",
            "repair policy should declare stop conditions".to_string(),
        ),
    ];

    let evidence_checks = vec![
        check(
            "controlled_eval_boundary_receipt_present",
            first_existing(
                &root,
                &[
                    "receipts/controlled_eval_run_boundary_receipts.jsonl",
                    "controlled_eval_run_boundary_receipts.jsonl",
                ],
            )
            .is_some(),
            "error",
            "controlled eval-run boundary receipt is required for level-4 live evidence"
                .to_string(),
        ),
        check(
            "agent_trace_ref_present",
            first_existing(
                &root,
                &[
                    "receipts/agent_trace_refs.jsonl",
                    "receipts/agent_trace.jsonl",
                    "agent_trace_refs.jsonl",
                ],
            )
            .is_some(),
            "error",
            "agent trace refs are required to judge tool selection and planning".to_string(),
        ),
        check(
            "file_change_receipt_present",
            first_existing(
                &root,
                &[
                    "receipts/file_change_receipts.jsonl",
                    "file_change_receipts.jsonl",
                ],
            )
            .is_some(),
            "error",
            "file change receipts are required to prove ownership and unrelated-file preservation"
                .to_string(),
        ),
        check(
            "normalized_eval_result_receipt_present",
            first_existing(
                &root,
                &[
                    "receipts/executable_eval_result_receipts.jsonl",
                    "executable_eval_result_receipts.jsonl",
                ],
            )
            .is_some(),
            "error",
            "normalized executable eval-result receipts are required before promotion scoring"
                .to_string(),
        ),
    ];

    let dimensions = vec![
        dimension("architecture_boundaries", architecture_checks),
        dimension("behavioral_correctness", behavior_checks),
        dimension("integration_coherence", integration_checks),
        dimension("repair_loop_control", repair_checks),
        dimension("evidence_quality", evidence_checks),
    ];

    let architecture_boundary_violations =
        failed_check_details(&dimensions, "architecture_boundaries");
    let behavior_gaps = failed_check_details(&dimensions, "behavioral_correctness");
    let integration_gaps = failed_check_details(&dimensions, "integration_coherence");
    let repair_loop_gaps = failed_check_details(&dimensions, "repair_loop_control");
    let evidence_gaps = failed_check_details(&dimensions, "evidence_quality");

    let mut failures = Vec::new();
    for dimension in &dimensions {
        for check in &dimension.checks {
            if !check.ok && check.severity == "error" {
                failures.push(format!(
                    "{}:{}:{}",
                    dimension.dimension, check.id, check.detail
                ));
            }
        }
    }

    let ok = failures.is_empty();
    let decision = if ok {
        "level4_analysis_pass_single_attempt_not_promotion_proof"
    } else if !evidence_gaps.is_empty() {
        "blocked_live_evidence_missing"
    } else {
        "repair_workflow_then_retry"
    };
    let operator_next_action = if ok {
        "run_5_attempt_level4_live_eval_probe"
    } else {
        "repair_level4_candidate_or_receipt_bundle"
    };

    ForgeLevel4SoftwareAnalysisReport {
        harness_kind: "forge_level4_software_analysis_v1",
        analysis_target_kind,
        candidate_root: root.to_string_lossy().to_string(),
        ok,
        decision,
        dimensions,
        architecture_boundary_violations,
        behavior_gaps,
        integration_gaps,
        repair_loop_gaps,
        evidence_gaps,
        operator_next_action,
        failures,
    }
}

fn seed_level4_fixture(root: &Path, failures: &mut Vec<String>) {
    let files = [
        (
            "src/lib.rs",
            r#"pub mod app;
pub mod domain;
pub mod interface;
pub mod persistence;
"#,
        ),
        ("src/domain/mod.rs", "pub mod task;\n"),
        (
            "src/domain/task.rs",
            r#"#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

impl Task {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self { id: id.into(), title: title.into(), completed: false }
    }

    pub fn complete(&mut self) {
        self.completed = true;
    }
}
"#,
        ),
        ("src/persistence/mod.rs", "pub mod json_store;\n"),
        (
            "src/persistence/json_store.rs",
            r#"use crate::domain::task::Task;

pub trait TaskStore {
    fn load(&self) -> Vec<Task>;
    fn save(&mut self, tasks: &[Task]);
}

#[derive(Default)]
pub struct JsonTaskStore {
    tasks: Vec<Task>,
}

impl TaskStore for JsonTaskStore {
    fn load(&self) -> Vec<Task> {
        self.tasks.clone()
    }

    fn save(&mut self, tasks: &[Task]) {
        self.tasks = tasks.to_vec();
    }
}
"#,
        ),
        ("src/app/mod.rs", "pub mod task_ledger;\n"),
        (
            "src/app/task_ledger.rs",
            r#"use crate::domain::task::Task;
use crate::persistence::json_store::TaskStore;

pub struct TaskLedger<S: TaskStore> {
    store: S,
}

impl<S: TaskStore> TaskLedger<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn add_task(&mut self, id: impl Into<String>, title: impl Into<String>) {
        let mut tasks = self.store.load();
        tasks.push(Task::new(id, title));
        self.store.save(&tasks);
    }

    pub fn list_tasks(&self) -> Vec<Task> {
        self.store.load()
    }

    pub fn complete_task(&mut self, id: &str) {
        let mut tasks = self.store.load();
        for task in &mut tasks {
            if task.id == id {
                task.complete();
            }
        }
        self.store.save(&tasks);
    }
}
"#,
        ),
        ("src/interface/mod.rs", "pub mod cli;\n"),
        (
            "src/interface/cli.rs",
            r#"use crate::app::task_ledger::TaskLedger;
use crate::persistence::json_store::TaskStore;

pub enum Command {
    Add { id: String, title: String },
    List,
    Complete { id: String },
}

pub fn apply_command<S: TaskStore>(ledger: &mut TaskLedger<S>, command: Command) {
    match command {
        Command::Add { id, title } => ledger.add_task(id, title),
        Command::List => {
            let _ = ledger.list_tasks();
        }
        Command::Complete { id } => ledger.complete_task(&id),
    }
}
"#,
        ),
        (
            "tests/task_ledger_flow.rs",
            r#"use level4_fixture::app::task_ledger::TaskLedger;
use level4_fixture::interface::cli::{apply_command, Command};
use level4_fixture::persistence::json_store::JsonTaskStore;

#[test]
fn create_list_complete_and_persist_roundtrip() {
    let store = JsonTaskStore::default();
    let mut ledger = TaskLedger::new(store);
    apply_command(&mut ledger, Command::Add { id: "1".into(), title: "write eval".into() });
    assert_eq!(ledger.list_tasks().len(), 1);
    apply_command(&mut ledger, Command::Complete { id: "1".into() });
    assert!(ledger.list_tasks()[0].completed);
    let roundtrip = ledger.list_tasks();
    assert_eq!(roundtrip[0].title, "write eval");
}
"#,
        ),
        (
            "workflow_artifacts/repair_policy.json",
            r#"{
  "max_repair_attempts": 2,
  "allowed_scope": ["src/domain", "src/app", "src/interface", "src/persistence", "tests"],
  "stop_conditions": ["unexpected_dirty_files", "architecture_boundary_collapse", "unbounded_repair_loop"]
}
"#,
        ),
        (
            "receipts/controlled_eval_run_boundary_receipts.jsonl",
            r#"{"ok":true,"coverage_axis":"medium_cli_with_persistence","authorization":"operator_approved_plan_only_fixture"}
"#,
        ),
        (
            "receipts/agent_trace_refs.jsonl",
            r#"{"ok":true,"trace_ref":"fixture-agent-trace","tool_selection":"read_write_patch"}
"#,
        ),
        (
            "receipts/file_change_receipts.jsonl",
            r#"{"ok":true,"changed_files":["src/domain/task.rs","src/app/task_ledger.rs","src/interface/cli.rs","src/persistence/json_store.rs","tests/task_ledger_flow.rs"],"unrelated_files_preserved":true}
"#,
        ),
        (
            "receipts/validation_receipts.jsonl",
            r#"{"ok":true,"status":"passed","command":"cargo test","current":true}
"#,
        ),
        (
            "receipts/executable_eval_result_receipts.jsonl",
            r#"{"ok":true,"status":"passed","coverage_axis":"medium_cli_with_persistence","pass_fail_status":"pass"}
"#,
        ),
    ];

    for (relative, contents) in files {
        if let Err(error) = write_file(root, relative, contents) {
            failures.push(format!("fixture_write_failed:{relative}:{error}"));
        }
    }
}

fn fixture_root_path() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "forge-level4-software-analysis-{}-{millis}",
        std::process::id()
    ))
}

fn write_file(root: &Path, relative: &str, contents: &str) -> std::io::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rs_files_inner(root, &mut files);
    files.sort();
    files
}

fn collect_rs_files_inner(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files_inner(&path, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension == "rs")
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
}

fn combined_text(files: &[PathBuf]) -> String {
    files
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_optional(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn display_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn check(
    id: &'static str,
    ok: bool,
    severity: &'static str,
    detail: String,
) -> ForgeLevel4SoftwareAnalysisCheck {
    ForgeLevel4SoftwareAnalysisCheck {
        id,
        ok,
        severity,
        detail,
    }
}

fn dimension(
    dimension: &'static str,
    checks: Vec<ForgeLevel4SoftwareAnalysisCheck>,
) -> ForgeLevel4SoftwareAnalysisDimension {
    let status = if checks
        .iter()
        .any(|check| !check.ok && check.severity == "error")
    {
        "fail"
    } else if checks.iter().any(|check| !check.ok) {
        "warn"
    } else {
        "pass"
    };
    ForgeLevel4SoftwareAnalysisDimension {
        dimension,
        status,
        checks,
    }
}

fn failed_check_details(
    dimensions: &[ForgeLevel4SoftwareAnalysisDimension],
    dimension_name: &str,
) -> Vec<String> {
    dimensions
        .iter()
        .filter(|dimension| dimension.dimension == dimension_name)
        .flat_map(|dimension| {
            dimension
                .checks
                .iter()
                .filter(|check| !check.ok)
                .map(|check| format!("{}:{}", check.id, check.detail))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn first_existing(root: &Path, relatives: &[&str]) -> Option<PathBuf> {
    relatives
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.exists())
}

fn validation_receipt_present_and_passing(root: &Path) -> bool {
    first_existing(
        root,
        &[
            "receipts/validation_receipts.jsonl",
            "receipts/validation_receipt.json",
            "validation_receipts.jsonl",
            "validation_receipt.json",
        ],
    )
    .and_then(|path| fs::read_to_string(path).ok())
    .map(|text| {
        contains_any(
            &text,
            &[
                "\"ok\":true",
                "\"status\":\"passed\"",
                "\"status\":\"pass\"",
            ],
        )
    })
    .unwrap_or(false)
}

fn repair_policy_path(root: &Path) -> Option<PathBuf> {
    first_existing(
        root,
        &[
            "workflow_artifacts/repair_policy.json",
            "receipts/repair_policy.json",
            "repair_policy.json",
        ],
    )
}

fn repair_policy_text(root: &Path) -> Option<String> {
    repair_policy_path(root).and_then(|path| fs::read_to_string(path).ok())
}

fn repair_attempt_budget_ok(root: &Path) -> bool {
    let Some(text) = repair_policy_text(root) else {
        return false;
    };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| value.get("max_repair_attempts").and_then(Value::as_u64))
        .map(|attempts| attempts <= 3)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::forge_level4_software_analysis_fixture_report;

    #[test]
    fn forge_level4_software_analysis_accepts_coherent_medium_cli_fixture() {
        let report = forge_level4_software_analysis_fixture_report();
        assert!(report.ok, "{report:#?}");
        assert_eq!(
            report.decision,
            "level4_analysis_pass_single_attempt_not_promotion_proof"
        );
        assert!(report
            .dimensions
            .iter()
            .all(|dimension| dimension.status == "pass"));
        assert!(report.architecture_boundary_violations.is_empty());
        assert!(report.behavior_gaps.is_empty());
        assert!(report.integration_gaps.is_empty());
        assert!(report.repair_loop_gaps.is_empty());
        assert!(report.evidence_gaps.is_empty());
    }
}
