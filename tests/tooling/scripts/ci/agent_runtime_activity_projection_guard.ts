#!/usr/bin/env node
/* eslint-disable no-console */

// Verifies Gateway compacts provider-shaped/raw JSON activity events into
// bounded user-facing trace rows before any Shell sees them. The fixtures cover
// Codex/Claude/Grok/OpenClaw/Hermes-style stream shapes without launching the
// providers, so this remains a deterministic Gateway route contract guard.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_activity_projection_guard_current.json');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const LAYER = 'gateway';
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-activity-projection-scratch');
const SESSION_ID = 'agent-runtime-activity-projection-session';
const AGENT_ID = 'agent-runtime-activity-projection-agent';

const PROVIDER_FIXTURES = {
  codex_cli: {
    expected: [
      'Runtime thread started.',
      'Checking workspace state before editing.',
      'Working on command: /bin/zsh -lc "pwd"',
      'Completed file change: /tmp/activity-projection-codex.txt',
      'Runtime completed the turn.',
    ],
    events: [
      { type: 'thread.started', thread_id: 'thread-activity-projection' },
      { type: 'decision_dialog', kind: 'decision_dialog', display_text: 'Checking workspace state before editing.' },
      { type: 'item.started', status: 'running', item: { type: 'command', command: '/bin/zsh -lc "pwd"' } },
      { type: 'item.completed', status: 'completed', item: { type: 'file_change', path: '/tmp/activity-projection-codex.txt' } },
      { type: 'turn.completed', usage: { input_tokens: 7, output_tokens: 3 } },
    ],
  },
  claude_code: {
    expected: [
      'I will inspect the requested change, then write the smallest safe patch.',
      'Working on command: npm test -- --watch=false',
      'Completed file change: /tmp/activity-projection-claude.ts',
      'Working on tool: TodoWrite',
    ],
    events: [
      { type: 'reasoning', kind: 'decision_dialog', display_text: 'I will inspect the requested change, then write the smallest safe patch.' },
      { type: 'tool_use', name: 'Bash', status: 'running', input: { command: 'npm test -- --watch=false' } },
      { type: 'tool_result', name: 'Write', status: 'completed', input: { file_path: '/tmp/activity-projection-claude.ts' } },
      { type: 'tool_use', name: 'TodoWrite', status: 'running', input: { todos: [{ content: 'finish projection guard' }] } },
    ],
  },
  grok_code: {
    expected: [
      'Working on search: agent runtime projection',
      'Working on command: python3 smoke.py',
      'Completed file change: /tmp/activity-projection-grok.py',
    ],
    events: [
      { event_type: 'search.started', state: 'running', query: 'agent runtime projection' },
      { event_type: 'command.started', state: 'running', args: { command: 'python3 smoke.py' } },
      { event_type: 'file.write.completed', state: 'completed', args: { file_path: '/tmp/activity-projection-grok.py' } },
    ],
  },
  openclaw: {
    expected: [
      'Working on command: rg activity projection',
      'Completed file change: /tmp/activity-projection-openclaw.md',
    ],
    events: [
      { type: 'runtime_event', kind: 'shell_command', status: 'running', payload: {}, command: 'rg activity projection' },
      { type: 'runtime_event', kind: 'file_patch', status: 'completed', path: '/tmp/activity-projection-openclaw.md' },
    ],
  },
  hermes_agent: {
    expected: [
      'Working on tool: planner.step',
      'Completed file change: /tmp/activity-projection-hermes.json',
    ],
    events: [
      { type: 'agent_step', status: 'running', tool: { name: 'planner.step' } },
      { type: 'artifact_write', status: 'completed', input: { path: '/tmp/activity-projection-hermes.json' } },
    ],
  },
};

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function isRuntimeLifecycleTitle(title) {
  return /^(Preparing|Loaded \d+ prior context|Checking .* availability|Starting .* session|Launching .* turn with bounded context pack|Runtime thread started|Runtime turn started|Runtime completed the turn)/.test(clean(title, 1000));
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function contextStatePath(engineId) {
  return path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}-${engineId}.json`);
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function rawActivityEvent(engineId, row, index) {
  if (clean(row && row.kind, 80) === 'decision_dialog' || clean(row && row.type, 80) === 'decision_dialog') {
    return {
      type: 'agent_activity_event',
      activity_kind: 'decision_dialog',
      provider_event_type: clean(row.type || `fixture.${engineId}.decision_dialog.${index}`, 160),
      status: clean(row.status || 'completed', 80),
      display_text: clean(row.display_text || row.text || row.summary || '', 4000),
    };
  }
  return {
    type: 'agent_activity_event',
    activity_kind: clean(row.kind || row.type || row.event_type || 'activity', 80),
    provider_event_type: clean(row.type || row.event_type || `fixture.${engineId}.${index}`, 160),
    status: clean(row.status || row.state || '', 80),
    display_text: JSON.stringify(row),
  };
}

function createRawActivityAdapter(engineId) {
  const fixture = PROVIDER_FIXTURES[engineId] || PROVIDER_FIXTURES.codex_cli;
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'activity_projection_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => ({
      type: 'turn.complete',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      session_id: message && message.session_id,
      turn_id: message && message.turn_id,
      status: 'completed',
      output_text: `${engineId} activity projection guard completed.`,
      output_preview: `${engineId} activity projection guard completed.`,
      activity_events: fixture.events.map((row, index) => rawActivityEvent(engineId, row, index)),
      activity_event_count: fixture.events.length,
      structured_activity: true,
    }),
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, engine_id: engineId, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, engine_id: engineId, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, engine_id: engineId, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, engine_id: engineId, receipt_refs: [] }),
  };
}

async function runEngineProbe(assembly, engineId) {
  const res = makeResponse();
  const sessionId = `${SESSION_ID}-${engineId}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: sessionId,
        engine_id: engineId,
        message: `run activity projection guard for ${engineId}`,
        input_text: `run activity projection guard for ${engineId}`,
        working_directory: ROOT,
      },
    },
    res,
    pathname: '/api/agent-runtime/turn',
    traceId: `validation:agent-runtime-activity-projection:${engineId}:${Date.now()}`,
    flags: {},
  });

  const payload = res.payload || {};
  const trace = payload.activity_trace && typeof payload.activity_trace === 'object' ? payload.activity_trace : {};
  const rows = Array.isArray(trace.rows) ? trace.rows : [];
  const titles = rows.map((row) => clean(row && row.title, 1000)).filter(Boolean);
  const kinds = rows.map((row) => clean(row && row.activity_kind, 160)).filter(Boolean);
  const thinkingHiddenTitles = rows
    .filter((row) => row && row.display_in_thinking_bubble === false)
    .map((row) => clean(row && row.title, 1000))
    .filter(Boolean);
  const thinkingVisibleTitles = rows
    .filter((row) => row && row.display_in_thinking_bubble !== false)
    .map((row) => clean(row && row.title, 1000))
    .filter(Boolean);
  return {
    engine_id: engineId,
    handled,
    status_code: res.statusCode,
    turn_status: clean(payload.status, 120),
    trace: {
      collapsed_by_default: trace.collapsed_by_default === true,
      collapse_label: clean(trace.collapse_label, 120),
      row_count: rows.length,
      titles,
      kinds,
      thinking_hidden_titles: thinkingHiddenTitles,
      thinking_visible_titles: thinkingVisibleTitles,
    },
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of Object.keys(PROVIDER_FIXTURES)) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of Object.keys(PROVIDER_FIXTURES)) {
    adapterFactories[engineId] = () => createRawActivityAdapter(engineId);
  }
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories,
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });

  const results = [];
  const violations = [];
  for (const engineId of Object.keys(PROVIDER_FIXTURES)) {
    const result = await runEngineProbe(assembly, engineId);
    results.push(result);
    const titles = result.trace.titles;
    const visibleTitles = result.trace.thinking_visible_titles || [];
    const hiddenTitles = result.trace.thinking_hidden_titles || [];
    if (!result.handled || result.status_code !== 200 || result.turn_status !== 'completed') {
      violations.push({ kind: 'activity_projection_turn_route_failed', engine_id: engineId, handled: result.handled, status_code: result.status_code, status: result.turn_status });
    }
    if (!result.trace.collapsed_by_default) violations.push({ kind: 'activity_trace_not_collapsed_by_default', engine_id: engineId });
    if (!/^Worked for \d/.test(result.trace.collapse_label)) violations.push({ kind: 'activity_trace_missing_worked_label', engine_id: engineId, label: result.trace.collapse_label });
    if (!titles.length) violations.push({ kind: 'activity_trace_rows_missing', engine_id: engineId });
    if (titles.some((title) => /^[{[]/.test(title))) violations.push({ kind: 'raw_json_activity_title_leaked', engine_id: engineId, titles });
    if (titles.some((title) => /\"type\"|thread_id|input_tokens|output_tokens|\"input\"|\"args\"/.test(title))) {
      violations.push({ kind: 'raw_provider_payload_detail_leaked', engine_id: engineId, titles });
    }
    if (titles.some((title) => /^Finished\b.*\bdone$/i.test(title) || /\bdone\s*▸?\s*$/.test(title))) {
      violations.push({ kind: 'status_suffix_sandwich_leaked', engine_id: engineId, titles });
    }
    if (['codex_cli', 'claude_code'].includes(engineId) && !result.trace.kinds.includes('decision_dialog')) {
      violations.push({ kind: 'golden_pair_decision_dialog_missing', engine_id: engineId, kinds: result.trace.kinds, titles });
    }
    if (titles.some((title) => isRuntimeLifecycleTitle(title) && !hiddenTitles.includes(title))) {
      violations.push({ kind: 'runtime_boot_activity_not_hidden_from_thinking_bubble', engine_id: engineId, hidden_titles: hiddenTitles, titles });
    }
    for (const expected of PROVIDER_FIXTURES[engineId].expected) {
      if (!titles.includes(expected)) violations.push({ kind: 'semantic_activity_title_missing', engine_id: engineId, expected, titles });
      if (!isRuntimeLifecycleTitle(expected) && !visibleTitles.includes(expected)) violations.push({ kind: 'semantic_activity_not_visible_in_thinking_bubble', engine_id: engineId, expected, visible_titles: visibleTitles });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_activity_projection_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    mode: 'deterministic_public_gateway_route_provider_fixtures',
    golden_external_engines: ['codex_cli', 'claude_code'],
    engines_tested: Object.keys(PROVIDER_FIXTURES),
    results,
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of Object.keys(PROVIDER_FIXTURES)) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_activity_projection_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of Object.keys(PROVIDER_FIXTURES)) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }
  process.exit(1);
});
