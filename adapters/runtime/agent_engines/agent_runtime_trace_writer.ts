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
  const eventType = cleanString(source.type || 'agent_runtime.event', 120);
  const engineId = cleanString(source.engine_id, 120);
  const sessionId = cleanString(source.session_id, 200);
  const turnId = cleanString(source.turn_id, 200);
  const requestId = cleanString(source.request_id, 200);
  return {
    type: 'agent_runtime_engine_trace_event',
    generated_at: new Date().toISOString(),
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
    status: cleanString(source.status || '', 120),
    error_code: cleanString(source.error_code || '', 120),
    retryable: source.retryable === true,
    artifact_ref: cleanString(source.artifact_ref || source.result_ref || '', 500),
    tool_call_ref: cleanString(source.tool_call_ref || '', 500),
    receipt_ref: cleanString(source.receipt_ref || '', 500),
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
  eventKindForType,
  createAgentRuntimeTraceWriter,
};
