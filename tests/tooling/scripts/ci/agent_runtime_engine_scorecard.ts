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
  return clean(liveWork.engine_id, 120) === engineId && liveWork.type === 'agent_runtime_live_work_eval';
}

function classify(score: number): string {
  if (score >= 0.85) return 'daily_driver_candidate';
  if (score >= 0.7) return 'practical_with_gaps';
  if (score >= 0.5) return 'integration_incomplete';
  return 'not_ready';
}

function nextActions(engineId: string, caps: Record<string, ReturnType<typeof capability>>): string[] {
  const out: string[] = [];
  if (caps.context_continuity.status !== 'pass') out.push('Run or fix context continuity eval for this engine.');
  if (caps.live_work_completion.status !== 'pass') out.push('Run live work eval through Gateway for this engine.');
  if (caps.approval_pause.status !== 'pass') out.push('Verify gated tool proposal pauses and resumes through Gateway approval route.');
  if (caps.durable_receipts.status !== 'pass') out.push('Ensure terminal projections include Gateway receipt refs.');
  if (caps.activity_trace.status !== 'pass') out.push('Normalize activity into bounded user-facing trace rows.');
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
  const engines = Array.isArray(registry.engines) ? registry.engines : [];
  const adapterContracts = readJson(clean(registry.private_adapter_contracts || 'validation/conformance/contracts/agent_runtime_adapter_contracts.json', 300));
  const adapterRows = Array.isArray(adapterContracts.adapters) ? adapterContracts.adapters : [];
  const conformanceOk = conformance.ok === true;

  const rows = engines.map((engine: JsonObject) => {
    const engineId = clean(engine.engine_id, 120);
    const adapterContract = adapterRows.find((row: JsonObject) => row && row.engine_id === engineId) || null;
    const continuity = continuityResult(contextEval, engineId);
    const liveApplies = liveWorkApplies(liveWork, engineId);
    const liveResults = liveWork.results || {};
    const liveCompletionOk = liveApplies && liveResults.completion && liveResults.completion.ok === true;
    const liveApprovalOk = liveApplies && liveResults.approval_pause && liveResults.approval_pause.ok === true && liveResults.approval_decision && liveResults.approval_decision.ok === true;
    const liveReceiptsOk = liveApplies && Number(liveResults.completion && liveResults.completion.receipt_refs || 0) >= 3;
    const liveActivityOk = liveApplies && liveResults.completion && liveResults.completion.activity_trace === true;
    const external = engineId !== 'infring_native';
    const install = engine.install && typeof engine.install === 'object' ? engine.install : {};
    const caps = {
      registered: capability(engineId ? 'pass' : 'fail', engineId ? 'Engine exists in registry.' : 'Missing engine_id.'),
      adapter_contract: capability(adapterContract || engine.implementation_path ? 'pass' : 'fail', adapterContract ? 'Private adapter contract row exists.' : clean(engine.implementation_path ? 'Implementation path declared.' : 'No adapter contract or implementation path.', 500)),
      discovery_metadata: capability(engine.discovery || !external ? 'pass' : 'fail', engine.discovery ? 'Discovery metadata declared.' : 'Native engine does not require external discovery metadata.'),
      model_catalog_metadata: capability(hasModelMetadata(engine) || engineId === 'infring_native' ? 'pass' : 'partial', hasModelMetadata(engine) ? 'Model discovery/catalog metadata present.' : 'Model metadata not explicit enough.'),
      context_continuity: capability(continuity && continuity.ok === true ? 'pass' : 'not_sampled', continuity ? clean(continuity.output_preview || 'Continuity eval row present.', 500) : 'No context continuity evidence row for this engine.'),
      live_work_completion: capability(liveCompletionOk ? 'pass' : liveApplies ? 'fail' : 'not_sampled', liveApplies ? `Latest live work eval targeted ${engineId}.` : 'Latest live work eval did not target this engine.'),
      approval_pause: capability(liveApprovalOk ? 'pass' : liveApplies ? 'fail' : 'not_sampled', liveApplies ? 'Latest live work eval included approval pause and decision.' : 'Approval pause not sampled for this engine.'),
      durable_receipts: capability(liveReceiptsOk ? 'pass' : liveApplies ? 'fail' : 'partial', liveReceiptsOk ? 'Latest live work eval returned receipt refs.' : 'Receipt evidence comes from contract/conformance, not live engine sample.'),
      activity_trace: capability(liveActivityOk ? 'pass' : liveApplies ? 'fail' : 'partial', liveActivityOk ? 'Latest live work eval returned bounded activity trace.' : 'Activity trace evidence comes from contract/conformance, not live engine sample.'),
      error_projection: capability(conformanceOk ? 'partial' : 'fail', conformanceOk ? 'Conformance requires hard-failure classes; live hard-failure sample may be separate.' : 'Conformance guard failed or missing.'),
    };
    if (external && !install.download_action_ref && caps.discovery_metadata.status === 'pass') {
      caps.discovery_metadata = capability('partial', 'Discovery exists but install/download action metadata is incomplete.');
    }
    const values = Object.values(caps);
    const score = values.reduce((sum, row) => sum + row.score, 0) / Math.max(1, values.length);
    return {
      engine_id: engineId,
      display_name: clean(engine.display_name || engineId, 120),
      engine_kind: clean(engine.engine_kind, 120),
      status: clean(engine.status, 120),
      score: Number(score.toFixed(3)),
      classification: classify(score),
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
