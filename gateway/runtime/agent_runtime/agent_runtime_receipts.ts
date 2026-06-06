#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::receipts.
//
// Gateway records bounded receipt rows for runtime-engine turns. Receipts may
// reference context packs, selected engines/models, approval pauses, final
// responses, failures, and approved effects, but they must not embed raw
// provider transcripts or unbounded Shell payloads.

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

const DEFAULT_MAX_RECEIPTS = 2000;

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function cleanDisplayText(value, maxLen = 4000) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .trim()
    .slice(0, maxLen);
}
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanReceiptComponent(value, maxLen = 200) { return cleanText(value, maxLen).replace(/[^A-Za-z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, '') || 'unknown'; }

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function stableDigest(value) {
  return createHash('sha256').update(JSON.stringify(value || {})).digest('hex');
}

function appendBoundedJsonl(filePath, rows, maxRows) {
  ensureDir(path.dirname(filePath));
  let existing = [];
  try {
    existing = fs.readFileSync(filePath, 'utf8').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  } catch {}
  const additions = (Array.isArray(rows) ? rows : []).map((row) => JSON.stringify(row));
  fs.writeFileSync(filePath, `${existing.concat(additions).slice(-Math.max(1, maxRows || 1)).join('\n')}\n`, 'utf8');
}

function createAgentRuntimeReceiptStore(options = {}) {
  const root = path.resolve(options.root || process.cwd());
  const receiptPath = path.resolve(
    options.receiptPath ||
      path.join(root, 'local', 'state', 'observability', 'agent_runtime', 'agent_runtime_receipts.jsonl'),
  );
  const maxReceipts = Math.max(1, Number(options.maxReceipts) || DEFAULT_MAX_RECEIPTS);

  function receiptRef(kind, traceId, turnId) {
    return `receipt/agent-runtime/${cleanReceiptComponent(kind, 120)}/${cleanReceiptComponent(traceId, 200)}/${cleanReceiptComponent(turnId, 200)}`;
  }

  function buildReceipt(kind, input, payload) {
    const source = input && typeof input === 'object' ? input : {};
    const body = payload && typeof payload === 'object' ? payload : {};
    const traceId = cleanText(source.traceId, 200);
    const turnId = cleanText(source.turnId, 200);
    const row = {
      type: 'agent_runtime_receipt',
      schema_version: 1,
      receipt_kind: cleanText(kind, 120),
      receipt_ref: receiptRef(kind, traceId, turnId),
      trace_id: traceId,
      engine_id: cleanEngineId(source.engineId),
      agent_id: cleanText(source.agentId, 160),
      session_id: cleanText(source.sessionId, 200),
      turn_id: turnId,
      status: cleanText(source.status, 80),
      generated_at: nowIso(),
      source_authority: 'gateway.runtime.agent_runtime_receipts',
      payload: body,
    };
    return {
      ...row,
      receipt_hash: stableDigest(row),
    };
  }

  function recordAgentRuntimeTurnReceipts(input) {
    const source = input && typeof input === 'object' ? input : {};
    const status = cleanText(source.status, 80);
    const contextPack = source.contextPack && typeof source.contextPack === 'object' ? source.contextPack : {};
    const modelProviderContext = source.modelProviderContext && typeof source.modelProviderContext === 'object'
      ? source.modelProviderContext
      : {};
    const pendingPermissionRequest = source.pendingPermissionRequest && typeof source.pendingPermissionRequest === 'object'
      ? source.pendingPermissionRequest
      : null;
    const rows = [
      buildReceipt('engine_selection', source, {
        selected_engine_id: cleanEngineId(source.engineId),
        selected_model_ref: cleanText(modelProviderContext.selected_model_ref || modelProviderContext.model || source.model, 200),
        model_source_authority: cleanText(modelProviderContext.source_authority || modelProviderContext.source || '', 160),
      }),
      buildReceipt('context_pack', source, {
        context_pack_ref: `context-pack/${cleanReceiptComponent(source.traceId, 200)}/${cleanReceiptComponent(source.turnId, 200)}`,
        source_basis: cleanText(contextPack.source_basis, 160),
        source_authority: cleanText(contextPack.source_authority, 200),
        fragment_count: Array.isArray(contextPack.fragments) ? contextPack.fragments.length : Number(contextPack.fragment_count || 0) || 0,
        row_count: Number(contextPack.row_count || 0) || 0,
        universal_tool_count: contextPack.universal_tool_grants && Array.isArray(contextPack.universal_tool_grants.tools)
          ? contextPack.universal_tool_grants.tools.length
          : 0,
      }),
    ];
    if (pendingPermissionRequest) {
      rows.push(buildReceipt('approval_pause', source, {
        approval_id: cleanText(pendingPermissionRequest.approval_id, 260),
        tool_id: cleanText(pendingPermissionRequest.tool_id, 120),
        capability: cleanText(pendingPermissionRequest.capability, 160),
        gatekeeper_kind: cleanText(pendingPermissionRequest.gatekeeper_kind || 'user', 80) || 'user',
        decision_route: cleanText(pendingPermissionRequest.approval_route, 300),
        resume_strategy: cleanText(pendingPermissionRequest.resume_strategy || 'gateway_apply_approved_effect_or_retry_next_turn', 120),
      }));
    } else if (status === 'failed_with_reason' || status === 'timed_out_with_reason') {
      rows.push(buildReceipt('failure_classification', source, {
        error_code: cleanText(source.errorCode, 120),
        reason_preview: cleanDisplayText(source.reason, 1200),
        retryable: source.retryable === true,
        timed_out: source.timedOut === true,
        timeout_ms: Number(source.timeoutMs || 0) || 0,
      }));
    } else {
      rows.push(buildReceipt('final_response', source, {
        output_preview: cleanDisplayText(source.outputPreview || source.outputText, 1200),
        result_ref: cleanText(source.resultRef, 240),
        adapter_receipt_ref: cleanText(source.adapterReceiptRef, 240),
      }));
    }
    appendBoundedJsonl(receiptPath, rows, maxReceipts);
    return {
      type: 'agent_runtime_receipt_projection',
      receipt_path: receiptPath,
      receipt_refs: rows.map((row) => row.receipt_ref),
      receipt_count: rows.length,
      receipt_hashes: rows.map((row) => row.receipt_hash),
    };
  }

  return {
    receiptPath,
    recordAgentRuntimeTurnReceipts,
  };
}

module.exports = {
  createAgentRuntimeReceiptStore,
};
