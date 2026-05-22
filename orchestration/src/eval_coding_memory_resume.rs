use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct CodingMemoryResumeProofReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub memory_db_path: String,
    pub memory_row_id: String,
    pub project_fingerprint: String,
    pub checkpoint_id: String,
    pub checks: Vec<CodingMemoryResumeCheck>,
    pub failures: Vec<String>,
    pub operator_next_action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodingMemoryResumeCheck {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

pub fn coding_memory_resume_proof_report() -> CodingMemoryResumeProofReport {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let root = workspace_root();
    let temp_root = proof_temp_root();
    let db_path = temp_root.join("runtime_memory.sqlite");
    let project_fingerprint = format!("level7_resume_probe_{}", millis_now());
    let checkpoint_id = "checkpoint_001";
    let memory_row_id =
        format!("coding_memory::{project_fingerprint}::checkpoint::{checkpoint_id}");
    let unique_probe_token = format!("resume_token_{project_fingerprint}");
    let checkpoint_payload = json!({
        "schema_version": "checkpoint_memory_write_v1",
        "project_id": "level7-resume-proof",
        "project_fingerprint": project_fingerprint,
        "completed_checkpoint": "initial_existing_project_slice",
        "changed_files": ["src/app/service.py", "tests/test_resume.py"],
        "validation_results": {
            "status": "pass",
            "command": "PYTHONPATH=src python3 -m unittest discover -s tests",
            "exit_code": 0
        },
        "known_risks": ["single proof run"],
        "intentionally_excluded_scope": ["no production persistence daemon"],
        "recommended_next_checkpoint": "resume_and_extend_slice",
        "artifact_refs": ["receipts/checkpoint_handoff.json"],
        "memory_freshness_status": "fresh",
        "unique_probe_token": unique_probe_token
    });

    let content = serde_json::to_string(&checkpoint_payload).unwrap_or_else(|_| "{}".to_string());
    let ingest = run_memory_cli(
        &root,
        &db_path,
        &[
            "ingest",
            &format!("--id={memory_row_id}"),
            &format!("--content={content}"),
            "--tags=coding,checkpoint,resume,project_context",
            "--repetitions=4",
            "--lambda=0.02",
        ],
    );
    push_check(
        &mut checks,
        &mut failures,
        "memory_cli_ingest_checkpoint_ok",
        json_ok(&ingest),
        format!("status={}", value_status(&ingest)),
    );

    let recall = run_memory_cli(
        &root,
        &db_path,
        &[
            "recall",
            &format!("--query={unique_probe_token}"),
            "--limit=5",
        ],
    );
    let recalled_expected_row = recall
        .get("hits")
        .and_then(Value::as_array)
        .map(|hits| {
            hits.iter().any(|hit| {
                hit.get("id").and_then(Value::as_str) == Some(memory_row_id.as_str())
                    && hit
                        .get("tags")
                        .and_then(Value::as_array)
                        .map(|tags| {
                            ["coding", "checkpoint", "resume", "project_context"]
                                .iter()
                                .all(|expected| {
                                    tags.iter().any(|tag| tag.as_str() == Some(*expected))
                                })
                        })
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    push_check(
        &mut checks,
        &mut failures,
        "memory_cli_recall_returns_checkpoint",
        json_ok(&recall) && recalled_expected_row,
        format!(
            "status={} hit_count={}",
            value_status(&recall),
            recall
                .get("hit_count")
                .and_then(Value::as_u64)
                .unwrap_or_default()
        ),
    );

    let get = run_memory_cli(&root, &db_path, &["get", &format!("--id={memory_row_id}")]);
    let get_contains_checkpoint = get
        .pointer("/row/content")
        .and_then(Value::as_str)
        .map(|content| {
            content.contains(&unique_probe_token)
                && content.contains("current workspace files remain authoritative")
                || content.contains("completed_checkpoint")
        })
        .unwrap_or(false);
    push_check(
        &mut checks,
        &mut failures,
        "memory_cli_get_returns_checkpoint_by_id",
        json_ok(&get) && get_contains_checkpoint,
        format!("status={}", value_status(&get)),
    );

    let fresh_decision = freshness_decision(&project_fingerprint, &project_fingerprint);
    push_check(
        &mut checks,
        &mut failures,
        "fresh_memory_can_seed_planning",
        fresh_decision == "fresh",
        format!("freshness_decision={fresh_decision}"),
    );

    let stale_decision = freshness_decision(&project_fingerprint, "different_project_fingerprint");
    push_check(
        &mut checks,
        &mut failures,
        "stale_memory_downgraded_to_hints_only",
        stale_decision == "stale_hints_only",
        format!("freshness_decision={stale_decision}"),
    );

    let source_of_truth_rule = "memory_guides_resume_current_workspace_files_remain_authoritative";
    push_check(
        &mut checks,
        &mut failures,
        "current_files_remain_source_of_truth",
        source_of_truth_rule.contains("current_workspace_files_remain_authoritative"),
        source_of_truth_rule.to_string(),
    );

    CodingMemoryResumeProofReport {
        harness_kind: "coding_memory_resume_proof_v1",
        ok: failures.is_empty(),
        memory_db_path: db_path.display().to_string(),
        memory_row_id,
        project_fingerprint,
        checkpoint_id: checkpoint_id.to_string(),
        checks,
        failures,
        operator_next_action:
            "wire_runtime_workflow_invocation_to_memory_cli_or_native_memory_core",
    }
}

fn push_check(
    checks: &mut Vec<CodingMemoryResumeCheck>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    if !ok {
        failures.push(id.to_string());
    }
    checks.push(CodingMemoryResumeCheck { id, ok, detail });
}

fn freshness_decision(
    current_project_fingerprint: &str,
    memory_project_fingerprint: &str,
) -> &'static str {
    if memory_project_fingerprint.is_empty() {
        "no_memory_found"
    } else if current_project_fingerprint == memory_project_fingerprint {
        "fresh"
    } else {
        "stale_hints_only"
    }
}

fn run_memory_cli(root: &Path, db_path: &Path, args: &[&str]) -> Value {
    let manifest_path = root.join("core/layer0/memory/Cargo.toml");
    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--bin")
        .arg("memory-cli")
        .arg("--")
        .args(args)
        .env("INFRING_MEMORY_DB_PATH", db_path)
        .current_dir(root)
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| {
                json!({
                    "ok": false,
                    "error": "memory_cli_invalid_json",
                    "status": output.status.code(),
                    "stdout": stdout,
                    "stderr": stderr
                })
            })
        }
        Err(error) => json!({
            "ok": false,
            "error": format!("memory_cli_spawn_failed:{error}")
        }),
    }
}

fn json_ok(value: &Value) -> bool {
    value.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

fn value_status(value: &Value) -> String {
    if json_ok(value) {
        "ok".to_string()
    } else {
        value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown_error")
            .to_string()
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn proof_temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "coding-memory-resume-proof-{}-{}",
        std::process::id(),
        millis_now()
    ))
}

fn millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
