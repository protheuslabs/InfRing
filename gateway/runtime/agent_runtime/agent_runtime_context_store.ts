#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::context-store.
//
// TRANSITIONAL FALLBACK ONLY.
//
// This module is not context authority. Kernel Layer-2 memory owns context
// truth and materialization. The Gateway may temporarily persist bounded
// bootstrap projection rows only while the Kernel materializer is unavailable
// to packaged runtime flows. Production runtime context must prefer
// `agent_runtime_context_materializer` through the Kernel bridge.

'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const FANOUT_TARGET = 7;
const HOT_TAIL_COUNT = 4;
const MAX_STORED_ATOMS = 512;
const MAX_BOOTSTRAP_ROWS = 49;
const MAX_FRAGMENT_COUNT = 48;
const MAX_TEXT_PREVIEW = 1200;
const CONTEXT_STORE_AUTHORITY_CLASSIFICATION = Object.freeze({
  role: 'transitional_gateway_cache',
  canonical_authority: false,
  boundary_contract: 'validation/conformance/contracts/context_authority_boundary_contract.json',
  canonical_authority_owner: 'kernel.layer2.memory',
  fallback_only_when: 'kernel_context_materializer_binary_unavailable',
  retirement_trigger: 'kernel_materializer_packaged_for_runtime',
  deadline: '2026-07-15',
});

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function cleanDisplayText(value, max = MAX_TEXT_PREVIEW) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{5,}/g, '\n\n\n\n')
    .trim()
    .slice(0, max);
}

function digest(value) {
  return crypto.createHash('sha256').update(JSON.stringify(value)).digest('hex');
}

function estimateTokens(value) {
  return Math.max(1, Math.ceil(String(value == null ? '' : value).length / 4));
}

function safeId(value, fallback = 'default') {
  return cleanString(value, 180).replace(/[^a-zA-Z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, '') || fallback;
}

function statePath(root, sessionId) {
  return path.join(root || process.cwd(), 'core', 'local', 'state', 'agent_runtime', 'context', `${safeId(sessionId, 'session')}.json`);
}

function emptyState(sessionId, agentId) {
  return {
    schema_version: 1,
    type: 'agent_runtime_context_state',
    session_id: cleanString(sessionId, 200),
    agent_id: cleanString(agentId, 160),
    fanout_target: FANOUT_TARGET,
    next_sequence_no: 1,
    atoms: [],
    updated_at: new Date().toISOString(),
  };
}

function loadState(root, sessionId, agentId) {
  const file = statePath(root, sessionId);
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (parsed && parsed.type === 'agent_runtime_context_state' && Array.isArray(parsed.atoms)) {
      parsed.session_id = cleanString(parsed.session_id || sessionId, 200);
      parsed.agent_id = cleanString(parsed.agent_id || agentId, 160);
      parsed.next_sequence_no = Math.max(1, Number(parsed.next_sequence_no) || (parsed.atoms.length + 1));
      return parsed;
    }
  } catch {}
  return emptyState(sessionId, agentId);
}

function saveState(root, state) {
  const file = statePath(root, state.session_id);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  state.updated_at = new Date().toISOString();
  fs.writeFileSync(file, `${JSON.stringify(state, null, 2)}\n`);
  return file;
}

function atomKey(atom) {
  return cleanString(atom.source_ref, 240) || `${cleanString(atom.role, 40)}:${digest(cleanString(atom.text_preview, MAX_TEXT_PREVIEW)).slice(0, 24)}`;
}

function normalizeRole(value) {
  const role = cleanString(value, 40).toLowerCase();
  if (role === 'human') return 'user';
  if (role === 'agent' || role === 'ai') return 'assistant';
  if (role === 'function') return 'tool';
  if (['user', 'assistant', 'tool', 'system'].includes(role)) return role;
  return 'message';
}

function sourceKindForRole(role, explicit) {
  const kind = cleanString(explicit, 80);
  if (kind && kind !== 'interaction_unit') return kind;
  if (role === 'user') return 'user_message';
  if (role === 'assistant') return 'assistant_message';
  if (role === 'tool') return 'tool_receipt';
  if (role === 'system') return 'system_event';
  return 'message_event';
}

function appendAtomToState(state, row) {
  const text = cleanDisplayText(row && (row.text_preview || row.text || row.message || row.content), MAX_TEXT_PREVIEW);
  if (!text) return null;
  const role = normalizeRole(row.role || row.origin_kind || row.actor);
  const sourceKind = sourceKindForRole(role, row.source_kind);
  const sourceRef = cleanString(row.source_ref || row.detail_ref || row.id || `${role}:${digest(text).slice(0, 16)}`, 240);
  const candidate = {
    atom_id: '',
    session_id: state.session_id,
    sequence_no: Number(state.next_sequence_no) || 1,
    source_kind: sourceKind,
    record_type: sourceKind,
    source_ref: sourceRef,
    source_authority: cleanString(row.source_authority || 'gateway_bounded_projection', 160),
    speaker_label: cleanString(row.speaker_label || row.origin_display_name || row.agent_name || role, 120),
    role,
    text_preview: text,
    token_count: Math.min(Number(row.token_count) || estimateTokens(text), 4000),
    timestamp_ms: Date.now(),
    task_refs: Array.isArray(row.task_refs) ? row.task_refs.map((item) => cleanString(item, 180)).filter(Boolean).slice(0, 12) : [],
    memory_version_refs: Array.isArray(row.memory_version_refs) ? row.memory_version_refs.map((item) => cleanString(item, 180)).filter(Boolean).slice(0, 12) : [],
    lineage_refs: Array.isArray(row.lineage_refs) ? row.lineage_refs.map((item) => cleanString(item, 180)).filter(Boolean).slice(0, 12) : [],
  };
  const key = atomKey(candidate);
  const existing = state.atoms.find((atom) => atomKey(atom) === key);
  if (existing) return existing;
  candidate.atom_id = `ctx_atom_${digest([state.session_id, candidate.sequence_no, sourceRef, text]).slice(0, 24)}`;
  state.atoms.push(candidate);
  state.next_sequence_no = candidate.sequence_no + 1;
  if (state.atoms.length > MAX_STORED_ATOMS) {
    state.atoms = state.atoms.slice(-MAX_STORED_ATOMS);
  }
  return candidate;
}

function buildSpan(sessionId, level, children, status) {
  const rows = Array.isArray(children) ? children : [];
  const first = rows[0] || {};
  const last = rows[rows.length - 1] || first;
  const startSeq = Number(first.start_seq || first.sequence_no || 0) || 0;
  const endSeq = Number(last.end_seq || last.sequence_no || startSeq) || startSeq;
  const childRefs = rows.map((row) => cleanString(row.atom_id || row.span_id || row.ref_id, 180)).filter(Boolean);
  const summary = rows
    .map((row) => cleanDisplayText(row.summary || row.text_preview || row.payload?.summary || row.payload?.text_preview || '', 220))
    .filter(Boolean)
    .join(' | ')
    .slice(0, 900);
  return {
    span_id: `ctx_span_${digest([sessionId, level, startSeq, endSeq, childRefs]).slice(0, 24)}`,
    session_id: sessionId,
    level,
    status,
    start_seq: startSeq,
    end_seq: endSeq,
    child_refs: childRefs,
    summary: summary || `context rollup level=${level} covers ${startSeq}-${endSeq} from ${childRefs.length} children`,
    decisions: dedupe(rows.flatMap((row) => Array.isArray(row.decisions) ? row.decisions : [])),
    constraints: dedupe(rows.flatMap((row) => Array.isArray(row.constraints) ? row.constraints : [])),
    open_loops: dedupe(rows.flatMap((row) => Array.isArray(row.open_loops) ? row.open_loops : [])),
    entities: dedupe(rows.flatMap((row) => Array.isArray(row.entities) ? row.entities : [])),
    task_refs: dedupe(rows.flatMap((row) => Array.isArray(row.task_refs) ? row.task_refs : [])),
    memory_version_refs: dedupe(rows.flatMap((row) => Array.isArray(row.memory_version_refs) ? row.memory_version_refs : [])),
    token_count: rows.reduce((sum, row) => sum + (Number(row.token_count) || estimateTokens(row.summary || row.text_preview || '')), 0),
    heat_score: 0.75,
    fidelity_score: 1,
    lineage_refs: dedupe(rows.flatMap((row) => Array.isArray(row.lineage_refs) ? row.lineage_refs : [])),
  };
}

function dedupe(values) {
  const out = [];
  const seen = new Set();
  for (const value of values || []) {
    const cleaned = cleanString(value, 180);
    if (!cleaned || seen.has(cleaned)) continue;
    seen.add(cleaned);
    out.push(cleaned);
  }
  return out;
}

function atomSemanticKey(atom) {
  const role = cleanString(atom && atom.role, 40).toLowerCase();
  const sourceKind = sourceKindForRole(role, atom && atom.source_kind).toLowerCase();
  const text = cleanDisplayText(atom && atom.text_preview, MAX_TEXT_PREVIEW)
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .replace(/[^\p{L}\p{N}\s._:/-]/gu, '')
    .trim();
  return digest([role, sourceKind, text]).slice(0, 32);
}

function dedupeAtomsForProjection(atoms) {
  const out = [];
  const seen = new Set();
  for (const atom of (Array.isArray(atoms) ? atoms : []).slice().reverse()) {
    const key = atomSemanticKey(atom);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    out.push(atom);
  }
  return out.reverse();
}

function buildTopology(sessionId, atoms) {
  const sortedAtoms = (Array.isArray(atoms) ? atoms : []).slice().sort((a, b) => Number(a.sequence_no) - Number(b.sequence_no));
  const spans = [];
  const level0 = [];
  for (let start = 0; start < sortedAtoms.length; start += FANOUT_TARGET) {
    const chunk = sortedAtoms.slice(start, start + FANOUT_TARGET);
    const span = buildSpan(sessionId, 0, chunk, chunk.length >= FANOUT_TARGET ? 'sealed' : 'active');
    level0.push(span);
    spans.push(span);
  }
  let sealed = level0.filter((span) => span.status === 'sealed');
  let level = 1;
  while (sealed.length >= FANOUT_TARGET && level <= 6) {
    const parents = [];
    for (let start = 0; start + FANOUT_TARGET <= sealed.length; start += FANOUT_TARGET) {
      const parent = buildSpan(sessionId, level, sealed.slice(start, start + FANOUT_TARGET), 'sealed');
      parents.push(parent);
      spans.push(parent);
    }
    sealed = parents;
    level += 1;
  }
  return { atoms: sortedAtoms, spans };
}

function selectedSpansByTier(spans) {
  const byLevel = new Map();
  for (const span of spans || []) {
    if (span.status !== 'sealed') continue;
    const rows = byLevel.get(span.level) || [];
    rows.push(span);
    byLevel.set(span.level, rows);
  }
  const selected = [];
  for (const level of Array.from(byLevel.keys()).sort((a, b) => a - b)) {
    const rows = byLevel.get(level).slice().sort((a, b) => Number(b.end_seq) - Number(a.end_seq));
    if (rows[0]) selected.push(rows[0]);
  }
  return selected;
}

function materializeFromState(state) {
  const projectionAtoms = dedupeAtomsForProjection(state.atoms);
  const topology = buildTopology(state.session_id, projectionAtoms);
  const hotAtoms = topology.atoms.slice(-HOT_TAIL_COUNT);
  const selectedSpans = selectedSpansByTier(topology.spans);
  const fragments = [
    ...hotAtoms.map((atom) => ({
      fragment_id: `ctx_fragment_${digest([atom.atom_id, 'atom']).slice(0, 24)}`,
      kind: 'atom',
      ref_id: atom.atom_id,
      level: 0,
      token_count: atom.token_count,
      payload: {
        source_kind: sourceKindForRole(atom.role, atom.source_kind),
        record_type: sourceKindForRole(atom.role, atom.source_kind),
        source_ref: atom.source_ref,
        source_authority: cleanString(atom.source_authority || 'gateway_runtime_context_store_pending_kernel_materializer_route', 160),
        speaker_label: cleanString(atom.speaker_label || atom.role, 120),
        role: atom.role,
        text_preview: atom.text_preview,
        sequence_no: atom.sequence_no,
      },
      lineage_refs: atom.lineage_refs,
    })),
    ...selectedSpans.map((span) => ({
      fragment_id: `ctx_fragment_${digest([span.span_id, 'span']).slice(0, 24)}`,
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
      lineage_refs: span.lineage_refs,
    })),
  ].slice(-MAX_FRAGMENT_COUNT);
  return {
    schema_version: 1,
    type: 'agent_runtime_context_pack',
    source_basis: 'core.layer2.memory.context_topology',
    source_authority: 'gateway_runtime_context_store_pending_kernel_materializer_route',
    canonical_model_refs: [
      'core/layer2/memory/src/context_atoms.rs',
      'core/layer2/memory/src/context_topology.rs',
      'core/layer2/memory/src/context_materializer.rs',
    ],
    session_id: state.session_id,
    agent_id: state.agent_id,
    fanout_target: FANOUT_TARGET,
    hot_tail_count: HOT_TAIL_COUNT,
    row_count: topology.atoms.length,
    raw_row_count: Array.isArray(state.atoms) ? state.atoms.length : topology.atoms.length,
    dedupe_policy: {
      type: 'semantic_projection_dedupe',
      key_basis: 'role+source_kind+text_preview',
      preserves_latest_duplicate: true,
    },
    frontier: {
      session_id: state.session_id,
      budget_tokens: 6000,
      used_tokens: fragments.reduce((sum, row) => sum + (Number(row.token_count) || 0), 0),
      hot_atom_refs: hotAtoms.map((row) => row.atom_id),
      warm_span_refs: selectedSpans.filter((row) => row.level === 0).map((row) => row.span_id),
      cool_span_refs: selectedSpans.filter((row) => row.level === 1 || row.level === 2).map((row) => row.span_id),
      cold_span_refs: selectedSpans.filter((row) => row.level >= 3).map((row) => row.span_id),
      pinned_anchor_refs: [],
      pressure_state: fragments.length > 24 ? 'medium' : 'low',
      fidelity_score: 1,
    },
    fragments,
  };
}

function ingestAgentRuntimeContextProjection(options = {}) {
  const root = options.root || process.cwd();
  const sessionId = cleanString(options.sessionId, 200) || 'session';
  const agentId = cleanString(options.agentId, 160) || 'default';
  const projection = options.projection && typeof options.projection === 'object' ? options.projection : {};
  const rows = Array.isArray(projection.rows) ? projection.rows.slice(-MAX_BOOTSTRAP_ROWS) : [];
  const state = loadState(root, sessionId, agentId);
  let appended = 0;
  for (const row of rows) {
    const atom = appendAtomToState(state, row);
    if (atom) appended += 1;
  }
  const pathWritten = saveState(root, state);
  return { ok: true, appended, atom_count: state.atoms.length, state_path: pathWritten };
}

function appendAgentRuntimeTurnAtoms(options = {}) {
  const root = options.root || process.cwd();
  const sessionId = cleanString(options.sessionId, 200) || 'session';
  const agentId = cleanString(options.agentId, 160) || 'default';
  const state = loadState(root, sessionId, agentId);
  const traceId = cleanString(options.traceId, 200);
  const turnId = cleanString(options.turnId, 200);
  const engineId = cleanString(options.engineId, 120);
  let appended = 0;
  const userText = cleanDisplayText(options.userText, 24000);
  if (userText) {
    if (appendAtomToState(state, {
      id: `${turnId}:user`,
      role: 'user',
      source_kind: 'user_message',
      source_ref: `turn/${turnId}/user`,
      source_authority: 'gateway_agent_runtime_turn_projection',
      speaker_label: 'user',
      text_preview: userText,
      lineage_refs: [traceId, turnId].filter(Boolean),
    })) appended += 1;
  }
  const assistantText = cleanDisplayText(options.assistantText, 24000);
  if (assistantText) {
    if (appendAtomToState(state, {
      id: `${turnId}:assistant:${engineId}`,
      role: 'assistant',
      source_kind: 'assistant_message',
      source_ref: `turn/${turnId}/assistant/${engineId}`,
      source_authority: 'gateway_agent_runtime_turn_projection',
      speaker_label: engineId || 'assistant',
      text_preview: assistantText,
      lineage_refs: [traceId, turnId, cleanString(options.resultRef, 240), cleanString(options.receiptRef, 240)].filter(Boolean),
    })) appended += 1;
  }
  const pathWritten = saveState(root, state);
  return { ok: true, appended, atom_count: state.atoms.length, state_path: pathWritten };
}

function appendAgentRuntimeApprovedEffectAtom(options = {}) {
  const root = options.root || process.cwd();
  const sessionId = cleanString(options.sessionId, 200) || 'session';
  const agentId = cleanString(options.agentId, 160) || 'default';
  const state = loadState(root, sessionId, agentId);
  const approvalId = safeId(options.approvalId, 'approval');
  const traceId = cleanString(options.traceId, 200);
  const turnId = cleanString(options.turnId, 200);
  const engineId = cleanString(options.engineId, 120);
  const toolId = cleanString(options.toolId, 120);
  const artifactPath = cleanString(options.path || options.artifactPath, 600);
  const artifactRef = cleanString(options.artifactRef || options.resultRef, 600);
  const receiptRef = cleanString(options.receiptRef, 600);
  const bytes = Math.max(0, Number(options.bytes) || 0);
  const sha256 = cleanString(options.sha256, 80);
  const contentPreview = cleanDisplayText(options.contentPreview, 900);
  const displayText = cleanDisplayText(options.displayText, 900);
  const text = [
    `InfRing approval gate executed ${toolId || 'approved tool call'}.`,
    artifactPath ? `Approved artifact path: ${artifactPath}.` : '',
    artifactRef ? `Artifact ref: ${artifactRef}.` : '',
    receiptRef ? `Effect receipt: ${receiptRef}.` : '',
    bytes ? `Size: ${bytes} bytes.` : '',
    sha256 ? `SHA-256: ${sha256}.` : '',
    displayText ? `Result: ${displayText}` : '',
    contentPreview ? `Content preview: ${contentPreview}` : '',
  ].filter(Boolean).join(' ');
  let appended = 0;
  if (appendAtomToState(state, {
    id: `approval:${approvalId}:effect`,
    role: 'tool',
    source_kind: 'tool_receipt',
    source_ref: `approval/${approvalId}/effect/${artifactPath || artifactRef || receiptRef || 'artifact'}`,
    source_authority: 'gateway_agent_runtime_approval_effect_context',
    speaker_label: 'InfRing approval gate',
    text_preview: text,
    token_count: Math.min(estimateTokens(text), 4000),
    lineage_refs: [traceId, turnId, approvalId, engineId, artifactRef, receiptRef].filter(Boolean),
    task_refs: [artifactPath, artifactRef].filter(Boolean).slice(0, 12),
  })) appended += 1;
  const pathWritten = saveState(root, state);
  return { ok: true, appended, atom_count: state.atoms.length, state_path: pathWritten };
}

function materializeAgentRuntimeContextPack(options = {}) {
  const root = options.root || process.cwd();
  const sessionId = cleanString(options.sessionId, 200) || 'session';
  const agentId = cleanString(options.agentId, 160) || 'default';
  const state = loadState(root, sessionId, agentId);
  return materializeFromState(state);
}

function loadAgentRuntimeContextRows(options = {}) {
  const root = options.root || process.cwd();
  const sessionId = cleanString(options.sessionId, 200) || 'session';
  const agentId = cleanString(options.agentId, 160) || 'default';
  const state = loadState(root, sessionId, agentId);
  return state.atoms.map((atom) => ({
    id: atom.atom_id,
    source_ref: atom.source_ref,
    source_kind: sourceKindForRole(atom.role, atom.source_kind),
    record_type: sourceKindForRole(atom.role, atom.source_kind),
    source_authority: cleanString(atom.source_authority || 'gateway_runtime_context_store_pending_kernel_materializer_route', 160),
    speaker_label: cleanString(atom.speaker_label || atom.role, 120),
    role: atom.role,
    text_preview: atom.text_preview,
    token_count: atom.token_count,
    task_refs: atom.task_refs,
    memory_version_refs: atom.memory_version_refs,
    lineage_refs: atom.lineage_refs,
  }));
}

module.exports = {
  CONTEXT_STORE_AUTHORITY_CLASSIFICATION,
  FANOUT_TARGET,
  HOT_TAIL_COUNT,
  ingestAgentRuntimeContextProjection,
  appendAgentRuntimeTurnAtoms,
  appendAgentRuntimeApprovedEffectAtom,
  materializeAgentRuntimeContextPack,
  loadAgentRuntimeContextRows,
};
