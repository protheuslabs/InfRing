#!/usr/bin/env node
'use strict';

const childProcess = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const ARTIFACT_PATH = path.join(
  ROOT,
  'core/local/artifacts/agent_runtime_framework_coordination_guard_current.json',
);
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_socket_contract.json';
const LAYER = 'gateway';
const GATE_TIMEOUT_MS = Number.parseInt(
  process.env.AGENT_RUNTIME_FRAMEWORK_COORDINATION_GATE_TIMEOUT_MS || '60000',
  10,
);
const PREVIEW_LIMIT = Number.parseInt(
  process.env.AGENT_RUNTIME_FRAMEWORK_COORDINATION_PREVIEW_CHARS || '2400',
  10,
);

const GATES = [
  {
    id: 'approval_lifecycle',
    script: 'tests/tooling/scripts/ci/agent_runtime_route_approval_lifecycle_guard.ts',
    coverage_scope: 'single_route_lifecycle',
  },
  {
    id: 'permission_pause_resume_normalization',
    script: 'tests/tooling/scripts/ci/agent_runtime_permission_pause_resume_normalization_guard.ts',
    coverage_scope: 'primary_external_cli_final_answer_proposal_pause_resume',
  },
  {
    id: 'rendered_permission_steering_contract',
    script: 'tests/tooling/scripts/ci/agent_runtime_rendered_permission_steering_contract_guard.ts',
    coverage_scope: 'shell_projection_only_permission_pause_and_user_steering_contract',
  },
  {
    id: 'artifact_proposal_apply',
    script: 'tests/tooling/scripts/ci/agent_runtime_artifact_proposal_apply_guard.ts',
    coverage_scope: 'primary_external_cli_create_edit_proposal_approval_apply',
  },
  {
    id: 'context_continuity',
    script: 'tests/tooling/scripts/ci/agent_runtime_context_continuity_eval.ts',
    coverage_scope: 'active_promotion_subset',
  },
  {
    id: 'context_switch_continuity',
    script: 'tests/tooling/scripts/ci/agent_runtime_context_switch_continuity_guard.ts',
    coverage_scope: 'primary_external_public_route_engine_switch',
  },
  {
    id: 'transcript_persistence_parity',
    script: 'tests/tooling/scripts/ci/agent_runtime_transcript_persistence_parity_guard.ts',
    coverage_scope: 'adapter_ready_subset',
  },
  {
    id: 'route_transcript_persistence',
    script: 'tests/tooling/scripts/ci/agent_runtime_route_transcript_persistence_guard.ts',
    coverage_scope: 'adapter_ready_subset',
  },
  {
    id: 'transcript_reload_persistence',
    script: 'tests/tooling/scripts/ci/agent_runtime_transcript_reload_persistence_guard.ts',
    coverage_scope: 'primary_external_completed_failed_stopped_reload_projection',
  },
  {
    id: 'activity_projection',
    script: 'tests/tooling/scripts/ci/agent_runtime_activity_projection_guard.ts',
    coverage_scope: 'provider_fixture_subset',
  },
  {
    id: 'activity_decision_trace_persistence',
    script: 'tests/tooling/scripts/ci/agent_runtime_activity_decision_trace_persistence_guard.ts',
    coverage_scope: 'primary_external_public_route_decision_dialog_tool_trace_reload_projection',
  },
  {
    id: 'model_projection',
    script: 'tests/tooling/scripts/ci/agent_runtime_model_projection_guard.ts',
    coverage_scope: 'selectable_projection_subset',
  },
  {
    id: 'model_projection_truth',
    script: 'tests/tooling/scripts/ci/agent_runtime_model_projection_truth_guard.ts',
    coverage_scope: 'primary_external_gateway_model_projection_truth',
  },
  {
    id: 'runtime_command_execution',
    script: 'tests/tooling/scripts/ci/agent_runtime_command_execution_guard.ts',
    coverage_scope: 'primary_external_safe_status_execution_and_manual_auth',
  },
  {
    id: 'engine_focus',
    script: 'tests/tooling/scripts/ci/agent_runtime_engine_focus_guard.ts',
    coverage_scope: 'agent_runtime_promotion_focus_policy',
  },
  {
    id: 'shell_replaceability',
    script: 'tests/tooling/scripts/ci/agent_runtime_shell_replaceability_guard.ts',
    coverage_scope: 'gateway_socket_contract',
  },
  {
    id: 'socket_live_handler',
    script: 'tests/tooling/scripts/ci/agent_runtime_socket_live_handler_guard.ts',
    coverage_scope: 'gateway_socket_handler',
  },
  {
    id: 'socket_upgrade_transport',
    script: 'tests/tooling/scripts/ci/agent_runtime_socket_upgrade_transport_guard.ts',
    coverage_scope: 'gateway_socket_transport',
  },
  {
    id: 'socket_disposable_gateway',
    script: 'tests/tooling/scripts/ci/agent_runtime_socket_disposable_gateway_guard.ts',
    coverage_scope: 'gateway_disposable_host_socket_transport',
  },
  {
    id: 'route_structured_transport',
    script: 'tests/tooling/scripts/ci/agent_runtime_route_structured_transport_guard.ts',
    coverage_scope: 'adapter_ready_subset',
  },
  {
    id: 'cli_adapter_timeout',
    script: 'tests/tooling/scripts/ci/agent_runtime_cli_adapter_timeout_guard.ts',
    coverage_scope: 'provider_error_projection',
  },
  {
    id: 'hard_failure_projection',
    script: 'tests/tooling/scripts/ci/agent_runtime_hard_failure_projection_guard.ts',
    coverage_scope: 'primary_external_public_route_quota_auth_rate_network_missing_runtime_failures',
  },
  {
    id: 'shadow_attachment_bridge',
    script: 'tests/tooling/scripts/ci/agent_runtime_shadow_attachment_guard.ts',
    coverage_scope: 'adapter_capability_policy',
  },
  {
    id: 'attachment_context_bridge',
    script: 'tests/tooling/scripts/ci/agent_runtime_attachment_context_bridge_guard.ts',
    coverage_scope: 'primary_external_public_route_large_paste_refs',
  },
  {
    id: 'stop_cancel_semantics',
    script: 'tests/tooling/scripts/ci/agent_runtime_stop_cancel_semantics_guard.ts',
    coverage_scope: 'primary_external_public_socket_cancel_transcript_persistence',
  },
  {
    id: 'real_work_replay',
    script: 'tests/tooling/scripts/ci/agent_runtime_real_work_replay_guard.ts',
    coverage_scope: 'full_engine_registry',
  },
  {
    id: 'working_directory_parity',
    script: 'tests/tooling/scripts/ci/agent_runtime_working_directory_parity_guard.ts',
    coverage_scope: 'adapter_ready_subset',
  },
  {
    id: 'working_directory_control',
    script: 'tests/tooling/scripts/ci/agent_runtime_working_directory_control_guard.ts',
    coverage_scope: 'primary_external_cli_selected_workspace_and_escape_rejection',
  },
  {
    id: 'native_transport_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_native_transport_probe.ts',
    coverage_scope: 'bounded_envelope_transport_probe',
  },
  {
    id: 'claude_stream_json_mapping_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_claude_stream_json_mapping_probe.ts',
    coverage_scope: 'claude_code_native_transport_candidate_mapping',
  },
  {
    id: 'codex_app_server_mapping_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_codex_app_server_mapping_probe.ts',
    coverage_scope: 'codex_cli_native_transport_candidate_mapping',
  },
  {
    id: 'opencode_sdk_mapping_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_opencode_sdk_mapping_probe.ts',
    coverage_scope: 'opencode_native_transport_candidate_mapping',
  },
  {
    id: 'opencode_sdk_live_acceptance_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_opencode_sdk_live_acceptance_probe.ts',
    coverage_scope: 'opencode_native_transport_live_acceptance_disabled_by_default',
  },
  {
    id: 'hermes_agent_mapping_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_hermes_agent_mapping_probe.ts',
    coverage_scope: 'hermes_agent_native_transport_candidate_mapping',
  },
  {
    id: 'hermes_agent_live_acceptance_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_hermes_agent_live_acceptance_probe.ts',
    coverage_scope: 'hermes_agent_native_transport_live_acceptance_disabled_by_default',
  },
  {
    id: 'grok_code_mapping_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_grok_code_mapping_probe.ts',
    coverage_scope: 'grok_code_native_transport_candidate_mapping',
  },
  {
    id: 'grok_code_live_acceptance_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_grok_code_live_acceptance_probe.ts',
    coverage_scope: 'grok_code_native_transport_live_acceptance_disabled_by_default',
  },
  {
    id: 'codex_app_server_live_acceptance_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_codex_app_server_live_acceptance_probe.ts',
    coverage_scope: 'codex_cli_native_transport_live_acceptance_disabled_by_default',
  },
  {
    id: 'claude_stream_json_live_acceptance_probe',
    script: 'tests/tooling/scripts/ci/agent_runtime_claude_stream_json_live_acceptance_probe.ts',
    coverage_scope: 'claude_code_native_transport_live_acceptance_disabled_by_default',
  },
  {
    id: 'provider_live_smoke_readiness',
    script: 'tests/tooling/scripts/ci/agent_runtime_provider_live_smoke_readiness_guard.ts',
    coverage_scope: 'primary_external_provider_live_smoke_opt_in_scope',
  },
  {
    id: 'openclaw_live_adapter_readiness',
    script: 'tests/tooling/scripts/ci/agent_runtime_openclaw_live_adapter_readiness_guard.ts',
    coverage_scope: 'secondary_openclaw_live_adapter_readiness_diagnostic',
  },
  {
    id: 'secondary_live_adapter_readiness',
    script: 'tests/tooling/scripts/ci/agent_runtime_secondary_live_adapter_readiness_guard.ts',
    coverage_scope: 'secondary_runtime_opt_in_live_adapter_readiness_diagnostic',
  },
  {
    id: 'transport_migration_pressure',
    script: 'tests/tooling/scripts/ci/agent_runtime_transport_migration_pressure_guard.ts',
    coverage_scope: 'engine_registry_transport_policy',
  },
  {
    id: 'engine_conformance',
    script: 'tests/tooling/scripts/ci/agent_runtime_engine_conformance_guard.ts',
    coverage_scope: 'engine_registry_conformance',
  },
  {
    id: 'engine_scorecard',
    script: 'tests/tooling/scripts/ci/agent_runtime_engine_scorecard.ts',
    coverage_scope: 'engine_registry_scorecard',
  },
  {
    id: 'graduation_baseline_contract',
    script: 'tests/tooling/scripts/ci/agent_runtime_graduation_baseline_contract_guard.ts',
    coverage_scope: 'agent_runtime_graduation_baseline_policy',
  },
  {
    id: 'golden_pair_promotion',
    script: 'tests/tooling/scripts/ci/agent_runtime_golden_pair_promotion_guard.ts',
    coverage_scope: 'codex_claude_practical_loop_with_gated_native_transport',
  },
  {
    id: 'promotion_sprawl',
    script: 'tests/tooling/scripts/ci/agent_runtime_promotion_sprawl_guard.ts',
    coverage_scope: 'external_runtime_daily_driver_promotion_scope',
  },
  {
    id: 'golden_pair_primitive_parity',
    script: 'tests/tooling/scripts/ci/agent_runtime_golden_pair_primitive_parity_guard.ts',
    coverage_scope: 'codex_claude_interchangeable_runtime_primitives',
  },
  {
    id: 'secondary_engine_gap_matrix',
    script: 'tests/tooling/scripts/ci/agent_runtime_secondary_engine_gap_matrix_guard.ts',
    coverage_scope: 'future_runtime_golden_pair_graduation_map',
  },
  {
    id: 'graduation_next_actions',
    script: 'tests/tooling/scripts/ci/agent_runtime_graduation_next_actions_guard.ts',
    coverage_scope: 'future_runtime_graduation_action_queue',
  },
  {
    id: 'secondary_promotion_plan',
    script: 'tests/tooling/scripts/ci/agent_runtime_secondary_promotion_plan_guard.ts',
    coverage_scope: 'future_runtime_ordered_promotion_plan',
  },
  {
    id: 'proof_summary_accounting',
    script: 'tests/tooling/scripts/ci/agent_runtime_proof_summary_guard.ts',
    coverage_scope: 'agent_runtime_proof_summary_policy',
  },
  {
    id: 'proof_ledger_separation',
    script: 'tests/tooling/scripts/ci/proof_ledger_separation_guard.ts',
    coverage_scope: 'agent_runtime_native_coding_accounting_boundary',
  },
  {
    id: 'framework_capability_regressions',
    script: 'tests/tooling/scripts/ci/agent_runtime_framework_capability_guard.ts',
    coverage_scope: 'adapter_capability_policy',
  },
];

function preview(value) {
  const text = String(value || '').trim();
  if (text.length <= PREVIEW_LIMIT) {
    return text;
  }
  return `${text.slice(0, PREVIEW_LIMIT)}... [truncated ${text.length - PREVIEW_LIMIT} chars]`;
}

function parseJsonCandidate(text) {
  const source = String(text || '').trim();
  if (!source) {
    return null;
  }

  const first = source.indexOf('{');
  const last = source.lastIndexOf('}');
  if (first < 0 || last <= first) {
    return null;
  }

  try {
    return JSON.parse(source.slice(first, last + 1));
  } catch {
    return null;
  }
}

function cleanEngineId(value) {
  return String(value == null ? '' : value)
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 120);
}

function uniqueEngineIds(values) {
  const out = [];
  const seen = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const id = cleanEngineId(value);
    if (!id || seen.has(id)) continue;
    seen.add(id);
    out.push(id);
  }
  return out;
}

function loadRegistryEngineIds() {
  try {
    const registry = JSON.parse(fs.readFileSync(path.join(ROOT, 'validation/conformance/contracts/agent_runtime_engine_registry.json'), 'utf8'));
    return uniqueEngineIds((Array.isArray(registry && registry.engines) ? registry.engines : []).map((row) => row && row.engine_id));
  } catch {
    return [];
  }
}

function engineIdsFromParsed(parsed) {
  if (!parsed || typeof parsed !== 'object') return [];
  const candidates = [];
  if (Array.isArray(parsed.engines_tested)) candidates.push(...parsed.engines_tested);
  if (Array.isArray(parsed.sampled_real_work_replay_engines)) candidates.push(...parsed.sampled_real_work_replay_engines);
  if (Array.isArray(parsed.engines)) {
    for (const row of parsed.engines) candidates.push(row && typeof row === 'object' ? row.engine_id : row);
  }
  for (const key of ['results', 'turn_results', 'rows', 'captures']) {
    if (!Array.isArray(parsed[key])) continue;
    for (const row of parsed[key]) candidates.push(row && typeof row === 'object' ? row.engine_id : row);
  }
  return uniqueEngineIds(candidates);
}

function parsedMeta(parsed) {
  const engines = engineIdsFromParsed(parsed);
  const warningCount =
    Number(parsed && parsed.summary && parsed.summary.warning_count) ||
    Number(parsed && parsed.summary && parsed.summary.warnings) ||
    Number(parsed && parsed.summary && parsed.summary.transport_migration_warning_count) ||
    (Array.isArray(parsed && parsed.warnings) ? parsed.warnings.length : 0);
  return {
    type: cleanEngineId(parsed && (parsed.type || parsed.kind)),
    engine_ids: engines,
    engine_count: engines.length,
    expected_unavailable_count: Number(parsed && parsed.expected_unavailable_count) || 0,
    successful_engine_count: Number(parsed && parsed.successful_engine_count) || 0,
    warning_count: Number.isFinite(warningCount) ? warningCount : 0,
  };
}

function parsedWarnings(parsed) {
  if (!parsed || typeof parsed !== 'object') return [];
  const explicitWarnings = Array.isArray(parsed.warnings) ? parsed.warnings : [];
  const rowPromotionWarnings = Array.isArray(parsed.rows)
    ? parsed.rows.flatMap((row) => {
        if (!row || typeof row !== 'object' || !Array.isArray(row.promotion_warnings)) return [];
        const engineId = cleanEngineId(row.engine_id || row.id || '', 120);
        return row.promotion_warnings.map((warning) => ({
          ...(warning && typeof warning === 'object' ? warning : { detail: warning }),
          engine_id:
            warning && typeof warning === 'object' && warning.engine_id
              ? warning.engine_id
              : engineId,
        }));
      })
    : [];
  const transportWarningEngines =
    parsed &&
    parsed.summary &&
    Array.isArray(parsed.summary.transport_migration_warning_engines)
      ? parsed.summary.transport_migration_warning_engines.map((engineId) => ({
          kind: 'bounded_envelope_review_window_active',
          engine_id: engineId,
          detail: `Bounded structured-source envelope review window active for ${engineId}.`,
        }))
      : [];
  const warningRows =
    explicitWarnings.length > 0
      ? explicitWarnings
      : rowPromotionWarnings.length > 0
        ? rowPromotionWarnings
        : transportWarningEngines;
  return warningRows.slice(0, 25).map((row) => {
    if (!row || typeof row !== 'object') return { detail: cleanEngineId(row, 240) };
    return {
      kind: cleanEngineId(row.kind || row.id || row.type || 'warning', 120),
      engine_id: cleanEngineId(row.engine_id || '', 120),
      path: cleanEngineId(row.path || '', 240),
      detail: cleanEngineId(row.detail || row.reason || '', 500),
      days_remaining: Number.isFinite(Number(row.days_remaining)) ? Number(row.days_remaining) : null,
      warning_window_days: Number.isFinite(Number(row.warning_window_days))
        ? Number(row.warning_window_days)
        : null,
    };
  });
}

function warningKey(row) {
  if (!row || typeof row !== 'object') return cleanEngineId(row, 240) || 'warning';
  const kind = cleanEngineId(row.kind || row.id || row.type || 'warning', 120);
  const engineId = cleanEngineId(row.engine_id || '', 120);
  if (
    engineId &&
    (kind === 'bounded_envelope_review_window_active' ||
      kind === 'upstream_native_transport_probe_pending')
  ) {
    return ['bounded_envelope_native_transport_pending', engineId].join('|');
  }
  return [
    kind,
    engineId,
    cleanEngineId(row.path || '', 240),
    cleanEngineId(row.detail || row.reason || '', 500),
  ].join('|');
}

function warningRollup(gateResults) {
  const rawWarningCount = gateResults.reduce(
    (sum, gate) => sum + (Number(gate.parsed_meta && gate.parsed_meta.warning_count) || 0),
    0,
  );
  const warnings = gateResults
    .filter((gate) => (Number(gate.parsed_meta && gate.parsed_meta.warning_count) || 0) > 0)
    .map((gate) => {
      const parsedWarnings = gate.parsed_warnings || [];
      const uniqueKeys = new Set(parsedWarnings.map(warningKey));
      return {
        id: gate.id,
        script: gate.script,
        raw_warning_count: Number(gate.parsed_meta && gate.parsed_meta.warning_count) || 0,
        unique_warning_count: uniqueKeys.size,
        warning_count: uniqueKeys.size || Number(gate.parsed_meta && gate.parsed_meta.warning_count) || 0,
        warnings: parsedWarnings,
      };
    });
  const uniqueWarningKeys = new Set();
  for (const gate of warnings) {
    for (const row of gate.warnings || []) uniqueWarningKeys.add(warningKey(row));
  }
  const uniqueWarningCount =
    uniqueWarningKeys.size ||
    warnings.reduce((sum, gate) => sum + (Number(gate.warning_count) || 0), 0);
  return {
    raw_warning_count: rawWarningCount,
    unique_warning_count: uniqueWarningCount,
    warnings,
  };
}

function runGate(gate) {
  const startedAt = Date.now();
  const child = childProcess.spawnSync(
    process.execPath,
    [ENTRYPOINT, gate.script],
    {
      cwd: ROOT,
      encoding: 'utf8',
      maxBuffer: 4 * 1024 * 1024,
      timeout: Number.isFinite(GATE_TIMEOUT_MS) && GATE_TIMEOUT_MS > 0
        ? GATE_TIMEOUT_MS
        : 60000,
    },
  );

  const parsed = parseJsonCandidate(child.stdout);
  const timedOut = child.error && child.error.code === 'ETIMEDOUT';
  const ok = child.status === 0 && !timedOut && (!parsed || parsed.ok !== false);

  return {
    id: gate.id,
    script: gate.script,
    ok,
    status: child.status,
    signal: child.signal || null,
    timed_out: Boolean(timedOut),
    duration_ms: Date.now() - startedAt,
    parsed_ok: parsed && typeof parsed.ok === 'boolean' ? parsed.ok : null,
    coverage_scope: gate.coverage_scope,
    parsed_meta: parsedMeta(parsed),
    parsed_warnings: parsedWarnings(parsed),
    stdout_preview: preview(child.stdout),
    stderr_preview: preview(child.stderr),
    error: child.error
      ? {
          code: child.error.code || null,
          message: child.error.message || String(child.error),
        }
      : null,
  };
}

function buildCoverageReport(gateResults) {
  const registryEngineIds = loadRegistryEngineIds();
  const registrySet = new Set(registryEngineIds);
  const checks = [];
  const failures = [];
  for (const gate of gateResults) {
    const engineIds = uniqueEngineIds(gate.parsed_meta && gate.parsed_meta.engine_ids);
    const missing = registryEngineIds.filter((id) => !engineIds.includes(id));
    const extra = engineIds.filter((id) => !registrySet.has(id));
    let ok = true;
    let note = '';
    if (gate.coverage_scope === 'full_engine_registry') {
      ok = missing.length === 0 && extra.length === 0 && registryEngineIds.length > 0;
      note = 'Must cover every engine registered in agent_runtime_engine_registry.json.';
    } else if (gate.coverage_scope && gate.coverage_scope.endsWith('_subset')) {
      ok = engineIds.length > 0 && extra.length === 0;
      note = 'Scoped subset proof: partial coverage is allowed only because this guard declares its subset scope.';
    } else {
      ok = true;
      note = 'Single-route or non-matrix proof.';
    }
    const row = {
      id: gate.id,
      scope: gate.coverage_scope,
      ok,
      engine_count: engineIds.length,
      engine_ids: engineIds,
      missing_registry_engine_ids: gate.coverage_scope === 'full_engine_registry' ? missing : [],
      extra_engine_ids: extra,
      note,
    };
    checks.push(row);
    if (!ok) failures.push(row);
  }
  return {
    ok: failures.length === 0,
    registry_engine_count: registryEngineIds.length,
    registry_engine_ids: registryEngineIds,
    checks,
    failures,
  };
}

function main() {
  const startedAt = Date.now();
  const gateResults = GATES.map(runGate);
  const coverage = buildCoverageReport(gateResults);
  const failures = gateResults.filter((gate) => !gate.ok);
  const warningSummary = warningRollup(gateResults);
  const allFailures = [
    ...failures.map((gate) => ({ kind: 'gate_failed', id: gate.id, gate })),
    ...coverage.failures.map((row) => ({ kind: 'coverage_failed', id: row.id, coverage: row })),
  ];
  const report = {
    ok: allFailures.length === 0,
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    kind: 'agent_runtime_framework_coordination_guard',
    version: 1,
    policy: {
      purpose:
        'Roll up the external-framework coordination proof surface without creating a second source of truth.',
      shell_cognition_policy:
        'No dashboard cognition: this guard only invokes Gateway/Validation proof scripts.',
      timeout_ms_per_gate: GATE_TIMEOUT_MS,
    },
    summary: {
      gate_count: gateResults.length,
      pass_count: gateResults.length - failures.length,
      failure_count: allFailures.length,
      warning_count: warningSummary.unique_warning_count,
      unique_warning_count: warningSummary.unique_warning_count,
      raw_warning_count: warningSummary.raw_warning_count,
      duration_ms: Date.now() - startedAt,
    },
    coverage,
    warnings: warningSummary.warnings,
    gates: gateResults,
    failures: allFailures.map((failure) => {
      if (failure.kind === 'coverage_failed') return failure;
      const gate = failure.gate;
      return {
        kind: failure.kind,
        id: gate.id,
        script: gate.script,
        status: gate.status,
        signal: gate.signal,
        timed_out: gate.timed_out,
        stderr_preview: gate.stderr_preview,
        stdout_preview: gate.stdout_preview,
      };
    }),
  };

  fs.mkdirSync(path.dirname(ARTIFACT_PATH), { recursive: true });
  fs.writeFileSync(ARTIFACT_PATH, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (!report.ok) {
    process.exitCode = 1;
  }
}

main();
