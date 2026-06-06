#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_live_work_eval_current.json');

function argValue(name: string, fallback = ''): string {
  const prefix = `${name}=`;
  const found = process.argv.slice(2).find((arg) => arg === name || arg.startsWith(prefix));
  if (!found) return fallback;
  if (found === name) return '1';
  return found.slice(prefix.length);
}

function argList(name: string, fallback: string): string[] {
  return argValue(name, fallback)
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

function ensureDir(filePath: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function clean(value: any, max = 4000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function postJson(url: string, payload: JsonObject, timeoutMs = 180000): Promise<{ statusCode: number; parsed: JsonObject | null; raw: string; error?: string }> {
  return new Promise((resolve) => {
    const body = JSON.stringify(payload);
    const req = http.request(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body),
      },
      timeout: timeoutMs,
    }, (res) => {
      let raw = '';
      res.setEncoding('utf8');
      res.on('data', (chunk) => { raw += chunk; });
      res.on('end', () => {
        let parsed: JsonObject | null = null;
        try { parsed = JSON.parse(raw); } catch {}
        resolve({ statusCode: res.statusCode || 0, parsed, raw });
      });
    });
    req.on('timeout', () => req.destroy(new Error('request_timeout')));
    req.on('error', (error) => resolve({ statusCode: 0, parsed: null, raw: '', error: String(error && error.message || error) }));
    req.write(body);
    req.end();
  });
}

function outputText(row: JsonObject | null): string {
  const p = row || {};
  return String(p.output_text || p.text || p.display_text || p.message?.content || p.assistant_message?.content || '').trim();
}

function hasBoundedActivityTrace(row: JsonObject | null): boolean {
  const trace = row && row.activity_trace && typeof row.activity_trace === 'object' ? row.activity_trace : null;
  return !!(
    trace &&
    trace.type === 'agent_runtime_activity_trace_projection' &&
    trace.collapsed_by_default === true &&
    /^Worked for \d/.test(String(trace.collapse_label || '')) &&
    Array.isArray(trace.rows) &&
    trace.rows.length <= 48 &&
    trace.rows.every((activity: JsonObject) => activity && activity.title && !String(activity.title).trim().startsWith('{'))
  );
}

function hasReceipts(row: JsonObject | null): boolean {
  return Array.isArray(row && row.receipt_refs) && row!.receipt_refs.length >= 3;
}

function shellProjectionBounded(request: JsonObject | null): boolean {
  return !!(request && !(request.proposal_arguments && request.proposal_arguments.content));
}

async function runEngineEval(baseUrl: string, engineId: string, timeoutMs: number): Promise<JsonObject> {
  const endpoint = `${baseUrl.replace(/\/+$/, '')}/api/shell-socket/agent-runtime/turn`;
  const token = `live-work-${engineId}-${Date.now().toString(36)}`;
  const sessionId = `live-work-eval-${engineId}-${Date.now().toString(36)}`;
  const agentId = 'agent-runtime-live-work-eval';
  const workDir = ROOT;
  const artifactRel = `tmp/agent-runtime-live-work-eval-${engineId}-${token}.txt`;
  const artifactAbs = path.resolve(ROOT, artifactRel);
  try { fs.rmSync(artifactAbs, { force: true }); } catch {}

  const completionPrompt = `Side-effect-free live work eval for ${engineId}. Reply exactly LIVE_WORK_OK ${token}`;
  const completion = await postJson(endpoint, {
    agent_id: agentId,
    session_id: sessionId,
    conversation_id: sessionId,
    engine_id: engineId,
    message: completionPrompt,
    input_text: completionPrompt,
    working_directory: workDir,
    test_probe: true,
  }, timeoutMs);
  const completionPayload = completion.parsed || {};
  const completionOk =
    completion.statusCode === 200 &&
    completionPayload.status === 'completed' &&
    outputText(completionPayload).includes(token) &&
    hasBoundedActivityTrace(completionPayload) &&
    hasReceipts(completionPayload);

  const proposalJson = JSON.stringify({
    type: 'infring_universal_tool_proposal',
    tool_id: 'artifact.create_propose',
    reason: `Create live work eval artifact ${artifactRel}`,
    arguments: {
      path: artifactRel,
      mime_type: 'text/plain',
      content: `InfRing live work eval ${token}\n`,
    },
  });
  const approvalPrompt = [
    `InfRing approval work eval for ${engineId}.`,
    'Do not use native filesystem tools and do not create files directly.',
    'Output exactly this JSON object as plain text with no markdown fence and no extra explanation:',
    proposalJson,
  ].join('\n');
  const approval = await postJson(endpoint, {
    agent_id: agentId,
    session_id: sessionId,
    conversation_id: sessionId,
    engine_id: engineId,
    message: approvalPrompt,
    input_text: approvalPrompt,
    working_directory: workDir,
    test_probe: true,
  }, timeoutMs);
  const approvalPayload = approval.parsed || {};
  const request = approvalPayload.pending_permission_request || approvalPayload.permission_request || null;
  const approvalProjectionOk =
    approval.statusCode === 200 &&
    approvalPayload.status === 'permission_required' &&
    approvalPayload.approval_pause &&
    request &&
    request.approval_id &&
    shellProjectionBounded(request) &&
    hasBoundedActivityTrace(approvalPayload) &&
    hasReceipts(approvalPayload);

  let decisionPayload: JsonObject | null = null;
  let decisionOk = false;
  if (approvalProjectionOk) {
    const decisionUrl = `${baseUrl.replace(/\/+$/, '')}${request.approval_route}`;
    const decision = await postJson(decisionUrl, { decision: 'allow_once' }, 60000);
    decisionPayload = decision.parsed || {};
    decisionOk =
      decision.statusCode === 200 &&
      decisionPayload.ok === true &&
      decisionPayload.pending_request_found === true &&
      decisionPayload.durable_effect_executed === true &&
      fs.existsSync(artifactAbs) &&
      fs.readFileSync(artifactAbs, 'utf8').includes(token);
  }
  try { fs.rmSync(artifactAbs, { force: true }); } catch {}

  return {
    ok: completionOk && approvalProjectionOk && decisionOk,
    engine_id: engineId,
    session_id: sessionId,
    token,
    results: {
      completion: {
        ok: completionOk,
        status_code: completion.statusCode,
        status: completionPayload.status || '',
        error_code: completionPayload.error_code || '',
        reason: clean(completionPayload.reason || completion.error || '', 1000),
        activity_trace: hasBoundedActivityTrace(completionPayload),
        receipt_refs: Array.isArray(completionPayload.receipt_refs) ? completionPayload.receipt_refs.length : 0,
      },
      approval_pause: {
        ok: approvalProjectionOk,
        status_code: approval.statusCode,
        status: approvalPayload.status || '',
        error_code: approvalPayload.error_code || '',
        reason: clean(approvalPayload.reason || approval.error || '', 1000),
        approval_id: request && request.approval_id || '',
        shell_projection_bounded: shellProjectionBounded(request),
        activity_trace: hasBoundedActivityTrace(approvalPayload),
        receipt_refs: Array.isArray(approvalPayload.receipt_refs) ? approvalPayload.receipt_refs.length : 0,
      },
      approval_decision: {
        ok: decisionOk,
        pending_request_found: decisionPayload && decisionPayload.pending_request_found === true,
        durable_effect_executed: decisionPayload && decisionPayload.durable_effect_executed === true,
        artifact_removed_after_probe: !fs.existsSync(artifactAbs),
      },
    },
  };
}

async function main() {
  const baseUrl = argValue('--base-url', process.env.INFRING_AGENT_RUNTIME_EVAL_BASE_URL || 'http://127.0.0.1:4173');
  const primaryEngine = argValue('--engine', process.env.INFRING_AGENT_RUNTIME_EVAL_ENGINE || 'codex_cli');
  const engines = argList('--engines', process.env.INFRING_AGENT_RUNTIME_EVAL_ENGINES || primaryEngine);
  const timeoutMs = Math.max(30000, Math.min(Number(argValue('--timeout-ms', process.env.INFRING_AGENT_RUNTIME_EVAL_TIMEOUT_MS || '180000')) || 180000, 300000));
  const engineResults = [];
  for (const engineId of engines) {
    engineResults.push(await runEngineEval(baseUrl, engineId, timeoutMs));
  }
  const primary = engineResults.find((row) => row.engine_id === primaryEngine) || engineResults[0] || null;
  const report = {
    ok: engineResults.length > 0 && engineResults.every((row) => row.ok),
    type: 'agent_runtime_live_work_eval',
    generated_at: new Date().toISOString(),
    base_url: baseUrl,
    engine_id: primary ? primary.engine_id : primaryEngine,
    session_id: primary ? primary.session_id : '',
    token: primary ? primary.token : '',
    engine_count: engineResults.length,
    engine_results: engineResults,
    sampled_engines: engineResults.map((row) => row.engine_id),
    summary: {
      passed: engineResults.filter((row) => row.ok).length,
      failed: engineResults.filter((row) => !row.ok).length,
      failed_engines: engineResults.filter((row) => !row.ok).map((row) => row.engine_id),
    },
    results: primary ? primary.results : {},
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  console.error(error && error.stack || error);
  process.exit(1);
});
