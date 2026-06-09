#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;
type CapabilityStatus = 'pass' | 'partial' | 'not_sampled' | 'fail' | 'not_applicable';

const ROOT = process.cwd();
const CONTRACT_PATH = 'validation/conformance/contracts/agent_runtime_engine_scorecard_contract.json';
const OUT_JSON = 'core/local/artifacts/agent_runtime_engine_scorecard_current.json';

function readJson(rel: string, fallback: JsonObject = {}): JsonObject {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
  } catch {
    return fallback;
  }
}

function exists(rel: string): boolean {
  return fs.existsSync(path.join(ROOT, rel));
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function clean(value: any, max = 240): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function capability(status: CapabilityStatus, evidence: string, score?: number) {
  const value = score == null
    ? status === 'pass' || status === 'not_applicable'
      ? 1
      : status === 'partial' || status === 'not_sampled'
        ? 0.5
        : 0
    : score;
  return { status, score: value, evidence: clean(evidence, 500) };
}

function hasModelMetadata(engine: JsonObject): boolean {
  if (engine.model_catalog || engine.model_catalog_policy || engine.modelDiscovery || engine.model_discovery) return true;
  const discovery = engine.discovery && typeof engine.discovery === 'object' ? engine.discovery : {};
  if (discovery.model_catalog || discovery.model_command || discovery.model_discovery || discovery.freshness_authority) return true;
  return JSON.stringify(engine).toLowerCase().includes('model');
}

function continuityResult(contextEval: JsonObject, engineId: string): JsonObject | null {
  const rows = Array.isArray(contextEval.results) ? contextEval.results : [];
  return rows.find((row: JsonObject) => row && row.engine_id === engineId) || null;
}

function liveWorkApplies(liveWork: JsonObject, engineId: string): boolean {
  if (liveWorkResult(liveWork, engineId)) return true;
  return clean(liveWork.engine_id, 120) === engineId && liveWork.type === 'agent_runtime_live_work_eval';
}

function liveWorkResult(liveWork: JsonObject, engineId: string): JsonObject | null {
  if (liveWork.type !== 'agent_runtime_live_work_eval') return null;
  const rows = Array.isArray(liveWork.engine_results) ? liveWork.engine_results : [];
  const row = rows.find((item: JsonObject) => item && item.engine_id === engineId);
  if (row) return row;
  if (clean(liveWork.engine_id, 120) === engineId) return liveWork;
  return null;
}

function realWorkReplayResult(realWorkReplay: JsonObject, engineId: string): JsonObject | null {
  if (realWorkReplay.type !== 'agent_runtime_real_work_replay_guard') return null;
  const rows = Array.isArray(realWorkReplay.results) ? realWorkReplay.results : [];
  return rows.find((item: JsonObject) => item && item.engine_id === engineId) || null;
}

function classify(score: number): string {
  if (score >= 0.85) return 'daily_driver_candidate';
  if (score >= 0.7) return 'practical_with_gaps';
  if (score >= 0.5) return 'integration_incomplete';
  return 'not_ready';
}

function boundedClassification(score: number, livePromotionEligible: boolean): string {
  const raw = classify(score);
  if (raw === 'daily_driver_candidate' && !livePromotionEligible) return 'practical_with_gaps';
  return raw;
}

function nextActions(engineId: string, caps: Record<string, ReturnType<typeof capability>>): string[] {
  const out: string[] = [];
  if (caps.live_adapter_evidence && caps.live_adapter_evidence.status !== 'pass') {
    out.push('Attach or sample a live selectable adapter before treating this engine as a daily-driver candidate.');
  }
  if (caps.context_continuity.status !== 'pass') out.push('Run or fix context continuity eval for this engine.');
  if (caps.live_work_completion.status !== 'pass') {
    if (caps.real_work_replay && caps.real_work_replay.status === 'partial') {
      out.push('Attach a live adapter before expecting useful work completion for this planned engine.');
    } else {
      out.push('Run live work eval through Gateway for this engine.');
    }
  }
  if (caps.real_work_replay && caps.real_work_replay.status === 'not_sampled') out.push('Add this engine to the cross-framework real-work replay proof.');
  if (caps.real_work_replay && caps.real_work_replay.status === 'fail') out.push('Fix the cross-framework real-work replay failure for this engine.');
  if (caps.approval_pause.status !== 'pass') out.push('Verify gated tool proposal pauses and resumes through Gateway approval route.');
  if (caps.durable_receipts.status !== 'pass') out.push('Ensure terminal projections include Gateway receipt refs.');
  if (caps.activity_trace.status !== 'pass') out.push('Normalize activity into bounded user-facing trace rows.');
  if (caps.structured_transport.status !== 'pass') out.push('Attach and validate Gateway-owned structured turn payloads before adapter dispatch.');
  if (caps.error_projection.status !== 'pass' && caps.error_projection.status !== 'not_applicable') out.push('Add or refresh hard-failure projection evidence.');
  if (!out.length && engineId !== 'infring_native') out.push('Promote this engine to broader live useful-work scenarios.');
  if (!out.length) out.push('Keep monitoring parity against external engines.');
  return out.slice(0, 5);
}

function main() {
  const contract = readJson(CONTRACT_PATH);
  const registryPath = clean(contract.engine_registry || 'validation/conformance/contracts/agent_runtime_engine_registry.json', 300);
  const evidenceInputs = contract.evidence_inputs || {};
  const registry = readJson(registryPath);
  const conformance = readJson(clean(evidenceInputs.conformance || 'core/local/artifacts/agent_runtime_engine_conformance_guard_current.json', 300));
  const contextEval = readJson(clean(evidenceInputs.context_continuity || 'core/local/artifacts/agent_runtime_context_continuity_eval_current.json', 300));
  const liveWork = readJson(clean(evidenceInputs.live_work || 'core/local/artifacts/agent_runtime_live_work_eval_current.json', 300));
  const realWorkReplay = readJson(clean(evidenceInputs.real_work_replay || 'core/local/artifacts/agent_runtime_real_work_replay_guard_current.json', 300));
  const structuredTransport = readJson(clean(evidenceInputs.structured_transport || 'core/local/artifacts/agent_runtime_structured_transport_eval_current.json', 300));
  const hardFailure = readJson(clean(evidenceInputs.hard_failure_injection || 'core/local/artifacts/agent_runtime_hard_failure_injection_eval_current.json', 300));
  const engines = Array.isArray(registry.engines) ? registry.engines : [];
  const adapterContracts = readJson(clean(registry.private_adapter_contracts || 'validation/conformance/contracts/agent_runtime_adapter_contracts.json', 300));
  const adapterRows = Array.isArray(adapterContracts.adapters) ? adapterContracts.adapters : [];
  const conformanceOk = conformance.ok === true;
  const sampledLiveWorkEngineSet = new Set(
    (Array.isArray(liveWork.sampled_engines) ? liveWork.sampled_engines : [liveWork.engine_id])
      .map((item: any) => clean(item, 120))
      .filter(Boolean),
  );
  const liveSelectableEngineSet = new Set(
    (Array.isArray(liveWork.live_selectable_engines) ? liveWork.live_selectable_engines : [])
      .map((item: any) => clean(item, 120))
      .filter(Boolean),
  );
  const catalogOnlyEngineSet = new Set(
    (Array.isArray(registry.validation_focus_policy && registry.validation_focus_policy.catalog_only_engines)
      ? registry.validation_focus_policy.catalog_only_engines
      : [])
      .map((item: any) => clean(item, 120))
      .filter(Boolean),
  );

  const rows = engines.map((engine: JsonObject) => {
    const engineId = clean(engine.engine_id, 120);
    const adapterContract = adapterRows.find((row: JsonObject) => row && row.engine_id === engineId) || null;
    const continuity = continuityResult(contextEval, engineId);
    const replayRow = realWorkReplayResult(realWorkReplay, engineId);
    const replayApplies = Boolean(replayRow);
    const liveApplies = liveWorkApplies(liveWork, engineId) || replayApplies;
    const liveRow = liveWorkResult(liveWork, engineId);
    const liveResults = liveRow && liveRow.results || {};
    const replayExpectedUnavailable = replayApplies && replayRow?.expected_unavailable === true;
    const replayCompletionOk = replayApplies && replayRow?.ok === true && replayRow?.artifact_quality?.ok === true;
    const replayApprovalOk = replayApplies && replayRow?.turn?.pending_permission === true && replayRow?.permission_request?.present === true && replayRow?.decision?.ok === true;
    const replayReceiptsOk = replayApplies && replayRow?.decision?.decision_receipt_hash_present === true;
    const replayActivityOk = replayApplies && replayRow?.turn?.activity_trace === true;
    const sampledLiveWork = sampledLiveWorkEngineSet.has(engineId) || Boolean(liveRow && !replayRow);
    const liveSelectable = liveSelectableEngineSet.has(engineId);
    const catalogOnly = catalogOnlyEngineSet.has(engineId);
    const statusText = clean(engine.status, 120);
    const plannedAdapter = statusText.includes('planned_adapter');
    const liveAdapterEvidenceOk = engineId === 'infring_native' || (sampledLiveWork && liveSelectable && liveRow && liveRow.classification !== 'expected_planned_adapter_unavailable');
    const liveCompletionOk = (liveResults.completion && liveResults.completion.ok === true) || replayCompletionOk;
    const liveApprovalOk = (liveResults.approval_pause && liveResults.approval_pause.ok === true && liveResults.approval_decision && liveResults.approval_decision.ok === true) || replayApprovalOk;
    const liveReceiptsOk = Number(liveResults.completion && liveResults.completion.receipt_refs || 0) >= 3 || replayReceiptsOk;
    const liveActivityOk = (liveResults.completion && liveResults.completion.activity_trace === true) || replayActivityOk;
    const structuredTransportOk = structuredTransport.ok === true && structuredTransport.type === 'agent_runtime_structured_transport_eval';
    const hardFailureOk = hardFailure.ok === true && hardFailure.type === 'agent_runtime_hard_failure_injection_eval';
    const external = engineId !== 'infring_native';
    const install = engine.install && typeof engine.install === 'object' ? engine.install : {};
    const caps = {
      registered: capability(engineId ? 'pass' : 'fail', engineId ? 'Engine exists in registry.' : 'Missing engine_id.'),
      adapter_contract: capability(adapterContract || engine.implementation_path ? 'pass' : 'fail', adapterContract ? 'Private adapter contract row exists.' : clean(engine.implementation_path ? 'Implementation path declared.' : 'No adapter contract or implementation path.', 500)),
      discovery_metadata: capability(engine.discovery || !external ? 'pass' : 'fail', engine.discovery ? 'Discovery metadata declared.' : 'Native engine does not require external discovery metadata.'),
      model_catalog_metadata: capability(hasModelMetadata(engine) || engineId === 'infring_native' ? 'pass' : 'partial', hasModelMetadata(engine) ? 'Model discovery/catalog metadata present.' : 'Model metadata not explicit enough.'),
      context_continuity: capability(continuity && continuity.ok === true ? 'pass' : 'not_sampled', continuity ? clean(continuity.output_preview || 'Continuity eval row present.', 500) : 'No context continuity evidence row for this engine.'),
      live_adapter_evidence: capability(
        liveAdapterEvidenceOk ? 'pass' : catalogOnly || plannedAdapter ? 'partial' : 'not_sampled',
        liveAdapterEvidenceOk
          ? 'Engine is live-selectable and sampled through live adapter evidence.'
          : catalogOnly
            ? 'Engine is catalog-only and must not be promoted as a daily-driver candidate without live adapter evidence.'
            : plannedAdapter
              ? 'Engine is planned and must attach a live adapter before daily-driver promotion.'
              : 'Engine lacks live selectable adapter evidence.',
      ),
      live_work_completion: capability(liveCompletionOk ? 'pass' : liveApplies ? 'fail' : 'not_sampled', liveApplies ? `Latest live work eval targeted ${engineId}.` : 'Latest live work eval did not target this engine.'),
      real_work_replay: capability(
        replayRow && replayRow.ok === true ? 'pass' : replayExpectedUnavailable ? 'partial' : replayRow ? 'fail' : 'not_sampled',
        replayRow && replayRow.ok === true
          ? 'Cross-framework real-work replay completed for this engine.'
          : replayExpectedUnavailable
            ? 'Cross-framework real-work replay sampled this planned engine and confirmed no live adapter is attached yet.'
            : replayRow
              ? 'Cross-framework real-work replay sampled this engine but failed.'
              : 'Cross-framework real-work replay has not sampled this engine.',
      ),
      approval_pause: capability(liveApprovalOk ? 'pass' : liveApplies ? 'fail' : 'not_sampled', liveApplies ? 'Latest live work eval included approval pause and decision.' : 'Approval pause not sampled for this engine.'),
      durable_receipts: capability(liveReceiptsOk ? 'pass' : liveApplies ? 'fail' : 'partial', liveReceiptsOk ? 'Latest live work eval returned receipt refs.' : 'Receipt evidence comes from contract/conformance, not live engine sample.'),
      activity_trace: capability(liveActivityOk ? 'pass' : liveApplies ? 'fail' : 'partial', liveActivityOk ? 'Latest live work eval returned bounded activity trace.' : 'Activity trace evidence comes from contract/conformance, not live engine sample.'),
      structured_transport: capability(
        structuredTransportOk ? 'pass' : conformanceOk ? 'partial' : 'fail',
        structuredTransportOk
          ? `Structured transport eval passed with ref ${clean(structuredTransport.structured_turn_ref, 160)}.`
          : conformanceOk
            ? 'Conformance declares structured transport target; structured transport eval evidence is missing or stale.'
            : 'Conformance guard failed or missing.',
      ),
      error_projection: capability(
        hardFailureOk ? 'pass' : conformanceOk ? 'partial' : 'fail',
        hardFailureOk
          ? `Hard-failure injection eval passed ${Number(hardFailure.summary && hardFailure.summary.passed) || 0} scenarios.`
          : conformanceOk
            ? 'Conformance requires hard-failure classes; hard-failure injection evidence is missing or stale.'
            : 'Conformance guard failed or missing.',
      ),
    };
    if (external && !install.download_action_ref && caps.discovery_metadata.status === 'pass') {
      caps.discovery_metadata = capability('partial', 'Discovery exists but install/download action metadata is incomplete.');
    }
    const values = Object.values(caps);
    const rawScore = values.reduce((sum, row) => sum + row.score, 0) / Math.max(1, values.length);
    const plannedAdapterCap = statusText.includes('planned_adapter') ? 0.79 : 1;
    const liveEvidenceCap = liveAdapterEvidenceOk || !external ? 1 : 0.84;
    const catalogOnlyCap = catalogOnly ? 0.84 : 1;
    const score = Math.min(rawScore, plannedAdapterCap, liveEvidenceCap, catalogOnlyCap);
    const livePromotionEligible = liveAdapterEvidenceOk && !catalogOnly && !plannedAdapter;
    return {
      engine_id: engineId,
      display_name: clean(engine.display_name || engineId, 120),
      engine_kind: clean(engine.engine_kind, 120),
      status: statusText,
      score: Number(score.toFixed(3)),
      classification: boundedClassification(score, livePromotionEligible),
      score_adjustments: {
        raw_score: Number(rawScore.toFixed(3)),
        planned_adapter_cap: plannedAdapterCap < 1 ? plannedAdapterCap : null,
        live_evidence_cap: liveEvidenceCap < 1 ? liveEvidenceCap : null,
        catalog_only_cap: catalogOnlyCap < 1 ? catalogOnlyCap : null,
        live_promotion_eligible: livePromotionEligible,
      },
      live_work_evidence: liveRow ? {
        working_directory: clean(liveRow.working_directory || liveWork.working_directory, 500),
        observed_working_directory: clean(liveRow.observed_working_directory, 500),
        working_directory_observation_source: clean(liveRow.working_directory_observation_source, 120),
        classification: clean(liveRow.classification, 160),
      } : replayRow ? {
        working_directory: clean(replayRow.working_directory || realWorkReplay.working_directory || 'gateway_real_work_replay_scratch', 500),
        observed_working_directory: clean(replayRow.observed_working_directory || replayRow.artifact_rel_path, 500),
        working_directory_observation_source: clean(replayRow.working_directory_observation_source || 'agent_runtime_real_work_replay_guard', 120),
        classification: clean(replayRow.ok ? 'real_work_replay_ok' : replayRow.expected_unavailable ? 'expected_planned_adapter_unavailable' : 'real_work_replay_failed', 160),
      } : null,
      capabilities: caps,
      next_actions: nextActions(engineId, caps),
    };
  });

  const summary = {
    engine_count: rows.length,
    daily_driver_candidates: rows.filter((row) => row.classification === 'daily_driver_candidate').length,
    practical_with_gaps: rows.filter((row) => row.classification === 'practical_with_gaps').length,
    integration_incomplete: rows.filter((row) => row.classification === 'integration_incomplete').length,
    not_ready: rows.filter((row) => row.classification === 'not_ready').length,
    latest_live_work_engine: clean(liveWork.engine_id, 120),
    sampled_live_work_engines: Array.isArray(liveWork.sampled_engines) ? liveWork.sampled_engines.map((item: any) => clean(item, 120)).filter(Boolean) : [clean(liveWork.engine_id, 120)].filter(Boolean),
    live_work_registry_engine_count: Number(liveWork.registry_engine_count) || 0,
    live_work_live_selectable_engines: Array.isArray(liveWork.live_selectable_engines) ? liveWork.live_selectable_engines.map((item: any) => clean(item, 120)).filter(Boolean) : [],
    live_work_unsampled_registry_engines: Array.isArray(liveWork.unsampled_registry_engines) ? liveWork.unsampled_registry_engines.map((item: any) => clean(item, 120)).filter(Boolean) : [],
    live_work_unsampled_live_selectable_engines: Array.isArray(liveWork.unsampled_live_selectable_engines) ? liveWork.unsampled_live_selectable_engines.map((item: any) => clean(item, 120)).filter(Boolean) : [],
    sampled_real_work_replay_engines: Array.isArray(realWorkReplay.engines_tested) ? realWorkReplay.engines_tested.map((item: any) => clean(item, 120)).filter(Boolean) : [],
    live_work_working_directory: clean(liveWork.working_directory, 500),
    sampled_live_work_working_directories: Array.from(new Set(
      (Array.isArray(liveWork.engine_results) ? liveWork.engine_results : [liveWork])
        .map((row: JsonObject) => clean(row && (row.observed_working_directory || row.working_directory || liveWork.working_directory), 500))
        .filter(Boolean),
    )),
    hard_failure_injection_ok: hardFailure.ok === true,
    structured_transport_eval_ok: structuredTransport.ok === true,
  };
  const report = {
    ok: rows.length > 0 && rows.every((row) => row.engine_id && row.score >= 0),
    type: 'agent_runtime_engine_scorecard',
    generated_at: new Date().toISOString(),
    contract: CONTRACT_PATH,
    evidence_inputs: evidenceInputs,
    summary,
    rows,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
