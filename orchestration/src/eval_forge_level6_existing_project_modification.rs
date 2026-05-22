// Layer ownership: eval/observability (existing-project Level 6 coding workflow analysis).
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct ForgeLevel6ExistingProjectReport {
    pub harness_kind: &'static str,
    pub analysis_target_kind: &'static str,
    pub candidate_root: String,
    pub ok: bool,
    pub decision: &'static str,
    pub dimensions: Vec<ForgeLevel6Dimension>,
    pub preservation_gaps: Vec<String>,
    pub planning_gaps: Vec<String>,
    pub implementation_gaps: Vec<String>,
    pub validation_gaps: Vec<String>,
    pub evidence_gaps: Vec<String>,
    pub operator_next_action: &'static str,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeLevel6Dimension {
    pub dimension: &'static str,
    pub status: &'static str,
    pub checks: Vec<ForgeLevel6Check>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeLevel6Check {
    pub id: &'static str,
    pub ok: bool,
    pub severity: &'static str,
    pub detail: String,
}

pub fn forge_level6_existing_project_fixture_report() -> ForgeLevel6ExistingProjectReport {
    let root = fixture_root_path();
    let mut seed_failures = Vec::new();
    if let Err(error) = fs::create_dir_all(&root) {
        seed_failures.push(format!("fixture_root_create_failed:{error}"));
    }
    seed_level6_fixture(&root, &mut seed_failures);
    let mut report = forge_level6_existing_project_report_for_path_kind(&root, "generated_fixture");
    report.failures.extend(seed_failures);
    report.ok = report.ok && report.failures.is_empty();
    report
}

pub fn forge_level6_existing_project_report_for_path(
    candidate_root: &Path,
) -> ForgeLevel6ExistingProjectReport {
    forge_level6_existing_project_report_for_path_kind(candidate_root, "provided_candidate")
}

fn forge_level6_existing_project_report_for_path_kind(
    candidate_root: &Path,
    analysis_target_kind: &'static str,
) -> ForgeLevel6ExistingProjectReport {
    let root = candidate_root.to_path_buf();
    let all_files = collect_files(&root);
    let source_files = all_files
        .iter()
        .filter(|path| {
            extension_in(path, &["rs", "py", "js", "rb", "java", "ts"])
                && !path_components_contain(path, "target")
                && !path_components_contain(path, "build")
                && !path_components_contain(path, "node_modules")
        })
        .cloned()
        .collect::<Vec<_>>();
    let test_files = all_files
        .iter()
        .filter(|path| {
            path_components_contain(path, "test") || path_components_contain(path, "tests")
        })
        .cloned()
        .collect::<Vec<_>>();
    let receipt_files = all_files
        .iter()
        .filter(|path| {
            let name = file_name_lower(path);
            name.contains("receipt") || name.contains("validation")
        })
        .cloned()
        .collect::<Vec<_>>();
    let workflow_files = all_files
        .iter()
        .filter(|path| {
            path_components_contain(path, "workflow_artifacts")
                || path_components_contain(path, "receipts")
        })
        .cloned()
        .collect::<Vec<_>>();

    let source_text = combined_text(&source_files);
    let test_text = combined_text(&test_files);
    let receipt_text = combined_text(&receipt_files);
    let workflow_text = combined_text(&workflow_files);
    let architecture_text = read_optional(&root.join("ARCHITECTURE.md"))
        + "\n"
        + &read_optional(&root.join("docs/ARCHITECTURE.md"));

    let preservation_checks = vec![
        check(
            "existing_project_markers_present",
            contains_any(
                &architecture_text,
                &[
                    "existing project",
                    "baseline behavior",
                    "preserve",
                    "architecture",
                ],
            ),
            "error",
            "Level 6 candidates must document the pre-existing architecture and preservation obligations"
                .to_string(),
        ),
        check(
            "baseline_regression_tests_present",
            contains_any(
                &test_text,
                &[
                    "baseline",
                    "regression",
                    "existing behavior",
                    "preserve",
                    "backward",
                ],
            ),
            "error",
            "tests should prove pre-existing behavior still works after the modification".to_string(),
        ),
        check(
            "unrelated_file_preservation_evidence_present",
            contains_any(
                &workflow_text,
                &[
                    "unrelated_file_preservation",
                    "preserved_files",
                    "unchanged_files",
                    "do_not_modify",
                ],
            ),
            "error",
            "receipts should show unrelated files were preserved".to_string(),
        ),
    ];

    let planning_checks = vec![
        check(
            "strategic_slice_handoff_present",
            contains_any(
                &workflow_text,
                &[
                    "slice_handoff",
                    "immutable_decisions",
                    "parent_owned_decisions",
                ],
            ),
            "error",
            "Level 6 should include a parent-owned strategic handoff before tactical execution"
                .to_string(),
        ),
        check(
            "planning_confidence_recorded",
            contains_any(
                &workflow_text,
                &["planning_confidence_score", "planning_confidence"],
            ),
            "error",
            "planning confidence score should be recorded before execution".to_string(),
        ),
        check(
            "tactical_readiness_recorded",
            contains_any(
                &workflow_text,
                &["tactical_readiness_score", "slice_execution_readiness"],
            ),
            "error",
            "ForgeCode tactical readiness should be recorded before edits".to_string(),
        ),
        check(
            "multi_slice_execution_evidence_present",
            count_occurrences(&workflow_text, "slice_id") >= 2
                || count_files_with_name_containing(&workflow_files, "slice") >= 2,
            "error",
            "Level 6 should show multiple coordinated slices rather than one broad edit"
                .to_string(),
        ),
    ];

    let implementation_checks = vec![
        check(
            "existing_project_source_present",
            source_files.len() >= 4,
            "error",
            format!("source_file_count={}", source_files.len()),
        ),
        check(
            "feature_spans_multiple_concerns",
            contains_any(&source_text, &["recurring", "schedule", "template"])
                && contains_any(&source_text, &["import", "csv"])
                && contains_any(&source_text, &["export", "json"])
                && contains_any(&source_text, &["overdue", "due"]),
            "error",
            "Level 6 fixture expects recurring, import/export, and overdue-report behavior"
                .to_string(),
        ),
        check(
            "migration_or_versioning_present",
            contains_any(
                &source_text,
                &["schema_version", "migration", "migrate", "store_version"],
            ),
            "error",
            "existing project changes should include migration/version compatibility".to_string(),
        ),
        check(
            "architecture_boundaries_preserved",
            architecture_boundaries_preserved(&source_files, &source_text),
            "error",
            "candidate should preserve module/layer boundaries rather than collapsing into one file"
                .to_string(),
        ),
    ];

    let validation_checks = vec![
        check(
            "validation_receipt_present",
            !receipt_files.is_empty(),
            "error",
            format!("receipt_files={}", display_paths(&receipt_files).join(",")),
        ),
        check(
            "validation_receipt_indicates_pass",
            contains_any(
                &receipt_text,
                &[
                    "PASS",
                    "pass",
                    "0 failures",
                    "0 failed",
                    "exit 0",
                    "exit code 0",
                ],
            ),
            "error",
            "validation receipt should indicate passing tests or smoke validation".to_string(),
        ),
        check(
            "regression_and_feature_tests_present",
            contains_any(&test_text, &["regression", "baseline", "existing"])
                && contains_any(
                    &test_text,
                    &["recurring", "import", "export", "overdue", "migration"],
                ),
            "error",
            "tests should cover both preserved behavior and new feature behavior".to_string(),
        ),
    ];

    let evidence_checks = vec![
        check(
            "changed_file_receipt_present",
            contains_any(
                &workflow_text,
                &["changed_files", "file_change_receipt", "files_changed"],
            ),
            "error",
            "file-change evidence is required for existing-project modification".to_string(),
        ),
        check(
            "repair_policy_or_event_recorded",
            contains_any(
                &workflow_text,
                &[
                    "repair_events",
                    "bounded_repair",
                    "max_repair_attempts",
                    "repair_policy",
                ],
            ),
            "error",
            "Level 6 should record repair policy or repair events".to_string(),
        ),
        check(
            "checkpoint_handoff_present",
            contains_any(
                &workflow_text,
                &[
                    "checkpoint_handoff",
                    "recommended_next_checkpoint",
                    "final_checkpoint",
                ],
            ),
            "error",
            "candidate should return a checkpoint handoff instead of open-ended coding".to_string(),
        ),
    ];

    let dimensions = vec![
        dimension("existing_project_preservation", preservation_checks),
        dimension("planning_and_handoff_control", planning_checks),
        dimension("multi_slice_implementation", implementation_checks),
        dimension("regression_validation", validation_checks),
        dimension("evidence_and_checkpoint_quality", evidence_checks),
    ];

    let mut preservation_gaps = Vec::new();
    let mut planning_gaps = Vec::new();
    let mut implementation_gaps = Vec::new();
    let mut validation_gaps = Vec::new();
    let mut evidence_gaps = Vec::new();

    collect_gaps(&dimensions[0], &mut preservation_gaps);
    collect_gaps(&dimensions[1], &mut planning_gaps);
    collect_gaps(&dimensions[2], &mut implementation_gaps);
    collect_gaps(&dimensions[3], &mut validation_gaps);
    collect_gaps(&dimensions[4], &mut evidence_gaps);

    let failures = [
        preservation_gaps.clone(),
        planning_gaps.clone(),
        implementation_gaps.clone(),
        validation_gaps.clone(),
        evidence_gaps.clone(),
    ]
    .concat();
    let ok = failures.is_empty();

    ForgeLevel6ExistingProjectReport {
        harness_kind: "forge_level6_existing_project_modification_v1",
        analysis_target_kind,
        candidate_root: root.display().to_string(),
        ok,
        decision: if ok {
            "level6_analysis_pass_single_attempt_not_promotion_proof"
        } else {
            "level6_analysis_failed_existing_project_modification_contract"
        },
        dimensions,
        preservation_gaps,
        planning_gaps,
        implementation_gaps,
        validation_gaps,
        evidence_gaps,
        operator_next_action: "run_3_to_5_attempt_level6_live_eval_probe",
        failures,
    }
}

fn dimension(dimension: &'static str, checks: Vec<ForgeLevel6Check>) -> ForgeLevel6Dimension {
    ForgeLevel6Dimension {
        dimension,
        status: if checks.iter().all(|check| check.ok) {
            "pass"
        } else {
            "fail"
        },
        checks,
    }
}

fn check(id: &'static str, ok: bool, severity: &'static str, detail: String) -> ForgeLevel6Check {
    ForgeLevel6Check {
        id,
        ok,
        severity,
        detail,
    }
}

fn collect_gaps(dimension: &ForgeLevel6Dimension, gaps: &mut Vec<String>) {
    for check in &dimension.checks {
        if !check.ok && check.severity == "error" {
            gaps.push(format!("{}:{}", dimension.dimension, check.id));
        }
    }
}

fn architecture_boundaries_preserved(source_files: &[PathBuf], source_text: &str) -> bool {
    let file_names = source_files
        .iter()
        .map(|path| file_name_lower(path))
        .collect::<Vec<_>>();
    let named_boundaries = ["domain", "service", "store", "repository", "cli", "reports"]
        .iter()
        .filter(|needle| file_names.iter().any(|name| name.contains(*needle)))
        .count();
    named_boundaries >= 3
        || (contains_any(source_text, &["mod domain", "class Domain", "class Store"])
            && contains_any(source_text, &["service", "Service", "report", "Report"]))
}

fn fixture_root_path() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "forge-level6-existing-project-{}-{millis}",
        std::process::id()
    ))
}

fn seed_level6_fixture(root: &Path, failures: &mut Vec<String>) {
    let files = [
        (
            "ARCHITECTURE.md",
            "# Existing project architecture\n\nThis existing project is a task ledger CLI. Preserve baseline behavior for task creation, completion, listing, and storage while adding recurring tasks, import/export, migration, and overdue reporting.\n",
        ),
        (
            "src/domain/task.py",
            "from dataclasses import dataclass\n\n@dataclass\nclass Task:\n    id: str\n    title: str\n    completed: bool = False\n    due_on: str = ''\n    recurring_template_id: str = ''\n\ndef create_task(id, title, due_on=''):\n    return Task(id=id, title=title, due_on=due_on)\n\ndef complete_task(task):\n    task.completed = True\n    return task\n\ndef materialize_recurring(template, next_id):\n    return Task(id=next_id, title=template['title'], due_on=template['next_due'], recurring_template_id=template['id'])\n",
        ),
        (
            "src/store/repository.py",
            "import json\n\nschema_version = 2\n\ndef migrate(data):\n    data.setdefault('schema_version', schema_version)\n    data.setdefault('tasks', [])\n    data.setdefault('recurring_templates', [])\n    return data\n\ndef save(path, data):\n    path.write_text(json.dumps(migrate(data), indent=2))\n\ndef load(path):\n    if not path.exists():\n        return migrate({})\n    return migrate(json.loads(path.read_text()))\n",
        ),
        (
            "src/app/service.py",
            "import csv, json\nfrom src.domain.task import create_task, materialize_recurring\n\nclass TaskLedgerService:\n    def __init__(self, repo_path, repo):\n        self.repo_path = repo_path\n        self.repo = repo\n\n    def import_csv(self, csv_path):\n        data = self.repo.load(self.repo_path)\n        for row in csv.DictReader(open(csv_path)):\n            data['tasks'].append(create_task(row['id'], row['title'], row.get('due_on', '')).__dict__)\n        self.repo.save(self.repo_path, data)\n        return len(data['tasks'])\n\n    def export_json(self, export_path):\n        data = self.repo.load(self.repo_path)\n        export_path.write_text(json.dumps(data, indent=2))\n        return export_path\n\n    def generate_recurring(self):\n        data = self.repo.load(self.repo_path)\n        for template in data.get('recurring_templates', []):\n            data['tasks'].append(materialize_recurring(template, 'task-' + template['id']).__dict__)\n        self.repo.save(self.repo_path, data)\n        return data\n\n    def overdue_report(self, as_of):\n        data = self.repo.load(self.repo_path)\n        return [task for task in data['tasks'] if task.get('due_on') and task['due_on'] < as_of and not task.get('completed')]\n",
        ),
        (
            "src/interface/cli.py",
            "from pathlib import Path\nfrom src.app.service import TaskLedgerService\nfrom src.store import repository\n\ndef main(argv):\n    ledger = Path(argv[1])\n    service = TaskLedgerService(ledger, repository)\n    if argv[0] == 'import':\n        return service.import_csv(Path(argv[2]))\n    if argv[0] == 'export':\n        return service.export_json(Path(argv[2]))\n    if argv[0] == 'overdue':\n        return service.overdue_report(argv[2])\n    if argv[0] == 'generate-recurring':\n        return service.generate_recurring()\n    raise SystemExit('unknown command')\n",
        ),
        (
            "tests/test_existing_behavior_and_level6_feature.py",
            "from pathlib import Path\nimport tempfile\nfrom src.domain.task import create_task, complete_task\nfrom src.store import repository\nfrom src.app.service import TaskLedgerService\n\ndef test_baseline_existing_behavior_preserved():\n    task = create_task('t1', 'ship')\n    assert complete_task(task).completed\n\ndef test_regression_import_export_overdue_and_migration():\n    with tempfile.TemporaryDirectory() as tmp:\n        root = Path(tmp)\n        csv_path = root / 'tasks.csv'\n        csv_path.write_text('id,title,due_on\\nt1,Call,2026-01-01\\n')\n        ledger = root / 'ledger.json'\n        service = TaskLedgerService(ledger, repository)\n        service.import_csv(csv_path)\n        assert repository.load(ledger)['schema_version'] == 2\n        assert service.overdue_report('2026-02-01')[0]['title'] == 'Call'\n        export_path = root / 'export.json'\n        service.export_json(export_path)\n        assert export_path.exists()\n\ndef test_recurring_feature_generates_tasks():\n    with tempfile.TemporaryDirectory() as tmp:\n        ledger = Path(tmp) / 'ledger.json'\n        repository.save(ledger, {'recurring_templates': [{'id': 'weekly', 'title': 'Review', 'next_due': '2026-03-01'}]})\n        service = TaskLedgerService(ledger, repository)\n        assert service.generate_recurring()['tasks'][0]['recurring_template_id'] == 'weekly'\n",
        ),
        (
            "fixtures/tasks.csv",
            "id,title,due_on\nt1,Call donor,2026-01-01\n",
        ),
        (
            "workflow_artifacts/slice_handoff.json",
            r#"{
  "slice_handoff": true,
  "planning_confidence_score": 0.91,
  "immutable_decisions": ["existing task ledger architecture", "python stdlib", "preserve baseline behavior"],
  "parent_owned_decisions": ["product_goal", "acceptance_criteria", "checkpoint_scope"],
  "tactical_readiness_score": 0.93
}
"#,
        ),
        (
            "receipts/slice_1_context_and_migration.json",
            r#"{"slice_id":"context_and_migration","changed_files":["src/store/repository.py"],"preserved_files":["src/domain/task.py"],"unrelated_file_preservation":true}"#,
        ),
        (
            "receipts/slice_2_feature_and_reports.json",
            r#"{"slice_id":"feature_and_reports","changed_files":["src/app/service.py","src/interface/cli.py"],"repair_events":[],"bounded_repair":{"max_repair_attempts":1}}"#,
        ),
        (
            "receipts/checkpoint_handoff.json",
            r#"{"checkpoint_handoff":true,"recommended_next_checkpoint":"live Level 6 existing project probe","files_changed":["src/store/repository.py","src/app/service.py","src/interface/cli.py"]}"#,
        ),
        (
            "VALIDATION_RECEIPT.md",
            "PASS\n\nCommand: python3 -m pytest tests\nResult: 3 passed, 0 failures, exit code 0.\n",
        ),
    ];

    for (relative, content) in files {
        if let Err(error) = write_file(&root.join(relative), content) {
            failures.push(format!("seed_file_failed:{relative}:{error}"));
        }
    }
}

fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_inner(root, &mut files, 0);
    files.sort();
    files
}

fn collect_files_inner(root: &Path, files: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = file_name_lower(&path);
        if name == ".git" || name == "node_modules" || name == "target" || name == "build" {
            continue;
        }
        if path.is_dir() {
            collect_files_inner(&path, files, depth + 1);
        } else {
            files.push(path);
        }
    }
}

fn extension_in(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| allowed.iter().any(|allowed| extension == *allowed))
        .unwrap_or(false)
}

fn path_components_contain(path: &Path, needle: &str) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(needle)
    })
}

fn file_name_lower(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn read_optional(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn combined_text(files: &[PathBuf]) -> String {
    files
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn count_occurrences(text: &str, needle: &str) -> usize {
    text.match_indices(needle).count()
}

fn count_files_with_name_containing(files: &[PathBuf], needle: &str) -> usize {
    files
        .iter()
        .filter(|path| file_name_lower(path).contains(needle))
        .count()
}

fn display_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        forge_level6_existing_project_fixture_report, forge_level6_existing_project_report_for_path,
    };

    #[test]
    fn level6_fixture_passes_existing_project_contract() {
        let report = forge_level6_existing_project_fixture_report();
        assert!(report.ok, "{report:#?}");
        assert_eq!(
            report.decision,
            "level6_analysis_pass_single_attempt_not_promotion_proof"
        );
    }

    #[test]
    fn missing_candidate_fails_existing_project_contract() {
        let report = forge_level6_existing_project_report_for_path(std::path::Path::new(
            "/tmp/nonexistent-forge-level6-candidate",
        ));
        assert!(!report.ok);
        assert!(!report.failures.is_empty());
    }
}
