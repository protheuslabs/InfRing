#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::trace-writer.

'use strict';

const fs = require('fs');
const path = require('path');

const DEFAULT_TRACE_PATH = path.join(
  'local',
  'state',
  'observability',
  'agent_runtime',
  'agent_runtime_events.jsonl',
);

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function cleanRefs(value, maxItems = 8, max = 500) {
  const rows = Array.isArray(value) ? value : (value == null || value === '' ? [] : [value]);
  const out = [];
  for (const row of rows) {
    const clean = cleanString(row, max);
    if (clean) out.push(clean);
    if (out.length >= maxItems) break;
  }
  return out;
}

function confidenceValue(value) {
  const num = Number(value);
  if (!Number.isFinite(num)) return 1;
  return Math.max(0, Math.min(1, num));
}

function eventKindForType(type) {
  const clean = cleanString(type, 120);
  if (clean.includes('health') || clean === 'heartbeat') return 'health';
  if (clean.includes('tool.')) return 'tool_call';
  if (clean.includes('artifact.')) return 'workflow_phase';
  if (clean.includes('receipt.')) return 'receipt';
  if (clean === 'error' || clean.includes('error')) return 'error';
  return 'gateway_boundary';
}

function compactEvent(event) {
  const source = event && typeof event === 'object' ? event : {};
  const generatedAt = new Date().toISOString();
  const eventType = cleanString(source.type || 'agent_runtime.event', 120);
  const engineId = cleanString(source.engine_id, 120);
  const sessionId = cleanString(source.session_id, 200);
  const turnId = cleanString(source.turn_id, 200);
  const requestId = cleanString(source.request_id, 200);
  const status = cleanString(source.status || '', 120);
  const errorCode = cleanString(source.error_code || '', 120);
  const artifactRef = cleanString(source.artifact_ref || source.result_ref || '', 500);
  const toolCallRef = cleanString(source.tool_call_ref || '', 500);
  const receiptRef = cleanString(source.receipt_ref || '', 500);
  const receiptRefs = cleanRefs(source.receipt_refs || receiptRef, 12, 500);
  const evidenceRefs = cleanRefs(source.evidence_refs || source.evidence_ref, 12, 500);
  return {
    schema_version: 1,
    type: 'agent_runtime_engine_trace_event',
    generated_at: generatedAt,
    timestamp: generatedAt,
    trace_id: cleanString(source.trace_id, 200),
    span_id: cleanString(source.span_id || `span:agent-runtime:${process.pid}:${Date.now()}`, 240),
    parent_span_id: cleanString(source.parent_span_id || '', 240),
    source_domain: 'gateway',
    producer: 'adapters.runtime.agent_engines',
    authority_class: 'external_gateway_boundary',
    event_kind: eventKindForType(eventType),
    subject: {
      engine_id: engineId,
      session_id: sessionId,
      turn_id: turnId,
    },
    correlation: {
      request_id: requestId,
      engine_id: engineId,
      session_id: sessionId,
      turn_id: turnId,
    },
    engine_id: engineId,
    session_id: sessionId,
    turn_id: turnId,
    event_type: eventType,
    status,
    error_code: errorCode,
    retryable: source.retryable === true,
    payload_schema: 'observability.traces.agent_runtime_engine_trace_event.v1',
    payload: {
      event_type: eventType,
      status,
      error_code: errorCode,
      retryable: source.retryable === true,
      artifact_ref: artifactRef,
      tool_call_ref: toolCallRef,
      receipt_ref: receiptRef,
    },
    evidence_refs: evidenceRefs,
    receipt_refs: receiptRefs,
    severity: cleanString(source.severity || (errorCode ? 'error' : 'info'), 40),
    confidence: confidenceValue(source.confidence),
    artifact_ref: artifactRef,
    tool_call_ref: toolCallRef,
    receipt_ref: receiptRef,
  };
}

function createAgentRuntimeTraceWriter(options = {}) {
  const root = options.root || process.cwd();
  const rel = options.tracePath || DEFAULT_TRACE_PATH;
  const fullPath = path.isAbsolute(rel) ? rel : path.join(root, rel);
  return {
    trace_path: fullPath,
    write(event) {
      const compact = compactEvent(event);
      if (!compact.trace_id || !compact.engine_id || !compact.event_type) return { ok: false, reason: 'missing_required_trace_fields' };
      fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      fs.appendFileSync(fullPath, `${JSON.stringify(compact)}\n`);
      return { ok: true, path: fullPath };
    },
  };
}

module.exports = {
  DEFAULT_TRACE_PATH,
  compactEvent,
  cleanRefs,
  eventKindForType,
  createAgentRuntimeTraceWriter,
};
