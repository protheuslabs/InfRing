// Layer ownership: eval/observability (Phase 1 primitive coding spine measurement).
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const WORKFLOW_UNDER_TEST: &str = "local_coding_phase1_mutation_spine";
const WORKFLOW_PATH: &str = "orchestration/src/control_plane/workflows/lab/composites/coding/local_coding_phase1_mutation_spine.workflow.json";

#[derive(Debug, Clone, Serialize)]
pub struct Phase1MutationSpineReport {
    pub harness_kind: &'static str,
    pub workflow_under_test: &'static str,
    pub ok: bool,
    pub sandbox_root: String,
    pub runs: Vec<Phase1MutationSpineRun>,
    pub aggregate: Phase1MutationSpineAggregate,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase1MutationSpineAggregate {
    pub run_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub workflow_contract_ok: bool,
    pub mutation_receipt_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase1MutationSpineRun {
    pub run_id: String,
    pub task_kind: &'static str,
    pub workflow_under_test: &'static str,
    pub sandbox_path: String,
    pub changed_files: Vec<String>,
    pub receipts: Vec<Phase1MutationSpineReceipt>,
    pub final_synthesis: Phase1FinalSynthesis,
    pub checks: Vec<Phase1MutationSpineCheck>,
    pub ok: bool,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase1MutationSpineReceipt {
    pub receipt_type: &'static str,
    pub tool: &'static str,
    pub status: &'static str,
    pub path: String,
    pub operation: &'static str,
    pub bytes_written: u64,
    pub scope_decision: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase1FinalSynthesis {
    pub status: &'static str,
    pub changed_files: Vec<String>,
    pub validation_status: &'static str,
    pub blockers: Vec<String>,
    pub receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Phase1MutationSpineCheck {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

pub fn phase1_mutation_spine_level1_report(run_count: usize) -> Phase1MutationSpineReport {
    let run_count = run_count.max(1);
    let sandbox_root = phase1_sandbox_root();
    let mut failures = Vec::new();
    if let Err(error) = std::fs::create_dir_all(&sandbox_root) {
        failures.push(format!("sandbox_root_create_failed:{error}"));
    }

    let workflow_contract = load_phase1_workflow_contract();
    if !workflow_contract.ok {
        failures.extend(workflow_contract.failures.clone());
    }

    let runs = (1..=run_count)
        .map(|index| run_phase1_level1_attempt(&sandbox_root, index, &workflow_contract))
        .collect::<Vec<_>>();
    let pass_count = runs.iter().filter(|run| run.ok).count();
    let mutation_receipt_count = runs
        .iter()
        .flat_map(|run| run.receipts.iter())
        .filter(|receipt| {
            receipt.receipt_type == "file_mutation_receipt_v1" && receipt.status == "success"
        })
        .count();

    let ok = failures.is_empty() && workflow_contract.ok && pass_count == run_count;
    Phase1MutationSpineReport {
        harness_kind: "local_coding_phase1_mutation_spine_level1_lab_v1",
        workflow_under_test: WORKFLOW_UNDER_TEST,
        ok,
        sandbox_root: sandbox_root.to_string_lossy().to_string(),
        aggregate: Phase1MutationSpineAggregate {
            run_count,
            pass_count,
            fail_count: run_count.saturating_sub(pass_count),
            workflow_contract_ok: workflow_contract.ok,
            mutation_receipt_count,
        },
        runs,
        failures,
    }
}

#[derive(Debug, Clone)]
struct Phase1WorkflowContractCheck {
    ok: bool,
    failures: Vec<String>,
}

fn load_phase1_workflow_contract() -> Phase1WorkflowContractCheck {
    let path = Path::new(WORKFLOW_PATH);
    let mut failures = Vec::new();
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            return Phase1WorkflowContractCheck {
                ok: false,
                failures: vec![format!("workflow_read_failed:{WORKFLOW_PATH}:{error}")],
            };
        }
    };
    let value = match serde_json::from_str::<Value>(&raw) {
        Ok(value) => value,
        Err(error) => {
            return Phase1WorkflowContractCheck {
                ok: false,
                failures: vec![format!(
                    "workflow_json_parse_failed:{WORKFLOW_PATH}:{error}"
                )],
            };
        }
    };
    if value.get("name").and_then(Value::as_str) != Some(WORKFLOW_UNDER_TEST) {
        failures.push("workflow_name_mismatch".to_string());
    }
    if value
        .pointer("/workflow_composition/primitive_level")
        .and_then(Value::as_u64)
        != Some(1)
    {
        failures.push("workflow_primitive_level_not_1".to_string());
    }
    let children = value
        .pointer("/workflow_composition/composed_of_workflow_ids")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let expected_children = [
        "coding_task_contract",
        "implementation_entry_gate",
        "file_mutation_executor",
        "incremental_receipt_journal",
        "final_receipt_synthesis",
    ];
    for expected in expected_children {
        if !children.contains(&expected) {
            failures.push(format!("missing_child_workflow:{expected}"));
        }
    }
    if value
        .pointer("/primitive_first_contract/case_specific_hardcoding_allowed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        failures.push("primitive_first_contract_allows_case_hardcoding".to_string());
    }
    Phase1WorkflowContractCheck {
        ok: failures.is_empty(),
        failures,
    }
}

fn run_phase1_level1_attempt(
    sandbox_root: &Path,
    index: usize,
    workflow_contract: &Phase1WorkflowContractCheck,
) -> Phase1MutationSpineRun {
    let run_id = format!("phase1_level1_run_{index:02}");
    let run_root = sandbox_root.join(&run_id);
    let mut failures = Vec::new();
    if let Err(error) = std::fs::create_dir_all(&run_root) {
        failures.push(format!("run_root_create_failed:{run_id}:{error}"));
    }

    let relative_path = "src/phase1_checksum.rs";
    let target_path = run_root.join(relative_path);
    let source = phase1_checksum_source();
    let mut receipts = Vec::new();
    match write_scoped_file(&run_root, relative_path, source) {
        Ok(bytes_written) => receipts.push(Phase1MutationSpineReceipt {
            receipt_type: "file_mutation_receipt_v1",
            tool: "file_write",
            status: "success",
            path: relative_path.to_string(),
            operation: "write",
            bytes_written,
            scope_decision: "allowed",
        }),
        Err(error) => failures.push(format!("file_mutation_failed:{relative_path}:{error}")),
    }

    let changed_files = if target_path.exists() {
        vec![relative_path.to_string()]
    } else {
        Vec::new()
    };
    let receipt_refs = receipts
        .iter()
        .enumerate()
        .map(|(receipt_index, receipt)| {
            format!(
                "{}:{}:{}",
                receipt.receipt_type,
                receipt.tool,
                receipt_index + 1
            )
        })
        .collect::<Vec<_>>();
    let final_synthesis = Phase1FinalSynthesis {
        status: if failures.is_empty() {
            "success"
        } else {
            "failed"
        },
        changed_files: changed_files.clone(),
        validation_status: "not_run_phase1",
        blockers: failures.clone(),
        receipt_refs,
    };

    let checks = vec![
        check(
            "workflow_under_test_is_phase1_spine",
            workflow_contract.ok,
            format!("{WORKFLOW_UNDER_TEST} contract loaded from {WORKFLOW_PATH}"),
        ),
        check(
            "single_file_materialized",
            target_path.exists(),
            target_path.to_string_lossy().to_string(),
        ),
        check(
            "successful_mutation_receipt_observed",
            receipts.iter().any(|receipt| {
                receipt.receipt_type == "file_mutation_receipt_v1"
                    && receipt.tool == "file_write"
                    && receipt.status == "success"
            }),
            format!("{} receipts", receipts.len()),
        ),
        check(
            "final_synthesis_uses_receipt_refs",
            !final_synthesis.receipt_refs.is_empty()
                && final_synthesis.changed_files == changed_files
                && final_synthesis.validation_status == "not_run_phase1",
            format!("receipt_refs={}", final_synthesis.receipt_refs.len()),
        ),
        check(
            "phase1_does_not_claim_validation",
            final_synthesis.validation_status == "not_run_phase1",
            final_synthesis.validation_status.to_string(),
        ),
    ];
    for check in &checks {
        if !check.ok {
            failures.push(format!("check_failed:{}", check.id));
        }
    }

    let ok = failures.is_empty() && checks.iter().all(|check| check.ok);
    Phase1MutationSpineRun {
        run_id,
        task_kind: "create_file",
        workflow_under_test: WORKFLOW_UNDER_TEST,
        sandbox_path: run_root.to_string_lossy().to_string(),
        changed_files,
        receipts,
        final_synthesis,
        checks,
        ok,
        failures,
    }
}

fn phase1_sandbox_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "local-coding-phase1-mutation-spine-{}",
        millis_now()
    ))
}

fn millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn write_scoped_file(root: &Path, relative_path: &str, contents: &str) -> std::io::Result<u64> {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, contents)?;
    std::fs::metadata(path).map(|metadata| metadata.len())
}

fn check(id: &'static str, ok: bool, detail: String) -> Phase1MutationSpineCheck {
    Phase1MutationSpineCheck { id, ok, detail }
}

fn phase1_checksum_source() -> &'static str {
    r#"pub fn weighted_checksum(input: &str) -> u64 {
    input
        .bytes()
        .enumerate()
        .map(|(index, byte)| (index as u64 + 1) * byte as u64)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::weighted_checksum;

    #[test]
    fn checksum_weights_later_bytes_more() {
        assert_eq!(weighted_checksum("abc"), 590);
    }
}
"#
}
