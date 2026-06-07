#!/usr/bin/env node
/* eslint-disable no-console */

// Proves native and external Agent Runtime turns use the same Gateway-owned
// transcript persistence and context-pack reload path. This intentionally does
// not launch provider CLIs: it validates the substrate that every engine must
// share before the Shell renders any rows.

'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_transcript_persistence_parity_guard_current.json');
const SESSION_ID = 'agent-runtime-transcript-parity-session';
const AGENT_ID = 'agent-runtime-transcript-parity-agent';
const TRACE_ID = 'trace-agent-runtime-transcript-parity';
const CONTINUITY_KEY = 'transcript-parity-key: copper-lynx-924';

function cleanText(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function writeArtifact(payload) {
  fs.mkdirSync(path.dirname(OUT_PATH), { recursive: true });
  fs.writeFileSync(OUT_PATH, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function load(relPath) {
  return require(path.join(ROOT, relPath));
}

function assertCondition(violations, condition, kind, details = {}) {
  if (!condition) violations.push({ kind, ...details });
}

function rowsForEngine(rows, engineId) {
  return rows.filter((row) => cleanText(row && row.agent_runtime_engine_id, 120) === engineId);
}

function rowText(rows) {
  return rows.map((row) => cleanText(row && (row.text || row.content || row.content_preview), 1200)).join('\n');
}

async function main() {
  const { createAgentRuntimeTranscriptStore } = load('gateway/runtime/agent_runtime/agent_runtime_transcripts.ts');
  const { buildAgentRuntimeContextPack } = load('gateway/runtime/agent_runtime/agent_runtime_context_pack.ts');

  const statusDir = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-agent-runtime-transcript-parity-'));
  const store = createAgentRuntimeTranscriptStore({ statusDir, maxRecords: 32, windowLimit: 80 });
  const engines = ['infring_native', 'codex_cli'];
  const violations = [];

  engines.forEach((engineId, index) => {
    store.appendAgentRuntimeTranscriptTurn({
      agentId: AGENT_ID,
      sessionId: SESSION_ID,
      traceId: `${TRACE_ID}-${engineId}`,
      turnId: `turn-${index + 1}-${engineId}`,
      engineId,
      userText: `User asks ${engineId} to remember ${CONTINUITY_KEY}.`,
      assistantText: `${engineId} confirms shared Gateway transcript persistence for ${CONTINUITY_KEY}.`,
    });
  });

  const basePayload = {
    type: 'session_projection',
    session_id: SESSION_ID,
    message_window: {
      rows: [],
      total_count: 0,
      before_cursor: null,
      after_cursor: null,
    },
  };
  const merged = store.mergeAgentRuntimeTranscriptPayload(basePayload, {
    agentId: AGENT_ID,
    sessionId: SESSION_ID,
    limit: 80,
  });
  const rows = Array.isArray(merged && merged.message_window && merged.message_window.rows)
    ? merged.message_window.rows
    : [];
  const mergedText = rowText(rows);

  assertCondition(violations, merged && merged.agent_runtime_transcript_overlay, 'transcript_overlay_missing');
  assertCondition(violations, rows.length === 4, 'unexpected_transcript_row_count', { row_count: rows.length });
  assertCondition(violations, mergedText.includes(CONTINUITY_KEY), 'continuity_key_missing_from_merged_projection');
  assertCondition(violations, rows.every((row) => row && row.projection_owner === 'gateway.runtime.agent_runtime_transcript'), 'projection_owner_not_gateway_owned');
  assertCondition(violations, rows.every((row) => row && row.source === 'agent_runtime_socket'), 'transcript_source_not_agent_runtime_socket');
  assertCondition(violations, rows.every((row) => cleanText(row.trace_id, 200)), 'trace_id_missing_from_transcript_row');
  assertCondition(violations, rows.every((row) => cleanText(row.detail_ref, 240)), 'detail_ref_missing_from_transcript_row');
  assertCondition(violations, rowsForEngine(rows, 'infring_native').length === 2, 'native_engine_rows_missing_or_extra', {
    native_row_count: rowsForEngine(rows, 'infring_native').length,
  });
  assertCondition(violations, rowsForEngine(rows, 'codex_cli').length === 2, 'external_engine_rows_missing_or_extra', {
    external_row_count: rowsForEngine(rows, 'codex_cli').length,
  });
  assertCondition(violations, Buffer.byteLength(JSON.stringify(merged), 'utf8') <= 65536, 'merged_projection_exceeds_default_gateway_budget', {
    byte_length: Buffer.byteLength(JSON.stringify(merged), 'utf8'),
  });

  const contextPack = buildAgentRuntimeContextPack({
    agentId: AGENT_ID,
    sessionId: SESSION_ID,
    traceId: TRACE_ID,
    body: {
      context_projection: {
        source: 'gateway.runtime.agent_runtime_transcript',
        rows,
      },
    },
  });
  const fragmentText = rowText((contextPack.fragments || []).map((fragment) => ({
    text: fragment && fragment.payload && (fragment.payload.text_preview || fragment.payload.summary),
  })));
  const sourceKinds = new Set((contextPack.fragments || [])
    .map((fragment) => fragment && fragment.payload && fragment.payload.source_kind)
    .filter(Boolean));

  assertCondition(violations, contextPack && contextPack.type === 'agent_runtime_context_pack', 'context_pack_not_materialized');
  assertCondition(violations, Number(contextPack.row_count) === 4, 'context_pack_row_count_mismatch', { row_count: contextPack.row_count });
  assertCondition(violations, fragmentText.includes(CONTINUITY_KEY), 'continuity_key_missing_from_context_pack');
  assertCondition(violations, sourceKinds.has('user_message'), 'context_pack_missing_user_message_source_kind');
  assertCondition(violations, sourceKinds.has('assistant_message'), 'context_pack_missing_assistant_message_source_kind');
  assertCondition(violations, Number(contextPack.fanout_target) === 7, 'context_pack_fanout_target_changed', { fanout_target: contextPack.fanout_target });

  const payload = {
    ok: violations.length === 0,
    type: 'agent_runtime_transcript_persistence_parity_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_gateway_transcript_store',
    status_dir: statusDir,
    transcript_path: store.transcriptPath,
    engines_tested: engines,
    row_count: rows.length,
    overlay: merged && merged.agent_runtime_transcript_overlay ? merged.agent_runtime_transcript_overlay : null,
    context_pack: {
      row_count: Number(contextPack && contextPack.row_count) || 0,
      fragment_count: Array.isArray(contextPack && contextPack.fragments) ? contextPack.fragments.length : 0,
      fanout_target: Number(contextPack && contextPack.fanout_target) || 0,
      source_authority: cleanText(contextPack && contextPack.source_authority, 200),
    },
    budget: {
      merged_projection_bytes: Buffer.byteLength(JSON.stringify(merged), 'utf8'),
      default_gateway_budget_bytes: 65536,
    },
    violations,
  };
  writeArtifact(payload);
  console.log(JSON.stringify(payload, null, 2));
  if (!payload.ok) process.exit(1);
}

main().catch((error) => {
  const payload = {
    ok: false,
    type: 'agent_runtime_transcript_persistence_parity_guard',
    error: cleanText(error && error.stack ? error.stack : error, 6000),
  };
  writeArtifact(payload);
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
});
