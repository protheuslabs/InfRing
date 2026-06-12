#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::transcripts.
//
// Gateway owns external-runtime transcript persistence and bounded projection
// overlays. The legacy dashboard host may serve HTTP routes, but it must not own
// transcript state or Shell-facing merge semantics.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const DEFAULT_MAX_RECORDS = 2000;
const DEFAULT_WINDOW_LIMIT = 80;

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanTranscriptComponent(value, maxLen = 200) { return cleanText(value, maxLen).replace(/[^A-Za-z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, '') || 'default'; }
function agentRuntimeSessionRef(agentId, sessionId) { return `${cleanTranscriptComponent(agentId, 160)}::${cleanTranscriptComponent(sessionId, 200)}`; }

function activityLineKindFromTraceRow(row) {
  const trace = row && typeof row === 'object' ? row : {};
  const joined = [
    trace.activity_kind,
    trace.provider_event_type,
    trace.title,
  ].map((item) => cleanText(item, 500).toLowerCase()).join(' ');
  if (/decision|reasoning|thought|plan|dialog/.test(joined)) return 'dialog';
  if (/file|write|edit|patch|diff|created|updated/.test(joined)) return 'write';
  if (/read|search|grep|find|open/.test(joined)) return 'read';
  if (/command|exec|shell|bash|tool|permission|approval|error|failed/.test(joined)) return 'tool';
  return 'status';
}

function activityStateFromTraceRow(row) {
  const status = cleanText(row && row.status, 80).toLowerCase();
  if (/fail|error/.test(status)) return 'error';
  if (/pause|block|pending/.test(status)) return 'blocked';
  if (/start|run|progress/.test(status)) return 'running';
  return 'done';
}

function transcriptActivityTraceRows(activityTrace) {
  const rows = Array.isArray(activityTrace && activityTrace.rows) ? activityTrace.rows : [];
  return rows.map((row, index) => {
    const title = cleanDisplayText(row && (row.title || row.display_text || row.text), 1400);
    if (!title) return null;
    return {
      id: cleanText(row && (row.detail_ref || row.item_id || row.sequence_no || `activity-${index + 1}`), 180),
      text: title,
      line_kind: activityLineKindFromTraceRow(row),
      state: activityStateFromTraceRow(row),
      status: cleanText(row && row.status, 80),
      activity_key: cleanText(row && (row.detail_ref || row.item_id || ''), 240),
      activity_target: cleanText(row && (row.target || row.path || ''), 900),
      engine_id: cleanEngineId(activityTrace && activityTrace.engine_id),
      ts: Date.now(),
    };
  }).filter(Boolean).slice(-48);
}

function transcriptDecisionDialogText(activityEvents) {
  const rows = Array.isArray(activityEvents) ? activityEvents : [];
  const lines = [];
  const seen = new Set();
  for (const event of rows) {
    const row = event && typeof event === 'object' ? event : {};
    const text = cleanDisplayText(row.display_text || row.text || row.summary || '', 2000);
    const joined = [
      row.activity_kind,
      row.provider_event_type,
      text,
    ].map((item) => cleanText(item, 500).toLowerCase()).join(' ');
    if (!text || !/decision|reasoning|thought|plan|dialog/.test(joined)) continue;
    const key = text.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    lines.push(text);
    if (lines.length >= 80) break;
  }
  return lines.join('\n');
}

function transcriptDecisionTool(input) {
  const activityTrace = input && input.activityTrace && typeof input.activityTrace === 'object' ? input.activityTrace : null;
  const activityEvents = Array.isArray(input && input.activityEvents) ? input.activityEvents : [];
  const traceRows = transcriptActivityTraceRows(activityTrace);
  const dialog = cleanDisplayText(input && input.decisionDialogText, 12000) || transcriptDecisionDialogText(activityEvents);
  if (!dialog && !traceRows.length) return null;
  const workedMs = Math.max(0, Number(input && input.workedMs) || Number(activityTrace && activityTrace.worked_ms) || 0);
  const workedLabel = cleanText(input && input.workedLabel, 120) || cleanText(activityTrace && activityTrace.collapse_label, 120);
  return {
    id: cleanText(input && input.toolId, 200) || `agent-runtime-work:${cleanTranscriptComponent(input && input.turnId, 160)}`,
    name: 'thought_process',
    status: 'completed',
    running: false,
    expanded: false,
    input: dialog,
    input_preview: dialog,
    result: '',
    result_preview: dialog,
    summary: workedLabel || 'Agent decision dialog',
    display_text: dialog,
    duration_ms: workedMs,
    agent_decision_dialog: true,
    agent_runtime_decision_dialog: true,
    agent_runtime_activity_trace: true,
    agent_runtime_trace_rows: traceRows,
    agent_runtime_engine_id: cleanEngineId(input && input.engineId),
    projection_kind: 'decision_dialog',
    projection_schema_version: 1,
  };
}

function transcriptActivityEvents(events) {
  return (Array.isArray(events) ? events : []).map((event, index) => {
    const row = event && typeof event === 'object' ? event : {};
    const text = cleanDisplayText(row.display_text || row.text || row.summary || '', 2000);
    const provider = cleanText(row.provider_event_type || row.event_type || '', 160);
    if (!text && !provider) return null;
    return {
      type: 'agent_activity_event',
      activity_kind: cleanText(row.activity_kind || row.kind || row.type || 'activity', 80),
      provider_event_type: provider,
      source: cleanText(row.source || 'gateway_agent_runtime_transcript', 120),
      sequence_no: Number(row.sequence_no || index + 1) || index + 1,
      item_id: cleanText(row.item_id || row.itemId || '', 200),
      status: cleanText(row.status || '', 80),
      text,
      display_text: text,
      engine_id: cleanEngineId(row.engine_id),
      trace_id: cleanText(row.trace_id, 200),
      session_id: cleanText(row.session_id, 200),
      turn_id: cleanText(row.turn_id, 200),
    };
  }).filter(Boolean).slice(-80);
}

function decodeAgentRuntimeSessionRef(value) {
  const raw = cleanText(decodeURIComponent(String(value || '')), 420);
  const parts = raw.split('::');
  if (parts.length >= 2) {
    return {
      agentId: cleanTranscriptComponent(parts.shift(), 160),
      sessionId: cleanTranscriptComponent(parts.join('::'), 200),
    };
  }
  return { agentId: '', sessionId: cleanTranscriptComponent(raw, 200) };
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
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

function createAgentRuntimeTranscriptStore(options = {}) {
  const statusDir = path.resolve(options.statusDir || path.join(process.cwd(), 'client', 'runtime', 'local', 'state', 'ui', 'infring_dashboard'));
  const transcriptPath = path.resolve(statusDir, 'agent_runtime_transcripts.jsonl');
  const maxRecords = Math.max(1, Number(options.maxRecords) || DEFAULT_MAX_RECORDS);
  const windowLimit = Math.max(1, Number(options.windowLimit) || DEFAULT_WINDOW_LIMIT);

  function readAgentRuntimeTranscriptRecords() {
    let raw = '';
    try { raw = fs.readFileSync(transcriptPath, 'utf8'); } catch { return []; }
    return raw.split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .slice(-maxRecords)
      .map((line) => {
        try { return JSON.parse(line); } catch { return null; }
      })
      .filter((row) => row && row.type === 'agent_runtime_transcript_turn');
  }

  function agentRuntimeTranscriptMessageRow(input) {
    const text = cleanDisplayText(input && input.text, 24000);
    if (!text) return null;
    const role = cleanText(input && input.role, 40) || 'assistant';
    const turnId = cleanTranscriptComponent(input && input.turnId, 200);
    const timestamp = cleanText(input && input.timestamp, 80) || nowIso();
    const activityTrace = input && input.activityTrace && typeof input.activityTrace === 'object' ? input.activityTrace : null;
    const activityEvents = transcriptActivityEvents(input && input.activityEvents);
    const decisionTool = role === 'assistant' ? transcriptDecisionTool({
      turnId,
      engineId: input && input.engineId,
      workedMs: input && input.workedMs,
      workedLabel: input && input.workedLabel,
      decisionDialogText: input && input.decisionDialogText,
      activityEvents,
      activityTrace,
    }) : null;
    const row = {
      id: cleanText(input && input.id, 240) || `agent_runtime:${turnId}:${role}`,
      role,
      origin_kind: role === 'user' ? 'user' : 'assistant',
      origin_display_name: role === 'user' ? 'You' : cleanText(input && input.engineId, 120),
      text,
      content: text,
      content_preview: cleanText(text, 4000),
      timestamp,
      created_at: timestamp,
      status: cleanText(input && input.status, 80) || 'completed',
      detail_ref: cleanText(input && input.detailRef, 240) || `agent-runtime-turn:${turnId}:${role}`,
      trace_id: cleanText(input && input.traceId, 200),
      agent_runtime_engine_id: cleanEngineId(input && input.engineId),
      source: 'agent_runtime_socket',
      projection_owner: 'gateway.runtime.agent_runtime_transcript',
    };
    if (role === 'assistant') {
      row.agent_activity_events = activityEvents;
      row.agent_activity_event_count = activityEvents.length;
      if (activityTrace) {
        row.activity_trace = activityTrace;
        row.agent_runtime_activity_trace = activityTrace;
        row.agent_runtime_worked_ms = Math.max(0, Number(input && input.workedMs) || Number(activityTrace.worked_ms) || 0);
        row.agent_runtime_worked_label = cleanText(input && input.workedLabel, 120) || cleanText(activityTrace.collapse_label, 120);
      }
      if (decisionTool) {
        row.tools = [decisionTool];
        row.agent_runtime_decision_dialog_text = decisionTool.display_text || '';
      }
    }
    return row;
  }

  function appendAgentRuntimeTranscriptTurn(input) {
    const agentId = cleanTranscriptComponent(input && input.agentId, 160);
    const sessionId = cleanTranscriptComponent(input && input.sessionId, 200);
    const engineId = cleanEngineId(input && input.engineId);
    const turnId = cleanTranscriptComponent(input && input.turnId, 200);
    const timestamp = nowIso();
    const messages = [
      agentRuntimeTranscriptMessageRow({
        id: `agent_runtime:${turnId}:user`,
        role: 'user',
        text: input && input.userText,
        timestamp,
        turnId,
        traceId: input && input.traceId,
        engineId,
      }),
      agentRuntimeTranscriptMessageRow({
        id: `agent_runtime:${turnId}:assistant`,
        role: 'assistant',
        text: input && input.assistantText,
        timestamp,
        turnId,
        traceId: input && input.traceId,
        engineId,
        status: input && input.pendingPermissionRequest
          ? 'permission_required'
          : cleanText(input && input.status, 80) || 'completed',
        activityEvents: input && input.activityEvents,
        activityTrace: input && input.activityTrace,
        workedMs: input && input.workedMs,
        workedLabel: input && input.workedLabel,
        decisionDialogText: input && input.decisionDialogText,
      }),
    ].filter(Boolean);
    if (!messages.length) return;
    appendBoundedJsonl(transcriptPath, {
      type: 'agent_runtime_transcript_turn',
      schema_version: 1,
      agent_id: agentId,
      session_id: sessionId,
      session_ref: agentRuntimeSessionRef(agentId, sessionId),
      session_aliases: Array.from(new Set([
        sessionId,
        agentId,
        `shell_${agentId}`,
        agentRuntimeSessionRef(agentId, sessionId),
        agentRuntimeSessionRef(agentId, agentId),
        agentRuntimeSessionRef(agentId, `shell_${agentId}`),
      ])).filter(Boolean),
      trace_id: cleanText(input && input.traceId, 200),
      turn_id: turnId,
      engine_id: engineId,
      created_at: timestamp,
      messages,
    }, maxRecords);
  }

  function agentRuntimeTranscriptSessionMatches(record, targetAgentId, targetSessionId, targetRef) {
    const recordAgentId = cleanTranscriptComponent(record && record.agent_id, 160);
    const recordSessionId = cleanTranscriptComponent(record && record.session_id, 200);
    const recordRef = cleanText(record && record.session_ref, 420);
    const aliases = Array.isArray(record && record.session_aliases)
      ? record.session_aliases.map((item) => cleanText(item, 420)).filter(Boolean)
      : [];
    if (targetAgentId && recordAgentId !== targetAgentId) return false;
    if (!targetSessionId) return true;
    if (recordSessionId === targetSessionId || recordRef === targetRef) return true;
    if (aliases.indexOf(targetSessionId) >= 0 || aliases.indexOf(targetRef) >= 0) return true;
    if (targetSessionId === targetAgentId && (recordSessionId === targetAgentId || recordSessionId === `shell_${targetAgentId}`)) return true;
    if (targetSessionId === `shell_${targetAgentId}` && (recordSessionId === targetAgentId || recordSessionId === `shell_${targetAgentId}`)) return true;
    return false;
  }

  function loadAgentRuntimeTranscriptRows(filter) {
    const targetAgentId = filter && filter.agentId ? cleanTranscriptComponent(filter.agentId, 160) : '';
    const targetSessionId = filter && filter.sessionId ? cleanTranscriptComponent(filter.sessionId, 200) : '';
    const targetRef = targetSessionId ? agentRuntimeSessionRef(targetAgentId, targetSessionId) : '';
    const records = readAgentRuntimeTranscriptRecords();
    const collectMessages = (selectedRecords) => {
      const out = [];
      for (const record of selectedRecords) {
        if (!Array.isArray(record.messages)) continue;
        for (const message of record.messages) {
          if (message && typeof message === 'object') out.push(message);
        }
      }
      return out;
    };
    const directRecords = records.filter((record) => agentRuntimeTranscriptSessionMatches(record, targetAgentId, targetSessionId, targetRef));
    const directRows = collectMessages(directRecords);
    if (directRows.length || !(filter && filter.allowAgentFallback === true) || !targetAgentId) return directRows;
    const fallbackRecords = records.filter((record) => {
      const recordAgentId = cleanTranscriptComponent(record && record.agent_id, 160);
      return recordAgentId === targetAgentId;
    });
    return collectMessages(fallbackRecords);
  }

  function mergeAgentRuntimeMessageRows(baseRows, overlayRows, limit) {
    const merged = new Map();
    const order = [];
    for (const row of [...(Array.isArray(baseRows) ? baseRows : []), ...(Array.isArray(overlayRows) ? overlayRows : [])]) {
      if (!row || typeof row !== 'object') continue;
      const key = cleanText(row.id || row.message_id || `${row.role || 'row'}:${row.timestamp || row.created_at || order.length}`, 260);
      if (!merged.has(key)) order.push(key);
      merged.set(key, row);
    }
    const rows = order.map((key, index) => ({ row: merged.get(key), index }));
    rows.sort((a, b) => {
      const at = cleanText(a.row && (a.row.timestamp || a.row.created_at || a.row.ts), 80);
      const bt = cleanText(b.row && (b.row.timestamp || b.row.created_at || b.row.ts), 80);
      if (at && bt && at !== bt) return at.localeCompare(bt, 'en');
      if (at && !bt) return 1;
      if (!at && bt) return -1;
      return a.index - b.index;
    });
    return rows.map((entry) => entry.row).slice(-Math.max(1, Math.min(windowLimit, Number(limit) || windowLimit)));
  }

  function mergeAgentRuntimeTranscriptPayload(payload, options) {
    const base = payload && typeof payload === 'object' ? { ...payload } : {};
    const agentId = options && options.agentId ? cleanTranscriptComponent(options.agentId, 160) : '';
    const sessionId = cleanTranscriptComponent(
      (options && options.sessionId) ||
        base.session_id ||
        base.current_session_id ||
        (base.session && base.session.id) ||
        agentId,
      200,
    );
    const limit = Math.max(1, Math.min(windowLimit, Number(options && options.limit) || windowLimit));
    const overlayRows = loadAgentRuntimeTranscriptRows({ agentId, sessionId, allowAgentFallback: true });
    if (!overlayRows.length) return base;
    let projectedRows = [];
    let projectedIntoMessageWindow = false;
    if (base.message_window && typeof base.message_window === 'object') {
      const rows = mergeAgentRuntimeMessageRows(base.message_window.rows, overlayRows, limit);
      projectedRows = rows;
      projectedIntoMessageWindow = true;
      base.message_window = {
        ...base.message_window,
        rows,
        total_count: Math.max(Number(base.message_window.total_count) || 0, rows.length),
        agent_runtime_transcript_overlay: true,
      };
    } else if (Array.isArray(base.messages)) {
      base.messages = mergeAgentRuntimeMessageRows(base.messages, overlayRows, limit);
      projectedRows = base.messages;
      base.message_count = Math.max(Number(base.message_count) || 0, base.messages.length);
    } else if (Array.isArray(base.turns)) {
      base.turns = mergeAgentRuntimeMessageRows(base.turns, overlayRows, limit);
      projectedRows = base.turns;
    } else {
      base.messages = mergeAgentRuntimeMessageRows([], overlayRows, limit);
      projectedRows = base.messages;
      base.message_count = base.messages.length;
    }
    if (projectedRows.length) {
      if (!projectedIntoMessageWindow && !Array.isArray(base.messages)) {
        base.messages = mergeAgentRuntimeMessageRows([], projectedRows, limit);
        base.message_count = Math.max(Number(base.message_count) || 0, base.messages.length);
      }
      if (base.session && typeof base.session === 'object') {
        base.session = { ...base.session };
        if (!Array.isArray(base.session.messages) || !base.session.messages.length) {
          base.session.messages = mergeAgentRuntimeMessageRows([], projectedRows, limit);
        }
        if (Array.isArray(base.session.sessions)) {
          const normalizedSessionId = cleanTranscriptComponent(sessionId, 200);
          let attached = false;
          base.session.sessions = base.session.sessions.map((row) => {
            if (!row || typeof row !== 'object') return row;
            const rowId = cleanTranscriptComponent(row.id || row.session_id || row.scope_token || '', 200);
            const rowActive = row.active === true;
            if (!attached && (rowActive || rowId === normalizedSessionId || !rowId)) {
              attached = true;
              return {
                ...row,
                messages: mergeAgentRuntimeMessageRows(row.messages, projectedRows, limit),
                agent_runtime_transcript_overlay: true,
              };
            }
            return row;
          });
        }
      }
    }
    base.agent_runtime_transcript_overlay = {
      source: 'gateway.runtime.agent_runtime_transcript',
      row_count: overlayRows.length,
      bounded: true,
      session_ref: agentRuntimeSessionRef(agentId, sessionId),
    };
    return base;
  }

  function agentRuntimeTranscriptFilterFromShellSocketPath(pathname) {
    const match = String(pathname || '').match(/^\/api\/shell-socket\/sessions\/([^/]+)\/messages$/);
    if (!match) return null;
    return decodeAgentRuntimeSessionRef(match[1]);
  }

  return {
    transcriptPath,
    appendAgentRuntimeTranscriptTurn,
    mergeAgentRuntimeTranscriptPayload,
    agentRuntimeTranscriptFilterFromShellSocketPath,
  };
}

module.exports = {
  createAgentRuntimeTranscriptStore,
};
