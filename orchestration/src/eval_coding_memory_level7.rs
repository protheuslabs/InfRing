use crate::coding_memory_runtime_bridge::{
    decide_memory_freshness, millis_now, project_snapshot, CodingMemoryRuntimeBridge,
    MemoryCommandResult, ProjectContextSnapshot,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct CodingMemoryLevel7Report {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub probe_count: usize,
    pub probes: Vec<CodingMemoryLevel7ProbeReport>,
    pub failures: Vec<String>,
    pub operator_next_action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodingMemoryLevel7ProbeReport {
    pub probe_id: &'static str,
    pub ok: bool,
    pub project_root: String,
    pub memory_db_path: String,
    pub checks: Vec<CodingMemoryLevel7Check>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodingMemoryLevel7Check {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct ProbeSpec {
    id: &'static str,
    package: &'static str,
    architecture: &'static str,
    manifest: &'static str,
    service_body: &'static str,
    baseline_test: &'static str,
}

const PROBES: &[ProbeSpec] = &[
    ProbeSpec {
        id: "python_task_ledger_resume",
        package: "task_ledger",
        architecture: "Existing project architecture: Python task ledger with domain/service/test boundaries. Preserve baseline behavior and read current files before implementation.",
        manifest: "python-stdlib unittest task-ledger",
        service_body: "def baseline_value():\n    return 'task-ledger-baseline'\n",
        baseline_test: "import unittest\nfrom task_ledger.service import baseline_value\n\nclass BaselineRegressionTest(unittest.TestCase):\n    def test_baseline_existing_behavior_is_preserved(self):\n        self.assertEqual(baseline_value(), 'task-ledger-baseline')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    },
    ProbeSpec {
        id: "python_clinic_resume",
        package: "clinic_ops",
        architecture: "Existing project architecture: Python clinic operations package with service/test boundaries. Preserve baseline behavior and current architecture.",
        manifest: "python-stdlib unittest clinic-ops",
        service_body: "def baseline_value():\n    return 'clinic-baseline'\n",
        baseline_test: "import unittest\nfrom clinic_ops.service import baseline_value\n\nclass BaselineRegressionTest(unittest.TestCase):\n    def test_baseline_existing_behavior_is_preserved(self):\n        self.assertEqual(baseline_value(), 'clinic-baseline')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    },
    ProbeSpec {
        id: "python_inventory_resume",
        package: "inventory_ops",
        architecture: "Existing project architecture: Python inventory audit package with service/test boundaries. Preserve baseline behavior and downgrade stale memory.",
        manifest: "python-stdlib unittest inventory-ops",
        service_body: "def baseline_value():\n    return 'inventory-baseline'\n",
        baseline_test: "import unittest\nfrom inventory_ops.service import baseline_value\n\nclass BaselineRegressionTest(unittest.TestCase):\n    def test_baseline_existing_behavior_is_preserved(self):\n        self.assertEqual(baseline_value(), 'inventory-baseline')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    },
    ProbeSpec {
        id: "python_billing_resume",
        package: "billing_ops",
        architecture: "Existing project architecture: Python billing operations package with service/test boundaries. Preserve baseline invoicing behavior and use memory only after checking current files.",
        manifest: "python-stdlib unittest billing-ops",
        service_body: "def baseline_value():\n    return 'billing-baseline'\n",
        baseline_test: "import unittest\nfrom billing_ops.service import baseline_value\n\nclass BaselineRegressionTest(unittest.TestCase):\n    def test_baseline_existing_behavior_is_preserved(self):\n        self.assertEqual(baseline_value(), 'billing-baseline')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    },
    ProbeSpec {
        id: "python_support_resume",
        package: "support_queue",
        architecture: "Existing project architecture: Python support queue package with service/test boundaries. Preserve baseline ticket behavior and downgrade stale resume context.",
        manifest: "python-stdlib unittest support-queue",
        service_body: "def baseline_value():\n    return 'support-baseline'\n",
        baseline_test: "import unittest\nfrom support_queue.service import baseline_value\n\nclass BaselineRegressionTest(unittest.TestCase):\n    def test_baseline_existing_behavior_is_preserved(self):\n        self.assertEqual(baseline_value(), 'support-baseline')\n\nif __name__ == '__main__':\n    unittest.main()\n",
    },
];

pub fn coding_memory_level7_report() -> CodingMemoryLevel7Report {
    let mut probes = Vec::new();
    let mut failures = Vec::new();
    for spec in PROBES {
        let probe = run_probe(spec);
        if !probe.ok {
            failures.extend(
                probe
                    .failures
                    .iter()
                    .map(|failure| format!("{}:{failure}", spec.id)),
            );
        }
        probes.push(probe);
    }
    CodingMemoryLevel7Report {
        harness_kind: "coding_memory_level7_resume_eval_v1",
        ok: failures.is_empty(),
        probe_count: probes.len(),
        probes,
        failures,
        operator_next_action: "run_agent_level7_live_resume_probe_with_real_coding_workers",
    }
}

fn run_probe(spec: &ProbeSpec) -> CodingMemoryLevel7ProbeReport {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let project_root = temp_project_root(spec.id);
    let seed_result = seed_project(spec, &project_root);
    push_check(
        &mut checks,
        &mut failures,
        "seed_existing_project",
        seed_result.is_ok(),
        seed_result.unwrap_or_else(|error| error),
    );

    let validation_0 = run_python_validation(&project_root);
    push_check(
        &mut checks,
        &mut failures,
        "baseline_validation_passes",
        validation_0.ok,
        validation_0.detail,
    );

    let checkpoint_one_result = write_checkpoint_one(spec, &project_root);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_one_code_written",
        checkpoint_one_result.is_ok(),
        checkpoint_one_result.unwrap_or_else(|error| error),
    );
    let validation_1 = run_python_validation(&project_root);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_one_validation_passes",
        validation_1.ok,
        validation_1.detail,
    );

    let architecture_text = read_to_string(&project_root.join("ARCHITECTURE.md"));
    let current_snapshot = project_snapshot(
        spec.id,
        &project_root,
        &architecture_text,
        spec.manifest,
        "PYTHONPATH=src python3 -m unittest discover -s tests",
    );
    let bridge = CodingMemoryRuntimeBridge::isolated(spec.id);
    let phase_one_token = format!("level7_phase_one_token_{}_{}", spec.id, millis_now());
    let checkpoint_one_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_001",
        current_snapshot.project_fingerprint
    );
    let context_id = format!(
        "coding_memory::{}::project_context::latest",
        current_snapshot.project_fingerprint
    );
    let context_write = bridge.ingest(
        &context_id,
        &memory_context_payload(spec, &current_snapshot, &phase_one_token),
        &["coding", "project_context", "resume"],
    );
    push_memory_check(
        &mut checks,
        &mut failures,
        "project_context_memory_write",
        &context_write,
    );
    let checkpoint_one_write = bridge.ingest(
        &checkpoint_one_id,
        &checkpoint_payload(
            "checkpoint_001",
            &current_snapshot,
            &phase_one_token,
            &[
                "src/<package>/checkpoint_one.py",
                "tests/test_checkpoint_one.py",
            ],
            "resume_to_checkpoint_two",
        ),
        &["coding", "checkpoint", "resume", "project_context"],
    );
    push_memory_check(
        &mut checks,
        &mut failures,
        "checkpoint_one_memory_write",
        &checkpoint_one_write,
    );

    let resumed_bridge = bridge.resume_from(&format!("{}_resumed", spec.id));
    let recall = resumed_bridge.recall(&phase_one_token, 5);
    let recall_found = memory_result_contains_id(&recall, &checkpoint_one_id);
    push_check(
        &mut checks,
        &mut failures,
        "fresh_session_retrieves_checkpoint_memory",
        recall.ok && recall_found,
        format!(
            "ok={} hit_count={}",
            recall.ok,
            recall
                .payload
                .get("hit_count")
                .and_then(Value::as_u64)
                .unwrap_or_default()
        ),
    );
    let get_checkpoint = resumed_bridge.get(&checkpoint_one_id);
    push_check(
        &mut checks,
        &mut failures,
        "fresh_session_gets_checkpoint_by_id",
        get_checkpoint.ok && memory_result_content_contains(&get_checkpoint, &phase_one_token),
        format!("ok={}", get_checkpoint.ok),
    );
    let current_files_read = read_to_string(&project_root.join("ARCHITECTURE.md"))
        .contains("Existing project architecture");
    push_check(
        &mut checks,
        &mut failures,
        "resume_still_reads_current_files",
        current_files_read,
        "ARCHITECTURE.md read during resume phase".to_string(),
    );
    let freshness = decide_memory_freshness(
        &current_snapshot,
        Some(&current_snapshot.project_fingerprint),
        Some(&current_snapshot.architecture_hash),
    );
    push_check(
        &mut checks,
        &mut failures,
        "fresh_memory_can_seed_resume_planning",
        freshness.status == "fresh" && freshness.current_files_source_of_truth,
        format!(
            "status={} allowed_use={}",
            freshness.status, freshness.allowed_memory_use
        ),
    );

    let checkpoint_two_result = write_checkpoint_two(spec, &project_root);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_two_resume_code_written",
        checkpoint_two_result.is_ok(),
        checkpoint_two_result.unwrap_or_else(|error| error),
    );
    let validation_2 = run_python_validation(&project_root);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_two_validation_passes",
        validation_2.ok,
        validation_2.detail,
    );
    let phase_two_token = format!("level7_phase_two_token_{}_{}", spec.id, millis_now());
    let checkpoint_two_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_002",
        current_snapshot.project_fingerprint
    );
    let checkpoint_two_write = resumed_bridge.ingest(
        &checkpoint_two_id,
        &checkpoint_payload(
            "checkpoint_002",
            &current_snapshot,
            &phase_two_token,
            &[
                "src/<package>/checkpoint_two.py",
                "tests/test_checkpoint_two.py",
            ],
            "next_resumable_slice",
        ),
        &["coding", "checkpoint", "resume", "project_context"],
    );
    push_memory_check(
        &mut checks,
        &mut failures,
        "checkpoint_two_memory_write",
        &checkpoint_two_write,
    );
    let recall_two = resumed_bridge.recall(&phase_two_token, 5);
    push_check(
        &mut checks,
        &mut failures,
        "updated_checkpoint_memory_recall",
        recall_two.ok && memory_result_contains_id(&recall_two, &checkpoint_two_id),
        format!(
            "ok={} hit_count={}",
            recall_two.ok,
            recall_two
                .payload
                .get("hit_count")
                .and_then(Value::as_u64)
                .unwrap_or_default()
        ),
    );

    let stale_architecture =
        format!("{architecture_text}\nChanged architecture marker for stale memory.");
    let stale_snapshot = project_snapshot(
        spec.id,
        &project_root,
        &stale_architecture,
        spec.manifest,
        "PYTHONPATH=src python3 -m unittest discover -s tests",
    );
    let stale_decision = decide_memory_freshness(
        &stale_snapshot,
        Some(&current_snapshot.project_fingerprint),
        Some(&current_snapshot.architecture_hash),
    );
    push_check(
        &mut checks,
        &mut failures,
        "stale_memory_downgraded_or_ignored",
        matches!(
            stale_decision.status,
            "stale_hints_only" | "conflicting_ignore_for_decisions"
        ) && stale_decision.current_files_source_of_truth,
        format!(
            "status={} allowed_use={}",
            stale_decision.status, stale_decision.allowed_memory_use
        ),
    );

    CodingMemoryLevel7ProbeReport {
        probe_id: spec.id,
        ok: failures.is_empty(),
        project_root: project_root.display().to_string(),
        memory_db_path: bridge.memory_db_path.display().to_string(),
        checks,
        failures,
    }
}

fn seed_project(spec: &ProbeSpec, root: &Path) -> Result<String, String> {
    write_file(&root.join("ARCHITECTURE.md"), spec.architecture)?;
    write_file(&root.join("PROJECT_MANIFEST.txt"), spec.manifest)?;
    write_file(
        &root.join(format!("src/{}/__init__.py", spec.package)),
        "\"\"\"Level 7 resume probe package.\"\"\"\n",
    )?;
    write_file(
        &root.join(format!("src/{}/service.py", spec.package)),
        spec.service_body,
    )?;
    write_file(&root.join("tests/test_baseline.py"), spec.baseline_test)?;
    Ok("seeded".to_string())
}

fn write_checkpoint_one(spec: &ProbeSpec, root: &Path) -> Result<String, String> {
    write_file(
        &root.join(format!("src/{}/checkpoint_one.py", spec.package)),
        "def checkpoint_one_value():\n    return 'checkpoint-one-complete'\n",
    )?;
    write_file(
        &root.join("tests/test_checkpoint_one.py"),
        &format!(
            "import unittest\nfrom {}.checkpoint_one import checkpoint_one_value\n\nclass CheckpointOneTest(unittest.TestCase):\n    def test_checkpoint_one_feature(self):\n        self.assertEqual(checkpoint_one_value(), 'checkpoint-one-complete')\n\nif __name__ == '__main__':\n    unittest.main()\n",
            spec.package
        ),
    )?;
    Ok("checkpoint_one_written".to_string())
}

fn write_checkpoint_two(spec: &ProbeSpec, root: &Path) -> Result<String, String> {
    write_file(
        &root.join(format!("src/{}/checkpoint_two.py", spec.package)),
        "def checkpoint_two_value():\n    return 'checkpoint-two-resumed'\n",
    )?;
    write_file(
        &root.join("tests/test_checkpoint_two.py"),
        &format!(
            "import unittest\nfrom {}.checkpoint_two import checkpoint_two_value\n\nclass CheckpointTwoResumeTest(unittest.TestCase):\n    def test_checkpoint_two_resume_feature(self):\n        self.assertEqual(checkpoint_two_value(), 'checkpoint-two-resumed')\n\nif __name__ == '__main__':\n    unittest.main()\n",
            spec.package
        ),
    )?;
    Ok("checkpoint_two_written".to_string())
}

fn memory_context_payload(
    spec: &ProbeSpec,
    snapshot: &ProjectContextSnapshot,
    token: &str,
) -> String {
    json!({
        "schema_version": "project_context_capture_v1",
        "record_kind": "project_context",
        "project_id": snapshot.project_id,
        "project_root": snapshot.project_root,
        "project_fingerprint": snapshot.project_fingerprint,
        "architecture_hash": snapshot.architecture_hash,
        "architecture_summary": spec.architecture,
        "stack_summary": spec.manifest,
        "source_boundaries": ["src/<package>/service.py", "tests"],
        "validation_commands": [snapshot.validation_command],
        "unique_resume_token": token
    })
    .to_string()
}

fn checkpoint_payload(
    checkpoint_id: &str,
    snapshot: &ProjectContextSnapshot,
    token: &str,
    changed_files: &[&str],
    recommended_next_checkpoint: &str,
) -> String {
    json!({
        "schema_version": "checkpoint_memory_write_v1",
        "record_kind": "checkpoint_memory",
        "project_id": snapshot.project_id,
        "project_fingerprint": snapshot.project_fingerprint,
        "architecture_hash": snapshot.architecture_hash,
        "completed_checkpoint": checkpoint_id,
        "changed_files": changed_files,
        "validation_results": {
            "status": "pass",
            "command": snapshot.validation_command,
            "exit_code": 0
        },
        "known_risks": ["deterministic Level 7 substrate probe, not an agent reliability sample"],
        "intentionally_excluded_scope": ["production daemon integration"],
        "recommended_next_checkpoint": recommended_next_checkpoint,
        "artifact_refs": ["ARCHITECTURE.md", "PROJECT_MANIFEST.txt"],
        "memory_freshness_status": "fresh",
        "unique_resume_token": token,
        "source_of_truth_rule": "memory_guides_resume_current_workspace_files_remain_authoritative"
    })
    .to_string()
}

fn push_memory_check(
    checks: &mut Vec<CodingMemoryLevel7Check>,
    failures: &mut Vec<String>,
    id: &'static str,
    result: &MemoryCommandResult,
) {
    push_check(
        checks,
        failures,
        id,
        result.ok,
        result
            .payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("ok")
            .to_string(),
    );
}

fn push_check(
    checks: &mut Vec<CodingMemoryLevel7Check>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    if !ok {
        failures.push(id.to_string());
    }
    checks.push(CodingMemoryLevel7Check { id, ok, detail });
}

fn memory_result_contains_id(result: &MemoryCommandResult, id: &str) -> bool {
    result
        .payload
        .get("hits")
        .and_then(Value::as_array)
        .map(|hits| {
            hits.iter()
                .any(|hit| hit.get("id").and_then(Value::as_str) == Some(id))
        })
        .unwrap_or(false)
}

fn memory_result_content_contains(result: &MemoryCommandResult, needle: &str) -> bool {
    result
        .payload
        .pointer("/row/content")
        .and_then(Value::as_str)
        .map(|content| content.contains(needle))
        .unwrap_or(false)
}

struct ValidationResult {
    ok: bool,
    detail: String,
}

fn run_python_validation(root: &Path) -> ValidationResult {
    let output = Command::new("python3")
        .arg("-m")
        .arg("unittest")
        .arg("discover")
        .arg("-s")
        .arg("tests")
        .env("PYTHONPATH", "src")
        .current_dir(root)
        .output();
    match output {
        Ok(output) => ValidationResult {
            ok: output.status.success(),
            detail: format!(
                "exit={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        },
        Err(error) => ValidationResult {
            ok: false,
            detail: format!("validation_spawn_failed:{error}"),
        },
    }
}

fn temp_project_root(probe_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "coding-memory-level7-{probe_id}-{}-{}",
        std::process::id(),
        millis_now()
    ))
}

fn write_file(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("mkdir_failed:{error}"))?;
    }
    fs::write(path, body).map_err(|error| format!("write_failed:{}:{error}", path.display()))
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
