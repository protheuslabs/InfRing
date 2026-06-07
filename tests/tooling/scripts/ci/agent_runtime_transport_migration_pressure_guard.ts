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
const VALID_CONTEXT_MODES = new Set(['native_session_bridge', 'prompt_text_compat', 'structured_json', 'context_refs', 'embedding_refs', 'native_infring_context']);
const VALID_TARGETS = new Set(['native_session_bridge', 'structured_json', 'context_refs', 'embedding_refs', 'native_infring_context']);
const PROMPT_COMPAT_STATUSES = new Set(['transitional', 'not_primary_transport']);

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function parseDate(value) {
  const text = clean(value, 40);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) return null;
  const date = new Date(`${text}T00:00:00.000Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function main() {
  const registry = JSON.parse(fs.readFileSync(REGISTRY_PATH, 'utf8'));
  const engines = Array.isArray(registry.engines) ? registry.engines : [];
  const violations = [];
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
    const promptStatus = clean(promptCompat && promptCompat.status, 120);
    const retireBy = clean(promptCompat && promptCompat.retire_by, 40);
    const retireDate = parseDate(retireBy);
    const nextStep = clean(promptCompat && promptCompat.next_step, 500);
    const row = {
      engine_id: engineId,
      status,
      context_transport_mode: contextMode,
      structured_transport_target: target,
      transport_migration_status: migrationStatus,
      prompt_text_compatibility_status: promptStatus,
      prompt_text_compatibility_retire_by: retireBy,
      prompt_text_next_step_present: !!nextStep,
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
      if (!nextStep) violations.push({ kind: 'prompt_text_next_step_missing', engine_id: engineId });
    } else {
      if (promptStatus === 'transitional') violations.push({ kind: 'non_prompt_engine_marked_prompt_transitional', engine_id: engineId, context_transport_mode: contextMode });
      if (retireBy) violations.push({ kind: 'non_prompt_engine_has_prompt_retire_deadline', engine_id: engineId, retire_by: retireBy });
    }

    if (status === 'adapter_seam_ready' || status === 'safe_cli_bridge') {
      if (contextMode === 'prompt_text_compat' && !retireDate) {
        violations.push({ kind: 'live_selectable_prompt_engine_without_retirement_pressure', engine_id: engineId });
      }
      if (contextMode !== 'prompt_text_compat' && target !== contextMode && contextMode !== 'native_session_bridge') {
        violations.push({ kind: 'live_selectable_structured_transport_mismatch', engine_id: engineId, context_transport_mode: contextMode, structured_transport_target: target });
      }
    }
  }

  const promptTextEngines = rows.filter((row) => row.context_transport_mode === 'prompt_text_compat').map((row) => row.engine_id);
  const structuredReadyOrTargeted = rows.filter((row) => row.structured_transport_target && row.structured_transport_target !== 'prompt_text_compat').map((row) => row.engine_id);
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_transport_migration_pressure_guard',
    generated_at: new Date().toISOString(),
    registry_path: 'validation/conformance/contracts/agent_runtime_engine_registry.json',
    policy: {
      prompt_text_is_transitional_only: true,
      prompt_text_engines_need_retire_by: true,
      prompt_text_engines_need_next_step: true,
      structured_transport_target_required_for_every_engine: true,
    },
    summary: {
      engine_count: rows.length,
      prompt_text_engine_count: promptTextEngines.length,
      structured_ready_or_targeted_count: structuredReadyOrTargeted.length,
      violation_count: violations.length,
    },
    prompt_text_engines: promptTextEngines,
    structured_ready_or_targeted: structuredReadyOrTargeted,
    rows,
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
