#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-ws-bridge (dashboard websocket surface adapter).

let wsDependencyWarned = false;
function resolveWebSocketServerCtor() {
  try {
    const runtime = require('ws');
    if (runtime && typeof runtime.WebSocketServer === 'function') return runtime.WebSocketServer;
  } catch {}
  return null;
}

function createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson }) {
  const WebSocketServerCtor = resolveWebSocketServerCtor();
  if (!WebSocketServerCtor) {
    if (!wsDependencyWarned) {
      wsDependencyWarned = true;
      console.warn('[infring dashboard] ws module unavailable; disabling local agent websocket bridge and falling back to HTTP transport.');
    }
    return {
      ws_enabled: false,
      ws_error: 'ws_module_missing',
      tryHandle() { return false; },
    };
  }
  const wss = new WebSocketServerCtor({ noServer: true, clientTracking: false, perMessageDeflate: false });
  const route = /^\/api\/agents\/([^/]+)\/ws$/;
  const enc = (agentId) => encodeURIComponent(String(agentId || '').trim());
  const send = (ws, payload) => {
    try { if (ws && ws.readyState === 1) ws.send(JSON.stringify(payload)); } catch {}
  };
  const parseJson = (raw) => { try { return JSON.parse(raw); } catch { return null; } };
  const toNum = (value, fallback = 0) => Number.isFinite(Number(value)) ? Number(value) : fallback;
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, Math.max(0, Number(ms) || 0)));
  const CONTEXT_LIMIT_TRUNCATION_NOTICE = 'more characters truncated';
  const DEFAULT_MAX_LIVE_TOOL_RESULT_CHARS = 40000;
  const TOOL_TIMELINE_CONTRACT_VERSION = 2;
  const MIN_KEEP_CHARS = 2000;
  const MAX_TOOL_RESULT_CONTEXT_SHARE = 0.3;
  const MIDDLE_OMISSION_MARKER = '\n\n[... middle content omitted - showing head and tail ...]\n\n';
  const formatContextLimitTruncationNotice = (truncatedChars) =>
    `[... ${Math.max(1, Math.floor(Number(truncatedChars) || 0))} ${CONTEXT_LIMIT_TRUNCATION_NOTICE}]`;
  const hasImportantTail = (text) => {
    const tail = String(text || '').slice(-2000).toLowerCase();
    return (
      /\b(error|exception|failed|fatal|traceback|panic|stack trace|errno|exit code)\b/.test(tail) ||
      /\}\s*$/.test(String(text || '').trim()) ||
      /\b(total|summary|result|complete|finished|done)\b/.test(tail)
    );
  };
  const truncateToolResultText = (text, maxChars) => {
    const raw = String(text || '');
    if (!raw || raw.length <= maxChars) return raw;
    const defaultSuffix = formatContextLimitTruncationNotice(Math.max(1, raw.length - maxChars));
    const budget = Math.max(MIN_KEEP_CHARS, maxChars - defaultSuffix.length);
    if (hasImportantTail(raw) && budget > MIN_KEEP_CHARS * 2) {
      const tailBudget = Math.min(Math.floor(budget * 0.3), 4000);
      const headBudget = budget - tailBudget - MIDDLE_OMISSION_MARKER.length;
      if (headBudget > MIN_KEEP_CHARS) {
        var headCut = headBudget;
        var headNewline = raw.lastIndexOf('\n', headBudget);
        if (headNewline > headBudget * 0.8) headCut = headNewline;
        var tailStart = raw.length - tailBudget;
        var tailNewline = raw.indexOf('\n', tailStart);
        if (tailNewline !== -1 && tailNewline < tailStart + tailBudget * 0.2) tailStart = tailNewline + 1;
        var kept = raw.slice(0, headCut) + MIDDLE_OMISSION_MARKER + raw.slice(tailStart);
        return kept + formatContextLimitTruncationNotice(Math.max(1, raw.length - kept.length));
      }
    }
    var cutPoint = budget;
    var lastNewline = raw.lastIndexOf('\n', budget);
    if (lastNewline > budget * 0.8) cutPoint = lastNewline;
    var keptHead = raw.slice(0, cutPoint);
    return keptHead + formatContextLimitTruncationNotice(Math.max(1, raw.length - keptHead.length));
  };
  const calculateMaxToolResultChars = (contextWindowTokens) => {
    const tokens = Math.max(1, Math.floor(Number(contextWindowTokens) || 0));
    if (tokens <= 1) return 12000;
    const maxTokens = Math.floor(tokens * MAX_TOOL_RESULT_CONTEXT_SHARE);
    const maxChars = Math.max(1024, maxTokens * 4);
    return Math.min(maxChars, DEFAULT_MAX_LIVE_TOOL_RESULT_CHARS);
  };
  const truncateToolRowsForContext = (rows, contextWindowTokens) => {
    const limit = calculateMaxToolResultChars(contextWindowTokens);
    return (Array.isArray(rows) ? rows : []).map((row) => {
      if (!row || typeof row !== 'object') return row;
      var next = { ...row };
      if (typeof next.result === 'string' && next.result.length > limit) {
        next.result = truncateToolResultText(next.result, limit);
      }
      return next;
    });
  };
  const toolIdentity = (row, idx, prefix = 'tool') => {
    const source = row && typeof row === 'object' ? row : {};
    const receipt = source.tool_attempt_receipt && typeof source.tool_attempt_receipt === 'object'
      ? source.tool_attempt_receipt
      : {};
    const name = cleanText(source.name || source.tool || receipt.tool_name || 'tool', 120).toLowerCase() || 'tool';
    const attemptId = cleanText(source.attempt_id || source.tool_attempt_id || receipt.attempt_id || '', 160);
    const attemptSequence = toNum(source.attempt_sequence || source.tool_attempt_sequence || idx + 1, idx + 1);
    const fallbackId = cleanText(source.id || `${prefix}-${name}-${attemptSequence}`, 160);
    return {
      id: attemptId || fallbackId,
      attemptId,
      attemptSequence,
      identityKey: attemptId || `${name}#${attemptSequence}`,
    };
  };
  const splitThoughtSentences = (raw, maxItems = 6) => {
    const normalized = String(raw || '').replace(/\s+/g, ' ').trim();
    if (!normalized) return [];
    const chunks = normalized.match(/[^.!?]+[.!?]+|[^.!?]+$/g) || [normalized];
    const out = [];
    for (let i = 0; i < chunks.length; i++) {
      const sentence = cleanText(chunks[i] || '', 280);
      if (!sentence) continue;
      out.push(sentence);
      if (out.length >= maxItems) break;
    }
    return out;
  };
  const stringifyStructuredValue = (value, max = 16000) => {
    if (typeof value === 'string') return cleanText(value, max);
    if (value == null) return '';
    try {
      return cleanText(JSON.stringify(value), max);
    } catch {
      return cleanText(String(value), max);
    }
  };
  const collectTextContentBlocks = (value, maxItems = 12, maxChars = 64000) => {
    const out = [];
    const push = (entry) => {
      if (!entry || out.length >= maxItems) return;
      if (typeof entry === 'string') {
        const text = cleanText(entry, maxChars);
        if (text) out.push(text);
        return;
      }
      if (typeof entry !== 'object') return;
      if (Array.isArray(entry)) {
        for (let i = 0; i < entry.length && out.length < maxItems; i++) push(entry[i]);
        return;
      }
      if (Array.isArray(entry.content)) push(entry.content);
      if (Array.isArray(entry.output)) push(entry.output);
      const text =
        typeof entry.text === 'string'
          ? entry.text
          : (typeof entry.content === 'string' ? entry.content : '');
      const cleaned = cleanText(text, maxChars);
      if (cleaned) out.push(cleaned);
    };
    push(value);
    return out;
  };
  const extractStructuredTextCandidate = (value, max = 24000) => {
    const direct = collectTextContentBlocks(value);
    if (direct.length) return cleanText(direct.join('\n'), max);
    if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
    const nestedValues = [value.content, value.result, value.output, value.message, value.summary, value.error, value.text];
    for (let i = 0; i < nestedValues.length; i++) {
      const nested = collectTextContentBlocks(nestedValues[i]);
      if (nested.length) return cleanText(nested.join('\n'), max);
    }
    return '';
  };
  const isErrorLikeToolStatus = (value) => {
    const normalized = cleanText(value || '', 80).toLowerCase();
    if (!normalized) return false;
    if (
      normalized === '0' ||
      normalized === 'ok' ||
      normalized === 'success' ||
      normalized === 'completed' ||
      normalized === 'running'
    ) {
      return false;
    }
    return /error|fail|timeout|timed[_\s-]?out|denied|cancel|invalid|forbidden/.test(normalized);
  };
  const readToolResultDetails = (value) => {
    if (!value || typeof value !== 'object') return null;
    const details = value.details;
    return details && typeof details === 'object' && !Array.isArray(details) ? details : null;
  };
  const readToolResultStatus = (value) => {
    if (!value || typeof value !== 'object') return '';
    const details = readToolResultDetails(value);
    const direct = cleanText(value.status || '', 80);
    if (direct) return direct;
    return cleanText((details && details.status) || '', 80);
  };
  const extractToolErrorText = (value) => {
    if (typeof value === 'string') {
      const direct = cleanText(value, 400);
      return isErrorLikeToolStatus(direct) ? direct : '';
    }
    if (!value || typeof value !== 'object') return '';
    const details = readToolResultDetails(value);
    const candidates = [
      value.error,
      value.message,
      value.reason,
      details && details.error,
      details && details.message,
      details && details.reason,
      readToolResultStatus(value),
    ];
    for (let i = 0; i < candidates.length; i++) {
      const text = cleanText(candidates[i] || '', 400);
      if (!text) continue;
      if (i < candidates.length - 1 || isErrorLikeToolStatus(text)) return text;
    }
    return '';
  };
  const extractStructuredToolResultText = (value, max = 24000) => {
    if (typeof value === 'string') return cleanText(value, max);
    if (!value || typeof value !== 'object') return '';
    const structured = extractStructuredTextCandidate(value, max);
    if (structured) return structured;
    const textBlocks = collectTextContentBlocks(value.content);
    if (textBlocks.length) return cleanText(textBlocks.join('\n'), max);
    const details = readToolResultDetails(value);
    const candidates = [
      value.result,
      value.output,
      value.summary,
      value.text,
      value.message,
      value.error,
      details && details.result,
      details && details.summary,
      details && details.output,
      details && details.message,
      details && details.error,
      details && details.reason,
      details && details.status,
    ];
    for (let i = 0; i < candidates.length; i++) {
      const structuredCandidate = extractStructuredTextCandidate(candidates[i], max);
      if (structuredCandidate) return structuredCandidate;
      const text = stringifyStructuredValue(candidates[i], max);
      if (text) return text;
    }
    return stringifyStructuredValue(value, max);
  };
  const normalizeToolContentType = (value) =>
    typeof value === 'string' ? value.toLowerCase() : '';
  const isToolCallContentType = (value) => {
    const type = normalizeToolContentType(value);
    return type === 'toolcall' || type === 'tool_call' || type === 'tooluse' || type === 'tool_use';
  };
  const isToolResultContentType = (value) => {
    const type = normalizeToolContentType(value);
    return type === 'toolresult' || type === 'tool_result' || type === 'tool_result_error';
  };
  const resolveToolBlockArgs = (block) => {
    if (!block || typeof block !== 'object') return '';
    return block.args ?? block.arguments ?? block.input ?? '';
  };
  const resolveToolUseId = (block) => {
    if (!block || typeof block !== 'object') return '';
    const id =
      (typeof block.id === 'string' && block.id.trim()) ||
      (typeof block.tool_use_id === 'string' && block.tool_use_id.trim()) ||
      (typeof block.toolUseId === 'string' && block.toolUseId.trim()) ||
      '';
    return cleanText(id, 160);
  };
  const structuredContentBlocks = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    const out = [];
    const pushBlocks = (value) => {
      if (!Array.isArray(value)) return;
      for (let i = 0; i < value.length; i++) out.push(value[i]);
    };
    pushBlocks(data.content);
    pushBlocks(data.response);
    if (data.response && typeof data.response === 'object' && !Array.isArray(data.response)) {
      if (typeof data.response.type === 'string') out.push(data.response);
      pushBlocks(data.response.content);
      if (data.response.message && typeof data.response.message === 'object') {
        pushBlocks(data.response.message.content);
      }
    }
    pushBlocks(data.output);
    if (data.message && typeof data.message === 'object') {
      if (typeof data.message.type === 'string') out.push(data.message);
      pushBlocks(data.message.content);
    }
    if (data.result && typeof data.result === 'object') {
      if (typeof data.result.type === 'string') out.push(data.result);
      pushBlocks(data.result.content);
      if (data.result.message && typeof data.result.message === 'object') {
        pushBlocks(data.result.message.content);
      }
    }
    return out;
  };
  const responseWorkflowFromPayload = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    return data.response_workflow && typeof data.response_workflow === 'object'
      ? data.response_workflow
      : null;
  };
  const workflowResponseTextFromPayload = (payload) => {
    const workflow = responseWorkflowFromPayload(payload);
    if (!workflow) return '';
    const status = cleanText(
      workflow &&
      workflow.final_llm_response &&
      workflow.final_llm_response.status,
      80
    ).toLowerCase();
    const response = typeof workflow.response === 'string'
      ? cleanText(workflow.response, 24000)
      : '';
    if (status !== 'synthesized' || !response) return '';
    if (textLooksNoFindingsPlaceholder(response)) return '';
    return response;
  };
  const assistantTextFromPayload = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    const workflowText = workflowResponseTextFromPayload(data);
    if (workflowText) return workflowText;
    if (typeof data.response === 'string') return String(data.response || '');
    if (typeof data.content === 'string') return String(data.content || '');
    if (data.response && typeof data.response === 'object' && !Array.isArray(data.response)) {
      if (typeof data.response.text === 'string') return String(data.response.text || '');
      if (typeof data.response.content === 'string') return String(data.response.content || '');
    }
    const blocks = structuredContentBlocks(data);
    if (!blocks.length) return '';
    const parts = [];
    for (let i = 0; i < blocks.length; i++) {
      const entry = blocks[i];
      if (typeof entry === 'string') {
        const text = cleanText(entry, 12000);
        if (text) parts.push(text);
        continue;
      }
      if (!entry || typeof entry !== 'object') continue;
      if (isToolCallContentType(entry.type) || isToolResultContentType(entry.type)) continue;
      const text =
        typeof entry.text === 'string'
          ? entry.text
          : (typeof entry.content === 'string' ? entry.content : '');
      const cleaned = cleanText(text, 12000);
      if (cleaned) parts.push(cleaned);
    }
    return cleanText(parts.join('\n\n'), 24000);
  };
  const normalizeToolRows = (rawTools) => {
    if (!Array.isArray(rawTools)) return [];
    const out = [];
    for (let i = 0; i < rawTools.length; i++) {
      const row = rawTools[i] && typeof rawTools[i] === 'object' ? rawTools[i] : {};
      const name = cleanText(row.name || row.tool || 'tool', 120).toLowerCase() || 'tool';
      const identity = toolIdentity({ ...row, name }, i, 'tool');
      const input = stringifyStructuredValue(row.input || row.arguments || row.args || '', 16000);
      const rawStatus = cleanText(readToolResultStatus(row) || row.status || '', 40).toLowerCase();
      const blocked = row.blocked === true || rawStatus === 'blocked' || rawStatus === 'policy_denied';
      const errorText = extractToolErrorText(row);
      const result =
        extractStructuredToolResultText(
          row.result != null || row.output != null || row.summary != null || row.content != null
            ? row
            : {},
          24000
        ) || errorText;
      const isError = !blocked && !!(row.is_error || row.error || errorText || (rawStatus && rawStatus !== 'ok'));
      out.push({
        id: identity.id,
        name,
        input,
        result,
        is_error: isError,
        blocked,
        status: rawStatus,
        attempt_id: identity.attemptId,
        attempt_sequence: identity.attemptSequence,
        identity_key: identity.identityKey,
        tool_attempt_receipt: row.tool_attempt_receipt || null,
      });
      if (out.length >= 16) break;
    }
    return out;
  };
  const structuredToolRows = (payload) => {
    const blocks = structuredContentBlocks(payload);
    if (!blocks.length) return [];
    const out = [];
    const byIdentity = new Map();
    const ensureRow = (row, idx) => {
      const identity = toolIdentity(row, idx, 'content');
      const key = identity.identityKey;
      let current = byIdentity.get(key) || null;
      if (!current) {
        current = {
          id: identity.id,
          name: cleanText(row.name || row.tool || 'tool', 120).toLowerCase() || 'tool',
          input: '',
          result: '',
          is_error: false,
          blocked: false,
          status: '',
          attempt_id: identity.attemptId,
          attempt_sequence: identity.attemptSequence,
          identity_key: identity.identityKey,
          tool_attempt_receipt: null,
        };
        byIdentity.set(key, current);
        out.push(current);
      }
      return current;
    };
    for (let i = 0; i < blocks.length; i++) {
      const entry = blocks[i];
      if (!entry || typeof entry !== 'object') continue;
      const block = entry;
      if (isToolCallContentType(block.type)) {
        const toolName = cleanText(block.name || block.tool || 'tool', 120).toLowerCase() || 'tool';
        const row = ensureRow({
          name: toolName,
          attempt_id: resolveToolUseId(block),
          attempt_sequence: out.length + 1,
        }, out.length);
        if (!row.input) row.input = stringifyStructuredValue(resolveToolBlockArgs(block), 16000);
        continue;
      }
      if (!isToolResultContentType(block.type)) continue;
      const toolUseId = resolveToolUseId(block);
      const toolName = cleanText(block.name || block.tool || 'tool', 120).toLowerCase() || 'tool';
      const row = ensureRow({
        name: toolName,
        attempt_id: toolUseId,
        attempt_sequence: out.length + 1,
      }, out.length);
      const rawStatus = cleanText(readToolResultStatus(block) || block.status || '', 40).toLowerCase();
      const blocked = block.blocked === true || rawStatus === 'blocked' || rawStatus === 'policy_denied';
      const errorText = extractToolErrorText(block);
      const result =
        extractStructuredToolResultText(
          {
            result: block.result,
            output: block.output,
            content: block.content,
            text: block.text,
            error: block.error,
            message: block.message,
            status: block.status,
            details: block.details,
          },
          24000
        ) || errorText;
      if (!row.result && result) row.result = result;
      if (!row.name || row.name === 'tool') row.name = toolName;
      const isError =
        block.is_error === true ||
        normalizeToolContentType(block.type) === 'tool_result_error' ||
        !!errorText ||
        (!!rawStatus && rawStatus !== 'ok' && !blocked);
      if (blocked) row.blocked = true;
      if (isError) row.is_error = true;
      if (rawStatus) row.status = rawStatus;
    }
    return out.slice(0, 16);
  };
  const toolCardFromAttempt = (rawAttempt, idx) => {
    const envelope = rawAttempt && typeof rawAttempt === 'object' ? rawAttempt : {};
    const attempt = envelope.attempt && typeof envelope.attempt === 'object' ? envelope.attempt : envelope;
    const toolName = cleanText(attempt.tool_name || attempt.tool || 'tool', 120).toLowerCase() || 'tool';
    const rawStatus = cleanText(attempt.status || attempt.outcome || '', 40).toLowerCase();
    const blocked = rawStatus === 'blocked' || rawStatus === 'policy_denied';
    const isError = !blocked && !!rawStatus && rawStatus !== 'ok';
    const identity = toolIdentity({
      name: toolName,
      attempt_id: attempt.attempt_id || '',
      attempt_sequence: idx + 1,
      tool_attempt_receipt: attempt,
    }, idx, 'attempt');
    let input = '';
    try {
      if (envelope.normalized_result && envelope.normalized_result.normalized_args) {
        input = cleanText(JSON.stringify(envelope.normalized_result.normalized_args), 16000);
      }
    } catch {}
    const result = cleanText(
      envelope.error || attempt.reason || (attempt.backend ? ('Attempted via ' + String(attempt.backend).replace(/_/g, ' ')) : '') || rawStatus || 'attempt recorded',
      24000
    );
    return {
      id: identity.id,
      name: toolName,
      input,
      result,
      is_error: isError,
      blocked,
      status: blocked ? 'blocked' : (rawStatus || (isError ? 'error' : 'ok')),
      attempt_id: identity.attemptId,
      attempt_sequence: identity.attemptSequence,
      identity_key: identity.identityKey,
      tool_attempt_receipt: attempt,
    };
  };
  const mergeToolRowSets = (baseRows, extraRows) => {
    const merged = Array.isArray(baseRows) ? baseRows.slice() : [];
    const incoming = Array.isArray(extraRows) ? extraRows : [];
    const claimedBaseIndexes = new Set();
    for (let i = 0; i < incoming.length; i++) {
      const candidate = incoming[i] || {};
      let matched = false;
      for (let j = 0; j < merged.length; j++) {
        const current = merged[j] || {};
        const sameAttempt = candidate.attempt_id && String(current.attempt_id || '').trim() === String(candidate.attempt_id || '').trim();
        const sameUnnamedTool = !candidate.attempt_id && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
        const adoptUnnamedBase = !sameAttempt && !current.attempt_id && !claimedBaseIndexes.has(j) && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
        if (!sameAttempt && !sameUnnamedTool && !adoptUnnamedBase) continue;
        if (!current.input && candidate.input) current.input = candidate.input;
        if (!current.result && candidate.result) current.result = candidate.result;
        if (candidate.blocked) current.blocked = true;
        if (candidate.status) current.status = candidate.status;
        if (candidate.is_error) current.is_error = true;
        if (candidate.id) current.id = candidate.id;
        if (candidate.attempt_id) current.attempt_id = candidate.attempt_id;
        if (candidate.attempt_sequence) current.attempt_sequence = candidate.attempt_sequence;
        if (candidate.identity_key) current.identity_key = candidate.identity_key;
        if (candidate.tool_attempt_receipt) current.tool_attempt_receipt = candidate.tool_attempt_receipt;
        claimedBaseIndexes.add(j);
        matched = true;
        break;
      }
      if (!matched) merged.push(candidate);
    }
    return merged.slice(0, 16);
  };
  const mergeResponseToolRows = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    const base = mergeToolRowSets(
      normalizeToolRows(data.tools),
      structuredToolRows(data)
    );
    const completion =
      data &&
      data.response_finalization &&
      data.response_finalization.tool_completion &&
      typeof data.response_finalization.tool_completion === 'object'
        ? data.response_finalization.tool_completion
        : null;
    const attempts = Array.isArray(completion && completion.tool_attempts)
      ? completion.tool_attempts
      : [];
    if (!attempts.length) return base;
    const merged = base.slice();
    for (let i = 0; i < attempts.length; i++) {
      const attemptCard = toolCardFromAttempt(attempts[i], i);
      const nextMerged = mergeToolRowSets(merged, [attemptCard]);
      merged.length = 0;
      Array.prototype.push.apply(merged, nextMerged);
    }
    return merged.slice(0, 16);
  };
  const normalizeToolCompletionSteps = (rawCompletion) => {
    if (!rawCompletion || typeof rawCompletion !== 'object') return [];
    const rows = Array.isArray(rawCompletion.live_tool_steps)
      ? rawCompletion.live_tool_steps
      : [];
    const out = [];
    for (let i = 0; i < rows.length; i++) {
      const row = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
      const tool = cleanText(row.tool || row.name || '', 120).toLowerCase();
      const status = cleanText(row.status || row.tool_status || '', 220);
      if (!tool || !status) continue;
      out.push({
        tool,
        status,
        is_error: !!row.is_error,
      });
      if (out.length >= 16) break;
    }
    return out;
  };
  const backfillToolRowsFromCompletion = (rows, payload) => {
    const merged = Array.isArray(rows) ? rows.map((row) => (row && typeof row === 'object' ? { ...row } : row)) : [];
    const data = payload && typeof payload === 'object' ? payload : {};
    const completion =
      data &&
      data.response_finalization &&
      data.response_finalization.tool_completion &&
      typeof data.response_finalization.tool_completion === 'object'
        ? data.response_finalization.tool_completion
        : null;
    const completionSteps = normalizeToolCompletionSteps(completion);
    if (!completionSteps.length) return merged.slice(0, 16);
    if (!merged.length) {
      for (let si = 0; si < completionSteps.length && merged.length < 16; si++) {
        const stepSeed = completionSteps[si] || {};
        const stepName = cleanText(stepSeed.tool || stepSeed.name || 'tool', 120) || 'tool';
        const stepStatus = cleanText(stepSeed.status || '', 220);
        if (!stepName && !stepStatus) continue;
        merged.push(normalizeToolRows([{
          id: `completion-step-${si + 1}-${stepName}`,
          name: stepName,
          result: stepStatus ? `Missing tool_result block; last known status: ${stepStatus}` : '',
          is_error: !!stepSeed.is_error,
          status: stepStatus ? stepStatus.toLowerCase() : '',
        }])[0]);
      }
    }
    for (let i = 0; i < merged.length; i++) {
      const row = merged[i] && typeof merged[i] === 'object' ? merged[i] : null;
      if (!row) continue;
      let step = null;
      const rowName = cleanText(row.name || '', 120).toLowerCase();
      const byIndex = completionSteps[i] || null;
      if (
        byIndex &&
        cleanText(byIndex.tool || '', 120).toLowerCase() === rowName &&
        cleanText(byIndex.status || '', 220)
      ) {
        step = byIndex;
      } else {
        for (let si = 0; si < completionSteps.length; si++) {
          const candidate = completionSteps[si] || {};
          if (
            cleanText(candidate.tool || '', 120).toLowerCase() === rowName &&
            cleanText(candidate.status || '', 220)
          ) {
            step = candidate;
            break;
          }
        }
      }
      if (!step) continue;
      const stepStatus = cleanText(step.status || '', 220);
      if (!cleanText(row.status || '', 80) && stepStatus) {
        row.status = stepStatus.toLowerCase();
      }
      if ((!cleanText(row.result || '', 24000)) && stepStatus) {
        row.result = `Missing tool_result block; last known status: ${stepStatus}`;
      }
      if (step.is_error && !row.blocked) {
        row.is_error = true;
      }
    }
    return merged.slice(0, 16);
  };
  const textLooksNoFindingsPlaceholder = (text) => {
    const lower = cleanText(text || '', 1200).replace(/\s+/g, ' ').trim().toLowerCase();
    if (!lower) return false;
    return (
      lower.includes("don't have usable tool findings from this turn yet") ||
      lower.includes('dont have usable tool findings from this turn yet') ||
      lower.includes('no usable findings yet') ||
      lower.includes("couldn't extract usable findings") ||
      lower.includes('could not extract usable findings') ||
      lower.includes("couldn't produce source-backed findings in this turn") ||
      lower.includes('search returned no useful information')
    );
  };
  const textMentionsContextGuard = (text) => {
    const lower = cleanText(text || '', 4000).replace(/\s+/g, ' ').trim().toLowerCase();
    if (!lower) return false;
    return (
      lower.includes('context overflow: estimated context size exceeds safe threshold during tool loop') ||
      lower.includes('more characters truncated') ||
      lower.includes('middle content omitted') ||
      lower.includes('safe context budget')
    );
  };
  const stripContextGuardMarkers = (text) =>
    cleanText(String(text || '')
      .replace(/\[\.\.\.\s+\d+\s+more characters truncated\]/gi, ' ')
      .replace(/context overflow:\s*estimated context size exceeds safe threshold during tool loop\.?/gi, ' ')
      .replace(/middle content omitted/gi, ' ')
      .replace(/\s+/g, ' '), 24000);
  const textLooksActionableWebDiagnostic = (text) => {
    const lower = cleanText(text || '', 2400).replace(/\s+/g, ' ').trim().toLowerCase();
    if (!lower) return false;
    return (
      textMentionsContextGuard(text) ||
      textLooksNoFindingsPlaceholder(text) ||
      lower.includes('low-signal') ||
      lower.includes('retrieval-quality miss') ||
      lower.includes('comparison findings') ||
      lower.includes('source-backed answer') ||
      lower.includes('source-backed findings')
    );
  };
  const parseStructuredToolInput = (tool) => {
    if (!tool || typeof tool !== 'object') return {};
    const input = tool.input;
    if (input && typeof input === 'object' && !Array.isArray(input)) return input;
    const raw = typeof input === 'string' ? input.trim() : '';
    if (!raw || raw.charAt(0) !== '{') return {};
    try {
      const parsed = JSON.parse(raw);
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
    } catch {
      return {};
    }
  };
  const toolInputMeta = (tool) => {
    const input = parseStructuredToolInput(tool);
    const query = cleanText(input.query || input.q || '', 180);
    if (query) return `"${query}"`;
    const url = cleanText(input.url || input.link || '', 220);
    if (url) return url;
    const filePath = cleanText(input.path || '', 220);
    if (filePath) return filePath;
    return '';
  };
  const lowSignalWebToolSummary = (tool) => {
    const label = cleanText(String(tool && tool.name ? tool.name : 'web tool').replace(/_/g, ' '), 80) || 'web tool';
    const subject = toolInputMeta(tool);
    const suffix = subject ? ` for ${subject}` : '';
    const result = cleanText(tool && tool.result ? tool.result : '', 24000);
    const lower = result.toLowerCase();
    if (textMentionsContextGuard(tool && tool.result)) {
      return `The ${label} step${suffix} returned more output than fit safely in context. Retry with a narrower query, one specific source URL, or continue from the partial result.`;
    }
    if (lower.includes('search returned no useful comparison findings') || lower.includes('retrieval-quality miss')) {
      return `The ${label} step${suffix} ran, but retrieval did not produce enough source coverage to compare both sides in this turn. That is a retrieval-quality miss, not proof the systems are equivalent. Retry with named competitors, one specific source URL per side, or continue from the recorded tool result.`;
    }
    return `The ${label} step${suffix} ran, but only low-signal web output came back. Retry with a narrower query, one specific source URL, or continue from the recorded tool result.`;
  };
  const toolResultSummarySnippet = (tool) => cleanText(tool && (tool.result || tool.status || ''), 220);
  const toolOnlyResponseSummary = (assistantText, tools) => {
    const _ = tools;
    const assistant = cleanText(assistantText || '', 24000);
    if (!assistant) return '';
    const lower = cleanText(assistantText || '', 1200).replace(/\s+/g, ' ').trim().toLowerCase();
    const lostHandoff =
      !lower ||
      lower === 'i lost the final response handoff for this turn. context is still intact, and i can continue from exactly where this left off.';
    if (lostHandoff || textLooksNoFindingsPlaceholder(assistant)) return '';
    return assistant;
  };
  const finalizationFallbackSummary = (payload, tools) => {
    const _ = [payload, tools];
    return '';
  };
  const toolStatusFromCompletion = (toolName, toolIndex, completionSteps, fallbackStatus) => {
    if (Array.isArray(completionSteps) && completionSteps.length > 0) {
      const byIndex = completionSteps[toolIndex] || null;
      if (
        byIndex &&
        String(byIndex.tool || '').toLowerCase() === String(toolName || '').toLowerCase() &&
        byIndex.status
      ) {
        return cleanText(byIndex.status, 220);
      }
      for (let i = 0; i < completionSteps.length; i++) {
        const row = completionSteps[i] || {};
        if (
          String(row.tool || '').toLowerCase() === String(toolName || '').toLowerCase() &&
          row.status
        ) {
          return cleanText(row.status, 220);
        }
      }
    }
    return cleanText(fallbackStatus || '', 220);
  };
  const replayToolTimeline = async (ws, agentId, tools, toolCompletion) => {
    if (!Array.isArray(tools) || tools.length < 1) return;
    const replayCount = Math.min(tools.length, 8);
    const completionSteps = normalizeToolCompletionSteps(toolCompletion);
    const timelineSource = toolCompletion && typeof toolCompletion === 'object'
      ? 'response_finalization_tool_completion'
      : 'response_tools_fallback';
    const fallbackToolStatus = cleanText(
      toolCompletion && typeof toolCompletion === 'object'
        ? toolCompletion.live_tool_status || ''
        : '',
      220
    );
    const basePauseMs = replayCount > 4 ? 90 : 120;
    for (let i = 0; i < replayCount; i++) {
      const tool = tools[i] || {};
      const toolName = cleanText(tool.name || 'tool', 120).toLowerCase() || 'tool';
      const toolInput = cleanText(tool.input || '', 16000);
      const toolResult = cleanText(tool.result || '', 24000);
      const toolError = !!tool.is_error;
      const attemptId = cleanText(tool.attempt_id || (tool.tool_attempt_receipt && tool.tool_attempt_receipt.attempt_id) || '', 160);
      const attemptSequence = toNum(tool.attempt_sequence || i + 1, i + 1);
      const toolStatus = toolStatusFromCompletion(
        toolName,
        i,
        completionSteps,
        i === 0 ? fallbackToolStatus : ''
      );
      send(ws, {
        type: 'tool_start',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        tool_status: toolStatus,
        tool_timeline_contract_version: TOOL_TIMELINE_CONTRACT_VERSION,
        tool_timeline_source: timelineSource,
      });
      if (toolStatus) {
        send(ws, {
          type: 'phase',
          agent_id: agentId,
          phase: 'tool_running',
          detail: toolStatus,
          tool: toolName,
          source: 'tool_completion_receipt',
          tool_timeline_contract_version: TOOL_TIMELINE_CONTRACT_VERSION,
          tool_timeline_source: timelineSource,
          progress_percent: Math.round(((i + 1) / replayCount) * 100),
          tool_step_index: i + 1,
          tool_step_total: replayCount,
        });
      }
      const thoughtLines =
        toolName === 'thought_process'
          ? splitThoughtSentences(toolInput || toolResult, 3)
          : [];
      for (let ti = 0; ti < thoughtLines.length; ti++) {
        send(ws, {
          type: 'phase',
        agent_id: agentId,
        phase: 'reasoning',
        detail: thoughtLines[ti],
        tool_timeline_contract_version: TOOL_TIMELINE_CONTRACT_VERSION,
        tool_timeline_source: timelineSource,
      });
        await sleep(basePauseMs);
      }
      send(ws, {
        type: 'tool_result',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        result: toolResult,
        is_error: toolError,
        tool_status: toolStatus,
        tool_timeline_contract_version: TOOL_TIMELINE_CONTRACT_VERSION,
        tool_timeline_source: timelineSource,
      });
      send(ws, {
        type: 'tool_end',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        tool_status: toolStatus,
        tool_timeline_contract_version: TOOL_TIMELINE_CONTRACT_VERSION,
        tool_timeline_source: timelineSource,
      });
      await sleep(basePauseMs);
    }
  };
  const sendContext = async (ws, agentId) => {
    const agent = await fetchBackendJson(flags, `/api/agents/${enc(agentId)}`, 8000).catch(() => ({}));
    const contextWindow = toNum(agent.context_window || agent.context_window_tokens || 0, 0);
    send(ws, { type: 'context_state', agent_id: agentId, context_tokens: 0, context_window: contextWindow, context_ratio: 0, context_pressure: contextWindow > 0 ? 'normal' : '' });
    return agent;
  };
  wss.on('connection', (ws, _req, agentId) => {
    const targetAgent = cleanText(agentId || '', 180);
    let agentName = '';
    let agentContextWindow = 0;
    let chain = Promise.resolve();
    chain = chain.then(async () => {
      const agent = await sendContext(ws, targetAgent);
      agentName = cleanText(agent.name || '', 120);
      agentContextWindow = toNum(agent.context_window || agent.context_window_tokens || 0, 0);
      send(ws, { type: 'connected', agent_id: targetAgent, agent_name: agentName || '' });
    }).catch((error) => send(ws, { type: 'error', content: cleanText(error && error.message ? error.message : 'ws_connect_failed', 260), agent_id: targetAgent }));
    ws.on('message', (chunk) => {
      const raw = Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk || '');
      chain = chain.then(async () => {
        const payload = parseJson(raw);
        if (!payload || typeof payload !== 'object') return;
        const msgType = cleanText(payload.type || '', 40).toLowerCase();
        if (!msgType || msgType === 'ping') { send(ws, { type: 'pong' }); return; }
        if (msgType === 'message') {
          const content = String(payload.content == null ? '' : payload.content).slice(0, 12000);
          if (!content.trim()) { send(ws, { type: 'error', content: 'message_required', agent_id: targetAgent }); return; }
          send(ws, { type: 'typing', state: 'start', agent_id: targetAgent });
          let res = null;
          let out = {};
          res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/message`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ message: content, attachments: Array.isArray(payload.attachments) ? payload.attachments : [] }),
          }, 180000);
          out = await res.json().catch(() => ({}));
          const hasStructuredResponse =
            !!(out && typeof out === 'object' && (
              typeof out.response === 'string' ||
              Array.isArray(out.response) ||
              Array.isArray(out.content) ||
              (out.response_finalization &&
                out.response_finalization.tool_completion &&
                Array.isArray(out.response_finalization.tool_completion.tool_attempts)) ||
              Array.isArray(out.tools)
            ));
          if ((!res.ok || out.ok === false) && !hasStructuredResponse) {
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          const effectiveContextWindow = toNum(
            out.context_window || out.context_window_tokens || 0,
            agentContextWindow
          );
          const toolRows = truncateToolRowsForContext(
            backfillToolRowsFromCompletion(mergeResponseToolRows(out), out),
            effectiveContextWindow
          );
          const toolCompletion =
            out &&
            out.response_finalization &&
            out.response_finalization.tool_completion &&
            typeof out.response_finalization.tool_completion === 'object'
              ? out.response_finalization.tool_completion
              : null;
          if (toolRows.length > 0) {
            await replayToolTimeline(ws, targetAgent, toolRows, toolCompletion);
          }
          const timelineSource = toolRows.length > 0
            ? (toolCompletion ? 'response_finalization_tool_completion' : 'response_tools_fallback')
            : '';
          const assistantContent = assistantTextFromPayload(out);
          const finalAssistantContent = toolOnlyResponseSummary(assistantContent, toolRows) || finalizationFallbackSummary(out, toolRows);
          send(ws, {
            type: 'response',
            agent_id: targetAgent,
            agent_name: agentName || cleanText(out.agent_name || '', 120) || '',
            content: finalAssistantContent,
            input_tokens: toNum(out.input_tokens || 0, 0),
            output_tokens: toNum(out.output_tokens || 0, 0),
            cost_usd: toNum(out.cost_usd || 0, 0),
            iterations: toNum(out.iterations || 1, 1),
            duration_ms: toNum(out.duration_ms || out.latency_ms || 0, 0),
            context_tokens: toNum(out.context_tokens || out.context_used_tokens || out.context_total_tokens || 0, 0),
            context_window: toNum(out.context_window || out.context_window_tokens || 0, 0),
            context_ratio: toNum(out.context_ratio || 0, 0),
            context_pressure: cleanText(out.context_pressure || '', 32),
            auto_route: out.auto_route || null,
            tools: toolRows,
            tool_timeline_contract_version: toolRows.length > 0 ? TOOL_TIMELINE_CONTRACT_VERSION : 0,
            tool_timeline_source: timelineSource,
            tool_timeline_events_replayed: toolRows.length > 0,
            response_workflow: out.response_workflow || null,
            response_finalization: out.response_finalization || null,
            turn_transaction: out.turn_transaction || null,
          });
          return;
        }
        if (msgType === 'terminal') {
          const command = String(payload.command == null ? '' : payload.command).slice(0, 16000);
          if (!command.trim()) { send(ws, { type: 'terminal_error', agent_id: targetAgent, message: 'command_required' }); return; }
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/terminal`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ command, cwd: cleanText(payload.cwd || '', 4000) }),
          }, 120000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok) {
            send(ws, { type: 'terminal_error', agent_id: targetAgent, message: cleanText(out.error || out.message || `backend_http_${res.status}`, 260) });
            return;
          }
          const stdout = String(out.stdout || '');
          const stderrBase = String(out.stderr || '');
          const stderr = stderrBase || (out.ok === false ? cleanText(out.message || out.error || '', 4000) : '');
          send(ws, {
            type: 'terminal_output',
            agent_id: targetAgent,
            stdout,
            stderr,
            exit_code: toNum(out.exit_code || (out.ok === false ? 1 : 0), 0),
            duration_ms: toNum(out.duration_ms || 0, 0),
            cwd: cleanText(out.cwd || '', 4000),
            requested_command: cleanText(out.requested_command || command, 16000),
            executed_command: cleanText(out.executed_command || command, 16000),
            command_translated: !!out.command_translated,
            translation_reason: cleanText(out.translation_reason || '', 240),
            suggestions: Array.isArray(out.suggestions) ? out.suggestions : [],
            permission_gate: out.permission_gate || null,
            filter_events: Array.isArray(out.filter_events) ? out.filter_events : [],
            low_signal_output: !!out.low_signal_output,
            recovery_hints: Array.isArray(out.recovery_hints) ? out.recovery_hints : [],
            tool_summary: out.tool_summary || null,
            tracking: out.tracking || null,
          });
          return;
        }
        if (msgType === 'command') {
          const command = cleanText(payload.command || '', 80).toLowerCase();
          const silent = !!payload.silent;
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/command`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ command, silent }),
          }, 12000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            const commandUnavailable = res.status === 404 || res.status === 405 || res.status === 501;
            if (commandUnavailable) {
              const ctx = await sendContext(ws, targetAgent).catch(() => null);
              const contextWindow = toNum(
                (ctx && (ctx.context_window || ctx.context_window_tokens)) ||
                  out.context_window ||
                  out.context_window_tokens ||
                  0,
                0
              );
              send(ws, {
                type: 'command_result',
                silent,
                agent_id: targetAgent,
                command: cleanText(command || 'unknown', 80),
                message: silent ? '' : 'Command unavailable on this runtime surface.',
                runtime_sync: null,
                context_window: contextWindow,
              });
              return;
            }
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, {
            type: 'command_result',
            silent,
            agent_id: targetAgent,
            command: cleanText(out.command || command || 'unknown', 80),
            message: cleanText(out.message || `Command '${command || 'unknown'}' acknowledged.`, 320),
            runtime_sync: out.runtime_sync || null,
            context_window: toNum(out.context_window || 0, 0),
            context_tokens: toNum(
              out.context_tokens ||
              out.context_used_tokens ||
              (out.context_pool && out.context_pool.active_tokens) ||
              0,
              0
            ),
            context_ratio: toNum(
              out.context_ratio ||
              (out.context_pool && out.context_pool.context_ratio) ||
              0,
              0
            ),
            context_pressure: cleanText(
              out.context_pressure ||
              (out.context_pool && out.context_pool.context_pressure) ||
              '',
              32
            ),
          });
          return;
        }
      }).catch((error) => send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(error && error.message ? error.message : 'ws_bridge_failed', 260) }));
    });
    ws.on('error', () => {});
  });
  return {
    ws_enabled: true,
    ws_error: '',
    tryHandle(req, socket, head) {
      const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
      const match = pathname.match(route);
      if (!match) return false;
      const agentId = cleanText(decodeURIComponent(match[1] || ''), 180);
      if (!agentId) { try { socket.destroy(); } catch {} return true; }
      wss.handleUpgrade(req, socket, head, (ws) => wss.emit('connection', ws, req, agentId));
      return true;
    },
  };
}

module.exports = { createAgentWsBridge };
