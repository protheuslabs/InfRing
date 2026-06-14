#!/usr/bin/env node
/* eslint-disable no-console */

// Ensures Gateway-owned steering interventions become ordered activity-trace
// rows. Shells should render this projection; they must not infer steering
// semantics themselves.

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_steering_trace_sequence_guard_current.json';
const SCRATCH_DIR = 'core/local/artifacts/agent-runtime-steering-trace-sequence-scratch';

function clean(value: any, max = 1000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function writeReport(report: JsonObject) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

function requireTs(rel: string): any {
  return require(path.join(ROOT, rel));
}

async function main() {
  const { createAgentRuntimeTurnProjectionStore } = requireTs('gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts');
  const { createAgentRuntimeSessionStateStore } = requireTs('gateway/runtime/agent_runtime/agent_runtime_session_state.ts');
  const scratchAbs = path.join(ROOT, SCRATCH_DIR);
  try { fs.rmSync(scratchAbs, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(scratchAbs, { recursive: true });
  const sessionStore = createAgentRuntimeSessionStateStore({
    statusDir: scratchAbs,
    loadRegistry: () => ({ engines: [{ engine_id: 'codex_cli', status: 'adapter_seam_ready' }] }),
    findEngine: (registry: JsonObject, id: string) => (registry.engines || []).find((row: JsonObject) => row.engine_id === id),
  });
  const queued = sessionStore.agentRuntimeSteerProjection('trace-steer-sequence-route', {
    engine_id: 'codex_cli',
    agent_id: 'agent-steer-sequence-guard',
    session_id: 'session-steer-sequence-guard',
    text: 'Please use the simplest safe implementation.',
    priority: 'steer',
  });
  const store = createAgentRuntimeTurnProjectionStore({
    root: ROOT,
    loadAgentRuntimeEngineRegistry: () => ({ engines: [{ engine_id: 'codex_cli', status: 'adapter_seam_ready' }] }),
    findAgentRuntimeEngine: (registry: JsonObject, id: string) => (registry.engines || []).find((row: JsonObject) => row.engine_id === id),
    loadAgentRuntimeWorkspace: () => ({ workspace_dir: ROOT, active_workspace: ROOT }),
    createRouter: () => ({
      healthCheck: async () => ({ status: 'available' }),
      startSession: async () => ({ status: 'started' }),
      streamTurn: async () => ({
        status: 'completed',
        result_ref: 'artifact/codex/result',
        receipt_ref: 'receipt/codex/turn',
        output_text: 'done after steering',
        output_preview: 'done after steering',
        activity_events: [{
          type: 'agent_activity_event',
          activity_kind: 'tool_call',
          provider_event_type: 'command.started',
          status: 'completed',
          display_text: 'Read project status.',
        }],
        activity_event_count: 1,
        structured_activity: true,
      }),
    }),
    drainAgentRuntimeSteeringInterventions: sessionStore.drainAgentRuntimeSteeringInterventions,
    recordAgentRuntimeTurnReceipts: () => ({ receipt_refs: ['receipt/test'], receipt_count: 1 }),
  });

  const result = await store.agentRuntimeTurnProjection('trace-steer-sequence-guard', {
    engine_id: 'codex_cli',
    agent_id: 'agent-steer-sequence-guard',
    session_id: 'session-steer-sequence-guard',
    turn_id: 'turn-steer-sequence-guard',
    input: { text: 'Continue after steer.' },
  });
  const rows = result.activity_trace && Array.isArray(result.activity_trace.rows) ? result.activity_trace.rows : [];
  const steerIndex = rows.findIndex((row: JsonObject) => row.activity_kind === 'user_steer');
  const toolIndex = rows.findIndex((row: JsonObject) => row.provider_event_type === 'command.started');
  const sequences = rows.map((row: JsonObject) => Number(row.sequence_no) || 0);
  const monotonic = sequences.every((value: number, index: number) => value === index + 1);
  const steerRow = steerIndex >= 0 ? rows[steerIndex] : null;
  const ackRow = queued?.steering_activity_row && typeof queued.steering_activity_row === 'object'
    ? queued.steering_activity_row
    : null;
  const shellSteerSource = fs.readFileSync(path.join(ROOT, 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/060-suggestions-and-hints.ts'), 'utf8');
  const shellQueueSource = fs.readFileSync(path.join(ROOT, 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/190-drawer-and-queue.part03.ts'), 'utf8');
  const shellTraceSource = fs.readFileSync(path.join(ROOT, 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/200-send-pipeline.part01.ts'), 'utf8');
  const checks = {
    steer_route_ack_ok: queued?.ok === true,
    steer_route_queued_next_turn: queued?.status === 'queued_next_turn',
    steer_ack_activity_row_present: ackRow?.activity_kind === 'user_steer',
    steer_ack_activity_row_is_user_timeline: ackRow?.timeline_role === 'user_steer',
    steer_ack_activity_row_has_user_text: clean(ackRow?.user_text, 1000).includes('simplest safe implementation'),
    turn_completed: result.status === 'completed',
    context_pack_counts_steering: result.context_pack?.steering_intervention_count === 1,
    steer_row_present: steerIndex >= 0,
    steer_row_has_user_text: clean(steerRow?.user_text, 1000).includes('simplest safe implementation'),
    steer_row_has_timeline_role: steerRow?.timeline_role === 'user_steer',
    steer_before_tool: steerIndex >= 0 && toolIndex >= 0 && steerIndex < toolIndex,
    sequence_numbers_monotonic: monotonic,
    worked_trace_present: result.activity_trace?.type === 'agent_runtime_activity_trace_projection',
    shell_queues_steer_followup_runtime_turn: shellSteerSource.includes("queue_kind: 'agent_runtime_steer_followup'"),
    shell_followup_does_not_append_duplicate_user_message: shellQueueSource.includes("queueKind === 'agent_runtime_steer_followup'"),
    shell_followup_marks_trigger_source: shellQueueSource.includes("trigger_source: 'steer_followup'"),
    shell_skips_steer_pending_for_nonsteer_activity: shellTraceSource.includes('row.agent_runtime_steer_pending && !isSteeringEvent'),
    shell_composer_submits_active_runtime_steer: shellTraceSource.includes('runtime_steer_direct: shouldSubmitRuntimeSteer') &&
      shellTraceSource.includes('selfSteer.steerPromptQueueItem(steerQueueId)'),
    shell_direct_runtime_steer_can_suppress_notice: shellSteerSource.includes('silent_steer_notice === true'),
    shell_runtime_completion_drains_queued_steer_followup: shellTraceSource.includes('drainQueueAfterRuntimeTurn') &&
      shellTraceSource.includes('selfRuntimeQueueDrain._processQueue()'),
    shell_next_turn_steer_reuses_user_text: shellSteerSource.includes("queue_kind: 'agent_runtime_steer_followup'") &&
      !shellSteerSource.includes("text: 'Continue with the queued user steering instruction.'"),
    shell_late_steer_ack_drains_when_idle: shellSteerSource.includes('selfSteerQueueDrain._processQueue()'),
  };
  const violations = Object.entries(checks)
    .filter(([, value]) => value !== true)
    .map(([kind]) => ({ kind }));
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_steering_trace_sequence_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    checks,
    row_projection: rows.map((row: JsonObject) => ({
      sequence_no: row.sequence_no,
      activity_kind: row.activity_kind,
      provider_event_type: row.provider_event_type,
      title: row.title,
      timeline_role: row.timeline_role,
      steering_id: row.steering_id,
      user_text_preview: clean(row.user_text_preview || row.user_text, 500),
    })),
    ack_projection: ackRow ? {
      activity_kind: ackRow.activity_kind,
      provider_event_type: ackRow.provider_event_type,
      timeline_role: ackRow.timeline_role,
      steering_id: ackRow.steering_id,
      user_text_preview: clean(ackRow.user_text_preview || ackRow.user_text, 500),
    } : null,
    violations,
  };
  try { fs.rmSync(scratchAbs, { recursive: true, force: true }); } catch {}
  writeReport(report);
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  writeReport({
    ok: false,
    type: 'agent_runtime_steering_trace_sequence_guard',
    generated_at: new Date().toISOString(),
    error: clean(error && (error.stack || error.message || error), 2000),
    violations: [{ kind: 'guard_exception' }],
  });
  process.exit(1);
});
