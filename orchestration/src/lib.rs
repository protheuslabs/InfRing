// Layer ownership: orchestration (non-canonical orchestration coordination only).
pub mod assimilation_authority;
pub mod clarification;
pub mod continuous_eval;
pub mod contracts;
pub mod control_plane;
pub mod eval;
pub mod eval_agent_feedback;
pub mod eval_agent_self_diagnosis;
pub mod eval_authority_calibration;
pub mod eval_chat_report;
pub mod eval_coding_safety_layer;
pub mod eval_feedback_router;
pub mod eval_forge_executable_proof_campaign;
pub mod eval_forge_level4_software_analysis;
pub mod eval_forge_level6_existing_project_modification;
pub mod eval_forge_software_complexity_ladder;
pub mod eval_issue_authority;
pub mod eval_local_coding_program_builder;
pub mod ingress;
pub mod legacy_ingress_budget;
pub mod observation_lifecycle;
pub mod orchestration_stability_guard;
pub mod planner;
pub mod posture;
pub mod progress;
pub mod recovery;
pub mod request_classifier;
pub mod result_packaging;
pub mod self_maintenance;
pub mod self_modification;
pub mod sequencing;
pub mod synthesis_receipt_guard;
pub mod telemetry;
pub mod tool_routing_authority;
pub mod transient_context;
pub mod trust_zones;

use contracts::{
    ClosureState, ControlPlaneClosureState, ControlPlaneDecisionTrace,
    ControlPlaneDecisionTraceStep, ControlPlaneHandoff, ControlPlaneLifecycleState,
    CoreExecutionObservation, DegradationReason, ExecutionCorrelation, ExecutionState,
    OrchestrationExecutionObservationUpdate, OrchestrationFallbackAction, OrchestrationPlan,
    OrchestrationRequest, OrchestrationResultPackage, PlanCandidate, PlanScore, PlanStatus,
    PlanVariant, ReceiptDebugMetadata, RecoveryDecision, RecoveryReason, RecoveryState,
    RuntimeQualitySignals, WorkflowStage, WorkflowStageState, WorkflowStageStatus,
    WorkflowTemplate,
};
use self_maintenance::contracts::{
    ArchitectureAuditInput, CiReportInput, HealthMetricInput, MemoryPressureInput,
    ObservationInputs, SupervisorMode,
};
use self_maintenance::GovernedSelfMaintenanceSupervisor;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use transient_context::{TransientContextStore, TransientSleepCleanupReport};

const DEFAULT_SLEEP_CYCLE_IDLE_GAP_MS: u64 = 8 * 60 * 60 * 1000;
const SELF_MAINTENANCE_RECOMMENDATION_RATE_LIMIT_WINDOW_MS: u64 = 30 * 60 * 1000;

#[derive(Debug, Clone)]
struct SelfMaintenanceProjection {
    fallback_actions: Vec<OrchestrationFallbackAction>,
    lifecycle_next_actions: Vec<String>,
}

#[derive(Debug, Clone)]
struct SelfMaintenanceRecommendationWindow {
    cluster_id: String,
    occurrence_count: u64,
    first_seen_ms: u64,
    last_seen_ms: u64,
}

#[derive(Debug)]
pub struct OrchestrationSurfaceRuntime {
    transient_context_store: TransientContextStore,
    self_maintenance_windows: BTreeMap<String, SelfMaintenanceRecommendationWindow>,
    last_activity_ms: Option<u64>,
    sleep_cycle_idle_gap_ms: u64,
}

impl Default for OrchestrationSurfaceRuntime {
    fn default() -> Self {
        Self {
            transient_context_store: TransientContextStore::default(),
            self_maintenance_windows: BTreeMap::new(),
            last_activity_ms: None,
            sleep_cycle_idle_gap_ms: DEFAULT_SLEEP_CYCLE_IDLE_GAP_MS,
        }
    }
}

impl OrchestrationSurfaceRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sleep_cycle_idle_gap_ms(mut self, idle_gap_ms: u64) -> Self {
        self.sleep_cycle_idle_gap_ms = idle_gap_ms;
        self
    }

    fn maybe_run_sleep_cycle_cleanup(&mut self, now_ms: u64) {
        let Some(last_activity_ms) = self.last_activity_ms else {
            return;
        };
        if now_ms <= last_activity_ms {
            return;
        }
        let idle_gap = now_ms.saturating_sub(last_activity_ms);
        if idle_gap < self.sleep_cycle_idle_gap_ms {
            return;
        }
        let cycle_id = format!("auto_idle_gap_{idle_gap}");
        let _ = self
            .transient_context_store
            .run_sleep_cycle_cleanup(cycle_id.as_str());
    }

    pub fn orchestrate(
        &mut self,
        request: OrchestrationRequest,
        now_ms: u64,
    ) -> OrchestrationResultPackage {
        self.maybe_run_sleep_cycle_cleanup(now_ms);
        let normalized = ingress::normalize_request(request);
        let typed_request = normalized.typed_request.clone();
        let classification = request_classifier::classify_request(&normalized);
        if let Err(err) = self.transient_context_store.upsert(
            typed_request.session_id.as_str(),
            transient_summary(&typed_request),
            now_ms,
            30_000,
        ) {
            self.last_activity_ms = Some(now_ms);
            let surface_adapter_fallback = classification.surface_adapter_fallback;
            let workflow_template = WorkflowTemplate::DiagnoseRetryEscalate;
            return OrchestrationResultPackage {
                summary: format!("orchestration_degraded:{err}"),
                progress_message:
                    "Transient orchestration context unavailable; halted before core contract planning"
                        .to_string(),
                execution_state: ExecutionState {
                    plan_status: PlanStatus::Blocked,
                    steps: Vec::new(),
                    recovery: Some(RecoveryState {
                        decision: RecoveryDecision::Halt,
                        reason: Some(RecoveryReason::TransportFailure),
                        retryable: true,
                        note: "transient context unavailable".to_string(),
                    }),
                    degradation: None,
                    correlation: ExecutionCorrelation {
                        orchestration_trace_id: format!(
                            "orch_{}_transient",
                            typed_request.session_id
                        ),
                        expected_core_contract_ids: Vec::new(),
                        observed_core_receipt_ids: Vec::new(),
                        observed_core_outcome_refs: Vec::new(),
                        receipt_metadata: ReceiptDebugMetadata {
                            decision_trace: ControlPlaneDecisionTrace {
                                chosen: "plan_transient_context_failed".to_string(),
                                alternatives_rejected: Vec::new(),
                                confidence: 0.0,
                                rationale: vec!["transient_context_unavailable".to_string()],
                                receipt_metadata: vec![format!(
                                    "orchestration_trace_id=orch_{}_transient",
                                    typed_request.session_id
                                )],
                                step_records: vec![ControlPlaneDecisionTraceStep {
                                    step_id: "transient_context_write".to_string(),
                                    inputs: vec![
                                        format!("session_id={}", typed_request.session_id),
                                        "phase=intake_normalization".to_string(),
                                    ],
                                    chosen_path: "halt_before_core_contract".to_string(),
                                    alternatives_rejected: vec![
                                        "continue_without_transient_context".to_string(),
                                    ],
                                    confidence: 0.0,
                                    receipt_metadata: vec![format!(
                                        "orchestration_trace_id=orch_{}_transient",
                                        typed_request.session_id
                                    )],
                                }],
                            },
                        },
                    },
                },
                recovery_applied: true,
                fallback_actions: Vec::new(),
                core_contract_calls: Vec::new(),
                requires_core_promotion: false,
                classification,
                selected_plan: PlanCandidate {
                    plan_id: "plan_transient_context_failed".to_string(),
                    variant: PlanVariant::ClarificationFirst,
                    steps: Vec::new(),
                    mutates_session_context: false,
                    context_preparation_rationale: None,
                    decomposition_family: "transient_context_failure".to_string(),
                    capability_graph: Vec::new(),
                    contract_family: "halt_before_core_contract".to_string(),
                    decomposition_signature:
                        "transient_context_failure:halt_before_core_contract:none"
                            .to_string(),
                    confidence: 0.0,
                    score: PlanScore {
                        overall: 0.0,
                        authority_cost: 0.0,
                        transport_dependency: 0.0,
                        mutation_risk: 0.0,
                        fallback_quality: 0.0,
                        target_specificity: 0.0,
                    },
                    requires_clarification: true,
                    blocked_on: Vec::new(),
                    degradation: vec![DegradationReason::TransportFailure],
                    capabilities: Vec::new(),
                    capability_probes: Vec::new(),
                    reasons: vec!["transient_context_unavailable".to_string()],
                },
                alternative_plans: Vec::new(),
                runtime_quality: RuntimeQualitySignals {
                    candidate_count: 1,
                    selected_variant: PlanVariant::ClarificationFirst,
                    selected_plan_degraded: false,
                    selected_plan_requires_clarification: true,
                    used_heuristic_probe: false,
                    heuristic_probe_source_count: 0,
                    blocked_precondition_count: 0,
                    executable_candidate_count: 0,
                    degraded_candidate_count: 0,
                    clarification_candidate_count: 1,
                    zero_executable_candidates: true,
                    all_candidates_degraded: false,
                    all_candidates_require_clarification: true,
                    surface_adapter_fallback,
                    typed_probe_contract_gap_count: 0,
                    decision_rationale_count: 1,
                    fallback_action_count: 0,
                    tool_failure_budget_failed_step_count: 0,
                    tool_failure_budget_limit: 0,
                    tool_failure_budget_exceeded: false,
                },
                workflow_quality: None,
                decision_trace: ControlPlaneDecisionTrace {
                    chosen: "plan_transient_context_failed".to_string(),
                    alternatives_rejected: Vec::new(),
                    confidence: 0.0,
                    rationale: vec!["transient_context_unavailable".to_string()],
                    receipt_metadata: vec![format!(
                        "orchestration_trace_id=orch_{}_transient",
                        typed_request.session_id
                    )],
                    step_records: vec![ControlPlaneDecisionTraceStep {
                        step_id: "transient_context_write".to_string(),
                        inputs: vec![
                            format!("session_id={}", typed_request.session_id),
                            "phase=intake_normalization".to_string(),
                        ],
                        chosen_path: "halt_before_core_contract".to_string(),
                        alternatives_rejected: vec![
                            "continue_without_transient_context".to_string(),
                        ],
                        confidence: 0.0,
                        receipt_metadata: vec![format!(
                            "orchestration_trace_id=orch_{}_transient",
                            typed_request.session_id
                        )],
                    }],
                },
                workflow_template: workflow_template.clone(),
                control_plane_lifecycle: ControlPlaneLifecycleState {
                    owner: control_plane::lifecycle::workflow_owner().to_string(),
                    template: workflow_template,
                    active_stage: WorkflowStage::RecoveryEscalation,
                    stages: vec![
                        WorkflowStageState {
                            stage: WorkflowStage::IntakeNormalization,
                            status: WorkflowStageStatus::Completed,
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            note: "request normalization completed before transient context write"
                                .to_string(),
                        },
                        WorkflowStageState {
                            stage: WorkflowStage::RecoveryEscalation,
                            status: WorkflowStageStatus::Blocked,
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            note: "transient context unavailable".to_string(),
                        },
                        WorkflowStageState {
                            stage: WorkflowStage::ResultPackaging,
                            status: WorkflowStageStatus::Completed,
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            note: "degraded halt package emitted".to_string(),
                        },
                    ],
                    handoff_chain: vec![
                        ControlPlaneHandoff {
                            handoff_id: "handoff_user_request_to_decomposition".to_string(),
                            from: "user_request_ingress".to_string(),
                            to: "decomposition_planning".to_string(),
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            artifact: "typed_request_snapshot".to_string(),
                            status: WorkflowStageStatus::Completed,
                        },
                        ControlPlaneHandoff {
                            handoff_id: "handoff_decomposition_to_coordination".to_string(),
                            from: "decomposition_planning".to_string(),
                            to: "coordination_sequencing".to_string(),
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            artifact: "selected_plan_recommendation".to_string(),
                            status: WorkflowStageStatus::Blocked,
                        },
                        ControlPlaneHandoff {
                            handoff_id: "handoff_coordination_to_core_execution".to_string(),
                            from: "coordination_sequencing".to_string(),
                            to: "core_contract_execution".to_string(),
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            artifact: "core_contract_call_envelope".to_string(),
                            status: WorkflowStageStatus::Blocked,
                        },
                        ControlPlaneHandoff {
                            handoff_id: "handoff_core_execution_to_verification".to_string(),
                            from: "core_contract_execution".to_string(),
                            to: "verification_closure".to_string(),
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            artifact: "execution_observation_snapshot".to_string(),
                            status: WorkflowStageStatus::Blocked,
                        },
                        ControlPlaneHandoff {
                            handoff_id: "handoff_verification_to_memory_packaging".to_string(),
                            from: "verification_closure".to_string(),
                            to: "memory_packaging_projection".to_string(),
                            owner: control_plane::lifecycle::workflow_owner().to_string(),
                            artifact: "result_package_projection".to_string(),
                            status: WorkflowStageStatus::Blocked,
                        },
                    ],
                    next_actions: vec!["restore_transient_context_then_retry".to_string()],
                    closure: ControlPlaneClosureState {
                        verification: ClosureState::Blocked,
                        receipt_correlation: ClosureState::Blocked,
                        memory_packaging: ClosureState::Blocked,
                    },
                },
            };
        }
        let execution_observation = self
            .transient_context_store
            .execution_observation(typed_request.session_id.as_str())
            .cloned();

        let clarification_prompt =
            clarification::build_clarification_prompt(&typed_request, &classification);
        let needs_clarification =
            classification.needs_clarification || clarification_prompt.is_some();
        let posture = posture::choose_execution_posture(
            classification.request_class.clone(),
            needs_clarification,
        );
        let planning_template_hint = control_plane::lifecycle::select_workflow_template(
            &typed_request,
            &classification,
            PlanStatus::Planned,
            needs_clarification,
            None,
        );
        let mut plan_candidates = sequencing::propose_decomposition_candidates_with_template(
            &typed_request,
            &classification,
            Some(&planning_template_hint),
        );
        let selected_plan = plan_candidates.first().cloned().unwrap_or_else(|| {
            sequencing::propose_decomposition_candidate_with_template(
                &typed_request,
                &classification,
                Some(&planning_template_hint),
            )
        });
        let alternative_plans = if plan_candidates.len() > 1 {
            plan_candidates.drain(1..).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let execution_state = progress::project_execution_state(
            &typed_request,
            execution_observation.as_ref(),
            &selected_plan,
            needs_clarification,
        );

        let plan = OrchestrationPlan {
            request_class: classification.request_class.clone(),
            classification,
            posture,
            needs_clarification,
            clarification_prompt,
            selected_plan,
            alternative_plans,
            execution_state,
        };
        let (plan, recovery_applied) =
            recovery::coordinate_recovery_escalation(&typed_request, plan);
        let (plan, reroute_applied) = sequencing::apply_retry_reroute_feedback(
            &typed_request,
            execution_observation.as_ref(),
            plan,
        );
        let self_maintenance_projection = project_self_maintenance_recommendations(
            &typed_request,
            &plan,
            now_ms,
            self.transient_context_store.len() as u64,
            &mut self.self_maintenance_windows,
        );
        let progress = progress::build_progress_projection(&plan);
        let tool_fallback_context =
            sequencing::decode_tool_fallback_context(&typed_request.payload);
        let mut fallback_actions = sequencing::project_fallback_actions(
            &typed_request,
            plan.request_class.clone(),
            tool_fallback_context.as_ref(),
        );
        if let Some(projection) = &self_maintenance_projection {
            fallback_actions.extend(projection.fallback_actions.clone());
        }
        let workflow_template = control_plane::lifecycle::select_workflow_template(
            &typed_request,
            &plan.classification,
            plan.execution_state.plan_status.clone(),
            plan.needs_clarification,
            plan.execution_state.recovery.as_ref(),
        );
        let mut control_plane_lifecycle = control_plane::lifecycle::build_lifecycle_state(
            workflow_template.clone(),
            &plan,
            fallback_actions.as_slice(),
        );
        if let Some(projection) = &self_maintenance_projection {
            control_plane_lifecycle
                .next_actions
                .extend(projection.lifecycle_next_actions.iter().cloned());
            dedupe_actions(&mut control_plane_lifecycle.next_actions);
        }
        self.last_activity_ms = Some(now_ms);
        result_packaging::shape_result_package(
            &plan,
            progress,
            recovery_applied || reroute_applied,
            fallback_actions,
            workflow_template,
            control_plane_lifecycle,
        )
    }

    pub fn sweep_transient(&mut self, now_ms: u64) -> usize {
        self.transient_context_store.sweep_expired(now_ms)
    }

    pub fn transient_entry_count(&self) -> usize {
        self.transient_context_store.len()
    }

    pub fn transient_ephemeral_count(&self) -> usize {
        self.transient_context_store.active_ephemeral_count()
    }

    pub fn begin_transient_restart(&mut self) {
        self.transient_context_store.begin_restart();
    }

    pub fn sweep_transient_before_resume(&mut self) -> Result<usize, String> {
        self.transient_context_store.sweep_stale_before_resume()
    }

    pub fn resume_transient_after_restart(&mut self) -> Result<(), String> {
        self.transient_context_store.resume_after_restart()
    }

    pub fn run_transient_sleep_cycle_cleanup(
        &mut self,
        sleep_cycle_id: &str,
    ) -> Result<TransientSleepCleanupReport, String> {
        self.transient_context_store
            .run_sleep_cycle_cleanup(sleep_cycle_id)
    }

    pub fn apply_execution_observation_update(
        &mut self,
        update: OrchestrationExecutionObservationUpdate,
    ) {
        let session_id = update.session_id.trim().to_string();
        if session_id.is_empty() {
            return;
        }
        let _ = self.transient_context_store.upsert_execution_observation(
            session_id.as_str(),
            update.observation,
            runtime_now_ms(),
        );
    }

    pub fn record_execution_observation(
        &mut self,
        session_id: impl Into<String>,
        observation: CoreExecutionObservation,
    ) {
        self.apply_execution_observation_update(OrchestrationExecutionObservationUpdate {
            session_id: session_id.into(),
            observation,
        });
    }

    pub fn clear_execution_observation(&mut self, session_id: &str) {
        let _ = self
            .transient_context_store
            .clear_execution_observation(session_id);
    }
}

fn transient_summary(request: &crate::contracts::TypedOrchestrationRequest) -> String {
    format!(
        "kind={:?};operation={:?};resource={:?};legacy_intent={}",
        request.request_kind, request.operation_kind, request.resource_kind, request.legacy_intent
    )
    .to_lowercase()
}

fn project_self_maintenance_recommendations(
    request: &crate::contracts::TypedOrchestrationRequest,
    plan: &OrchestrationPlan,
    now_ms: u64,
    transient_entry_count: u64,
    recommendation_windows: &mut BTreeMap<String, SelfMaintenanceRecommendationWindow>,
) -> Option<SelfMaintenanceProjection> {
    if !should_project_self_maintenance(plan) {
        return None;
    }
    prune_self_maintenance_windows(recommendation_windows, now_ms);
    let cluster_id = self_maintenance_cluster_id(request, plan);
    let window = recommendation_windows
        .entry(cluster_id.clone())
        .and_modify(|window| {
            window.occurrence_count = window.occurrence_count.saturating_add(1);
            window.last_seen_ms = now_ms;
        })
        .or_insert_with(|| SelfMaintenanceRecommendationWindow {
            cluster_id: cluster_id.clone(),
            occurrence_count: 1,
            first_seen_ms: now_ms,
            last_seen_ms: now_ms,
        });
    let occurrence_count = window.occurrence_count;
    let first_seen_ms = window.first_seen_ms;
    let last_seen_ms = window.last_seen_ms;
    let rate_limited = occurrence_count > 1
        && last_seen_ms.saturating_sub(first_seen_ms)
            <= SELF_MAINTENANCE_RECOMMENDATION_RATE_LIMIT_WINDOW_MS;
    let observation_inputs =
        build_self_maintenance_observation_inputs(request, plan, transient_entry_count);
    let mut supervisor = GovernedSelfMaintenanceSupervisor::new(
        SupervisorMode::ObserveOnly,
        format!("orchestration-control-plane:{}", request.session_id),
    );
    let run = supervisor.run_cycle(observation_inputs, now_ms).ok()?;
    let unresolved = run.claim_bundle.unresolved_questions.len();
    let conflicts = run.claim_bundle.conflicts.len();
    let candidate_task_count = run.generated_task_ids.len();
    let mut lifecycle_next_actions =
        vec![format!("self_maintenance_review_cluster:{}", window.cluster_id).to_lowercase()];
    if unresolved > 0 {
        lifecycle_next_actions
            .push("self_maintenance_clarify_unresolved_observations_before_retry".to_string());
    }
    if conflicts > 0 {
        lifecycle_next_actions.push("self_maintenance_resolve_conflicting_evidence".to_string());
    }

    let fallback_actions = if rate_limited {
        Vec::new()
    } else {
        vec![OrchestrationFallbackAction {
            kind: "self_maintenance_review".to_string(),
            label: "Review maintenance signals".to_string(),
            reason: format!(
                "self-maintenance observe-only window cluster_id={cluster_id}; occurrences={occurrence_count}; rate_limited={rate_limited}; candidate_tasks={candidate_task_count}; unresolved={unresolved}; conflicts={conflicts}; first_seen_ms={first_seen_ms}; last_seen_ms={last_seen_ms}"
            ),
            backend_class: None,
            reason_code: None,
        }]
    };

    Some(SelfMaintenanceProjection {
        fallback_actions,
        lifecycle_next_actions,
    })
}

fn prune_self_maintenance_windows(
    recommendation_windows: &mut BTreeMap<String, SelfMaintenanceRecommendationWindow>,
    now_ms: u64,
) {
    recommendation_windows.retain(|_, window| {
        now_ms.saturating_sub(window.last_seen_ms)
            <= SELF_MAINTENANCE_RECOMMENDATION_RATE_LIMIT_WINDOW_MS
    });
}

fn self_maintenance_cluster_id(
    request: &crate::contracts::TypedOrchestrationRequest,
    plan: &OrchestrationPlan,
) -> String {
    let mut blocked_on = plan
        .selected_plan
        .blocked_on
        .iter()
        .map(|precondition| format!("{precondition:?}").to_lowercase())
        .collect::<Vec<_>>();
    blocked_on.sort();
    let mut capabilities = plan
        .selected_plan
        .capabilities
        .iter()
        .map(|capability| format!("{capability:?}").to_lowercase())
        .collect::<Vec<_>>();
    capabilities.sort();
    let recovery_reason = plan
        .execution_state
        .recovery
        .as_ref()
        .and_then(|recovery| recovery.reason.as_ref())
        .map(|reason| format!("{reason:?}").to_lowercase())
        .unwrap_or_else(|| "none".to_string());
    let status = format!("{:?}", plan.execution_state.plan_status).to_lowercase();
    let variant = format!("{:?}", plan.selected_plan.variant).to_lowercase();
    format!(
        "session={};plan={};variant={};status={};recovery={};blocked={};capabilities={};fallback={};clarify={};empty_steps={}",
        request.session_id,
        plan.selected_plan.plan_id,
        variant,
        status,
        recovery_reason,
        blocked_on.join(","),
        capabilities.join(","),
        plan.classification.surface_adapter_fallback,
        plan.needs_clarification,
        plan.selected_plan.steps.is_empty()
    )
}

fn should_project_self_maintenance(plan: &OrchestrationPlan) -> bool {
    matches!(
        plan.execution_state.plan_status,
        PlanStatus::Failed
            | PlanStatus::Blocked
            | PlanStatus::Degraded
            | PlanStatus::ClarificationRequired
    ) || plan.needs_clarification
        || plan.classification.surface_adapter_fallback
        || !plan.selected_plan.blocked_on.is_empty()
        || plan.selected_plan.steps.is_empty()
}

fn build_self_maintenance_observation_inputs(
    request: &crate::contracts::TypedOrchestrationRequest,
    plan: &OrchestrationPlan,
    transient_entry_count: u64,
) -> ObservationInputs {
    let mut inputs = ObservationInputs::empty();

    if plan.classification.surface_adapter_fallback || plan.needs_clarification {
        inputs.architecture_audits.push(ArchitectureAuditInput {
            audit_id: format!("ingress-control-gap-{}", request.session_id),
            summary:
                "surface adapter fallback or clarification-first routing indicates ingress drift"
                    .to_string(),
            severity: "medium".to_string(),
            source_ref: "orchestration/src/ingress.rs".to_string(),
        });
    }

    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Failed | PlanStatus::Blocked | PlanStatus::Degraded
    ) {
        inputs.ci_reports.push(CiReportInput {
            report_id: format!("orchestration-plan-status-{}", request.session_id),
            status: format!("{:?}", plan.execution_state.plan_status).to_ascii_lowercase(),
            summary: "control-plane plan status indicates degraded runtime quality".to_string(),
            source_ref: "orchestration/src/lib.rs".to_string(),
        });
    }

    inputs.task_fabric_signals.stale_tasks = plan
        .selected_plan
        .blocked_on
        .iter()
        .map(|precondition| format!("blocked_precondition::{precondition:?}").to_lowercase())
        .collect::<Vec<_>>();

    inputs.task_fabric_signals.blocked_tasks = plan
        .execution_state
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.status,
                crate::contracts::StepStatus::Blocked | crate::contracts::StepStatus::Failed
            )
        })
        .map(|step| step.step_id.clone())
        .collect::<Vec<_>>();

    let risk_score = self_maintenance_risk_score(plan);
    inputs.health_metrics.push(HealthMetricInput {
        metric_name: "orchestration_control_plane_risk_score".to_string(),
        observed: risk_score,
        threshold: 0.25,
        source_ref: "orchestration/src/lib.rs".to_string(),
    });

    inputs.memory_pressure.push(MemoryPressureInput {
        scope: "orchestration_transient_context".to_string(),
        used_bytes: transient_entry_count,
        limit_bytes: 64,
    });

    inputs
}

fn self_maintenance_risk_score(plan: &OrchestrationPlan) -> f64 {
    let mut risk = 0.0_f64;
    if plan.needs_clarification || plan.selected_plan.requires_clarification {
        risk += 0.35;
    }
    if !plan.selected_plan.blocked_on.is_empty() {
        risk += 0.35;
    }
    if plan.selected_plan.steps.is_empty() {
        risk += 0.30;
    }
    if plan.classification.surface_adapter_fallback {
        risk += 0.20;
    }
    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Failed | PlanStatus::Blocked | PlanStatus::Degraded
    ) {
        risk += 0.45;
    }
    risk.clamp(0.0, 1.0)
}

fn dedupe_actions(actions: &mut Vec<String>) {
    actions.sort();
    actions.dedup();
}

fn runtime_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
