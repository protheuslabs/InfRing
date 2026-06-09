#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'core', 'local', 'artifacts');
const OUT_JSON = path.join(ARTIFACT_DIR, 'agent_runtime_approval_pause_resume_guard_current.json');
const FIXTURE_ROOT = path.join(ARTIFACT_DIR, 'agent_runtime_approval_pause_resume_guard_fixture');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const LAYER = 'gateway';

function readJson(relPath) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relPath), 'utf8'));
}

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function pushViolation(violations, kind, detail = {}) {
  violations.push({ kind, ...detail });
}

async function main() {
  const violations = [];
  fs.rmSync(FIXTURE_ROOT, { recursive: true, force: true });
  fs.mkdirSync(FIXTURE_ROOT, { recursive: true });

  const contract = readJson('validation/conformance/contracts/agent_runtime_turn_outcome_contract.json');
  const rule = contract.approval_pause_resume_rule || {};
  if (rule.pending_request_must_include_resume_token !== true) pushViolation(violations, 'contract_missing_pending_resume_token_rule');
  if (rule.pending_request_must_include_turn_status !== 'permission_required') pushViolation(violations, 'contract_missing_pending_turn_status_rule');
  if (rule.approval_pause_must_include_resume_token !== true) pushViolation(violations, 'contract_missing_pause_resume_token_rule');
  if (rule.decision_ack_must_include_decision_receipt !== true) pushViolation(violations, 'contract_missing_decision_receipt_rule');
  if (rule.decision_ack_must_include_resume_token !== true) pushViolation(violations, 'contract_missing_decision_resume_token_rule');

  const { createAgentRuntimeApprovalStore } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_approvals.ts'));
  const { createAgentRuntimeTurnProjectionStore } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts'));
  const { createAgentRuntimeReceiptStore } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_receipts.ts'));

  const approvalStore = createAgentRuntimeApprovalStore({ root: FIXTURE_ROOT });
  const receiptStore = createAgentRuntimeReceiptStore({
    root: FIXTURE_ROOT,
    receiptPath: path.join(FIXTURE_ROOT, 'receipts.jsonl'),
    maxReceipts: 50,
  });

  const storePending = approvalStore.recordAgentRuntimePendingApproval({
    approval_id: 'approval_guard_store',
    trace_id: 'trace-approval-store',
    request_id: 'request-approval-store',
    engine_id: 'codex_cli',
    session_id: 'session-approval-store',
    turn_id: 'turn-approval-store',
    tool_call_ref: 'tool-proposal/artifact.create_propose/trace-approval-store/turn-approval-store',
    tool_id: 'artifact.create_propose',
    capability: 'propose_artifact_create',
    reason: 'Create approval guard artifact.',
    proposal_arguments: {
      path: 'tmp/store-approved.txt',
      mime_type: 'text/plain',
      content: 'approval guard store artifact\n',
    },
  });
  if (!storePending || storePending.status !== 'paused_pending_approval') pushViolation(violations, 'store_pending_not_paused', { pending: storePending });
  if (storePending && storePending.turn_status !== 'permission_required') pushViolation(violations, 'store_pending_turn_status_missing', { pending: storePending });
  if (storePending && !storePending.resume_token) pushViolation(violations, 'store_pending_resume_token_missing', { pending: storePending });
  if (storePending && storePending.proposal_arguments !== undefined) pushViolation(violations, 'store_pending_leaks_proposal_arguments');
  if (storePending && !storePending.proposal_arguments_ref) pushViolation(violations, 'store_pending_arguments_ref_missing');

  const storeDecision = approvalStore.agentRuntimeApprovalDecisionProjection('trace-approval-store', 'approval_guard_store', {
    decision: 'allow_once',
  });
  const storeArtifact = path.join(FIXTURE_ROOT, 'tmp', 'store-approved.txt');
  if (!storeDecision || storeDecision.ok !== true) pushViolation(violations, 'store_decision_not_ok', { decision: storeDecision });
  if (storeDecision && storeDecision.pending_request_found !== true) pushViolation(violations, 'store_decision_missing_pending_request', { decision: storeDecision });
  if (storeDecision && !storeDecision.resume_token) pushViolation(violations, 'store_decision_resume_token_missing', { decision: storeDecision });
  if (storeDecision && storeDecision.resumed !== true) pushViolation(violations, 'store_decision_not_resumed', { decision: storeDecision });
  if (storeDecision && storeDecision.durable_effect_executed !== true) pushViolation(violations, 'store_decision_effect_not_executed', { decision: storeDecision });
  if (storeDecision && (!storeDecision.decision_receipt || storeDecision.decision_receipt.type !== 'agent_runtime_approval_decision_receipt')) {
    pushViolation(violations, 'store_decision_receipt_missing', { decision: storeDecision });
  }
  if (!fs.existsSync(storeArtifact) || !fs.readFileSync(storeArtifact, 'utf8').includes('approval guard store artifact')) {
    pushViolation(violations, 'store_decision_artifact_missing', { path: storeArtifact });
  }

  const turnProjection = createAgentRuntimeTurnProjectionStore({
    root: FIXTURE_ROOT,
    loadAgentRuntimeEngineRegistry: () => ({ engines: [{ engine_id: 'codex_cli' }] }),
    findAgentRuntimeEngine: () => ({ engine_id: 'codex_cli', status: 'available' }),
    loadAgentRuntimeWorkspace: () => ({
      workspace_dir: FIXTURE_ROOT,
      active_workspace: FIXTURE_ROOT,
      display_label: 'approval-fixture',
      git_root: '',
      git_root_label: '',
      permission_boundary: 'workspace',
    }),
    createRouter: () => ({
      healthCheck: async () => ({ status: 'available', discovery_source: 'fixture' }),
      startSession: async () => ({ status: 'started' }),
      streamTurn: async (_message, onActivity) => {
        const activity = {
          type: 'agent_activity_event',
          activity_kind: 'activity',
          provider_event_type: 'fixture.permission_required',
          status: 'paused',
          display_text: 'Fixture runtime reached a gated artifact proposal.',
        };
        if (typeof onActivity === 'function') onActivity(activity);
        return {
          type: 'turn.complete',
          status: 'completed',
          output_text: 'Runtime reached a gated artifact proposal.',
          permission_request: {
            type: 'permission.requested',
            approval_id: 'approval_guard_turn',
            trace_id: 'trace-approval-turn',
            request_id: 'request-approval-turn',
            engine_id: 'codex_cli',
            session_id: 'session-approval-turn',
            turn_id: 'turn-approval-turn',
            tool_call_ref: 'tool-proposal/artifact.create_propose/trace-approval-turn/turn-approval-turn',
            tool_id: 'artifact.create_propose',
            capability: 'propose_artifact_create',
            reason: 'Create approval guard turn artifact.',
            source: 'fixture_runtime',
            argument_keys: ['path', 'mime_type', 'content'],
            proposal_arguments: {
              path: 'tmp/turn-approved.txt',
              mime_type: 'text/plain',
              content: 'approval guard turn artifact\n',
            },
          },
          activity_events: [activity],
          activity_event_count: 1,
          structured_activity: true,
        };
      },
    }),
    sanitizeAgentRuntimeProposalArguments: approvalStore.sanitizeAgentRuntimeProposalArguments,
    recordAgentRuntimePendingApproval: approvalStore.recordAgentRuntimePendingApproval,
    mergeAgentRuntimeApprovalPermissionPolicy: approvalStore.mergeAgentRuntimeApprovalPermissionPolicy,
    recordAgentRuntimeTurnReceipts: receiptStore.recordAgentRuntimeTurnReceipts,
  });

  const projectedTurn = await turnProjection.agentRuntimeTurnProjection('trace-approval-turn', {
    engine_id: 'codex_cli',
    agent_id: 'approval-guard-agent',
    session_id: 'session-approval-turn',
    turn_id: 'turn-approval-turn',
    message: 'create an approval-guard artifact',
  });
  const pending = projectedTurn && projectedTurn.pending_permission_request;
  if (!projectedTurn || projectedTurn.status !== 'permission_required') pushViolation(violations, 'turn_projection_not_permission_required', { projectedTurn });
  if (!pending || pending.status !== 'paused_pending_approval') pushViolation(violations, 'turn_projection_pending_not_paused', { pending });
  if (pending && pending.turn_status !== 'permission_required') pushViolation(violations, 'turn_projection_pending_turn_status_missing', { pending });
  if (pending && !pending.resume_token) pushViolation(violations, 'turn_projection_pending_resume_token_missing', { pending });
  if (pending && pending.proposal_arguments !== undefined) pushViolation(violations, 'turn_projection_pending_leaks_proposal_arguments');
  if (!projectedTurn.approval_pause || projectedTurn.approval_pause.resume_token !== (pending && pending.resume_token)) {
    pushViolation(violations, 'turn_projection_approval_pause_resume_token_mismatch', {
      approval_pause: projectedTurn && projectedTurn.approval_pause,
      pending,
    });
  }
  if (!Array.isArray(projectedTurn.receipt_refs) || !projectedTurn.receipt_refs.some((ref) => String(ref).includes('/approval_pause/'))) {
    pushViolation(violations, 'turn_projection_approval_pause_receipt_missing', { receipt_refs: projectedTurn && projectedTurn.receipt_refs });
  }

  const turnDecision = approvalStore.agentRuntimeApprovalDecisionProjection('trace-approval-turn', 'approval_guard_turn', {
    decision: 'allow_once',
  });
  const turnArtifact = path.join(FIXTURE_ROOT, 'tmp', 'turn-approved.txt');
  if (!turnDecision || turnDecision.ok !== true || turnDecision.pending_request_found !== true) {
    pushViolation(violations, 'turn_decision_not_resolved_from_pending_request', { decision: turnDecision });
  }
  if (turnDecision && turnDecision.resume_token !== (pending && pending.resume_token)) {
    pushViolation(violations, 'turn_decision_resume_token_mismatch', { decision: turnDecision, pending });
  }
  if (turnDecision && (!turnDecision.decision_receipt_ref || !turnDecision.decision_receipt)) {
    pushViolation(violations, 'turn_decision_receipt_missing', { decision: turnDecision });
  }
  if (!fs.existsSync(turnArtifact) || !fs.readFileSync(turnArtifact, 'utf8').includes('approval guard turn artifact')) {
    pushViolation(violations, 'turn_decision_artifact_missing', { path: turnArtifact });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_approval_pause_resume_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    checked_contract: 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json',
    fixture_root: FIXTURE_ROOT,
    checks: {
      pending_request_bounded: !!(pending && pending.proposal_arguments === undefined && pending.proposal_arguments_ref),
      turn_projection_permission_required: projectedTurn && projectedTurn.status === 'permission_required',
      decision_receipt_present: !!(turnDecision && turnDecision.decision_receipt_ref && turnDecision.decision_receipt),
      durable_effect_executed: fs.existsSync(turnArtifact),
    },
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  console.error(error && error.stack || error);
  process.exit(1);
});
