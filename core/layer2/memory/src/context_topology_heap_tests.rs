use crate::context_atoms::ContextAtomSourceKind;
use crate::context_topology::ContextAppendInput;
use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap};
use crate::policy::DefaultVerityMemoryPolicy;
use crate::schemas::{CapabilityAction, CapabilityToken, MemoryScope};

fn route() -> NexusRouteContext {
    NexusRouteContext {
        issuer: "memory_topology_tests".to_string(),
        source: "client_ingress".to_string(),
        target: "memory_heap".to_string(),
        schema_id: "memory.context.topology".to_string(),
        lease_id: "lease_ctx".to_string(),
        template_version_id: Some("v1".to_string()),
        ttl_ms: Some(30_000),
    }
}

fn token() -> CapabilityToken {
    CapabilityToken {
        token_id: "cap_ctx".to_string(),
        principal_id: "agent:alpha".to_string(),
        scopes: vec![MemoryScope::Core, MemoryScope::Agent("alpha".to_string())],
        allowed_actions: vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::MaterializeContext,
        ],
        expires_at_ms: u64::MAX,
        verity_class: "standard".to_string(),
        receipt_id: "cap_receipt".to_string(),
    }
}

#[test]
fn topology_receipts_and_lineage_emit_for_append_rollup_and_rebuild() {
    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let route = route();
    let cap = token();
    for idx in 0..7 {
        let outcome = heap
            .append_context_atom(
                &route,
                "agent:alpha",
                &cap,
                ContextAppendInput {
                    session_id: "session-1".to_string(),
                    source_kind: ContextAtomSourceKind::InteractionUnit,
                    source_ref: format!("turn-{idx}"),
                    text_preview: format!("turn {idx} topology receipt preview"),
                    token_count: 300,
                    task_refs: vec!["task-open".to_string()],
                    memory_version_refs: vec![],
                    semantic_boundary: true,
                    workflow_boundary: false,
                    lineage_refs: vec!["lineage:atom".to_string()],
                },
                vec!["lineage:request".to_string()],
            )
            .expect("append");
        assert!(!outcome.atom.atom_id.is_empty());
    }
    let rebuild = heap
        .rebuild_context_topology(
            &route,
            "agent:alpha",
            &cap,
            "session-1",
            vec!["lineage:rebuild".to_string()],
        )
        .expect("rebuild");
    assert_eq!(rebuild.session_id, "session-1");

    let event_types = heap
        .receipts()
        .iter()
        .map(|row| row.event_type.clone())
        .collect::<Vec<_>>();
    assert!(event_types.iter().any(|row| row == "context_atom_append"));
    assert!(event_types.iter().any(|row| row == "context_span_seal"));
    assert!(event_types.iter().any(|row| row == "context_span_rollup"));
    assert!(event_types
        .iter()
        .any(|row| row == "context_topology_rebuild"));
    assert!(heap
        .receipts()
        .iter()
        .any(|row| row.lineage_refs.iter().any(|lin| lin == "lineage:request")));
}

#[test]
fn topology_materialization_preserves_pinned_anchor_and_does_not_mutate_task_fabric() {
    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let route = route();
    let cap = token();
    let _ = heap
        .append_context_atom(
            &route,
            "agent:alpha",
            &cap,
            ContextAppendInput {
                session_id: "session-2".to_string(),
                source_kind: ContextAtomSourceKind::StatusSummary,
                source_ref: "status:ready".to_string(),
                text_preview: "status ready with task blocker 42".to_string(),
                token_count: 180,
                task_refs: vec!["task:blocker:42".to_string()],
                memory_version_refs: vec![],
                semantic_boundary: false,
                workflow_boundary: false,
                lineage_refs: vec!["lineage:summary".to_string()],
            },
            vec!["lineage:req".to_string()],
        )
        .expect("append");

    let materialized = heap
        .materialize_context_topology(
            &route,
            "agent:alpha",
            &cap,
            "session-2",
            vec![],
            256,
            vec!["task:blocker:42".to_string()],
            vec!["lineage:mat".to_string()],
        )
        .expect("materialize");

    assert!(materialized
        .frontier
        .pinned_anchor_refs
        .contains(&"task:blocker:42".to_string()));
    assert!(heap.graph_subsystem().get_node("task:blocker:42").is_none());
    let frontier_receipt = heap
        .receipts()
        .iter()
        .rev()
        .find(|row| row.event_type == "context_frontier_update")
        .expect("frontier receipt");
    assert!(frontier_receipt.details.get("hot_tokens").is_some());
    assert!(frontier_receipt.details.get("warm_tokens").is_some());
    assert!(frontier_receipt.details.get("cool_tokens").is_some());
    assert!(frontier_receipt.details.get("cold_tokens").is_some());
    assert!(frontier_receipt.details.get("pinned_tokens").is_some());
}

#[test]
fn topology_materialization_runs_background_compaction_and_emits_rollup_receipts() {
    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let route = route();
    let cap = token();
    heap.context_topology.config.fanout_target = 99;
    for idx in 0..12 {
        let _ = heap
            .append_context_atom(
                &route,
                "agent:alpha",
                &cap,
                ContextAppendInput {
                    session_id: "session-3".to_string(),
                    source_kind: ContextAtomSourceKind::InteractionUnit,
                    source_ref: format!("turn-{idx}"),
                    text_preview: format!("turn {idx} compaction preview"),
                    token_count: 180,
                    task_refs: vec!["task-open".to_string()],
                    memory_version_refs: vec![],
                    semantic_boundary: true,
                    workflow_boundary: false,
                    lineage_refs: vec!["lineage:tick".to_string()],
                },
                vec!["lineage:req".to_string()],
            )
            .expect("append");
    }
    let rollup_receipts_before = heap
        .receipts()
        .iter()
        .filter(|row| row.event_type == "context_span_rollup")
        .count();
    heap.context_topology.config.fanout_target = 2;
    let _ = heap
        .materialize_context_topology(
            &route,
            "agent:alpha",
            &cap,
            "session-3",
            vec![],
            512,
            vec![],
            vec!["lineage:materialize".to_string()],
        )
        .expect("materialize");
    let rollup_receipts_after = heap
        .receipts()
        .iter()
        .filter(|row| row.event_type == "context_span_rollup")
        .count();
    assert!(rollup_receipts_after > rollup_receipts_before);
    let spans = heap.context_topology().session_spans("session-3");
    assert!(spans.iter().any(|row| row.level > 0
        && matches!(
            row.status,
            crate::context_topology::ContextSpanStatus::Sealed
        )));
}
