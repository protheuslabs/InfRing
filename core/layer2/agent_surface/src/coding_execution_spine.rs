// Layer ownership: Core Layer 2 / Orchestration boundary.
//
// This module is the primitive coding execution state machine. It intentionally
// does not call providers, read files, write files, run commands, or know about
// eval levels. Tooling and workflow adapters feed it normalized evidence; the
// spine decides the next required state transition from that evidence.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingTaskKind {
    CreateFile,
    ExistingProjectPatch,
    DebugRepair,
    Refactor,
    ProjectSlice,
    ExplanationOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingExecutionState {
    TaskContracted,
    ContextNeeded,
    ContextReady,
    MutationNeeded,
    MutationObserved,
    ValidationNeeded,
    ValidationPassed,
    RepairNeeded,
    ClosedSuccess,
    ClosedBlocked,
    FailedBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingSpineAction {
    BuildContext,
    Mutate,
    Validate,
    Repair,
    CloseSuccess,
    CloseBlocked,
    FailBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodingTaskContract {
    pub task_id: String,
    pub task_kind: CodingTaskKind,
    pub requires_context: bool,
    pub requires_mutation: bool,
    pub requires_validation: bool,
    pub requires_public_interface_check: bool,
    pub allowed_write_roots: Vec<String>,
    pub target_artifacts: Vec<String>,
    pub public_surface_requirements: Vec<String>,
    pub max_repair_turns: u32,
}

impl CodingTaskContract {
    pub fn explanation_only(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            task_kind: CodingTaskKind::ExplanationOnly,
            requires_context: false,
            requires_mutation: false,
            requires_validation: false,
            requires_public_interface_check: false,
            allowed_write_roots: Vec::new(),
            target_artifacts: Vec::new(),
            public_surface_requirements: Vec::new(),
            max_repair_turns: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextEvidence {
    pub receipt_ref: String,
    pub selected_paths: Vec<String>,
    pub sufficient_for_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationEvidence {
    pub receipt_ref: String,
    pub tool_name: String,
    pub changed_paths: Vec<String>,
    pub artifact_roles: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationEvidence {
    pub receipt_ref: String,
    pub command: String,
    pub success: bool,
    pub after_mutation_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicInterfaceEvidence {
    pub receipt_ref: String,
    pub success: bool,
    pub missing_requirements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockerEvidence {
    pub receipt_ref: String,
    pub reason: String,
    pub needs_user_input: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodingSpineDecision {
    pub state: CodingExecutionState,
    pub action: CodingSpineAction,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingExecutionSpine {
    contract: CodingTaskContract,
    state: CodingExecutionState,
    context: Vec<ContextEvidence>,
    mutations: Vec<MutationEvidence>,
    validations: Vec<ValidationEvidence>,
    public_interfaces: Vec<PublicInterfaceEvidence>,
    blockers: Vec<BlockerEvidence>,
    repair_turns: u32,
}

impl CodingExecutionSpine {
    pub fn new(contract: CodingTaskContract) -> Self {
        Self {
            contract,
            state: CodingExecutionState::TaskContracted,
            context: Vec::new(),
            mutations: Vec::new(),
            validations: Vec::new(),
            public_interfaces: Vec::new(),
            blockers: Vec::new(),
            repair_turns: 0,
        }
    }

    pub fn contract(&self) -> &CodingTaskContract {
        &self.contract
    }

    pub fn state(&self) -> &CodingExecutionState {
        &self.state
    }

    pub fn context(&self) -> &[ContextEvidence] {
        &self.context
    }

    pub fn mutations(&self) -> &[MutationEvidence] {
        &self.mutations
    }

    pub fn validations(&self) -> &[ValidationEvidence] {
        &self.validations
    }

    pub fn public_interfaces(&self) -> &[PublicInterfaceEvidence] {
        &self.public_interfaces
    }

    pub fn blockers(&self) -> &[BlockerEvidence] {
        &self.blockers
    }

    pub fn record_context(&mut self, evidence: ContextEvidence) -> CodingSpineDecision {
        self.context.push(evidence);
        self.decide()
    }

    pub fn record_mutation(&mut self, evidence: MutationEvidence) -> CodingSpineDecision {
        self.mutations.push(evidence);
        self.decide()
    }

    pub fn record_validation(&mut self, evidence: ValidationEvidence) -> CodingSpineDecision {
        self.validations.push(evidence);
        self.decide()
    }

    pub fn record_public_interface(
        &mut self,
        evidence: PublicInterfaceEvidence,
    ) -> CodingSpineDecision {
        self.public_interfaces.push(evidence);
        self.decide()
    }

    pub fn record_blocker(&mut self, evidence: BlockerEvidence) -> CodingSpineDecision {
        self.blockers.push(evidence);
        self.decide()
    }

    pub fn record_repair_turn(&mut self) -> CodingSpineDecision {
        self.repair_turns = self.repair_turns.saturating_add(1);
        self.decide()
    }

    pub fn decide(&mut self) -> CodingSpineDecision {
        let decision = self.next_decision();
        self.state = decision.state.clone();
        decision
    }

    pub fn next_decision(&self) -> CodingSpineDecision {
        if let Some(blocker) = self.blockers.last() {
            return CodingSpineDecision {
                state: CodingExecutionState::ClosedBlocked,
                action: CodingSpineAction::CloseBlocked,
                reason: blocker.reason.clone(),
            };
        }

        if self.repair_turns > self.contract.max_repair_turns {
            return CodingSpineDecision {
                state: CodingExecutionState::FailedBudget,
                action: CodingSpineAction::FailBudget,
                reason: "repair_budget_exhausted".to_string(),
            };
        }

        if self.contract.requires_context && !self.has_sufficient_context() {
            return CodingSpineDecision {
                state: CodingExecutionState::ContextNeeded,
                action: CodingSpineAction::BuildContext,
                reason: "context_required_before_mutation".to_string(),
            };
        }

        if self.contract.requires_mutation && !self.has_successful_mutation() {
            return CodingSpineDecision {
                state: CodingExecutionState::MutationNeeded,
                action: CodingSpineAction::Mutate,
                reason: "mutation_receipt_required".to_string(),
            };
        }

        if let Some(missing_role) = self.missing_required_target_artifact_role() {
            return CodingSpineDecision {
                state: CodingExecutionState::RepairNeeded,
                action: CodingSpineAction::Repair,
                reason: format!("target_artifact_mutation_missing:{missing_role}"),
            };
        }

        if self.contract.requires_validation && !self.has_successful_validation_after_mutation() {
            if self.has_failed_validation_after_mutation() {
                return CodingSpineDecision {
                    state: CodingExecutionState::RepairNeeded,
                    action: CodingSpineAction::Repair,
                    reason: "validation_failed_after_mutation".to_string(),
                };
            }
            return CodingSpineDecision {
                state: CodingExecutionState::ValidationNeeded,
                action: CodingSpineAction::Validate,
                reason: "validation_required_after_mutation".to_string(),
            };
        }

        if self.contract.requires_public_interface_check && !self.has_public_interface_success() {
            return CodingSpineDecision {
                state: CodingExecutionState::RepairNeeded,
                action: CodingSpineAction::Repair,
                reason: "public_interface_evidence_missing_or_failed".to_string(),
            };
        }

        CodingSpineDecision {
            state: CodingExecutionState::ClosedSuccess,
            action: CodingSpineAction::CloseSuccess,
            reason: "required_evidence_satisfied".to_string(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            CodingExecutionState::ClosedSuccess
                | CodingExecutionState::ClosedBlocked
                | CodingExecutionState::FailedBudget
        )
    }

    fn has_sufficient_context(&self) -> bool {
        self.context
            .iter()
            .any(|evidence| evidence.sufficient_for_mutation)
    }

    fn latest_successful_mutation_index(&self) -> Option<usize> {
        self.mutations
            .iter()
            .enumerate()
            .rev()
            .find(|(_, evidence)| evidence.success && !evidence.changed_paths.is_empty())
            .map(|(idx, _)| idx)
    }

    fn has_successful_mutation(&self) -> bool {
        self.latest_successful_mutation_index().is_some()
    }

    fn missing_required_target_artifact_role(&self) -> Option<String> {
        for required in &self.contract.target_artifacts {
            if !self
                .mutations
                .iter()
                .filter(|evidence| evidence.success)
                .flat_map(|evidence| evidence.artifact_roles.iter())
                .any(|role| role == required)
            {
                return Some(required.clone());
            }
        }
        None
    }

    fn has_successful_validation_after_mutation(&self) -> bool {
        let Some(mutation_idx) = self.latest_successful_mutation_index() else {
            return !self.contract.requires_mutation;
        };
        self.validations.iter().any(|evidence| {
            evidence.success
                && evidence
                    .after_mutation_index
                    .map(|idx| idx >= mutation_idx)
                    .unwrap_or(false)
        })
    }

    fn has_failed_validation_after_mutation(&self) -> bool {
        let Some(mutation_idx) = self.latest_successful_mutation_index() else {
            return false;
        };
        self.validations.iter().any(|evidence| {
            !evidence.success
                && evidence
                    .after_mutation_index
                    .map(|idx| idx >= mutation_idx)
                    .unwrap_or(false)
        })
    }

    fn has_public_interface_success(&self) -> bool {
        self.public_interfaces
            .iter()
            .any(|evidence| evidence.success && evidence.missing_requirements.is_empty())
    }
}
