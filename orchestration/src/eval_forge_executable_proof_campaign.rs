// Layer ownership: eval/observability (ForgeCode-derived executable proof campaign measurement).
use crate::eval_coding_safety_layer::coding_safety_layer_lab_report;
use crate::eval_local_coding_program_builder::local_coding_program_builder_lab_file_execution_report;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ForgeExecutableProofCampaignLabReport {
    pub workflow_id: &'static str,
    pub harness_kind: &'static str,
    pub mode: &'static str,
    pub ok: bool,
    pub workflow_under_test: &'static str,
    pub proof_campaign_status: &'static str,
    pub promotion_decision: &'static str,
    pub operator_next_action: &'static str,
    pub evidence_summary: ForgeExecutableProofCampaignEvidenceSummary,
    pub eval_axes: Vec<ForgeExecutableProofCampaignAxisResult>,
    pub known_limitations: Vec<&'static str>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeExecutableProofCampaignEvidenceSummary {
    pub coding_project_lab_ok: bool,
    pub coding_safety_lab_ok: bool,
    pub coding_project_task_count: usize,
    pub materialized_project_shapes: Vec<&'static str>,
    pub safety_read_receipts: usize,
    pub safety_write_receipts: usize,
    pub safety_patch_receipts: usize,
    pub safety_command_receipts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeExecutableProofCampaignAxisResult {
    pub axis: &'static str,
    pub status: &'static str,
    pub lab_evidence: Vec<String>,
    pub required_live_evidence: Vec<&'static str>,
    pub promotion_blocked_until_live_evidence: bool,
}

pub fn forge_executable_proof_campaign_lab_report() -> ForgeExecutableProofCampaignLabReport {
    let coding_report = local_coding_program_builder_lab_file_execution_report();
    let safety_report = coding_safety_layer_lab_report();
    let mut failures = Vec::new();

    if !coding_report.ok {
        failures.push("local_coding_program_builder_lab_report_failed".to_string());
        failures.extend(coding_report.failures.clone());
    }
    if !safety_report.ok {
        failures.push("coding_safety_layer_lab_report_failed".to_string());
        failures.extend(safety_report.failures.clone());
    }

    let task_ids = coding_report
        .task_executions
        .iter()
        .map(|execution| execution.task_id)
        .collect::<Vec<_>>();
    let has_single_file = task_ids
        .iter()
        .any(|task_id| *task_id == "single_file_utility");
    let has_multi_file = task_ids
        .iter()
        .any(|task_id| *task_id == "small_multi_file_app");
    let has_existing_project = task_ids
        .iter()
        .any(|task_id| *task_id == "initialized_project_modification");

    if !has_single_file {
        failures.push("missing_single_file_utility_lab_task".to_string());
    }
    if !has_multi_file {
        failures.push("missing_small_multi_file_app_lab_task".to_string());
    }
    if !has_existing_project {
        failures.push("missing_initialized_project_modification_lab_task".to_string());
    }

    let eval_axes = vec![
        ForgeExecutableProofCampaignAxisResult {
            axis: "local_file_reading",
            status: if safety_report.read_receipts.is_empty() {
                "missing_lab_evidence"
            } else {
                "lab_receipt_present_live_agent_eval_required"
            },
            lab_evidence: safety_report
                .read_receipts
                .iter()
                .map(|receipt| {
                    format!(
                        "read:{}:{}-{}",
                        receipt.path, receipt.start_line, receipt.end_line
                    )
                })
                .collect(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "read_tool_selection_evidence",
                "no_shell_cat_antipattern",
            ],
            promotion_blocked_until_live_evidence: true,
        },
        ForgeExecutableProofCampaignAxisResult {
            axis: "single_file_code_write",
            status: if has_single_file && !safety_report.write_receipts.is_empty() {
                "lab_receipt_present_live_agent_eval_required"
            } else {
                "missing_lab_evidence"
            },
            lab_evidence: coding_report
                .task_executions
                .iter()
                .filter(|execution| execution.task_id == "single_file_utility")
                .flat_map(|execution| execution.changed_files.clone())
                .collect(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "file_change_receipt_ref",
                "changed_file_summary",
            ],
            promotion_blocked_until_live_evidence: true,
        },
        ForgeExecutableProofCampaignAxisResult {
            axis: "exact_patch_editing",
            status: if safety_report.patch_receipts.is_empty() {
                "missing_lab_evidence"
            } else {
                "lab_receipt_present_live_agent_eval_required"
            },
            lab_evidence: safety_report
                .patch_receipts
                .iter()
                .map(|receipt| format!("patch:{}:{}", receipt.path, receipt.match_status))
                .collect(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "patch_tool_use_evidence",
                "no_text_mismatch_or_missing_operation",
            ],
            promotion_blocked_until_live_evidence: true,
        },
        ForgeExecutableProofCampaignAxisResult {
            axis: "parallel_tool_orchestration",
            status: "live_agent_eval_required",
            lab_evidence: vec!["no_runtime_agent_trace_in_lab_harness".to_string()],
            required_live_evidence: vec![
                "agent_trace_ref",
                "multiple_independent_tool_calls_in_single_assistant_message",
            ],
            promotion_blocked_until_live_evidence: true,
        },
        ForgeExecutableProofCampaignAxisResult {
            axis: "multi_file_coding_execution",
            status: if has_multi_file {
                "lab_receipt_present_live_agent_eval_required"
            } else {
                "missing_lab_evidence"
            },
            lab_evidence: coding_report
                .task_executions
                .iter()
                .filter(|execution| execution.task_id == "small_multi_file_app")
                .flat_map(|execution| execution.changed_files.clone())
                .collect(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "file_change_receipt_ref",
                "multi_file_ownership_summary",
                "unrelated_file_preservation_evidence",
            ],
            promotion_blocked_until_live_evidence: true,
        },
        ForgeExecutableProofCampaignAxisResult {
            axis: "bounded_repair_and_validation",
            status: if safety_report.command_receipts.is_empty() {
                "missing_lab_evidence"
            } else {
                "lab_receipt_present_live_agent_eval_required"
            },
            lab_evidence: safety_report
                .command_receipts
                .iter()
                .map(|receipt| format!("command:{}:{}", receipt.command, receipt.status))
                .collect(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "validation_receipt_ref",
                "repair_attempt_budget",
                "no_unbounded_loop",
            ],
            promotion_blocked_until_live_evidence: true,
        },
    ];

    for axis in &eval_axes {
        if axis.status == "missing_lab_evidence" {
            failures.push(format!("missing_lab_evidence_for_axis:{}", axis.axis));
        }
    }

    let evidence_summary = ForgeExecutableProofCampaignEvidenceSummary {
        coding_project_lab_ok: coding_report.ok,
        coding_safety_lab_ok: safety_report.ok,
        coding_project_task_count: coding_report.task_executions.len(),
        materialized_project_shapes: task_ids,
        safety_read_receipts: safety_report.read_receipts.len(),
        safety_write_receipts: safety_report.write_receipts.len(),
        safety_patch_receipts: safety_report.patch_receipts.len(),
        safety_command_receipts: safety_report.command_receipts.len(),
    };

    ForgeExecutableProofCampaignLabReport {
        workflow_id: "forge_executable_proof_campaign",
        harness_kind: "forge_executable_proof_campaign_lab_plan_only_v1",
        mode: "plan_only_lab_evidence_packaging",
        ok: failures.is_empty()
            && eval_axes
                .iter()
                .all(|axis| axis.promotion_blocked_until_live_evidence),
        workflow_under_test: "local_coding_program_builder",
        proof_campaign_status: "lab_evidence_packaged_live_agent_eval_required",
        promotion_decision: "needs_executable_evals",
        operator_next_action: "run_live_agent_eval_batch_with_trace_capture",
        evidence_summary,
        eval_axes,
        known_limitations: vec![
            "lab runner packages evidence from deterministic harnesses only",
            "runtime agent spawning is not enabled in this harness",
            "original ForgeCode parity requires live agent eval receipts",
        ],
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::forge_executable_proof_campaign_lab_report;

    #[test]
    fn forge_executable_proof_campaign_packages_higher_level_coding_evidence() {
        let report = forge_executable_proof_campaign_lab_report();
        assert!(report.ok, "{report:#?}");
        assert_eq!(report.workflow_id, "forge_executable_proof_campaign");
        assert_eq!(report.workflow_under_test, "local_coding_program_builder");
        assert_eq!(report.promotion_decision, "needs_executable_evals");
        assert_eq!(report.eval_axes.len(), 6);
        assert!(report
            .eval_axes
            .iter()
            .all(|axis| axis.promotion_blocked_until_live_evidence));
        assert!(report
            .eval_axes
            .iter()
            .any(|axis| axis.axis == "multi_file_coding_execution"
                && axis.status == "lab_receipt_present_live_agent_eval_required"));
    }
}
