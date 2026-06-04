#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::shell-socket-eval-issue-ingress.
//
// Gateway-only ingress for Shell-originated eval/issue submission. The Shell may
// submit selected refs and bounded summaries; it must not upload raw transcript,
// raw trace, or raw runtime context bodies through this path.

'use strict';

const RAW_CONTEXT_FIELDS = [
  'raw',
  'raw_context',
  'raw_messages',
  'raw_transcript',
  'conversation',
  'conversation_tree',
  'messages',
  'trace_body',
  'workflow_graph',
  'plan_graph',
  'execution_observation',
  'tool_payload',
  'raw_tool_input',
  'raw_tool_result',
];

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function cleanId(value, max = 180) {
  return cleanString(value, max).replace(/[^A-Za-z0-9_.:/@#-]+/g, '_').replace(/^_+|_+$/g, '');
}

function cleanStringArray(value, maxItems = 24, maxLen = 300) {
  const rows = Array.isArray(value) ? value : (value == null ? [] : [value]);
  const out = [];
  for (const row of rows) {
    const clean = cleanString(row, maxLen);
    if (clean) out.push(clean);
    if (out.length >= maxItems) break;
  }
  return out;
}

function isShellSocketEvalIssueIngressPath(pathname) {
  return String(pathname || '') === '/api/shell-socket/issues';
}

function boundedIssueBody(body, traceId) {
  const source = body && typeof body === 'object' && !Array.isArray(body) ? body : {};
  const blocked = RAW_CONTEXT_FIELDS.filter((field) => Object.prototype.hasOwnProperty.call(source, field));
  const refs = {
    evidence_refs: cleanStringArray(source.evidence_refs || source.evidence_ref, 32, 360),
    detail_refs: cleanStringArray(source.detail_refs || source.detail_ref, 32, 360),
    trace_refs: cleanStringArray(source.trace_refs || source.trace_id || traceId, 16, 240),
    source_refs: cleanStringArray(source.source_refs || source.source_ref, 24, 360),
    artifact_refs: cleanStringArray(source.artifact_refs || source.artifact_ref, 24, 360),
  };
  return {
    agent_id: cleanId(source.agent_id || source.target_agent_id, 120),
    title: cleanString(source.title || source.issue_title || source.summary, 180),
    summary: cleanString(source.summary || source.description || source.problem_statement, 1200),
    severity: cleanId(source.severity || 'needs_triage', 40),
    category: cleanId(source.category || source.issue_category || 'eval_feedback', 80),
    owner_guess: cleanString(source.owner_guess || source.owner || '', 120),
    root_cause_hypothesis: cleanString(source.root_cause_hypothesis || source.hypothesis || '', 800),
    next_action: cleanString(source.next_action || source.recommended_action || '', 800),
    recurrence: cleanString(source.recurrence || source.recurrence_signal || '', 240),
    freshness: cleanString(source.freshness || source.freshness_signal || '', 240),
    dedupe_key: cleanString(source.dedupe_key || source.issue_dedupe_key || '', 180),
    refs,
    blocked_raw_fields: blocked,
    gateway_ingress: {
      trace_id: cleanString(traceId, 180),
      route_id: 'shell_socket.submit_issue',
      capability_id: 'submit_issue',
      route_class: 'request_ingress',
      gateway_owner: 'gateway.runtime',
      authority_owner: 'assurance_eval_issue_owner',
      raw_context_upload_allowed: false,
      legacy_shell_policy_allowed: false,
    },
  };
}

function byteSize(value) {
  try { return Buffer.byteLength(JSON.stringify(value || {}), 'utf8'); } catch { return 65537; }
}

async function shellSocketEvalIssueIngress(options) {
  const traceId = cleanString(options && options.traceId, 180);
  const body = boundedIssueBody(options && options.body, traceId);
  if (!body.agent_id) {
    return {
      status: 400,
      payload: {
        ok: false,
        type: 'shell_socket_eval_issue_agent_id_required',
        trace_id: traceId,
        gateway_ingress: body.gateway_ingress,
      },
    };
  }
  if (byteSize(body) > 65536) {
    return {
      status: 413,
      payload: {
        ok: false,
        type: 'shell_socket_eval_issue_too_large',
        trace_id: traceId,
        max_request_bytes: 65536,
        gateway_ingress: body.gateway_ingress,
      },
    };
  }
  try {
    const upstream = await options.fetchBackend(options.flags, '/api/shell-socket/issues', {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-infring-trace-id': traceId,
      },
      body: JSON.stringify(body),
      cache: 'no-store',
    }, 120000);
    const text = await upstream.text();
    let payload = {};
    try { payload = text ? JSON.parse(text) : {}; } catch { payload = {}; }
    const base = payload && typeof payload === 'object' ? payload : {};
    return {
      status: upstream.status || (base.ok === false ? 400 : 202),
      payload: {
        ...base,
        trace_id: traceId,
        gateway_ingress: body.gateway_ingress,
        blocked_raw_fields: body.blocked_raw_fields,
      },
    };
  } catch (error) {
    return {
      status: 503,
      payload: {
        ok: false,
        type: 'shell_socket_eval_issue_ingress_unavailable',
        trace_id: traceId,
        error: cleanString(error && error.message ? error.message : error, 240),
        gateway_ingress: body.gateway_ingress,
        retry_after_ms: 1000,
      },
    };
  }
}

module.exports = {
  isShellSocketEvalIssueIngressPath,
  shellSocketEvalIssueIngress,
};
