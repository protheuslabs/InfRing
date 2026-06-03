use crate::schemas::{
    ContextManifest, ContextManifestEntryRef, MemoryKind, MemoryScope, MemoryVersion,
    OwnerExportRedactionPolicy,
};
use crate::{
    context_atoms::ContextAtom,
    context_budget::ContextBudgetReport,
    context_topology::{ContextFrontier, ContextSpan},
};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaterializedMemoryEntry {
    pub object_id: String,
    pub version_id: String,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub payload: Value,
    pub redacted: bool,
    pub lineage_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextMaterialization {
    pub manifest: ContextManifest,
    pub entries: Vec<MaterializedMemoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextFragmentKind {
    Atom,
    Span,
    MemoryVersion,
    TaskAnchor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextFragment {
    pub fragment_id: String,
    pub kind: ContextFragmentKind,
    pub ref_id: String,
    pub level: Option<u32>,
    pub token_count: u32,
    pub payload: Value,
    pub lineage_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextTopologyMaterialization {
    pub manifest: ContextManifest,
    pub frontier: ContextFrontier,
    pub budget_report: ContextBudgetReport,
    pub fragments: Vec<ContextFragment>,
    pub compatibility_entries: Vec<MaterializedMemoryEntry>,
}

fn allows_scope(requested_scopes: &[MemoryScope], scope: &MemoryScope) -> bool {
    requested_scopes.is_empty() || requested_scopes.iter().any(|row| row == scope)
}

fn should_materialize_version(
    requested_scopes: &[MemoryScope],
    include_ephemeral: bool,
    version: &MemoryVersion,
) -> bool {
    if !allows_scope(requested_scopes, &version.scope) {
        return false;
    }
    !matches!(version.scope, MemoryScope::Ephemeral) || include_ephemeral
}

fn context_manifest_id(
    principal_id: &str,
    manifest_entries: &[ContextManifestEntryRef],
    timestamp_ms: u64,
) -> String {
    format!(
        "context_{}",
        &deterministic_hash(&(principal_id.to_string(), manifest_entries, timestamp_ms))[..24]
    )
}

pub fn materialize_context(
    principal_id: &str,
    requested_scopes: &[MemoryScope],
    redaction_policy: OwnerExportRedactionPolicy,
    source_versions: &[MemoryVersion],
) -> ContextMaterialization {
    let mut entries = Vec::<MaterializedMemoryEntry>::new();
    let mut manifest_entries = Vec::<ContextManifestEntryRef>::new();
    let explicit_ephemeral = requested_scopes
        .iter()
        .any(|scope| matches!(scope, MemoryScope::Ephemeral));
    let timestamp_ms = now_ms();

    for version in source_versions {
        if !should_materialize_version(requested_scopes, explicit_ephemeral, version) {
            continue;
        }
        let (payload, redacted) =
            render_scope_payload(&version.scope, &version.payload, &redaction_policy);
        entries.push(MaterializedMemoryEntry {
            object_id: version.object_id.clone(),
            version_id: version.version_id.clone(),
            scope: version.scope.clone(),
            kind: version.kind.clone(),
            payload,
            redacted,
            lineage_refs: version.lineage_refs.clone(),
        });
        manifest_entries.push(ContextManifestEntryRef {
            object_id: version.object_id.clone(),
            version_id: version.version_id.clone(),
            scope: version.scope.clone(),
            kind: version.kind.clone(),
            trust_state: version.trust_state.clone(),
            redacted,
        });
    }

    let manifest = ContextManifest {
        context_manifest_id: context_manifest_id(principal_id, &manifest_entries, timestamp_ms),
        principal_id: principal_id.to_string(),
        requested_scopes: requested_scopes.to_vec(),
        redaction_policy,
        entries: manifest_entries,
        lineage_refs: entries
            .iter()
            .flat_map(|row| row.lineage_refs.clone())
            .collect::<Vec<_>>(),
        timestamp_ms,
    };
    ContextMaterialization { manifest, entries }
}

#[allow(clippy::too_many_arguments)]
pub fn materialize_topology_context(
    principal_id: &str,
    requested_scopes: &[MemoryScope],
    redaction_policy: OwnerExportRedactionPolicy,
    source_versions: &[MemoryVersion],
    atoms: &[ContextAtom],
    spans: &[ContextSpan],
    frontier: ContextFrontier,
    budget_report: ContextBudgetReport,
) -> ContextTopologyMaterialization {
    let flat = materialize_context(
        principal_id,
        requested_scopes,
        redaction_policy,
        source_versions,
    );
    let mut fragments = Vec::<ContextFragment>::new();

    for atom_id in &frontier.hot_atom_refs {
        if let Some(atom) = atoms.iter().find(|row| &row.atom_id == atom_id) {
            fragments.push(ContextFragment {
                fragment_id: format!(
                    "ctx_fragment_{}",
                    &deterministic_hash(&(atom.atom_id.clone(), "atom"))[..24]
                ),
                kind: ContextFragmentKind::Atom,
                ref_id: atom.atom_id.clone(),
                level: Some(0),
                token_count: atom.token_count,
                payload: json!({
                    "source_kind": atom.source_kind,
                    "source_ref": atom.source_ref,
                    "text_preview": atom.text_preview,
                    "task_refs": atom.task_refs,
                    "memory_version_refs": atom.memory_version_refs,
                    "sequence_no": atom.sequence_no,
                }),
                lineage_refs: atom.lineage_refs.clone(),
            });
        }
    }

    for span_id in frontier
        .warm_span_refs
        .iter()
        .chain(frontier.cool_span_refs.iter())
        .chain(frontier.cold_span_refs.iter())
    {
        if let Some(span) = spans.iter().find(|row| &row.span_id == span_id) {
            fragments.push(ContextFragment {
                fragment_id: format!(
                    "ctx_fragment_{}",
                    &deterministic_hash(&(span.span_id.clone(), "span"))[..24]
                ),
                kind: ContextFragmentKind::Span,
                ref_id: span.span_id.clone(),
                level: Some(span.level),
                token_count: span.token_count,
                payload: json!({
                    "summary": span.summary,
                    "decisions": span.decisions,
                    "constraints": span.constraints,
                    "open_loops": span.open_loops,
                    "entities": span.entities,
                    "task_refs": span.task_refs,
                    "memory_version_refs": span.memory_version_refs,
                    "fidelity_score": span.fidelity_score,
                    "status": span.status,
                    "coverage": { "start_seq": span.start_seq, "end_seq": span.end_seq }
                }),
                lineage_refs: span.lineage_refs.clone(),
            });
        }
    }

    for entry in &flat.entries {
        fragments.push(ContextFragment {
            fragment_id: format!(
                "ctx_fragment_{}",
                &deterministic_hash(&(entry.version_id.clone(), "memory_version"))[..24]
            ),
            kind: ContextFragmentKind::MemoryVersion,
            ref_id: entry.version_id.clone(),
            level: None,
            token_count: estimate_payload_tokens(&entry.payload),
            payload: entry.payload.clone(),
            lineage_refs: entry.lineage_refs.clone(),
        });
    }

    for anchor in &frontier.pinned_anchor_refs {
        fragments.push(ContextFragment {
            fragment_id: format!(
                "ctx_fragment_{}",
                &deterministic_hash(&(anchor.clone(), "task_anchor"))[..24]
            ),
            kind: ContextFragmentKind::TaskAnchor,
            ref_id: anchor.clone(),
            level: None,
            token_count: 24,
            payload: json!({ "anchor_ref": anchor }),
            lineage_refs: vec![anchor.clone()],
        });
    }
    ContextTopologyMaterialization {
        manifest: flat.manifest,
        frontier,
        budget_report,
        fragments,
        compatibility_entries: flat.entries,
    }
}

fn render_scope_payload(
    scope: &MemoryScope,
    payload: &Value,
    redaction_policy: &OwnerExportRedactionPolicy,
) -> (Value, bool) {
    if !matches!(scope, MemoryScope::Owner) {
        return (payload.clone(), false);
    }
    match redaction_policy {
        OwnerExportRedactionPolicy::AllowFull => (payload.clone(), false),
        OwnerExportRedactionPolicy::AllowRedacted => {
            let summary = summarize_payload(payload);
            (json!({ "redacted": true, "summary": summary }), true)
        }
        OwnerExportRedactionPolicy::SummarizeOnly => {
            let summary = summarize_payload(payload);
            (json!({ "summary_only": summary }), true)
        }
        OwnerExportRedactionPolicy::Deny => (json!({ "denied": true }), true),
    }
}

fn summarize_payload(payload: &Value) -> String {
    let raw = match payload {
        Value::String(row) => row.clone(),
        _ => serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()),
    };
    raw.split_whitespace()
        .take(24)
        .collect::<Vec<_>>()
        .join(" ")
}

fn estimate_payload_tokens(payload: &Value) -> u32 {
    let text = match payload {
        Value::String(row) => row.clone(),
        _ => serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()),
    };
    ((text.len() / 4).max(1)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::TrustState;
    use serde_json::json;

    fn sample_version(scope: MemoryScope, object_id: &str, version_id: &str) -> MemoryVersion {
        MemoryVersion {
            version_id: version_id.to_string(),
            object_id: object_id.to_string(),
            scope,
            kind: MemoryKind::Episodic,
            parent_version_id: None,
            lineage_refs: vec!["lineage:test".to_string()],
            receipt_id: "receipt_test".to_string(),
            trust_state: TrustState::Validated,
            salience: crate::schemas::MemorySalience::for_kind(
                &MemoryKind::Episodic,
                &TrustState::Validated,
            ),
            derivation: None,
            payload: json!({"value":"ok"}),
            payload_hash: "hash".to_string(),
            timestamp_ms: 1,
            proposed_by: "agent:test".to_string(),
        }
    }

    #[test]
    fn default_materialization_excludes_ephemeral_scope() {
        let versions = vec![
            sample_version(MemoryScope::Ephemeral, "obj_e", "v1"),
            sample_version(MemoryScope::Agent("alpha".to_string()), "obj_a", "v2"),
        ];
        let materialized = materialize_context(
            "agent:alpha",
            &[],
            OwnerExportRedactionPolicy::AllowFull,
            versions.as_slice(),
        );
        assert_eq!(materialized.entries.len(), 1);
        assert_eq!(materialized.entries[0].object_id, "obj_a");
    }

    #[test]
    fn explicit_ephemeral_scope_request_includes_ephemeral() {
        let versions = vec![
            sample_version(MemoryScope::Ephemeral, "obj_e", "v1"),
            sample_version(MemoryScope::Agent("alpha".to_string()), "obj_a", "v2"),
        ];
        let materialized = materialize_context(
            "agent:alpha",
            &[
                MemoryScope::Ephemeral,
                MemoryScope::Agent("alpha".to_string()),
            ],
            OwnerExportRedactionPolicy::AllowFull,
            versions.as_slice(),
        );
        assert_eq!(materialized.entries.len(), 2);
        assert!(materialized
            .entries
            .iter()
            .any(|entry| entry.scope == MemoryScope::Ephemeral));
    }
}
