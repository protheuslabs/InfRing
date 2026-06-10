#!/usr/bin/env node
/* eslint-disable no-console */

// Enforces pressure against prompt-text compatibility becoming permanent.
// The Gateway may render prompt text from AgentRuntimeStructuredTurn for CLI
// compatibility, but every engine must declare its current transport, target
// transport, and migration posture in the engine registry.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const REGISTRY_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_engine_registry.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_transport_migration_pressure_guard_current.json');
const REAL_WORK_REPLAY_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_real_work_replay_guard_current.json');
const NATIVE_TRANSPORT_PROBE_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_native_transport_probe_current.json');
const VALID_CONTEXT_MODES = new Set(['native_session_bridge', 'prompt_text_compat', 'bounded_stdin_context_envelope', 'bounded_cli_arg_context_envelope', 'structured_json', 'context_refs', 'embedding_refs', 'native_infring_context']);
const VALID_TARGETS = new Set(['native_session_bridge', 'structured_json', 'native_structured_context_api', 'context_refs', 'embedding_refs', 'native_infring_context']);
const BOUNDED_ENVELOPE_MODES = new Set(['bounded_stdin_context_envelope', 'bounded_cli_arg_context_envelope']);
const GOLDEN_EXTERNAL_ENGINES = ['codex_cli', 'claude_code'];
const REQUIRED_BOUNDED_ENVELOPE_EXIT_CRITERIA = new Set([
  'upstream_native_transport_probe',
  'structured_context_mapping',
  'activity_trace_mapping',
  'approval_resume_mapping',
  'golden_pair_parity_replay',
]);
const PROMPT_COMPAT_STATUSES = new Set(['transitional', 'not_primary_transport']);
const PROMPT_TEXT_RETIRE_WARNING_DAYS = Number.parseInt(
  process.env.AGENT_RUNTIME_PROMPT_TEXT_RETIRE_WARNING_DAYS || '30',
  10,
);
const BOUNDED_ENVELOPE_REVIEW_WARNING_DAYS = Number.parseInt(
  process.env.AGENT_RUNTIME_BOUNDED_ENVELOPE_REVIEW_WARNING_DAYS || '45',
  10,
);

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return {};
  }
}

function parseDate(value) {
  const text = clean(value, 40);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) return null;
  const date = new Date(`${text}T00:00:00.000Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function daysUntil(date) {
  if (!date) return null;
  const now = new Date();
  const todayMs = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  return Math.ceil((date.getTime() - todayMs) / 86400000);
}

function realWorkReplayRow(realWorkReplay, engineId) {
  const rows = Array.isArray(realWorkReplay && realWorkReplay.results) ? realWorkReplay.results : [];
  return rows.find((row) => row && clean(row.engine_id, 120) === engineId) || null;
}

function nativeTransportProbeRow(nativeTransportProbe, engineId) {
  const rows = Array.isArray(nativeTransportProbe && nativeTransportProbe.rows) ? nativeTransportProbe.rows : [];
  return rows.find((row) => row && clean(row.engine_id, 120) === engineId) || null;
}

function replayUsabilityOk(row) {
  const checks = row && row.usability_checks && typeof row.usability_checks === 'object'
    ? row.usability_checks
    : {};
  return !!(
    row &&
    row.ok === true &&
    Object.keys(checks).length > 0 &&
    Object.values(checks).every((value) => value === true)
  );
}

function boundedEnvelopeExitCriteriaStatus({ engineId, contextMode, target, boundedEnvelope, replayRow, nativeProbeRow, goldenPairReplayOk }) {
  const nativeProbe = boundedEnvelope && boundedEnvelope.native_transport_probe && typeof boundedEnvelope.native_transport_probe === 'object'
    ? boundedEnvelope.native_transport_probe
    : null;
  const nativeProbeStatus = clean(nativeProbe && nativeProbe.status, 120);
  const artifactProbeStatus = clean(nativeProbeRow && nativeProbeRow.probe_status, 120);
  const artifactProbeAvailable = nativeProbeRow && nativeProbeRow.typed_turn_api_available === true;
  const nativeProbePassed = artifactProbeAvailable || ['passed', 'available', 'native_transport_available'].includes(nativeProbeStatus);
  const checks = replayRow && replayRow.usability_checks && typeof replayRow.usability_checks === 'object'
    ? replayRow.usability_checks
    : {};
  const approvalOk = checks.pending_pause_visible === true &&
    checks.bounded_approval_projection === true &&
    checks.approval_route_present === true &&
    checks.decision_receipt_present === true;
  const result = {
    upstream_native_transport_probe: {
      status: nativeProbePassed ? 'passed' : 'pending',
      evidence: artifactProbeStatus
        ? clean(nativeProbeRow && nativeProbeRow.evidence, 500)
        : nativeProbeStatus
          ? `Native transport probe status: ${nativeProbeStatus}.`
          : 'No upstream native structured transport probe recorded yet.',
    },
    structured_context_mapping: {
      status: BOUNDED_ENVELOPE_MODES.has(contextMode) && target === 'native_structured_context_api' && boundedEnvelope ? 'passed' : 'pending',
      evidence: BOUNDED_ENVELOPE_MODES.has(contextMode) && target === 'native_structured_context_api' && boundedEnvelope
        ? 'Gateway structured turn remains source authority before bounded CLI/stdin rendering.'
        : 'Structured-source bounded envelope mapping is not fully declared.',
    },
    activity_trace_mapping: {
      status: checks.activity_trace_projection_present === true ? 'passed' : 'pending',
      evidence: checks.activity_trace_projection_present === true
        ? 'Real-work replay observed bounded activity trace projection.'
        : 'Real-work replay has not proven activity trace mapping for this engine.',
    },
    approval_resume_mapping: {
      status: approvalOk ? 'passed' : 'pending',
      evidence: approvalOk
        ? 'Real-work replay observed approval pause, bounded projection, approval route, and decision receipt.'
        : 'Real-work replay has not proven approval pause/resume mapping for this engine.',
    },
    golden_pair_parity_replay: {
      status: GOLDEN_EXTERNAL_ENGINES.includes(engineId)
        ? replayUsabilityOk(replayRow) ? 'passed' : 'pending'
        : goldenPairReplayOk ? 'covered_by_golden_pair' : 'pending',
      evidence: GOLDEN_EXTERNAL_ENGINES.includes(engineId)
        ? replayUsabilityOk(replayRow)
          ? 'This golden-pair engine passed practical real-work replay.'
          : 'This golden-pair engine has not passed practical real-work replay.'
        : goldenPairReplayOk
          ? 'Codex and Claude Code passed the golden-pair practical replay; this engine remains secondary.'
          : 'Golden-pair practical replay is not fully passing yet.',
    },
  };
  return result;
}

function pendingExitCriteria(statuses) {
  return Object.entries(statuses || {})
    .filter(([, row]) => row && row.status === 'pending')
    .map(([key]) => key);
}

function main() {
  const registry = JSON.parse(fs.readFileSync(REGISTRY_PATH, 'utf8'));
  const realWorkReplay = readJson(REAL_WORK_REPLAY_PATH);
  const nativeTransportProbe = readJson(NATIVE_TRANSPORT_PROBE_PATH);
  const engines = Array.isArray(registry.engines) ? registry.engines : [];
  const goldenPairReplayOk = GOLDEN_EXTERNAL_ENGINES.every((engineId) => replayUsabilityOk(realWorkReplayRow(realWorkReplay, engineId)));
  const violations = [];
  const warnings = [];
  const rows = [];
  for (const engine of engines) {
    const engineId = clean(engine && engine.engine_id, 120);
    const status = clean(engine && engine.status, 120);
    const contextMode = clean(engine && engine.context_transport_mode, 120);
    const target = clean(engine && engine.structured_transport_target, 120);
    const migrationStatus = clean(engine && engine.transport_migration_status, 160);
    const promptCompat = engine && engine.prompt_text_compatibility && typeof engine.prompt_text_compatibility === 'object'
      ? engine.prompt_text_compatibility
      : null;
    const boundedEnvelope = engine && engine.structured_source_envelope && typeof engine.structured_source_envelope === 'object'
      ? engine.structured_source_envelope
      : null;
    const promptStatus = clean(promptCompat && promptCompat.status, 120);
    const retireBy = clean(promptCompat && promptCompat.retire_by, 40);
    const retireDate = parseDate(retireBy);
    const nextStep = clean(promptCompat && promptCompat.next_step, 500);
    const retireDaysRemaining = daysUntil(retireDate);
    const boundedEnvelopeStatus = clean(boundedEnvelope && boundedEnvelope.status, 120);
    const boundedEnvelopeReviewBy = clean(boundedEnvelope && boundedEnvelope.review_by, 40);
    const boundedEnvelopeReviewDate = parseDate(boundedEnvelopeReviewBy);
    const boundedEnvelopeReviewDaysRemaining = daysUntil(boundedEnvelopeReviewDate);
    const boundedEnvelopeNextStep = clean(boundedEnvelope && boundedEnvelope.next_step, 500);
    const boundedEnvelopeTarget = clean(boundedEnvelope && boundedEnvelope.target, 120);
    const boundedEnvelopeExitCriteria = Array.isArray(boundedEnvelope && boundedEnvelope.exit_criteria)
      ? boundedEnvelope.exit_criteria.map((item) => clean(item, 160)).filter(Boolean)
      : [];
    const boundedEnvelopeExitCriteriaSet = new Set(boundedEnvelopeExitCriteria);
    const boundedEnvelopeMissingExitCriteria = Array.from(REQUIRED_BOUNDED_ENVELOPE_EXIT_CRITERIA)
      .filter((criterion) => !boundedEnvelopeExitCriteriaSet.has(criterion));
    const replayRow = realWorkReplayRow(realWorkReplay, engineId);
    const nativeProbeRow = nativeTransportProbeRow(nativeTransportProbe, engineId);
    const exitCriteriaStatus = boundedEnvelopeExitCriteriaStatus({
      engineId,
      contextMode,
      target,
      boundedEnvelope,
      replayRow,
      nativeProbeRow,
      goldenPairReplayOk,
    });
    const pendingCriteria = pendingExitCriteria(exitCriteriaStatus);
    const row = {
      engine_id: engineId,
      status,
      context_transport_mode: contextMode,
      structured_transport_target: target,
      transport_migration_status: migrationStatus,
      prompt_text_compatibility_status: promptStatus,
      prompt_text_compatibility_retire_by: retireBy,
      prompt_text_retire_days_remaining: retireDaysRemaining,
      prompt_text_next_step_present: !!nextStep,
      structured_source_envelope_status: boundedEnvelopeStatus,
      structured_source_envelope_target: boundedEnvelopeTarget,
      structured_source_envelope_review_by: boundedEnvelopeReviewBy,
      structured_source_envelope_review_days_remaining: boundedEnvelopeReviewDaysRemaining,
      structured_source_envelope_next_step_present: !!boundedEnvelopeNextStep,
      structured_source_envelope_exit_criteria_count: boundedEnvelopeExitCriteria.length,
      structured_source_envelope_missing_exit_criteria: boundedEnvelopeMissingExitCriteria,
      structured_source_envelope_exit_criteria_status: BOUNDED_ENVELOPE_MODES.has(contextMode) ? exitCriteriaStatus : null,
      structured_source_envelope_pending_exit_criteria: BOUNDED_ENVELOPE_MODES.has(contextMode) ? pendingCriteria : [],
    };
    rows.push(row);

    if (!engineId) violations.push({ kind: 'engine_id_missing', row });
    if (!VALID_CONTEXT_MODES.has(contextMode)) violations.push({ kind: 'context_transport_mode_invalid_or_missing', engine_id: engineId, context_transport_mode: contextMode });
    if (!VALID_TARGETS.has(target)) violations.push({ kind: 'structured_transport_target_invalid_or_missing', engine_id: engineId, structured_transport_target: target });
    if (!migrationStatus) violations.push({ kind: 'transport_migration_status_missing', engine_id: engineId });
    if (!promptCompat) violations.push({ kind: 'prompt_text_compatibility_policy_missing', engine_id: engineId });
    if (promptCompat && !PROMPT_COMPAT_STATUSES.has(promptStatus)) {
      violations.push({ kind: 'prompt_text_compatibility_status_invalid', engine_id: engineId, status: promptStatus });
    }

    if (contextMode === 'prompt_text_compat') {
      if (target === 'prompt_text_compat') violations.push({ kind: 'prompt_text_target_cannot_be_prompt_text', engine_id: engineId });
      if (!/^transitional/.test(migrationStatus)) violations.push({ kind: 'prompt_text_engine_not_marked_transitional', engine_id: engineId, transport_migration_status: migrationStatus });
      if (promptStatus !== 'transitional') violations.push({ kind: 'prompt_text_engine_policy_not_transitional', engine_id: engineId, status: promptStatus });
      if (!retireDate) violations.push({ kind: 'prompt_text_retire_by_missing_or_invalid', engine_id: engineId, retire_by: retireBy });
      if (retireDaysRemaining !== null && retireDaysRemaining < 0) {
        violations.push({
          kind: 'prompt_text_retire_by_expired',
          engine_id: engineId,
          retire_by: retireBy,
          days_remaining: retireDaysRemaining,
        });
      } else if (
        retireDaysRemaining !== null &&
        retireDaysRemaining <= Math.max(0, PROMPT_TEXT_RETIRE_WARNING_DAYS)
      ) {
        warnings.push({
          kind: 'prompt_text_retire_window_active',
          engine_id: engineId,
          retire_by: retireBy,
          days_remaining: retireDaysRemaining,
          warning_window_days: Math.max(0, PROMPT_TEXT_RETIRE_WARNING_DAYS),
        });
      }
      if (!nextStep) violations.push({ kind: 'prompt_text_next_step_missing', engine_id: engineId });
    } else {
      if (promptStatus === 'transitional') violations.push({ kind: 'non_prompt_engine_marked_prompt_transitional', engine_id: engineId, context_transport_mode: contextMode });
      if (retireBy) violations.push({ kind: 'non_prompt_engine_has_prompt_retire_deadline', engine_id: engineId, retire_by: retireBy });
    }

    if (BOUNDED_ENVELOPE_MODES.has(contextMode)) {
      if (target !== 'native_structured_context_api') {
        violations.push({ kind: 'bounded_envelope_target_not_native_structured_api', engine_id: engineId, structured_transport_target: target });
      }
      if (!boundedEnvelope) violations.push({ kind: 'bounded_envelope_policy_missing', engine_id: engineId });
      if (boundedEnvelope && boundedEnvelopeStatus !== 'active_transitional_envelope') {
        violations.push({ kind: 'bounded_envelope_status_invalid', engine_id: engineId, status: boundedEnvelopeStatus });
      }
      if (boundedEnvelope && boundedEnvelopeTarget !== 'native_structured_context_api') {
        violations.push({ kind: 'bounded_envelope_policy_target_invalid', engine_id: engineId, target: boundedEnvelopeTarget });
      }
      if (boundedEnvelope && !boundedEnvelopeReviewDate) {
        violations.push({ kind: 'bounded_envelope_review_by_missing_or_invalid', engine_id: engineId, review_by: boundedEnvelopeReviewBy });
      }
      if (boundedEnvelopeReviewDaysRemaining !== null && boundedEnvelopeReviewDaysRemaining < 0) {
        violations.push({
          kind: 'bounded_envelope_review_by_expired',
          engine_id: engineId,
          review_by: boundedEnvelopeReviewBy,
          days_remaining: boundedEnvelopeReviewDaysRemaining,
        });
      } else if (
        boundedEnvelopeReviewDaysRemaining !== null &&
        boundedEnvelopeReviewDaysRemaining <= Math.max(0, BOUNDED_ENVELOPE_REVIEW_WARNING_DAYS) &&
        pendingCriteria.length > 0
      ) {
        warnings.push({
          kind: 'bounded_envelope_review_window_active',
          engine_id: engineId,
          review_by: boundedEnvelopeReviewBy,
          days_remaining: boundedEnvelopeReviewDaysRemaining,
          warning_window_days: Math.max(0, BOUNDED_ENVELOPE_REVIEW_WARNING_DAYS),
          exit_criteria: boundedEnvelopeExitCriteria,
          exit_criteria_status: exitCriteriaStatus,
          pending_exit_criteria: pendingCriteria,
          next_action: pendingCriteria.length === 1 && pendingCriteria[0] === 'upstream_native_transport_probe'
            ? 'Probe upstream runtime documentation or local runtime capabilities for a stable typed turn/session/context API that can accept AgentRuntimeStructuredTurn without bounded prompt rendering.'
            : pendingCriteria.length > 0
              ? `Complete or renew bounded-envelope exit criteria still pending: ${pendingCriteria.join(', ')}.`
              : 'All bounded-envelope exit criteria are covered by current evidence; renew or clear the review window.',
        });
      }
      if (!boundedEnvelopeNextStep) violations.push({ kind: 'bounded_envelope_next_step_missing', engine_id: engineId });
      if (boundedEnvelopeMissingExitCriteria.length > 0) {
        violations.push({
          kind: 'bounded_envelope_exit_criteria_incomplete',
          engine_id: engineId,
          missing_exit_criteria: boundedEnvelopeMissingExitCriteria,
        });
      }
    }

    if (status === 'adapter_seam_ready' || status === 'safe_cli_bridge') {
      if (contextMode === 'prompt_text_compat' && !retireDate) {
        violations.push({ kind: 'live_selectable_prompt_engine_without_retirement_pressure', engine_id: engineId });
      }
      const boundedCliStructuredTarget =
        (contextMode === 'bounded_stdin_context_envelope' || contextMode === 'bounded_cli_arg_context_envelope') &&
        (target === 'native_structured_context_api' || target === 'structured_json');
      if (contextMode !== 'prompt_text_compat' && target !== contextMode && contextMode !== 'native_session_bridge' && !boundedCliStructuredTarget) {
        violations.push({ kind: 'live_selectable_structured_transport_mismatch', engine_id: engineId, context_transport_mode: contextMode, structured_transport_target: target });
      }
    }
  }

  const promptTextEngines = rows.filter((row) => row.context_transport_mode === 'prompt_text_compat').map((row) => row.engine_id);
  const boundedEnvelopeEngines = rows.filter((row) => BOUNDED_ENVELOPE_MODES.has(row.context_transport_mode)).map((row) => row.engine_id);
  const structuredReadyOrTargeted = rows.filter((row) => row.structured_transport_target && row.structured_transport_target !== 'prompt_text_compat').map((row) => row.engine_id);
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_transport_migration_pressure_guard',
    generated_at: new Date().toISOString(),
    registry_path: 'validation/conformance/contracts/agent_runtime_engine_registry.json',
    native_transport_probe_artifact: 'core/local/artifacts/agent_runtime_native_transport_probe_current.json',
    policy: {
      prompt_text_is_transitional_only: true,
      prompt_text_engines_need_retire_by: true,
      prompt_text_engines_need_next_step: true,
      prompt_text_retire_by_expires: true,
      prompt_text_retire_warning_days: Math.max(0, PROMPT_TEXT_RETIRE_WARNING_DAYS),
      bounded_envelope_review_warning_days: Math.max(0, BOUNDED_ENVELOPE_REVIEW_WARNING_DAYS),
      structured_transport_target_required_for_every_engine: true,
      bounded_envelopes_are_transitional: true,
      bounded_envelopes_must_target_native_structured_context_api: true,
      bounded_envelopes_must_have_exit_criteria: true,
      bounded_envelope_required_exit_criteria: Array.from(REQUIRED_BOUNDED_ENVELOPE_EXIT_CRITERIA),
    },
    summary: {
      engine_count: rows.length,
      prompt_text_engine_count: promptTextEngines.length,
      bounded_envelope_engine_count: boundedEnvelopeEngines.length,
      structured_ready_or_targeted_count: structuredReadyOrTargeted.length,
      warning_count: warnings.length,
      violation_count: violations.length,
    },
    prompt_text_engines: promptTextEngines,
    bounded_envelope_engines: boundedEnvelopeEngines,
    structured_ready_or_targeted: structuredReadyOrTargeted,
    rows,
    warnings,
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
