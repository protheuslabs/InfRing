use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextAtomSourceKind {
    InteractionUnit,
    ToolResultBundle,
    StatusSummary,
    WorkflowBoundary,
    ExternalReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextAtom {
    pub atom_id: String,
    pub session_id: String,
    pub sequence_no: u64,
    pub source_kind: ContextAtomSourceKind,
    pub source_ref: String,
    pub text_preview: String,
    pub token_count: u32,
    pub timestamp_ms: u64,
    pub task_refs: Vec<String>,
    pub memory_version_refs: Vec<String>,
    pub lineage_refs: Vec<String>,
}

impl ContextAtom {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: &str,
        sequence_no: u64,
        source_kind: ContextAtomSourceKind,
        source_ref: &str,
        text_preview: &str,
        token_count: u32,
        task_refs: Vec<String>,
        memory_version_refs: Vec<String>,
        lineage_refs: Vec<String>,
    ) -> Self {
        let atom_id = format!(
            "ctx_atom_{}",
            &deterministic_hash(&(
                session_id.to_string(),
                sequence_no,
                source_ref.to_string(),
                text_preview.to_string(),
                token_count,
                now_ms()
            ))[..24]
        );
        Self {
            atom_id,
            session_id: session_id.to_string(),
            sequence_no,
            source_kind,
            source_ref: source_ref.to_string(),
            text_preview: text_preview.to_string(),
            token_count,
            timestamp_ms: now_ms(),
            task_refs,
            memory_version_refs,
            lineage_refs,
        }
    }
}
