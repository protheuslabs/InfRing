// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

mod anti_entropy; mod assimilation_handoff; mod authority; mod auto_run; mod big_picture_regression; mod boot_watch; mod causal_calibration; mod causal_hypothesis; mod cli_args; mod collector; mod diagnostic_authorization; mod diagnostic_executor; mod diagnostic_regression_executor; mod diagnostic_request; mod diagnostic_result; mod diagnostic_run_artifact; mod dossier_comparison; mod evidence; mod failure_level; mod feedback_quality; mod finding_lifecycle; mod findings_io; mod governance; mod graders; mod incident_clustering; mod incident_diagnostic_followup; mod incident_event; mod incident_report; mod incident_synthesis; mod invariant_registry; mod issue_cluster_semantics; mod issue_synthesis; mod maintenance_synthesis; mod problem_reliability; mod release_gate_synthesis; mod report_budget; #[cfg(test)] mod report_budget_tests; mod report_failure_levels; mod report_output; mod report_promotion; mod report_stream_budget; mod report_summary; #[cfg(test)] mod report_summary_tests; mod rsi_handoff; mod scheduler; mod self_dossier; mod self_dossier_markdown; #[cfg(test)] mod self_dossier_tests; mod self_study; mod system_understanding_dossier; mod system_understanding_worksheet; mod waivers;
pub use assimilation_handoff::build_external_assimilation_transfer_plan;
pub use authority::{authority_rule, kernel_sentinel_contract};
pub use big_picture_regression::{
    assess_kernel_sentinel_big_picture_regression, kernel_sentinel_big_picture_regression_model,
    KernelSentinelBigPictureAssessment, KernelSentinelBigPictureInput,
    KernelSentinelBigPictureMode, KERNEL_SENTINEL_BIG_PICTURE_SCHEMA_VERSION,
};
pub use causal_calibration::kernel_sentinel_causal_calibration_model;
pub use causal_hypothesis::kernel_sentinel_causal_hypothesis_model;
use cli_args::{bool_flag, option_path, option_usize, state_dir_from_args};
pub use diagnostic_authorization::{
    authorize_kernel_sentinel_diagnostic_request, kernel_sentinel_diagnostic_authorization_model,
    kernel_sentinel_diagnostic_failure_probe_policies,
    KernelSentinelDiagnosticAuthorizationDecision, KernelSentinelDiagnosticAuthorizationStatus,
};
pub use diagnostic_executor::{
    execute_kernel_sentinel_golden_replay_probe, execute_kernel_sentinel_read_only_topology_probe,
    kernel_sentinel_diagnostic_executor_model, KernelSentinelGoldenReplaySnapshot,
    KernelSentinelTopologyHealthSnapshot,
};
pub use diagnostic_regression_executor::{
    execute_kernel_sentinel_targeted_regression_probe,
    kernel_sentinel_targeted_regression_executor_model, KernelSentinelTargetedRegressionSnapshot,
};
pub use diagnostic_request::{
    kernel_sentinel_diagnostic_request_model, validate_kernel_sentinel_diagnostic_request,
    KernelSentinelDiagnosticBudgetImpact, KernelSentinelDiagnosticProbeClass,
    KernelSentinelDiagnosticRequest, KernelSentinelDiagnosticSafetyClass,
    KERNEL_SENTINEL_DIAGNOSTIC_REQUEST_SCHEMA_VERSION,
};
pub use diagnostic_result::{
    kernel_sentinel_diagnostic_result_model, validate_kernel_sentinel_diagnostic_result,
    KernelSentinelDiagnosticOutcome, KernelSentinelDiagnosticResult,
    KernelSentinelDiagnosticStopReason, KERNEL_SENTINEL_DIAGNOSTIC_RESULT_SCHEMA_VERSION,
};
pub use diagnostic_run_artifact::{
    attach_diagnostic_context_to_issue_draft, build_kernel_sentinel_diagnostic_report_section,
    build_kernel_sentinel_diagnostic_run_artifact, KERNEL_SENTINEL_DIAGNOSTIC_RUN_ARTIFACT_NAME,
};
pub use dossier_comparison::build_external_assimilation_dossier_comparison;
pub use evidence::{ingest_evidence_sources, KernelSentinelEvidenceIngestion};
pub use failure_level::{
    kernel_sentinel_failure_level_for_finding, kernel_sentinel_failure_level_for_parts,
    kernel_sentinel_failure_level_taxonomy, kernel_sentinel_root_frame_for_finding,
    kernel_sentinel_semantic_frame_for_finding, kernel_sentinel_semantic_frame_for_parts,
    KernelSentinelFailureLevel, KERNEL_SENTINEL_FAILURE_LEVELS,
};
pub use feedback_quality::{
    kernel_sentinel_feedback_quality_model, review_kernel_sentinel_feedback_quality,
    KernelSentinelFeedbackQualityReview, KernelSentinelFeedbackReviewInput,
    KernelSentinelFeedbackReviewStatus, KERNEL_SENTINEL_FEEDBACK_QUALITY_SCHEMA_VERSION,
};
use finding_lifecycle::{dedupe_findings, sanitize_finding};
use findings_io::read_jsonl_findings;
pub use incident_clustering::{
    cluster_kernel_sentinel_incident_events, KernelSentinelIncidentCluster,
    KernelSentinelIncidentClusterKey,
};
pub use incident_diagnostic_followup::build_incident_diagnostic_follow_up_request;
pub use incident_event::{
    kernel_sentinel_incident_event_model, validate_kernel_sentinel_incident_event,
    KernelSentinelIncidentEvent, KernelSentinelIncidentEvidenceLevel,
    KERNEL_SENTINEL_INCIDENT_EVENT_SCHEMA_VERSION,
};
pub use incident_report::kernel_sentinel_architectural_incident_report_section;
pub use incident_synthesis::{
    kernel_sentinel_architectural_issue_template,
    synthesize_kernel_sentinel_architectural_incidents, KernelSentinelArchitecturalIncident,
};
pub use invariant_registry::{
    kernel_sentinel_invariant_by_id, kernel_sentinel_invariant_registry,
    kernel_sentinel_invariant_registry_report, KernelSentinelInvariant, KERNEL_SENTINEL_INVARIANTS,
};
use report_budget::{
    build_final_report, DEFAULT_FINAL_REPORT_BYTE_BUDGET, DEFAULT_FINAL_REPORT_FINDING_LIMIT,
};
use report_summary::{
    count_by_category, count_by_severity, count_by_status, count_malformed_by_source,
    count_malformed_by_source_kind, critical_open_count, release_blockers,
};
pub use system_understanding_dossier::{
    kernel_system_understanding_dossier_model, validate_system_understanding_dossier,
    SystemUnderstandingCapabilityKind, SystemUnderstandingCapabilityRow,
    SystemUnderstandingCapabilityValue, SystemUnderstandingDossier,
    SystemUnderstandingDossierStatus, SystemUnderstandingDossierTargetMode,
    SystemUnderstandingTransferTarget, SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
};

pub const KERNEL_SENTINEL_NAME: &str = "Kernel Sentinel";
pub const KERNEL_SENTINEL_MODULE_ID: &str = "kernel_sentinel";
pub const KERNEL_SENTINEL_CLI_DOMAIN: &str = "kernel-sentinel";
pub const KERNEL_SENTINEL_CONTRACT_VERSION: u32 = 1;
pub const KERNEL_SENTINEL_FINDING_SCHEMA_VERSION: u32 = 1;
const DEFAULT_REPORT_FINDING_LIMIT: usize = 200;

fn trace_id_from_args(args: &[String], generated_at: &str) -> String {
    let prefix = "--trace-id=";
    args.iter()
        .find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("observability:{generated_at}:kernel-sentinel"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelEvidenceSource {
    KernelReceipt,
    RuntimeObservation,
    ReleaseProofPack,
    GatewayHealth,
    QueueBackpressure,
    ControlPlaneEval,
    ShellTelemetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelAuthorityClass {
    DeterministicKernelAuthority,
    AdvisoryWorkflowQuality,
    PresentationTelemetryOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelAuthorityRule {
    pub source: KernelSentinelEvidenceSource,
    pub authority_class: KernelSentinelAuthorityClass,
    pub may_open_finding: bool,
    pub may_write_verdict: bool,
    pub may_block_release: bool,
    pub may_waive_finding: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelFindingCategory {
    ReceiptIntegrity,
    CapabilityEnforcement,
    StateTransition,
    NexusBoundary,
    Boundedness,
    GatewayIsolation,
    QueueBackpressure,
    RetryStorm,
    ReleaseEvidence,
    SelfMaintenanceLoop,
    SecurityBoundary,
    RuntimeCorrectness,
    PerformanceRegression,
    AutomationCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KernelSentinelFinding {
    pub schema_version: u32,
    pub id: String,
    pub severity: KernelSentinelSeverity,
    pub category: KernelSentinelFindingCategory,
    pub fingerprint: String,
    pub evidence: Vec<String>,
    pub summary: String,
    pub recommended_action: String,
    pub status: String,
}

impl Serialize for KernelSentinelFinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let failure_level = kernel_sentinel_failure_level_for_finding(self);
        let root_frame = kernel_sentinel_root_frame_for_finding(self);
        let mut state = serializer.serialize_struct("KernelSentinelFinding", 14)?;
        state.serialize_field("schema_version", &self.schema_version)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("severity", &self.severity)?;
        state.serialize_field("category", &self.category)?;
        state.serialize_field("fingerprint", &self.fingerprint)?;
        state.serialize_field("evidence", &self.evidence)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("recommended_action", &self.recommended_action)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("failure_level", failure_level.code())?;
        state.serialize_field("failure_class", failure_level.failure_class())?;
        state.serialize_field("root_frame", root_frame)?;
        state.serialize_field("remediation_level", failure_level.remediation_level())?;
        state.serialize_field("review_depth", failure_level.review_depth())?;
        state.end()
    }
}

pub fn validate_finding(finding: &KernelSentinelFinding) -> Result<(), String> {
    if finding.schema_version != KERNEL_SENTINEL_FINDING_SCHEMA_VERSION {
        return Err("invalid_schema_version".to_string());
    }
    for (field, value) in [
        ("id", finding.id.as_str()),
        ("fingerprint", finding.fingerprint.as_str()),
        ("summary", finding.summary.as_str()),
        ("recommended_action", finding.recommended_action.as_str()),
        ("status", finding.status.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("missing_{field}"));
        }
    }
    if finding.evidence.is_empty() || finding.evidence.iter().any(|row| row.trim().is_empty()) {
        return Err("missing_evidence".to_string());
    }
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{body}\n")).map_err(|err| err.to_string())
}

pub fn build_report(root: &Path, args: &[String]) -> (Value, Value, i32) {
    let dir = state_dir_from_args(root, args);
    let findings_path = option_path(args, "--findings-path", dir.join("findings.jsonl"));
    let (mut findings, mut malformed) = read_jsonl_findings(&findings_path);
    let KernelSentinelEvidenceIngestion {
        findings: evidence_findings,
        malformed_records: evidence_malformed,
        report: evidence_report,
    } = ingest_evidence_sources(&dir, args);
    findings.extend(evidence_findings);
    malformed.extend(evidence_malformed);
    let (boot_watch_report, boot_watch_findings) = boot_watch::build_boot_watch_report(&dir, args);
    findings.extend(boot_watch_findings);
    let (waiver_report, waiver_findings) = waivers::apply_waivers(&mut findings, &dir, args);
    findings.extend(waiver_findings);
    let (governance_preflight, governance_findings) =
        governance::build_governance_preflight(&findings, &evidence_report, args);
    findings.extend(governance_findings);
    let architectural_incident_report =
        incident_report::kernel_sentinel_architectural_incident_report_section(&findings);
    let causal_hypothesis_synthesis = causal_hypothesis::build_kernel_sentinel_causal_hypotheses(
        &findings,
        &architectural_incident_report,
        args,
    );
    let causal_calibration = causal_calibration::build_kernel_sentinel_causal_calibration(
        &dir,
        &causal_hypothesis_synthesis,
        args,
    );
    let issue_synthesis = issue_synthesis::build_issue_synthesis(&findings, args);
    let maintenance_synthesis = maintenance_synthesis::build_maintenance_synthesis(&findings, args);
    let deduped = dedupe_findings(findings);
    let report_finding_limit =
        option_usize(args, "--report-finding-limit", DEFAULT_REPORT_FINDING_LIMIT);
    let final_report_finding_limit = option_usize(
        args,
        "--final-report-finding-limit",
        DEFAULT_FINAL_REPORT_FINDING_LIMIT,
    );
    let final_report_byte_budget = option_usize(
        args,
        "--final-report-byte-budget",
        DEFAULT_FINAL_REPORT_BYTE_BUDGET,
    );
    let report_findings = deduped
        .iter()
        .cloned()
        .take(report_finding_limit)
        .map(sanitize_finding)
        .collect::<Vec<_>>();
    let truncated_finding_count = deduped.len().saturating_sub(report_findings.len());
    let critical_open_count = critical_open_count(&deduped);
    let release_gate = governance::build_release_gate(
        &deduped,
        &malformed,
        &architectural_incident_report,
        &issue_synthesis,
        &maintenance_synthesis,
        &governance_preflight,
        &evidence_report,
    );
    let scheduler_health = scheduler::build_scheduler_health_summary(root, args);
    let scheduler_stale = scheduler_health["stale"].as_bool().unwrap_or(true);
    let scheduler_status = scheduler_health["status"]
        .as_str()
        .unwrap_or("unconfigured")
        .to_string();
    let scheduler_running = scheduler_health["running"].as_bool().unwrap_or(false);
    let scheduler_operator_warning = scheduler_health["operator_warning"].clone();
    let scheduler_next_action = scheduler_health["next_action"].clone();
    let strict = bool_flag(args, "--strict");
    let generated_at = crate::now_iso();
    let trace_id = trace_id_from_args(args, &generated_at);
    let release_gate_pass = release_gate["pass"].as_bool().unwrap_or(false);
    let release_blockers = release_blockers(
        critical_open_count,
        malformed.len(),
        release_gate_pass,
        scheduler_stale,
    );
    let verdict_state = if !malformed.is_empty() {
        "invalid"
    } else if critical_open_count > 0 || !release_gate_pass || scheduler_stale {
        "release_fail"
    } else {
        "allow"
    };
    let verdict = json!({
        "ok": malformed.is_empty()
            && critical_open_count == 0
            && release_gate_pass
            && !scheduler_stale,
        "type": "kernel_sentinel_verdict",
        "trace_id": trace_id.clone(),
        "parent_span_id": null,
        "contract_version": KERNEL_SENTINEL_CONTRACT_VERSION,
        "verdict": verdict_state,
        "strict": strict,
        "critical_open_count": critical_open_count,
        "malformed_finding_count": malformed.len(),
        "finding_count": deduped.len(),
        "scheduler_stale": scheduler_stale,
        "scheduler_running": scheduler_running,
        "scheduler_status": scheduler_status,
        "operator_warning": scheduler_operator_warning,
        "next_action": scheduler_next_action,
        "release_blockers": release_blockers.clone(),
        "receipt_hash": null
    });
    let mut verdict = verdict;
    verdict["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&verdict));
    let mut report = json!({
        "ok": verdict["ok"],
        "type": "kernel_sentinel_report",
        "trace_id": trace_id,
        "parent_span_id": null,
        "generated_at": generated_at,
        "canonical_name": KERNEL_SENTINEL_NAME,
        "state_dir": dir,
        "operator_summary": {
            "status_counts": count_by_status(&deduped),
            "severity_counts": count_by_severity(&deduped),
            "category_counts": count_by_category(&deduped),
            "critical_open_count": critical_open_count,
            "malformed_finding_count": malformed.len(),
            "malformed_by_source_kind": count_malformed_by_source_kind(&malformed),
            "malformed_by_source": count_malformed_by_source(&malformed),
            "report_finding_limit": report_finding_limit,
            "final_report_finding_limit": final_report_finding_limit,
            "final_report_byte_budget": final_report_byte_budget,
            "reported_finding_count": report_findings.len(),
            "truncated_finding_count": truncated_finding_count,
            "release_gate_pass": release_gate_pass,
            "causal_hypothesis_count": causal_hypothesis_synthesis["hypothesis_count"],
            "causal_calibrated_hypothesis_count": causal_calibration["calibrated_hypothesis_count"],
            "causal_promotion_ready_count": causal_calibration["promotion_ready_count"],
            "observation_state": evidence_report["observation_state"],
            "data_starved": evidence_report["data_starved"],
            "partial_evidence": evidence_report["partial_evidence"],
            "malformed_evidence": evidence_report["malformed_evidence"],
            "evidence_record_count": evidence_report["normalized_record_count"],
            "malformed_evidence_count": evidence_report["malformed_record_count"],
            "present_source_count": evidence_report["present_source_count"],
            "missing_source_count": evidence_report["missing_source_count"],
            "present_required_source_count": evidence_report["present_required_source_count"],
            "missing_required_source_count": evidence_report["missing_required_source_count"],
            "present_optional_source_count": evidence_report["present_optional_source_count"],
            "missing_optional_source_count": evidence_report["missing_optional_source_count"],
            "source_coverage": {
                "required": {
                    "present_count": evidence_report["present_required_source_count"],
                    "missing_count": evidence_report["missing_required_source_count"],
                    "ready": evidence_report["missing_required_source_count"].as_u64().unwrap_or(u64::MAX) == 0
                },
                "optional": {
                    "present_count": evidence_report["present_optional_source_count"],
                    "missing_count": evidence_report["missing_optional_source_count"],
                    "fully_present": evidence_report["missing_optional_source_count"].as_u64().unwrap_or(u64::MAX) == 0
                }
            },
            "stale_evidence": evidence_report["stale_evidence"],
            "stale_record_count": evidence_report["stale_record_count"],
            "freshness_observed_record_count": evidence_report["freshness_observed_record_count"],
            "stale_evidence_seconds": evidence_report["stale_evidence_seconds"],
            "max_evidence_age_seconds": evidence_report["max_evidence_age_seconds"],
            "scheduler_stale": scheduler_stale,
            "scheduler_running": scheduler_running,
            "scheduler_status": scheduler_status,
            "scheduler_operator_warning": scheduler_health["operator_warning"].clone(),
            "scheduler_next_action": scheduler_health["next_action"].clone(),
            "scheduler_health": scheduler_health,
            "release_blockers": release_blockers
        },
        "contract": kernel_sentinel_contract(),
        "finding_schema_version": KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
        "findings_path": findings_path,
        "evidence_ingestion": evidence_report,
        "boot_watch": boot_watch_report,
        "governance_preflight": governance_preflight,
        "architectural_incident_report": architectural_incident_report,
        "causal_hypothesis_synthesis": causal_hypothesis_synthesis,
        "causal_calibration": causal_calibration,
        "waivers": waiver_report,
        "release_gate": release_gate,
        "issue_synthesis": issue_synthesis,
        "maintenance_synthesis": maintenance_synthesis,
        "findings": report_findings,
        "malformed_findings": malformed,
        "verdict": verdict
    });
    let final_report = build_final_report(
        &report,
        &dir,
        final_report_finding_limit,
        final_report_byte_budget,
    );
    report["report_budget"] = final_report["report_budget"].clone();
    report["final_report"] = final_report;
    let exit = if strict
        && (critical_open_count > 0
            || !release_gate_pass
            || scheduler_stale
            || !report["malformed_findings"].as_array().unwrap().is_empty())
    {
        2
    } else {
        0
    };
    let verdict = report["verdict"].clone();
    (report, verdict, exit)
}

pub fn run(root: &Path, args: &[String]) -> i32 {
    let command = args.first().map(String::as_str).unwrap_or("help");
    if command == "help" || command == "--help" || command == "-h" {
        println!("infring-ops kernel-sentinel <run|status|report|auto|collect|tick|schedule|heartbeat|dream|help> [--strict=1|0] [--state-dir=<path>|--state-root=<path>] [--findings-path=<path>] [--evidence-dir=<path>] [--collector-artifact=<path>] [--require-evidence=1] [--issue-threshold=<n>] [--suggestion-threshold=<n>] [--automation-threshold=<n>] [--boot-self-check=1] [--watch-refresh=1] [--waivers-path=<path>] [--cadence=tick|heartbeat|dream|maintenance|release] [--auto-artifact=<path>] [--schedule-artifact=<path>] [--interval-seconds=<n>] [--heartbeat-interval-seconds=<n>] [--dream-idle-seconds=<n>] [--dream-max-without-seconds=<n>] [--stale-window-seconds=<n>] [--max-stale-minutes=<n>] [--max-runtime-ms=<n>] [--final-report-finding-limit=<n>] [--final-report-byte-budget=<n>]");
        println!(
            "{}",
            serde_json::to_string_pretty(&kernel_sentinel_contract()).unwrap()
        );
        return 0;
    }
    let rest = args.iter().skip(1).cloned().collect::<Vec<_>>();
    if command == "auto" {
        return auto_run::run_auto(root, &rest);
    }
    if command == "collect" {
        return collector::run_collect(root, &rest);
    }
    if command == "tick" {
        return scheduler::run_tick(root, &rest);
    }
    if command == "schedule" {
        return scheduler::run_schedule(root, &rest);
    }
    if command == "heartbeat" {
        return scheduler::run_heartbeat(root, &rest);
    }
    if command == "dream" {
        return scheduler::run_dream(root, &rest);
    }
    report_output::run_report_command(root, command, &rest)
}

#[cfg(test)]
mod root_tests;
