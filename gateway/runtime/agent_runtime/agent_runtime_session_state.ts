#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::session-state.
//
// Gateway owns external-runtime session defaults and turn steering queues.
// Shells may request selection or steering, but durable state and next-turn
// materialization remain behind the Gateway boundary.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const DEFAULT_STEERING_MAX_RECORDS = 240;

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function writeJson(file, payload) {
  ensureDir(path.dirname(file));
  fs.writeFileSync(file, `${JSON.stringify(payload, null, 2)}\n`);
}

function appendBoundedJsonl(filePath, value, maxRows) {
  ensureDir(path.dirname(filePath));
  let rows = [];
  try {
    rows = fs.readFileSync(filePath, 'utf8').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  } catch {}
  rows.push(JSON.stringify(value));
  fs.writeFileSync(filePath, `${rows.slice(-Math.max(1, maxRows || 1)).join('\n')}\n`, 'utf8');
}

function createAgentRuntimeSessionStateStore(options = {}) {
  const statusDir = path.resolve(options.statusDir || path.join(process.cwd(), 'client', 'runtime', 'local', 'state', 'ui', 'infring_dashboard'));
  const selectionPath = path.resolve(statusDir, 'agent_runtime_selection.json');
  const steeringPath = path.resolve(statusDir, 'agent_runtime_steering.jsonl');
  const steeringMaxRecords = Math.max(1, Number(options.steeringMaxRecords) || DEFAULT_STEERING_MAX_RECORDS);

  function loadAgentRuntimeSelection() {
    try {
      const parsed = JSON.parse(fs.readFileSync(selectionPath, 'utf8'));
      return {
        engine_id: cleanEngineId(parsed && parsed.engine_id) || 'infring_native',
        updated_at: cleanText(parsed && parsed.updated_at, 80),
        source: cleanText(parsed && parsed.source, 120),
      };
    } catch {
      return { engine_id: 'infring_native', updated_at: '', source: 'default' };
    }
  }

  function saveAgentRuntimeSelection(engineId, traceId) {
    const cleanId = cleanEngineId(engineId || 'infring_native') || 'infring_native';
    const row = {
      type: 'agent_runtime_selection',
      schema_version: 1,
      engine_id: cleanId,
      updated_at: nowIso(),
      trace_id: cleanText(traceId, 200),
      source: 'gateway_agent_runtime_session_state',
    };
    writeJson(selectionPath, row);
    return row;
  }

  function readAgentRuntimeSteeringRecords() {
    let raw = '';
    try { raw = fs.readFileSync(steeringPath, 'utf8'); } catch { return []; }
    return raw.split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .slice(-steeringMaxRecords)
      .map((line) => {
        try { return JSON.parse(line); } catch { return null; }
      })
      .filter((row) => row && typeof row === 'object');
  }

  function writeAgentRuntimeSteeringRecords(rows) {
    ensureDir(path.dirname(steeringPath));
    const serialized = (Array.isArray(rows) ? rows : [])
      .slice(-steeringMaxRecords)
      .map((row) => JSON.stringify(row))
      .join('\n');
    fs.writeFileSync(steeringPath, serialized ? `${serialized}\n` : '', 'utf8');
  }

  function queueAgentRuntimeSteeringIntervention(traceId, body) {
    const engineId = cleanEngineId(body && (body.engine_id || body.agent_runtime_engine_id || body.runtime_engine_id)) || 'infring_native';
    const agentId = cleanText(body && body.agent_id, 160) || 'default';
    const sessionId = cleanText(body && body.session_id, 200) || `shell_${agentId}`;
    const text = cleanDisplayText(body && (body.text || body.message || body.content), 12000);
    if (!text) return { ok: false, status_code: 400, type: 'agent_runtime_steer_projection', trace_id: traceId, engine_id: engineId, error: 'steer_text_required' };
    const record = {
      steering_id: `steer_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`,
      type: 'agent_runtime_steering_intervention',
      trace_id: cleanText(traceId, 200),
      engine_id: engineId,
      agent_id: agentId,
      session_id: sessionId,
      text,
      text_preview: cleanText(text, 4000),
      priority: cleanText(body && body.priority, 40) || 'steer',
      requested_mode: cleanText(body && body.mode, 40) || 'auto',
      steering_mode: 'next_turn',
      status: 'queued_next_turn',
      created_at: nowIso(),
      source_authority: 'gateway_agent_runtime_steer_route',
      attachments: Array.isArray(body && body.attachments)
        ? body.attachments.map((item) => cleanText(item && (item.name || item.filename || item.path || item.id || item), 240)).filter(Boolean).slice(0, 12)
        : [],
    };
    appendBoundedJsonl(steeringPath, record, steeringMaxRecords);
    return {
      ok: true,
      status_code: 200,
      type: 'agent_runtime_steer_projection',
      trace_id: traceId,
      engine_id: engineId,
      agent_id: agentId,
      session_id: sessionId,
      steering_id: record.steering_id,
      steering_mode: record.steering_mode,
      status: record.status,
      live_injected: false,
      applies_to: 'next_turn',
      display_text: 'Steering recorded for the next runtime turn.',
    };
  }

  function drainAgentRuntimeSteeringInterventions(options = {}) {
    const agentId = cleanText(options.agentId, 160) || 'default';
    const sessionId = cleanText(options.sessionId, 200) || `shell_${agentId}`;
    const engineId = cleanEngineId(options.engineId) || 'infring_native';
    const records = readAgentRuntimeSteeringRecords();
    const applied = [];
    const kept = [];
    const appliedAt = nowIso();
    for (const row of records) {
      if (!row || row.applied_at || cleanText(row.status, 80) !== 'queued_next_turn') {
        kept.push(row);
        continue;
      }
      const rowEngine = cleanEngineId(row.engine_id);
      const rowAgent = cleanText(row.agent_id, 160);
      const rowSession = cleanText(row.session_id, 200);
      const matches = rowEngine === engineId && (rowSession === sessionId || rowAgent === agentId);
      if (!matches || applied.length >= 7) {
        kept.push(row);
        continue;
      }
      const updated = {
        ...row,
        status: 'applied_to_next_turn',
        applied_at: appliedAt,
        applied_trace_id: cleanText(options.traceId, 200),
        applied_turn_id: cleanText(options.turnId, 200),
      };
      kept.push(updated);
      applied.push({
        steering_id: cleanText(row.steering_id, 200),
        text: cleanDisplayText(row.text, 12000),
        text_preview: cleanText(row.text_preview || row.text, 1000),
        created_at: cleanText(row.created_at, 80),
        priority: cleanText(row.priority || 'steer', 40),
        source_authority: cleanText(row.source_authority || 'gateway_agent_runtime_steer_route', 160),
      });
    }
    if (applied.length) writeAgentRuntimeSteeringRecords(kept);
    return applied;
  }

  return {
    selectionPath,
    steeringPath,
    loadAgentRuntimeSelection,
    saveAgentRuntimeSelection,
    queueAgentRuntimeSteeringIntervention,
    drainAgentRuntimeSteeringInterventions,
  };
}

module.exports = {
  createAgentRuntimeSessionStateStore,
};
