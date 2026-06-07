#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::context-pack.
//
// Gateway owns bounded context-pack construction for external Agent Runtime
// turns. This is projection/materialization glue only: Kernel Layer-2 memory
// remains canonical context authority, while this module shapes bounded packs
// for engine adapters and legacy hosts.

'use strict';

const {
  cleanGatewayText: cleanText,
  cleanGatewayDisplayText: cleanDisplayText,
} = require('../gateway_text_boundary.ts');

const AGENT_RUNTIME_CONTEXT_FANOUT_TARGET = 7;
const AGENT_RUNTIME_CONTEXT_HOT_TAIL_COUNT = 4;
const AGENT_RUNTIME_CONTEXT_MAX_ROWS = 49;
const AGENT_RUNTIME_CONTEXT_ROW_TEXT_MAX = 1200;
function estimateContextTokens(value) {
  return Math.max(1, Math.ceil(String(value == null ? '' : value).length / 4));
}
function cleanContextRole(value) {
  const role = cleanText(value, 40).toLowerCase();
  if (role === 'user' || role === 'human') return 'user';
  if (role === 'assistant' || role === 'agent' || role === 'ai') return 'assistant';
  if (role === 'tool' || role === 'function') return 'tool';
  if (role === 'system') return 'system';
  return 'message';
}
function contextRowText(row) {
  if (!row || typeof row !== 'object') return '';
  return cleanDisplayText(
    row.text_preview || row.content_preview || row.visible_text || row.text || row.message || row.content || '',
    AGENT_RUNTIME_CONTEXT_ROW_TEXT_MAX,
  );
}
function contextRef(prefix, sessionId, level, startSeq, endSeq) {
  const cleanSession = cleanText(sessionId, 120).replace(/[^a-zA-Z0-9_.:-]+/g, '_') || 'session';
  return `${prefix}_${cleanSession}_l${level}_${startSeq}_${endSeq}`;
}
function buildSpan(sessionId, level, children, status) {
  const rows = Array.isArray(children) ? children : [];
  const first = rows[0] || {};
  const last = rows[rows.length - 1] || first;
  const startSeq = Number(first.start_seq || first.sequence_no || 0) || 0;
  const endSeq = Number(last.end_seq || last.sequence_no || startSeq) || startSeq;
  const childRefs = rows.map((row) => cleanText(row.atom_id || row.span_id || row.ref_id, 160)).filter(Boolean);
  const decisions = [];
  const constraints = [];
  const openLoops = [];
  const entities = [];
  const taskRefs = [];
  const memoryVersionRefs = [];
  for (const row of rows) {
    for (const [source, target] of [
      [row.decisions, decisions],
      [row.constraints, constraints],
      [row.open_loops, openLoops],
      [row.entities, entities],
      [row.task_refs, taskRefs],
      [row.memory_version_refs, memoryVersionRefs],
    ]) {
      if (Array.isArray(source)) {
        for (const item of source) {
          const cleaned = cleanText(item, 180);
          if (cleaned && !target.includes(cleaned)) target.push(cleaned);
        }
      }
    }
  }
  const excerpt = rows
    .map((row) => cleanText(row.summary || row.text_preview || row.payload?.text_preview || row.ref_id || row.atom_id || row.span_id, 220))
    .filter(Boolean)
    .join(' | ')
    .slice(0, 900);
  return {
    span_id: contextRef('ctx_span_projection', sessionId, level, startSeq, endSeq),
    session_id: cleanText(sessionId, 200),
    level,
    status,
    start_seq: startSeq,
    end_seq: endSeq,
    child_refs: childRefs,
    summary: excerpt || `context rollup level=${level} covers ${startSeq}-${endSeq} from ${childRefs.length} children`,
    decisions,
    constraints,
    open_loops: openLoops,
    entities,
    task_refs: taskRefs,
    memory_version_refs: memoryVersionRefs,
    token_count: rows.reduce((sum, row) => sum + (Number(row.token_count) || estimateContextTokens(row.summary || row.text_preview || '')), 0),
    fidelity_score: 1,
  };
}
function buildAgentRuntimeContextPack(options = {}) {
  const body = options.body && typeof options.body === 'object' ? options.body : {};
  const sessionId = cleanText(options.sessionId || body.session_id, 200) || 'session';
  const agentId = cleanText(options.agentId || body.agent_id, 160) || 'default';
  const projection = body.context_projection && typeof body.context_projection === 'object' ? body.context_projection : {};
  const sourceRows = Array.isArray(projection.rows) ? projection.rows : [];
  const rawRows = sourceRows.slice(-(AGENT_RUNTIME_CONTEXT_MAX_ROWS * 2));
  const rows = [];
  const seenRows = new Set();
  for (const row of rawRows.slice().reverse()) {
    const text = contextRowText(row)
      .toLowerCase()
      .replace(/\s+/g, ' ')
      .replace(/[^\p{L}\p{N}\s._:/-]/gu, '')
      .trim();
    if (!text) continue;
    const role = cleanContextRole(row.role || row.origin_kind || row.actor);
    const key = `${role}:${text}`;
    if (seenRows.has(key)) continue;
    seenRows.add(key);
    rows.push(row);
  }
  rows.reverse();
  if (rows.length > AGENT_RUNTIME_CONTEXT_MAX_ROWS) rows.splice(0, rows.length - AGENT_RUNTIME_CONTEXT_MAX_ROWS);
  const atoms = [];
  rows.forEach((row, idx) => {
    const text = contextRowText(row);
    if (!text) return;
    const sequenceNo = idx + 1;
    const role = cleanContextRole(row.role || row.origin_kind || row.actor);
    const sourceKind = row.source_kind || (role === 'user'
      ? 'user_message'
      : role === 'assistant'
        ? 'assistant_message'
        : role === 'tool'
          ? 'tool_receipt'
          : role === 'system'
            ? 'system_event'
            : 'message_event');
    atoms.push({
      atom_id: cleanText(row.atom_id || row.id, 160) || contextRef('ctx_atom_projection', sessionId, 0, sequenceNo, sequenceNo),
      session_id: sessionId,
      sequence_no: sequenceNo,
      source_kind: sourceKind,
      record_type: sourceKind,
      source_ref: cleanText(row.detail_ref || row.id || `message-${sequenceNo}`, 240),
      source_authority: cleanText(row.source_authority || projection.source || 'shell_bounded_message_projection', 160),
      speaker_label: cleanText(row.speaker_label || row.origin_display_name || row.agent_name || role, 120),
      role,
      text_preview: text,
      token_count: Math.min(Number(row.token_count) || estimateContextTokens(text), 4000),
      task_refs: Array.isArray(row.task_refs) ? row.task_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 12) : [],
      memory_version_refs: Array.isArray(row.memory_version_refs) ? row.memory_version_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 12) : [],
      lineage_refs: Array.isArray(row.lineage_refs) ? row.lineage_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 12) : [],
    });
  });
  const spans = [];
  const level0 = [];
  for (let start = 0; start < atoms.length; start += AGENT_RUNTIME_CONTEXT_FANOUT_TARGET) {
    const chunk = atoms.slice(start, start + AGENT_RUNTIME_CONTEXT_FANOUT_TARGET);
    const status = chunk.length >= AGENT_RUNTIME_CONTEXT_FANOUT_TARGET ? 'sealed' : 'active';
    const span = buildSpan(sessionId, 0, chunk, status);
    level0.push(span);
    spans.push(span);
  }
  let sealed = level0.filter((row) => row.status === 'sealed');
  let level = 1;
  while (sealed.length >= AGENT_RUNTIME_CONTEXT_FANOUT_TARGET && level <= 5) {
    const parents = [];
    for (let start = 0; start + AGENT_RUNTIME_CONTEXT_FANOUT_TARGET <= sealed.length; start += AGENT_RUNTIME_CONTEXT_FANOUT_TARGET) {
      const parent = buildSpan(sessionId, level, sealed.slice(start, start + AGENT_RUNTIME_CONTEXT_FANOUT_TARGET), 'sealed');
      parents.push(parent);
      spans.push(parent);
    }
    sealed = parents;
    level += 1;
  }
  const byLevel = new Map();
  for (const span of spans) {
    const arr = byLevel.get(span.level) || [];
    arr.push(span);
    byLevel.set(span.level, arr);
  }
  const selectedSpans = [];
  for (const levelKey of Array.from(byLevel.keys()).sort((a, b) => a - b)) {
    const arr = byLevel.get(levelKey).slice().sort((a, b) => Number(b.end_seq) - Number(a.end_seq));
    if (arr[0]) selectedSpans.push(arr[0]);
  }
  const hotAtoms = atoms.slice(-AGENT_RUNTIME_CONTEXT_HOT_TAIL_COUNT);
  const fragments = [
    ...hotAtoms.map((atom) => ({
      fragment_id: contextRef('ctx_fragment_atom_projection', sessionId, 0, atom.sequence_no, atom.sequence_no),
      kind: 'atom',
      ref_id: atom.atom_id,
      level: 0,
      token_count: atom.token_count,
      payload: {
        source_kind: atom.source_kind,
        record_type: atom.record_type || atom.source_kind,
        source_ref: atom.source_ref,
        source_authority: atom.source_authority,
        speaker_label: atom.speaker_label,
        role: atom.role,
        text_preview: atom.text_preview,
        sequence_no: atom.sequence_no,
      },
      lineage_refs: atom.lineage_refs,
    })),
    ...selectedSpans.map((span) => ({
      fragment_id: contextRef('ctx_fragment_span_projection', sessionId, span.level, span.start_seq, span.end_seq),
      kind: 'span',
      ref_id: span.span_id,
      level: span.level,
      token_count: span.token_count,
      payload: {
        summary: span.summary,
        decisions: span.decisions,
        constraints: span.constraints,
        open_loops: span.open_loops,
        entities: span.entities,
        task_refs: span.task_refs,
        memory_version_refs: span.memory_version_refs,
        fidelity_score: span.fidelity_score,
        status: span.status,
        coverage: { start_seq: span.start_seq, end_seq: span.end_seq },
      },
      lineage_refs: [],
    })),
  ];
  return {
    schema_version: 1,
    type: 'agent_runtime_context_pack',
    source_basis: 'core.layer2.memory.context_topology_projection',
    source_authority: 'gateway_bounded_projection_pending_kernel_materializer_route',
    canonical_model_refs: [
      'core/layer2/memory/src/context_atoms.rs',
      'core/layer2/memory/src/context_topology.rs',
      'core/layer2/memory/src/context_materializer.rs',
    ],
    session_id: sessionId,
    agent_id: agentId,
    fanout_target: AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
    hot_tail_count: AGENT_RUNTIME_CONTEXT_HOT_TAIL_COUNT,
    row_count: atoms.length,
    raw_row_count: sourceRows.length,
    dedupe_policy: {
      type: 'gateway_projection_tail_window',
      key_basis: 'source_projection_rows',
      preserves_latest_duplicate: true,
    },
    frontier: {
      session_id: sessionId,
      hot_atom_refs: hotAtoms.map((row) => row.atom_id),
      warm_span_refs: selectedSpans.filter((row) => row.level === 0).map((row) => row.span_id),
      cool_span_refs: selectedSpans.filter((row) => row.level === 1 || row.level === 2).map((row) => row.span_id),
      cold_span_refs: selectedSpans.filter((row) => row.level >= 3).map((row) => row.span_id),
      pinned_anchor_refs: Array.isArray(projection.pinned_anchor_refs) ? projection.pinned_anchor_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 16) : [],
      budget_tokens: 6000,
      used_tokens: fragments.reduce((sum, row) => sum + (Number(row.token_count) || 0), 0),
      pressure_state: fragments.length > 24 ? 'medium' : 'low',
      fidelity_score: 1,
    },
    fragments,
  };
}

module.exports = {
  AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
  buildAgentRuntimeContextPack,
};
