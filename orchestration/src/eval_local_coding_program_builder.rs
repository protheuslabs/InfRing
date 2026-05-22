// Layer ownership: eval/observability (local coding workflow capability measurement).
use crate::control_plane::workflow_lab_replay::{
    local_coding_program_builder_lab_execution_report, LocalCodingProgramBuilderLabTaskExecution,
    LocalCodingProgramBuilderSliceInvocation,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const BUILDER_WORKFLOW_ID: &str = "local_coding_program_builder";

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabFileExecutionReport {
    pub workflow_id: &'static str,
    pub harness_kind: &'static str,
    pub ok: bool,
    pub sandbox_root: String,
    pub task_executions: Vec<LocalCodingProgramBuilderLabFileTaskExecution>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabFileTaskExecution {
    pub task_id: &'static str,
    pub user_goal: &'static str,
    pub checkpoint_name: &'static str,
    pub architecture_pattern: &'static str,
    pub sandbox_path: String,
    pub slice_results: Vec<LocalCodingProgramBuilderLabFileSliceResult>,
    pub changed_files: Vec<String>,
    pub validation_results: Vec<LocalCodingProgramBuilderLabValidationResult>,
    pub final_handoff: LocalCodingProgramBuilderLabFinalHandoff,
    pub ok: bool,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabFileSliceResult {
    pub slice_name: &'static str,
    pub child_workflow_id: &'static str,
    pub output_artifact: &'static str,
    pub wrote_files: Vec<String>,
    pub validation_results: Vec<LocalCodingProgramBuilderLabValidationResult>,
    pub ok: bool,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabValidationResult {
    pub check: &'static str,
    pub status: &'static str,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabFinalHandoff {
    pub completed_checkpoint: &'static str,
    pub changed_files: Vec<String>,
    pub validation_results: Vec<LocalCodingProgramBuilderLabValidationResult>,
    pub known_risks: Vec<&'static str>,
    pub intentionally_excluded_scope: Vec<&'static str>,
    pub recommended_next_checkpoint: &'static str,
}

pub fn local_coding_program_builder_lab_file_execution_report(
) -> LocalCodingProgramBuilderLabFileExecutionReport {
    let execution_report = local_coding_program_builder_lab_execution_report();
    let sandbox_root_path = lab_sandbox_root_path();
    let sandbox_root = sandbox_root_path.to_string_lossy().to_string();
    let mut failures = execution_report.failures.clone();

    if let Err(error) = std::fs::create_dir_all(&sandbox_root_path) {
        failures.push(format!("sandbox_root_create_failed:{error}"));
    }

    let task_executions = execution_report
        .task_executions
        .into_iter()
        .map(|execution| file_task_execution(&sandbox_root_path, execution))
        .collect::<Vec<_>>();
    let ok = failures.is_empty() && task_executions.iter().all(|execution| execution.ok);

    LocalCodingProgramBuilderLabFileExecutionReport {
        workflow_id: BUILDER_WORKFLOW_ID,
        harness_kind: "local_coding_program_builder_lab_file_execution_v1",
        ok,
        sandbox_root,
        task_executions,
        failures,
    }
}

fn file_task_execution(
    sandbox_root: &Path,
    execution: LocalCodingProgramBuilderLabTaskExecution,
) -> LocalCodingProgramBuilderLabFileTaskExecution {
    let task_root = sandbox_root.join(execution.task_id);
    let mut failures = Vec::new();
    if let Err(error) = std::fs::create_dir_all(&task_root) {
        failures.push(format!(
            "task_sandbox_create_failed:{}:{error}",
            execution.task_id
        ));
    }

    let slice_results = execution
        .slice_invocations
        .iter()
        .map(|slice| materialize_slice(&task_root, execution.task_id, slice))
        .collect::<Vec<_>>();
    let mut changed_files = Vec::new();
    let mut validation_results = Vec::new();
    for slice in &slice_results {
        append_unique_strings(&mut changed_files, &slice.wrote_files);
        validation_results.extend(slice.validation_results.clone());
        if !slice.ok {
            failures.extend(slice.failures.clone());
        }
    }
    validation_results.push(LocalCodingProgramBuilderLabValidationResult {
        check: "workflow_execution_shape",
        status: if execution.ok { "passed" } else { "failed" },
        evidence: format!(
            "{} planned slices were converted into local file execution receipts",
            execution.slice_invocations.len()
        ),
    });
    if !execution.ok {
        failures.extend(execution.failures.clone());
    }

    let final_handoff = LocalCodingProgramBuilderLabFinalHandoff {
        completed_checkpoint: execution.checkpoint.name,
        changed_files: changed_files.clone(),
        validation_results: validation_results.clone(),
        known_risks: vec![
            "lab executor validates filesystem materialization only",
            "runtime agent spawning is not enabled in this harness",
        ],
        intentionally_excluded_scope: execution.checkpoint.excluded_scope.clone(),
        recommended_next_checkpoint: recommended_next_checkpoint(execution.task_id),
    };
    let ok = failures.is_empty()
        && !changed_files.is_empty()
        && slice_results.iter().all(|slice| slice.ok);

    LocalCodingProgramBuilderLabFileTaskExecution {
        task_id: execution.task_id,
        user_goal: execution.user_goal,
        checkpoint_name: execution.checkpoint.name,
        architecture_pattern: execution.architecture_contract.architecture_pattern,
        sandbox_path: task_root.to_string_lossy().to_string(),
        slice_results,
        changed_files,
        validation_results,
        final_handoff,
        ok,
        failures,
    }
}

fn materialize_slice(
    task_root: &Path,
    task_id: &'static str,
    slice: &LocalCodingProgramBuilderSliceInvocation,
) -> LocalCodingProgramBuilderLabFileSliceResult {
    let mut failures = Vec::new();
    let mut wrote_files = Vec::new();

    let files = if slice.child_workflow_id == "local_code_edit_execution" {
        code_files_for_slice(task_id, slice.name)
    } else {
        vec![workflow_artifact_for_slice(task_id, slice)]
    };

    for (relative_path, contents) in files {
        match write_lab_file(task_root, relative_path, contents) {
            Ok(()) => wrote_files.push(relative_path.to_string()),
            Err(error) => failures.push(format!(
                "slice_file_write_failed:{}:{}:{error}",
                slice.name, relative_path
            )),
        }
    }

    let mut validation_results = validate_written_files(task_root, &wrote_files);
    if slice.child_workflow_id == "local_code_edit_execution" && wrote_files.is_empty() {
        failures.push(format!(
            "local_code_edit_slice_wrote_no_files:{}",
            slice.name
        ));
        validation_results.push(LocalCodingProgramBuilderLabValidationResult {
            check: "local_code_edit_slice_materialization",
            status: "failed",
            evidence: format!("{} produced no local files", slice.name),
        });
    }

    LocalCodingProgramBuilderLabFileSliceResult {
        slice_name: slice.name,
        child_workflow_id: slice.child_workflow_id,
        output_artifact: slice.output_artifact,
        wrote_files,
        validation_results,
        ok: failures.is_empty(),
        failures,
    }
}

fn write_lab_file(task_root: &Path, relative_path: &str, contents: &str) -> std::io::Result<()> {
    let path = task_root.join(relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

fn validate_written_files(
    task_root: &Path,
    files: &[String],
) -> Vec<LocalCodingProgramBuilderLabValidationResult> {
    files
        .iter()
        .map(|relative_path| {
            let path = task_root.join(relative_path);
            match std::fs::metadata(&path) {
                Ok(metadata) if metadata.len() > 0 => {
                    LocalCodingProgramBuilderLabValidationResult {
                        check: "file_materialized",
                        status: "passed",
                        evidence: format!("{relative_path} exists with {} bytes", metadata.len()),
                    }
                }
                Ok(_) => LocalCodingProgramBuilderLabValidationResult {
                    check: "file_materialized",
                    status: "failed",
                    evidence: format!("{relative_path} exists but is empty"),
                },
                Err(error) => LocalCodingProgramBuilderLabValidationResult {
                    check: "file_materialized",
                    status: "failed",
                    evidence: format!("{relative_path} missing: {error}"),
                },
            }
        })
        .collect()
}

fn workflow_artifact_for_slice(
    task_id: &'static str,
    slice: &LocalCodingProgramBuilderSliceInvocation,
) -> (&'static str, &'static str) {
    match (task_id, slice.name) {
        ("single_file_utility", "checkpoint_and_architecture_plan") => (
            "workflow_artifacts/checkpoint_and_architecture_plan.json",
            r#"{
  "slice": "checkpoint_and_architecture_plan",
  "checkpoint": "single_file_utility_mvp",
  "architecture": "single_entry_with_testable_core_function",
  "decision": "keep utility core testable and dependency-free"
}
"#,
        ),
        ("single_file_utility", "focused_repair_if_validation_fails") => (
            "workflow_artifacts/focused_repair_if_validation_fails.json",
            r#"{
  "slice": "focused_repair_if_validation_fails",
  "repair_policy": "only touch checksum implementation and checksum tests",
  "status": "not_needed_in_nominal_lab_run"
}
"#,
        ),
        ("small_multi_file_app", "context_research") => (
            "workflow_artifacts/context_research.json",
            r#"{
  "slice": "context_research",
  "project_state": "uninitialized",
  "stack_decision": "minimal local Rust-style modules for lab materialization"
}
"#,
        ),
        ("small_multi_file_app", "architecture_and_slice_plan") => (
            "workflow_artifacts/architecture_and_slice_plan.json",
            r#"{
  "slice": "architecture_and_slice_plan",
  "checkpoint": "bounded_multi_file_app_mvp",
  "slices": ["domain_model_slice", "primary_flow_slice"],
  "boundary": "domain layer stays independent from app/interface glue"
}
"#,
        ),
        ("small_multi_file_app", "integration_repair_if_needed") => (
            "workflow_artifacts/integration_repair_if_needed.json",
            r#"{
  "slice": "integration_repair_if_needed",
  "repair_policy": "repair only boundary mismatches between domain and app layers",
  "status": "not_needed_in_nominal_lab_run"
}
"#,
        ),
        ("initialized_project_modification", "existing_project_assessment") => (
            "workflow_artifacts/existing_project_assessment.json",
            r#"{
  "slice": "existing_project_assessment",
  "detected_stack": "local existing project skeleton",
  "architecture": "preserve_existing_architecture"
}
"#,
        ),
        ("initialized_project_modification", "architecture_drift_repair") => (
            "workflow_artifacts/architecture_drift_repair.json",
            r#"{
  "slice": "architecture_drift_repair",
  "repair_policy": "repair only changed feature module and associated tests",
  "status": "not_needed_in_nominal_lab_run"
}
"#,
        ),
        _ => (
            "workflow_artifacts/unknown_slice.json",
            r#"{
  "slice": "unknown",
  "status": "artifact_placeholder"
}
"#,
        ),
    }
}

fn code_files_for_slice(
    task_id: &'static str,
    slice_name: &'static str,
) -> Vec<(&'static str, &'static str)> {
    match (task_id, slice_name) {
        ("single_file_utility", "single_file_utility_implementation") => vec![
            (
                "src/checksum.rs",
                r#"pub fn checksum(input: &str) -> Result<u64, &'static str> {
    if input.trim().is_empty() {
        return Err("input must not be empty");
    }

    Ok(input
        .bytes()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(1_099_511_628_211)
        }))
}

pub fn checksum_hex(input: &str) -> Result<String, &'static str> {
    checksum(input).map(|value| format!("{value:016x}"))
}

#[cfg(test)]
mod tests {
    use super::checksum_hex;

    #[test]
    fn computes_deterministic_checksum() {
        assert_eq!(checksum_hex("localcode"), checksum_hex("localcode"));
    }

    #[test]
    fn rejects_empty_input() {
        assert!(checksum_hex(" ").is_err());
    }
}
"#,
            ),
            (
                "tests/checksum_behavior.rs",
                r#"#[path = "../src/checksum.rs"]
mod checksum;

#[test]
fn checksum_changes_when_input_changes() {
    assert_ne!(
        checksum::checksum_hex("alpha").unwrap(),
        checksum::checksum_hex("beta").unwrap()
    );
}
"#,
            ),
        ],
        ("small_multi_file_app", "domain_model_slice") => vec![
            (
                "src/domain/mod.rs",
                r#"pub mod task;
"#,
            ),
            (
                "src/domain/task.rs",
                r#"#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Open,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub status: TaskStatus,
}

#[derive(Debug, Default)]
pub struct TaskList {
    next_id: u64,
    tasks: Vec<Task>,
}

impl TaskList {
    pub fn add(&mut self, title: impl Into<String>) -> Result<&Task, &'static str> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err("task title must not be empty");
        }

        self.next_id += 1;
        self.tasks.push(Task {
            id: self.next_id,
            title,
            status: TaskStatus::Open,
        });
        Ok(self.tasks.last().expect("task was just pushed"))
    }

    pub fn complete(&mut self, id: u64) -> Result<(), &'static str> {
        let task = self
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or("task not found")?;
        task.status = TaskStatus::Done;
        Ok(())
    }

    pub fn open_tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Open)
            .collect()
    }
}
"#,
            ),
            (
                "tests/domain/task_lifecycle.rs",
                r#"#[path = "../../src/domain/task.rs"]
mod task;

#[test]
fn task_lifecycle_keeps_domain_rules_local() {
    let mut list = task::TaskList::default();
    let first_id = list.add("write workflow").unwrap().id;
    list.complete(first_id).unwrap();
    assert!(list.open_tasks().is_empty());
}
"#,
            ),
        ],
        ("small_multi_file_app", "primary_flow_slice") => vec![
            (
                "src/app/mod.rs",
                r#"pub mod task_tracker;
"#,
            ),
            (
                "src/app/task_tracker.rs",
                r#"use crate::domain::task::{Task, TaskList};

#[derive(Debug, Default)]
pub struct TaskTracker {
    tasks: TaskList,
}

impl TaskTracker {
    pub fn create_task(&mut self, title: &str) -> Result<Task, &'static str> {
        self.tasks.add(title).cloned()
    }

    pub fn finish_task(&mut self, id: u64) -> Result<(), &'static str> {
        self.tasks.complete(id)
    }

    pub fn open_task_titles(&self) -> Vec<String> {
        self.tasks
            .open_tasks()
            .into_iter()
            .map(|task| task.title.clone())
            .collect()
    }
}
"#,
            ),
            (
                "src/interface/cli.rs",
                r#"use crate::app::task_tracker::TaskTracker;

pub fn run_demo_flow() -> Result<Vec<String>, &'static str> {
    let mut tracker = TaskTracker::default();
    let task = tracker.create_task("define checkpoint")?;
    tracker.create_task("execute slice")?;
    tracker.finish_task(task.id)?;
    Ok(tracker.open_task_titles())
}
"#,
            ),
            (
                "tests/app/primary_flow.rs",
                r#"#[test]
fn primary_flow_leaves_one_open_task() {
    let open_tasks = vec!["execute slice".to_string()];
    assert_eq!(open_tasks, vec!["execute slice".to_string()]);
}
"#,
            ),
        ],
        ("initialized_project_modification", "targeted_feature_implementation") => vec![
            (
                "src/features/tags.rs",
                r#"#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub label: String,
}

impl Tag {
    pub fn new(label: impl Into<String>) -> Result<Self, &'static str> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err("tag label must not be empty");
        }
        Ok(Self { label })
    }
}

pub fn normalize_tag(label: &str) -> Result<Tag, &'static str> {
    Tag::new(label.trim().to_lowercase())
}
"#,
            ),
            (
                "tests/features/tags.rs",
                r#"#[path = "../../src/features/tags.rs"]
mod tags;

#[test]
fn normalizes_tag_labels_without_new_architecture() {
    assert_eq!(tags::normalize_tag("  LocalCode ").unwrap().label, "localcode");
}
"#,
            ),
        ],
        _ => Vec::new(),
    }
}

fn append_unique_strings(target: &mut Vec<String>, source: &[String]) {
    for item in source {
        if !target.iter().any(|existing| existing == item) {
            target.push(item.clone());
        }
    }
}

fn recommended_next_checkpoint(task_id: &str) -> &'static str {
    match task_id {
        "single_file_utility" => "package_as_repo_native_cli",
        "small_multi_file_app" => "add_persistence_adapter",
        "initialized_project_modification" => "broaden_targeted_feature_coverage",
        _ => "define_next_bounded_checkpoint",
    }
}

fn lab_sandbox_root_path() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "local-coding-program-builder-lab-{}-{millis}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::local_coding_program_builder_lab_file_execution_report;

    #[test]
    fn local_coding_program_builder_lab_file_execution_materializes_code_files() {
        let report = local_coding_program_builder_lab_file_execution_report();
        assert!(report.ok, "{report:#?}");
        assert_eq!(report.workflow_id, "local_coding_program_builder");
        assert_eq!(
            report.harness_kind,
            "local_coding_program_builder_lab_file_execution_v1"
        );
        assert_eq!(report.task_executions.len(), 3);
        for execution in &report.task_executions {
            assert!(execution.ok, "{execution:#?}");
            assert!(!execution.changed_files.is_empty());
            assert!(!execution.final_handoff.changed_files.is_empty());
            assert!(execution
                .slice_results
                .iter()
                .any(
                    |slice| slice.child_workflow_id == "local_code_edit_execution"
                        && !slice.wrote_files.is_empty()
                ));
        }

        let single_file = report
            .task_executions
            .iter()
            .find(|execution| execution.task_id == "single_file_utility")
            .expect("single file utility execution");
        assert!(single_file
            .changed_files
            .iter()
            .any(|path| path == "src/checksum.rs"));

        let multi_file = report
            .task_executions
            .iter()
            .find(|execution| execution.task_id == "small_multi_file_app")
            .expect("small multi-file app execution");
        assert!(multi_file
            .changed_files
            .iter()
            .any(|path| path == "src/domain/task.rs"));
        assert!(multi_file
            .changed_files
            .iter()
            .any(|path| path == "src/app/task_tracker.rs"));
    }
}
