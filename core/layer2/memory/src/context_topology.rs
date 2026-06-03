use crate::context_atoms::{ContextAtom, ContextAtomSourceKind};
use crate::context_budget::{build_frontier, ContextBudgetReport, ContextBudgetRequest};
use crate::context_compaction::{
    build_rollup_parent, contiguous_exact_coverage, should_seal_level0,
};
use crate::deterministic_hash;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_FANOUT_TARGET: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSpanStatus {
    Active,
    Sealed,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextSpan {
    pub span_id: String,
    pub session_id: String,
    pub level: u32,
    pub status: ContextSpanStatus,
    pub start_seq: u64,
    pub end_seq: u64,
    pub child_refs: Vec<String>,
    pub summary: String,
    pub decisions: Vec<String>,
    pub constraints: Vec<String>,
    pub open_loops: Vec<String>,
    pub entities: Vec<String>,
    pub task_refs: Vec<String>,
    pub memory_version_refs: Vec<String>,
    pub token_count: u32,
    pub heat_score: f32,
    pub fidelity_score: f32,
    pub receipt_id: String,
    pub lineage_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPressureState {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextFrontier {
    pub session_id: String,
    pub budget_tokens: u32,
    pub used_tokens: u32,
    pub hot_atom_refs: Vec<String>,
    pub warm_span_refs: Vec<String>,
    pub cool_span_refs: Vec<String>,
    pub cold_span_refs: Vec<String>,
    pub pinned_anchor_refs: Vec<String>,
    pub pressure_state: ContextPressureState,
    pub fidelity_score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTopologyConfig {
    pub fanout_target: usize,
}

impl Default for ContextTopologyConfig {
    fn default() -> Self {
        Self {
            fanout_target: DEFAULT_FANOUT_TARGET,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextAppendInput {
    pub session_id: String,
    pub source_kind: ContextAtomSourceKind,
    pub source_ref: String,
    pub text_preview: String,
    pub token_count: u32,
    pub task_refs: Vec<String>,
    pub memory_version_refs: Vec<String>,
    pub semantic_boundary: bool,
    pub workflow_boundary: bool,
    pub lineage_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextAppendOutcome {
    pub atom: ContextAtom,
    pub sealed_spans: Vec<ContextSpan>,
    pub rolled_up_spans: Vec<ContextSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTopologyRebuildReport {
    pub session_id: String,
    pub atom_count: usize,
    pub rebuilt_span_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ContextTopology {
    pub config: ContextTopologyConfig,
    atoms_by_session: BTreeMap<String, Vec<ContextAtom>>,
    spans_by_session: BTreeMap<String, Vec<ContextSpan>>,
    frontiers_by_session: BTreeMap<String, ContextFrontier>,
    next_sequence_by_session: BTreeMap<String, u64>,
}

impl ContextTopology {
    pub fn append_atom(
        &mut self,
        input: ContextAppendInput,
    ) -> Result<ContextAppendOutcome, String> {
        let sequence_no = self
            .next_sequence_by_session
            .entry(input.session_id.clone())
            .and_modify(|row| *row = row.saturating_add(1))
            .or_insert(1);
        let atom = ContextAtom::new(
            input.session_id.as_str(),
            *sequence_no,
            input.source_kind,
            input.source_ref.as_str(),
            input.text_preview.as_str(),
            input.token_count,
            dedupe(input.task_refs),
            dedupe(input.memory_version_refs),
            dedupe(input.lineage_refs.clone()),
        );
        let session_atoms = self
            .atoms_by_session
            .entry(input.session_id.clone())
            .or_default();
        session_atoms.push(atom.clone());

        let session_spans = self
            .spans_by_session
            .entry(input.session_id.clone())
            .or_default();
        let active_idx = session_spans
            .iter()
            .position(|row| row.level == 0 && matches!(row.status, ContextSpanStatus::Active));
        let idx = if let Some(index) = active_idx {
            index
        } else {
            session_spans.push(new_active_span(
                input.session_id.as_str(),
                atom.sequence_no,
                atom.sequence_no,
            ));
            session_spans.len().saturating_sub(1)
        };
        let active = session_spans
            .get_mut(idx)
            .ok_or_else(|| "context_active_span_missing".to_string())?;
        active.end_seq = atom.sequence_no;
        active.child_refs.push(atom.atom_id.clone());
        active.token_count = active.token_count.saturating_add(atom.token_count);
        active.task_refs.extend(atom.task_refs.clone());
        active.task_refs = dedupe(active.task_refs.clone());
        active
            .memory_version_refs
            .extend(atom.memory_version_refs.clone());
        active.memory_version_refs = dedupe(active.memory_version_refs.clone());
        active.lineage_refs.extend(atom.lineage_refs.clone());
        active.lineage_refs = dedupe(active.lineage_refs.clone());
        active.heat_score = ((active.heat_score * 0.92) + 0.08).clamp(0.0, 1.0);

        let mut sealed_spans = Vec::<ContextSpan>::new();
        if should_seal_level0(active, input.semantic_boundary, input.workflow_boundary) {
            active.status = ContextSpanStatus::Sealed;
            active.summary = format!(
                "level-0 span covering sequence {}-{}",
                active.start_seq, active.end_seq
            );
            active.fidelity_score = 1.0;
            sealed_spans.push(active.clone());
        }

        let rolled_up_spans = self.rollup_eligible_sealed(input.session_id.as_str())?;
        Ok(ContextAppendOutcome {
            atom,
            sealed_spans,
            rolled_up_spans,
        })
    }

    pub fn materialize_frontier(
        &mut self,
        request: ContextBudgetRequest,
    ) -> (ContextFrontier, ContextBudgetReport) {
        let atoms = self
            .atoms_by_session
            .get(request.session_id.as_str())
            .cloned()
            .unwrap_or_default();
        let spans = self
            .spans_by_session
            .get(request.session_id.as_str())
            .cloned()
            .unwrap_or_default();
        let previous = self
            .frontiers_by_session
            .get(request.session_id.as_str())
            .cloned();
        let (frontier, report) = build_frontier(
            &request,
            atoms.as_slice(),
            spans.as_slice(),
            previous.as_ref(),
        );
        self.frontiers_by_session
            .insert(request.session_id.clone(), frontier.clone());
        (frontier, report)
    }

    pub fn rebuild_session_topology(
        &mut self,
        session_id: &str,
    ) -> Result<ContextTopologyRebuildReport, String> {
        let atoms = self
            .atoms_by_session
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        if atoms.is_empty() {
            self.spans_by_session.remove(session_id);
            self.frontiers_by_session.remove(session_id);
            return Ok(ContextTopologyRebuildReport {
                session_id: session_id.to_string(),
                atom_count: 0,
                rebuilt_span_count: 0,
            });
        }
        let mut sorted_atoms = atoms;
        sorted_atoms.sort_by(|a, b| a.sequence_no.cmp(&b.sequence_no));
        let mut spans = Vec::<ContextSpan>::new();
        for chunk in sorted_atoms.chunks(self.config.fanout_target.max(1)) {
            let start = chunk.first().map(|row| row.sequence_no).unwrap_or(0);
            let end = chunk.last().map(|row| row.sequence_no).unwrap_or(0);
            let full_chunk = chunk.len() >= self.config.fanout_target.max(1);
            let status = if full_chunk {
                ContextSpanStatus::Sealed
            } else {
                ContextSpanStatus::Active
            };
            spans.push(ContextSpan {
                span_id: format!(
                    "ctx_span_{}",
                    &deterministic_hash(&(
                        session_id.to_string(),
                        0u32,
                        start,
                        end,
                        status.clone()
                    ))[..24]
                ),
                session_id: session_id.to_string(),
                level: 0,
                status,
                start_seq: start,
                end_seq: end,
                child_refs: chunk.iter().map(|row| row.atom_id.clone()).collect(),
                summary: format!("rebuilt level-0 span {}-{}", start, end),
                decisions: vec![],
                constraints: vec![],
                open_loops: vec![],
                entities: vec![],
                task_refs: dedupe(chunk.iter().flat_map(|row| row.task_refs.clone()).collect()),
                memory_version_refs: dedupe(
                    chunk
                        .iter()
                        .flat_map(|row| row.memory_version_refs.clone())
                        .collect(),
                ),
                token_count: chunk.iter().map(|row| row.token_count).sum(),
                heat_score: 0.55,
                fidelity_score: 1.0,
                receipt_id: String::new(),
                lineage_refs: dedupe(
                    chunk
                        .iter()
                        .flat_map(|row| row.lineage_refs.clone())
                        .collect(),
                ),
            });
        }
        self.spans_by_session.insert(session_id.to_string(), spans);
        let rolled = self.rollup_eligible_sealed(session_id)?;
        Ok(ContextTopologyRebuildReport {
            session_id: session_id.to_string(),
            atom_count: sorted_atoms.len(),
            rebuilt_span_count: self
                .spans_by_session
                .get(session_id)
                .map(|rows| rows.len())
                .unwrap_or(0)
                + rolled.len(),
        })
    }

    pub fn set_span_receipt(&mut self, session_id: &str, span_id: &str, receipt_id: &str) {
        if let Some(rows) = self.spans_by_session.get_mut(session_id) {
            if let Some(span) = rows.iter_mut().find(|row| row.span_id == span_id) {
                span.receipt_id = receipt_id.to_string();
            }
        }
    }

    pub fn session_atoms(&self, session_id: &str) -> Vec<ContextAtom> {
        self.atoms_by_session
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn session_spans(&self, session_id: &str) -> Vec<ContextSpan> {
        self.spans_by_session
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn session_frontier(&self, session_id: &str) -> Option<ContextFrontier> {
        self.frontiers_by_session.get(session_id).cloned()
    }

    pub fn compact_sealed_session(&mut self, session_id: &str) -> Result<Vec<ContextSpan>, String> {
        self.rollup_eligible_sealed(session_id)
    }

    fn rollup_eligible_sealed(&mut self, session_id: &str) -> Result<Vec<ContextSpan>, String> {
        let mut rolled = Vec::<ContextSpan>::new();
        loop {
            let Some(level) = self.lowest_rollup_level(session_id) else {
                break;
            };
            let candidates = self
                .spans_by_session
                .get(session_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|row| row.level == level && matches!(row.status, ContextSpanStatus::Sealed))
                .collect::<Vec<_>>();
            if candidates.len() < self.config.fanout_target.max(1) {
                break;
            }
            let mut sorted = candidates;
            sorted.sort_by(|a, b| {
                a.start_seq
                    .cmp(&b.start_seq)
                    .then(a.end_seq.cmp(&b.end_seq))
            });
            let chunk = sorted
                .into_iter()
                .take(self.config.fanout_target.max(1))
                .collect::<Vec<_>>();
            let mut parent =
                build_rollup_parent(session_id, level.saturating_add(1), chunk.as_slice())?;
            if !contiguous_exact_coverage(&parent, chunk.as_slice()) {
                return Err("context_rollup_contiguity_violation".to_string());
            }
            parent.status = ContextSpanStatus::Sealed;
            let chunk_ids = chunk
                .iter()
                .map(|row| row.span_id.clone())
                .collect::<BTreeSet<_>>();
            if let Some(rows) = self.spans_by_session.get_mut(session_id) {
                for row in rows.iter_mut() {
                    if chunk_ids.contains(&row.span_id) {
                        row.status = ContextSpanStatus::Superseded;
                    }
                }
                rows.push(parent.clone());
            }
            rolled.push(parent);
        }
        Ok(rolled)
    }

    fn lowest_rollup_level(&self, session_id: &str) -> Option<u32> {
        self.spans_by_session
            .get(session_id)
            .map(|rows| {
                rows.iter()
                    .filter(|row| matches!(row.status, ContextSpanStatus::Sealed))
                    .map(|row| row.level)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .next()
            })
            .unwrap_or(None)
    }
}

fn new_active_span(session_id: &str, start_seq: u64, end_seq: u64) -> ContextSpan {
    ContextSpan {
        span_id: format!(
            "ctx_span_{}",
            &deterministic_hash(&(session_id.to_string(), 0u32, start_seq, end_seq, "active"))
                [..24]
        ),
        session_id: session_id.to_string(),
        level: 0,
        status: ContextSpanStatus::Active,
        start_seq,
        end_seq,
        child_refs: Vec::new(),
        summary: String::new(),
        decisions: Vec::new(),
        constraints: Vec::new(),
        open_loops: Vec::new(),
        entities: Vec::new(),
        task_refs: Vec::new(),
        memory_version_refs: Vec::new(),
        token_count: 0,
        heat_score: 0.92,
        fidelity_score: 1.0,
        receipt_id: String::new(),
        lineage_refs: Vec::new(),
    }
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut out = values
        .into_iter()
        .filter(|row| !row.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_span_is_never_compacted() {
        let mut topology = ContextTopology::default();
        for idx in 0..8 {
            topology
                .append_atom(ContextAppendInput {
                    session_id: "s".to_string(),
                    source_kind: ContextAtomSourceKind::InteractionUnit,
                    source_ref: format!("turn-{idx}"),
                    text_preview: format!("turn {idx} bounded context preview"),
                    token_count: 400,
                    task_refs: vec!["task-open".to_string()],
                    memory_version_refs: vec![],
                    semantic_boundary: false,
                    workflow_boundary: false,
                    lineage_refs: vec!["lineage".to_string()],
                })
                .expect("append");
        }
        let spans = topology.session_spans("s");
        assert!(spans
            .iter()
            .any(|row| row.level == 0 && matches!(row.status, ContextSpanStatus::Active)));
        assert!(!spans
            .iter()
            .any(|row| row.level > 0 && matches!(row.status, ContextSpanStatus::Active)));
    }

    #[test]
    fn rebuild_from_atoms_preserves_coverage() {
        let mut topology = ContextTopology::default();
        for idx in 0..15 {
            topology
                .append_atom(ContextAppendInput {
                    session_id: "s".to_string(),
                    source_kind: ContextAtomSourceKind::InteractionUnit,
                    source_ref: format!("turn-{idx}"),
                    text_preview: format!("turn {idx} rebuild preview"),
                    token_count: 120,
                    task_refs: vec!["task-1".to_string()],
                    memory_version_refs: vec!["mem-1".to_string()],
                    semantic_boundary: false,
                    workflow_boundary: false,
                    lineage_refs: vec!["lin".to_string()],
                })
                .expect("append");
        }
        let before = topology.session_spans("s");
        let before_range = (
            before.iter().map(|row| row.start_seq).min().unwrap_or(0),
            before.iter().map(|row| row.end_seq).max().unwrap_or(0),
        );
        let report = topology.rebuild_session_topology("s").expect("rebuild");
        assert_eq!(report.atom_count, 15);
        let after = topology.session_spans("s");
        let after_range = (
            after.iter().map(|row| row.start_seq).min().unwrap_or(0),
            after.iter().map(|row| row.end_seq).max().unwrap_or(0),
        );
        assert_eq!(before_range, after_range);
    }
}
