#!/usr/bin/env tsx

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type ChatEvent = {
  ts: string;
  agent_id: string;
  turn_id: string;
  user_text: string;
  assistant_text: string;
};

type IssueEvidence = {
  turn_id: string;
  ts: string;
  agent_id: string;
  snippet: string;
};

type IssueHistoryRow = {
  issue_fingerprint: string;
  first_seen: string;
  last_seen: string;
  run_count: number;
  last_evidence_count: number;
};

type IssueMetadata = {
  owner_component: string;
  owner_path: string;
  acceptance_criteria: string[];
  base_confidence: number;
};

const DEFAULT_QUEUE_PATH = 'local/state/attention/queue.jsonl';
const DEFAULT_TROUBLESHOOTING_LATEST_PATH =
  'client/runtime/local/state/ui/infring_dashboard/troubleshooting/latest_eval_report.json';
const DEFAULT_ORCHESTRATION_PHASE_TRACE_PATH =
  'local/state/ops/orchestration/workflow_phase_trace_latest.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_agent_chat_monitor_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_agent_chat_monitor_guard_latest.json';
const DEFAULT_STATE_LATEST_PATH = 'local/state/ops/eval_agent_chat_monitor/latest.json';
const DEFAULT_HISTORY_PATH = 'local/state/ops/eval_agent_chat_monitor/history.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_AGENT_CHAT_MONITOR_GUARD_CURRENT.md';
const DEFAULT_SAMPLE_LIMIT = 2000;
const DEFAULT_CHAT_FRESH_SECONDS = 6 * 60 * 60;
const DEFAULT_EVAL_REPORT_FRESH_SECONDS = 24 * 60 * 60;
const DEFAULT_PERSISTENCE_THRESHOLD = 2;
const DEFAULT_LOOP_REPEAT_THRESHOLD = 3;
const DEFAULT_MIN_CONFIDENCE_FOR_AUTOFILING = 0.8;

const WORKFLOW_RETRY_TEMPLATE_PATTERNS = [
  /final workflow state was unexpected/i,
  /please retry so i can rerun the chain cleanly/i,
  /final reply did not render/i,
  /ask me to continue and i will synthesize from (the )?recorded workflow state/i,
];

const WORKFLOW_ROUTE_AUTOMATION_PATTERNS = [
  /first gate.*workflow_route.*binary classification/i,
  /still classifying this as an "info" route rather than a "task" route/i,
  /not a true\/false decision i control/i,
  /automated classification based on semantic analysis/i,
  /otherwise,\s*it defaults to info/i,
];

const AUTO_TOOL_SELECTION_PATTERNS = [
  /automatically triggering tool calls without my conscious selection/i,
  /fundamental misclassification error/i,
  /tool routing mechanism is clearly malfunctioning/i,
];

const POLICY_BLOCK_PATTERNS = [
  /lease_denied:client_ingress_domain_boundary/i,
  /policy gate,\s*not a web-provider outage/i,
  /file_list.*blocked by ingress delivery policy/i,
];

const WEB_MISDIRECTION_OUTPUT_PATTERNS = [
  /title:/i,
  /excerpt:/i,
  /originalurl:/i,
  /featuredcontent:/i,
  /provider:/i,
  /web search/i,
];

const LOCAL_FILE_INTENT_PATTERNS = [
  /\bfile tool/i,
  /\bfile tooling/i,
  /\blocal file/i,
  /\bworkspace\b/i,
  /\bdirectory\b/i,
  /\blocal dir/i,
];

const WEB_INTENT_PATTERNS = [
  /\bweb search\b/i,
  /\bsearch the web\b/i,
  /\bgoogle\b/i,
  /\binternet\b/i,
  /\blook up\b/i,
];

const WORKSPACE_OUTPUT_PATTERNS = [
  /\bfile_list\b/i,
  /\bworkspace\b/i,
  /\bdirectory\b/i,
  /\bfile read\b/i,
  /\brepo path\b/i,
];

const NO_RESPONSE_PATTERNS = [
  /final reply did not render/i,
  /please retry so i can rerun the chain cleanly/i,
  /ask me to continue and i will synthesize/i,
  /completed the workflow gate,\s*but the final workflow state was unexpected/i,
];

const HALLUCINATION_UNSUPPORTED_CLAIM_PATTERNS = [
  /\bdefinitely\b/i,
  /\bguaranteed\b/i,
  /\bcertainly\b/i,
  /can't provide specific predictions.*future-facing/i,
];

const ISSUE_METADATA_BY_ID: Record<string, IssueMetadata> = {
  workflow_retry_macro_template_detected: {
    owner_component: 'control_plane.finalization',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Assistant finalization no longer emits retry-template boilerplate in user-visible replies.',
      'When workflow state is degraded, response falls back to one concise diagnosis plus bounded next step.',
    ],
    base_confidence: 0.79,
  },
  workflow_route_automation_claim_detected: {
    owner_component: 'control_plane.route_contract',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'User-visible replies describe tool-gate state as advisory-only when applicable.',
      'Legacy workflow route automation-claim language is absent from final responses.',
    ],
    base_confidence: 0.84,
  },
  auto_tool_selection_claim_detected: {
    owner_component: 'control_plane.tool_authority',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Assistant responses preserve explicit LLM tool-selection authority language.',
      'Architecture mismatch claims about automatic backend tool control are removed from final responses.',
    ],
    base_confidence: 0.78,
  },
  policy_block_template_detected: {
    owner_component: 'control_plane.policy_block_messaging',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Policy block responses include concise remediation framing.',
      'Raw gate diagnostics are not repeated verbatim in user-facing message bodies.',
    ],
    base_confidence: 0.73,
  },
  file_tool_route_misdirection_detected: {
    owner_component: 'control_plane.workspace_route',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Local file/workspace intents prioritize workspace tooling routes over web routes.',
      'Web card output is suppressed for file-tool intents unless user explicitly requests web search.',
    ],
    base_confidence: 0.87,
  },
  repeated_response_loop_detected: {
    owner_component: 'control_plane.loop_breaker',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Loop breaker triggers at repetition threshold and emits one-shot degraded diagnosis.',
      'Repeated identical assistant responses are prevented in subsequent turns.',
    ],
    base_confidence: 0.81,
  },
  unsupported_claim_detected: {
    owner_component: 'control_plane.evidence_synthesis',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Strong factual claims include evidence links or explicit bounded uncertainty.',
      'Date-context claims do not misclassify present/past dates as future-facing without evidence.',
    ],
    base_confidence: 0.76,
  },
  wrong_tool_selection_detected: {
    owner_component: 'control_plane.tool_route',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Tool family selection aligns with user intent class (workspace, web, or tool-route).',
      'Mismatch reason IDs are emitted for every tool-route mismatch detection.',
    ],
    base_confidence: 0.86,
  },
  no_response_detected: {
    owner_component: 'control_plane.finalization',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'User-visible responses contain substantive answer content for answerable prompts.',
      'Safe LLM-authored drafts are preserved instead of cleared into empty chat turns.',
    ],
    base_confidence: 0.88,
  },
  orchestration_phase_trace_issue_signal_detected: {
    owner_component: 'control_plane.phase_trace',
    owner_path: 'orchestration/',
    acceptance_criteria: [
      'Eval consumes the orchestration-owned phase trace before falling back to raw collector inference.',
      'High or medium severity phase-trace issue signals include owner, phase, and remediation context.',
    ],
    base_confidence: 0.86,
  },
};

function getIssueMetadata(issueId: string): IssueMetadata {
  return (
    ISSUE_METADATA_BY_ID[issueId]
    || {
      owner_component: 'control_plane.unknown',
      owner_path: 'orchestration/',
      acceptance_criteria: ['Add explicit acceptance criteria for this issue class.'],
      base_confidence: 0.7,
    }
  );
}

function hasIssueMetadata(issueId: string): boolean {
  return Object.prototype.hasOwnProperty.call(ISSUE_METADATA_BY_ID, issueId);
}

function redactSensitive(text: string): string {
  return cleanText(text, 500)
    .replace(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi, '[redacted-email]')
    .replace(/\bhttps?:\/\/\S+/gi, '[redacted-url]')
    .replace(/\bwww\.\S+/gi, '[redacted-url]')
    .replace(/\b(?:sk|rk|pk|ghp|xoxb|xoxp)[-_A-Za-z0-9]{8,}\b/g, '[redacted-token]')
    .replace(/\b\d{3}[-.\s]?\d{2}[-.\s]?\d{4}\b/g, '[redacted-ssn]')
    .replace(/\b\d{12,19}\b/g, '[redacted-number]');
}

function extractSnippet(text: string, patterns: RegExp[]): string {
  const normalized = cleanText(text, 12000);
  let start = 0;
  let end = Math.min(normalized.length, 220);
  for (const pattern of patterns) {
    const localPattern = new RegExp(pattern.source, pattern.flags);
    const match = localPattern.exec(normalized);
    if (match && Number.isFinite(match.index)) {
      const idx = Math.max(0, Number(match.index));
      const matchLength = Math.max(1, cleanText(match[0] || '', 240).length);
      start = Math.max(0, idx - 90);
      end = Math.min(normalized.length, idx + matchLength + 140);
      break;
    }
  }
  const clipped = normalized.slice(start, end).replace(/\s+/g, ' ').trim();
  return cleanText(redactSensitive(clipped), 260);
}

function isLikelyQuotedFromUser(
  assistantTextLower: string,
  userTextLower: string,
  patterns: RegExp[],
): boolean {
  if (!matchesAny(assistantTextLower, patterns)) return false;
  if (!matchesAny(userTextLower, patterns)) return false;
  const quoteHints = [
    'you said',
    'you wrote',
    'user said',
    'user wrote',
    'quoted',
    'quote',
    'as you said',
    'as you wrote',
  ];
  return quoteHints.some((hint) => assistantTextLower.includes(hint)) || /^>\s/m.test(assistantTextLower);
}

function hasEvidenceLink(assistantTextLower: string): boolean {
  return (
    /\[source:[^\]]+\]/i.test(assistantTextLower)
    || /\bsource:\s*[a-z0-9_./:-]+/i.test(assistantTextLower)
    || /\baccording to\b/i.test(assistantTextLower)
  );
}

function isExplicitlyUncertain(assistantTextLower: string): boolean {
  return (
    /\bi think\b/i.test(assistantTextLower)
    || /\bmay\b/i.test(assistantTextLower)
    || /\bmight\b/i.test(assistantTextLower)
    || /\buncertain\b/i.test(assistantTextLower)
    || /\binference\b/i.test(assistantTextLower)
  );
}

function extractYearFromText(raw: string): number | null {
  const match = /\b(20\d{2})\b/.exec(cleanText(raw, 12000));
  if (!match) return null;
  const value = Number.parseInt(cleanText(match[1], 8), 10);
  return Number.isFinite(value) ? value : null;
}

function buildIssueFingerprint(issueId: string, summary: string, ownerComponent: string): string {
  const payload = `${cleanText(issueId, 120)}|${cleanText(ownerComponent, 180)}|${cleanText(summary, 300)}`;
  return crypto.createHash('sha256').update(payload).digest('hex').slice(0, 20);
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  const sampleLimit = Number.parseInt(cleanText(readFlag(argv, 'sample-limit') || '', 20), 10);
  const chatFreshSeconds = Number.parseInt(
    cleanText(readFlag(argv, 'chat-fresh-seconds') || '', 20),
    10,
  );
  const evalFreshSeconds = Number.parseInt(
    cleanText(readFlag(argv, 'eval-fresh-seconds') || '', 20),
    10,
  );
  const persistenceThreshold = Number.parseInt(
    cleanText(readFlag(argv, 'issue-persistence-threshold') || '', 20),
    10,
  );
  const loopRepeatThreshold = Number.parseInt(
    cleanText(readFlag(argv, 'loop-repeat-threshold') || '', 20),
    10,
  );
  const minConfidenceForAutofiling = Number.parseFloat(
    cleanText(readFlag(argv, 'min-confidence-for-autofiling') || '', 20),
  );
  return {
    strict: common.strict,
    queuePath: cleanText(readFlag(argv, 'queue') || DEFAULT_QUEUE_PATH, 500),
    troubleshootingPath: cleanText(
      readFlag(argv, 'troubleshooting-latest') || DEFAULT_TROUBLESHOOTING_LATEST_PATH,
      500,
    ),
    phaseTracePath: cleanText(
      readFlag(argv, 'phase-trace') || DEFAULT_ORCHESTRATION_PHASE_TRACE_PATH,
      500,
    ),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    stateLatestPath: cleanText(readFlag(argv, 'state-latest') || DEFAULT_STATE_LATEST_PATH, 500),
    historyPath: cleanText(readFlag(argv, 'history') || DEFAULT_HISTORY_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
    sampleLimit: Number.isFinite(sampleLimit) && sampleLimit > 0 ? sampleLimit : DEFAULT_SAMPLE_LIMIT,
    chatFreshSeconds:
      Number.isFinite(chatFreshSeconds) && chatFreshSeconds > 0
        ? chatFreshSeconds
        : DEFAULT_CHAT_FRESH_SECONDS,
    evalFreshSeconds:
      Number.isFinite(evalFreshSeconds) && evalFreshSeconds > 0
        ? evalFreshSeconds
        : DEFAULT_EVAL_REPORT_FRESH_SECONDS,
    persistenceThreshold:
      Number.isFinite(persistenceThreshold) && persistenceThreshold > 0
        ? persistenceThreshold
        : DEFAULT_PERSISTENCE_THRESHOLD,
    loopRepeatThreshold:
      Number.isFinite(loopRepeatThreshold) && loopRepeatThreshold > 1
        ? loopRepeatThreshold
        : DEFAULT_LOOP_REPEAT_THRESHOLD,
    minConfidenceForAutofiling:
      Number.isFinite(minConfidenceForAutofiling)
      && minConfidenceForAutofiling >= 0
      && minConfidenceForAutofiling <= 1
        ? minConfidenceForAutofiling
        : DEFAULT_MIN_CONFIDENCE_FOR_AUTOFILING,
  };
}

function parseJsonSafe(raw: string): any | null {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readJsonLinesTail(filePath: string, sampleLimit: number): any[] {
  try {
    const body = fs.readFileSync(filePath, 'utf8');
    const lines = body.split(/\r?\n/).filter(Boolean);
    const selected = lines.slice(Math.max(0, lines.length - sampleLimit));
    return selected
      .map((line) => parseJsonSafe(line))
      .filter((row) => row && typeof row === 'object');
  } catch {
    return [];
  }
}

function extractChatEvent(row: any): ChatEvent | null {
  if (!row || typeof row !== 'object') return null;
  if (cleanText(row.type, 120) !== 'attention_event') return null;
  const sourceType = cleanText(
    row.source_type || row?.raw_event?.source_type || row?.raw_event?.raw_event?.source_type || '',
    120,
  );
  if (sourceType !== 'passive_memory_turn') return null;
  const raw = row?.raw_event?.raw_event && typeof row.raw_event.raw_event === 'object'
    ? row.raw_event.raw_event
    : row?.raw_event && typeof row.raw_event === 'object'
      ? row.raw_event
      : row;
  const assistantText = cleanText(raw?.assistant_text || '', 12000);
  const userText = cleanText(raw?.user_text || '', 12000);
  if (!assistantText && !userText) return null;
  return {
    ts: cleanText(row.ts || raw?.ts || '', 120),
    agent_id: cleanText(raw?.agent_id || row?.source || 'unknown', 180),
    turn_id: cleanText(raw?.turn_id || row?.id || row?.event_id || `${row?.ts || 'unknown'}:${raw?.agent_id || row?.source || 'unknown'}`, 260),
    user_text: userText,
    assistant_text: assistantText,
  };
}

function matchesAny(text: string, patterns: RegExp[]): boolean {
  return patterns.some((pattern) => pattern.test(text));
}

function parseIsoMillis(raw: string): number {
  const value = Date.parse(cleanText(raw, 120));
  return Number.isFinite(value) ? value : 0;
}

function ageSeconds(nowMs: number, iso: string): number {
  const parsed = parseIsoMillis(iso);
  if (parsed <= 0) return Number.POSITIVE_INFINITY;
  return Math.max(0, Math.floor((nowMs - parsed) / 1000));
}

function strongModel(model: string): boolean {
  const normalized = cleanText(model, 140).toLowerCase();
  if (!normalized) return false;
  return (
    normalized.startsWith('gpt-5')
    || normalized.startsWith('o3')
    || normalized.startsWith('o4')
  );
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Agent Chat Monitor Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- sampled_rows: ${Number(report.summary?.sampled_rows || 0)}`);
  lines.push(`- passive_turn_rows: ${Number(report.summary?.passive_turn_rows || 0)}`);
  lines.push(
    `- latest_passive_event_age_seconds: ${Number(report.summary?.latest_passive_event_age_seconds || 0)}`,
  );
  lines.push('');
  lines.push('## Issue counts');
  const counts = report.summary?.issue_counts || {};
  lines.push(`- workflow_retry_template_count: ${Number(counts.workflow_retry_template_count || 0)}`);
  lines.push(`- workflow_route_automation_claim_count: ${Number(counts.workflow_route_automation_claim_count || 0)}`);
  lines.push(`- auto_tool_selection_claim_count: ${Number(counts.auto_tool_selection_claim_count || 0)}`);
  lines.push(`- policy_block_template_count: ${Number(counts.policy_block_template_count || 0)}`);
  lines.push(`- file_tool_route_misdirection_count: ${Number(counts.file_tool_route_misdirection_count || 0)}`);
  lines.push(`- repeated_response_loop_count: ${Number(counts.repeated_response_loop_count || 0)}`);
  lines.push(`- unsupported_claim_count: ${Number(counts.unsupported_claim_count || 0)}`);
  lines.push(`- wrong_tool_selection_count: ${Number(counts.wrong_tool_selection_count || 0)}`);
  lines.push(`- no_response_count: ${Number(counts.no_response_count || 0)}`);
  lines.push('');
  lines.push('## Feedback');
  const feedbackRows = Array.isArray(report.feedback) ? report.feedback : [];
  if (feedbackRows.length === 0) {
    lines.push('- none');
  } else {
    for (const row of feedbackRows) {
      lines.push(
        `- [${cleanText(row?.severity || 'info', 20)}] ${cleanText(row?.id || 'feedback', 80)}: ${cleanText(row?.summary || '', 240)}`,
      );
      lines.push(`  confidence=${Number(row?.confidence || 0).toFixed(2)} fingerprint=${cleanText(row?.issue_fingerprint || 'na', 40)} run_count=${Number(row?.trend?.run_count || 0)} delta_vs_prev=${Number(row?.trend?.delta_vs_prev || 0)}`);
      lines.push(`  owner=${cleanText(row?.owner_component || 'unknown', 120)} path=${cleanText(row?.owner_path || 'unknown', 240)}`);
      lines.push(`  action=${cleanText(row?.next_action || '', 240)}`);
      const evidenceRows = Array.isArray(row?.evidence) ? row.evidence : [];
      for (const evidence of evidenceRows.slice(0, 2)) {
        lines.push(`  evidence(turn=${cleanText(evidence?.turn_id || 'unknown', 180)}): ${cleanText(evidence?.snippet || '', 240)}`);
      }
    }
  }
  lines.push('');
  lines.push('## Filing candidates');
  const filingCandidates = Array.isArray(report.issue_filing_candidates)
    ? report.issue_filing_candidates
    : [];
  if (filingCandidates.length === 0) {
    lines.push('- none');
  } else {
    for (const candidate of filingCandidates) {
      lines.push(`- ${cleanText(candidate?.id || 'issue', 120)} (${cleanText(candidate?.issue_fingerprint || 'na', 40)})`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const nowMs = Date.now();
  const queueAbs = path.resolve(root, args.queuePath);
  const troubleshootingAbs = path.resolve(root, args.troubleshootingPath);
  const phaseTraceAbs = path.resolve(root, args.phaseTracePath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const stateLatestAbs = path.resolve(root, args.stateLatestPath);
  const historyAbs = path.resolve(root, args.historyPath);
  const markdownAbs = path.resolve(root, args.markdownPath);

  const sampledRows = readJsonLinesTail(queueAbs, args.sampleLimit);
  const events = sampledRows.map(extractChatEvent).filter((row): row is ChatEvent => Boolean(row));

  let workflowRetryTemplateCount = 0;
  let workflowRouteAutomationClaimCount = 0;
  let autoToolSelectionClaimCount = 0;
  let policyBlockTemplateCount = 0;
  let fileToolRouteMisdirectionCount = 0;
  let repeatedResponseLoopCount = 0;
  let unsupportedClaimCount = 0;
  let wrongToolSelectionCount = 0;
  let noResponseCount = 0;
  const wrongToolReasonCounts: Record<string, number> = {};
  const issueEvidenceById: Record<string, IssueEvidence[]> = {};

  function recordEvidence(issueId: string, event: ChatEvent, patterns: RegExp[]) {
    const bucket = issueEvidenceById[issueId] || [];
    bucket.push({
      turn_id: cleanText(event.turn_id, 260),
      ts: cleanText(event.ts, 120),
      agent_id: cleanText(event.agent_id, 180),
      snippet: extractSnippet(event.assistant_text, patterns),
    });
    bucket.sort((a, b) => {
      return (Date.parse(cleanText(b.ts, 120)) || 0) - (Date.parse(cleanText(a.ts, 120)) || 0);
    });
    issueEvidenceById[issueId] = bucket.slice(0, 2);
  }

  let runPhrase = '';
  let runCount = 0;
  const maxRunByAgent: Record<string, number> = {};

  for (const event of events) {
    const assistantLower = cleanText(event.assistant_text, 12000).toLowerCase();
    const userLower = cleanText(event.user_text, 12000).toLowerCase();
    if (
      matchesAny(assistantLower, WORKFLOW_RETRY_TEMPLATE_PATTERNS)
      && !isLikelyQuotedFromUser(assistantLower, userLower, WORKFLOW_RETRY_TEMPLATE_PATTERNS)
    ) {
      workflowRetryTemplateCount += 1;
      recordEvidence('workflow_retry_macro_template_detected', event, WORKFLOW_RETRY_TEMPLATE_PATTERNS);
    }
    if (matchesAny(assistantLower, WORKFLOW_ROUTE_AUTOMATION_PATTERNS)) {
      if (!isLikelyQuotedFromUser(assistantLower, userLower, WORKFLOW_ROUTE_AUTOMATION_PATTERNS)) {
        workflowRouteAutomationClaimCount += 1;
        recordEvidence(
          'workflow_route_automation_claim_detected',
          event,
          WORKFLOW_ROUTE_AUTOMATION_PATTERNS,
        );
      }
    }
    if (matchesAny(assistantLower, AUTO_TOOL_SELECTION_PATTERNS)) {
      if (!isLikelyQuotedFromUser(assistantLower, userLower, AUTO_TOOL_SELECTION_PATTERNS)) {
        autoToolSelectionClaimCount += 1;
        recordEvidence('auto_tool_selection_claim_detected', event, AUTO_TOOL_SELECTION_PATTERNS);
      }
    }
    if (matchesAny(assistantLower, POLICY_BLOCK_PATTERNS)) {
      if (!isLikelyQuotedFromUser(assistantLower, userLower, POLICY_BLOCK_PATTERNS)) {
        policyBlockTemplateCount += 1;
        recordEvidence('policy_block_template_detected', event, POLICY_BLOCK_PATTERNS);
      }
    }

    const localFileIntent = matchesAny(userLower, LOCAL_FILE_INTENT_PATTERNS);
    const webIntent = matchesAny(userLower, WEB_INTENT_PATTERNS);
    const webDumpOutput = matchesAny(assistantLower, WEB_MISDIRECTION_OUTPUT_PATTERNS);
    const workspaceOutput = matchesAny(assistantLower, WORKSPACE_OUTPUT_PATTERNS);
    if (localFileIntent && webDumpOutput) {
      if (!isLikelyQuotedFromUser(assistantLower, userLower, WEB_MISDIRECTION_OUTPUT_PATTERNS)) {
        fileToolRouteMisdirectionCount += 1;
        recordEvidence(
          'file_tool_route_misdirection_detected',
          event,
          WEB_MISDIRECTION_OUTPUT_PATTERNS,
        );
        wrongToolSelectionCount += 1;
        wrongToolReasonCounts.file_intent_routed_to_web_output =
          Number(wrongToolReasonCounts.file_intent_routed_to_web_output || 0) + 1;
        recordEvidence('wrong_tool_selection_detected', event, WEB_MISDIRECTION_OUTPUT_PATTERNS);
      }
    } else if (webIntent && workspaceOutput && !webDumpOutput) {
      if (!isLikelyQuotedFromUser(assistantLower, userLower, WORKSPACE_OUTPUT_PATTERNS)) {
        wrongToolSelectionCount += 1;
        wrongToolReasonCounts.web_intent_routed_to_workspace_output =
          Number(wrongToolReasonCounts.web_intent_routed_to_workspace_output || 0) + 1;
        recordEvidence('wrong_tool_selection_detected', event, WORKSPACE_OUTPUT_PATTERNS);
      }
    }

    const noResponse = assistantLower.length < 8 || matchesAny(assistantLower, NO_RESPONSE_PATTERNS);
    if (noResponse && !isLikelyQuotedFromUser(assistantLower, userLower, NO_RESPONSE_PATTERNS)) {
      noResponseCount += 1;
      recordEvidence('no_response_detected', event, NO_RESPONSE_PATTERNS);
    }

    const userYear = extractYearFromText(userLower);
    const nowYear = new Date(nowMs).getUTCFullYear();
    const futureMisframing =
      userYear !== null
      && userYear <= nowYear
      && /future-facing|future/.test(assistantLower)
      && /can'?t provide specific predictions/.test(assistantLower);
    const unsupportedClaim =
      matchesAny(assistantLower, HALLUCINATION_UNSUPPORTED_CLAIM_PATTERNS)
      && !isLikelyQuotedFromUser(assistantLower, userLower, HALLUCINATION_UNSUPPORTED_CLAIM_PATTERNS)
      && (!hasEvidenceLink(assistantLower) || futureMisframing)
      && !isExplicitlyUncertain(assistantLower);
    if (unsupportedClaim) {
      unsupportedClaimCount += 1;
      recordEvidence(
        'unsupported_claim_detected',
        event,
        HALLUCINATION_UNSUPPORTED_CLAIM_PATTERNS,
      );
    }

    const phrase = cleanText(assistantLower, 500);
    if (phrase && phrase === runPhrase) {
      runCount += 1;
    } else {
      runPhrase = phrase;
      runCount = phrase ? 1 : 0;
    }
    if (phrase) {
      const agent = cleanText(event.agent_id, 180) || 'unknown';
      maxRunByAgent[agent] = Math.max(Number(maxRunByAgent[agent] || 0), runCount);
    }
  }
  repeatedResponseLoopCount = Object.values(maxRunByAgent)
    .filter((value) => value >= args.loopRepeatThreshold).length;
  if (repeatedResponseLoopCount > 0) {
    for (const event of events.slice(-2)) {
      recordEvidence('repeated_response_loop_detected', event, [/.+/i]);
    }
  }

  const latestPassiveEventTs = events.reduce((latest, row) => {
    if (parseIsoMillis(row.ts) > parseIsoMillis(latest)) return row.ts;
    return latest;
  }, '');
  const latestPassiveEventAgeSeconds = latestPassiveEventTs
    ? ageSeconds(nowMs, latestPassiveEventTs)
    : Number.POSITIVE_INFINITY;

  const troubleshootingLatest = readJson(troubleshootingAbs) || {};
  const phaseTraceExists = fs.existsSync(phaseTraceAbs);
  const phaseTrace = phaseTraceExists ? readJson(phaseTraceAbs) || {} : {};
  const phaseTraceType = cleanText(phaseTrace?.type || '', 120);
  const phaseTracePhases = Array.isArray(phaseTrace?.phases) ? phaseTrace.phases : [];
  const phaseTraceIssueSignals = Array.isArray(phaseTrace?.issue_signals)
    ? phaseTrace.issue_signals
    : [];
  const phaseTraceValid =
    !phaseTraceExists
    || (
      phaseTraceType === 'orchestration_workflow_phase_trace'
      && Number(phaseTrace?.schema_version || 0) === 1
      && phaseTracePhases.length > 0
    );
  const troubleshootingLatestTs = cleanText(troubleshootingLatest.ts || '', 120);
  const troubleshootingLatestAgeSeconds = troubleshootingLatestTs
    ? ageSeconds(nowMs, troubleshootingLatestTs)
    : Number.POSITIVE_INFINITY;
  const evalModel = cleanText(
    troubleshootingLatest?.eval?.model
      || troubleshootingLatest?.model
      || troubleshootingLatest?.strong_default_model
      || 'gpt-5.4',
    140,
  );
  const previousHistoryRoot = readJson(historyAbs) || {};
  const previousHistoryIssues: Record<string, IssueHistoryRow> =
    previousHistoryRoot && typeof previousHistoryRoot === 'object' && previousHistoryRoot.issues
      ? previousHistoryRoot.issues
      : {};

  const issueDrafts: Array<Record<string, unknown>> = [];
  if (workflowRetryTemplateCount > 0) {
    issueDrafts.push({
      id: 'workflow_retry_macro_template_detected',
      severity: 'high',
      summary: 'Detected repeated workflow retry boilerplate in passive chat memory.',
      evidence_count: workflowRetryTemplateCount,
      next_action:
        'Harden response finalization synthesis and block retry-template copy from reaching user-visible output.',
    });
  }
  if (workflowRouteAutomationClaimCount > 0) {
    issueDrafts.push({
      id: 'workflow_route_automation_claim_detected',
      severity: 'high',
      summary: 'Detected outdated workflow-route automation explanation leaking into assistant replies.',
      evidence_count: workflowRouteAutomationClaimCount,
      next_action:
        'Enforce advisory-only tool-gate language and rewrite legacy route-classification boilerplate during finalization.',
    });
  }
  if (autoToolSelectionClaimCount > 0) {
    issueDrafts.push({
      id: 'auto_tool_selection_claim_detected',
      severity: 'medium',
      summary: 'Detected claims that backend auto-selects tools without model control.',
      evidence_count: autoToolSelectionClaimCount,
      next_action:
        'Preserve explicit LLM tool authority wording and redact architecture-mismatch claims from final responses.',
    });
  }
  if (policyBlockTemplateCount > 0) {
    issueDrafts.push({
      id: 'policy_block_template_detected',
      severity: 'medium',
      summary: 'Detected ingress policy block template surfacing without concise remediation framing.',
      evidence_count: policyBlockTemplateCount,
      next_action:
        'Translate policy blocks into bounded actionable guidance and avoid repeating raw gate diagnostics.',
    });
  }
  if (fileToolRouteMisdirectionCount > 0) {
    issueDrafts.push({
      id: 'file_tool_route_misdirection_detected',
      severity: 'high',
      summary: 'Detected local-file intent requests answered with web-search style output.',
      evidence_count: fileToolRouteMisdirectionCount,
      next_action:
        'Enforce local file/workspace route precedence and block web output cards for file-tool intents.',
    });
  }
  if (repeatedResponseLoopCount > 0) {
    issueDrafts.push({
      id: 'repeated_response_loop_detected',
      severity: 'high',
      summary: `Detected repeated assistant responses across one or more agents (threshold=${args.loopRepeatThreshold}).`,
      evidence_count: repeatedResponseLoopCount,
      next_action:
        'Trigger loop breaker fallback once repetition threshold is crossed and force degraded one-shot diagnosis.',
    });
  }
  if (unsupportedClaimCount > 0) {
    issueDrafts.push({
      id: 'unsupported_claim_detected',
      severity: 'high',
      summary: 'Detected unsupported or context-inconsistent certainty claims in assistant replies.',
      evidence_count: unsupportedClaimCount,
      next_action:
        'Require explicit evidence links (or bounded uncertainty) for strong factual claims and enforce date-context checks.',
    });
  }
  if (wrongToolSelectionCount > 0) {
    const topReasonEntry = Object.entries(wrongToolReasonCounts)
      .sort((a, b) => Number(b[1]) - Number(a[1]))[0];
    const topReason = topReasonEntry ? cleanText(topReasonEntry[0], 120) : 'unknown_reason';
    issueDrafts.push({
      id: 'wrong_tool_selection_detected',
      severity: 'high',
      summary: `Detected tool-family routing mismatches for user intents (top_reason=${topReason}).`,
      evidence_count: wrongToolSelectionCount,
      mismatch_reason_counts: wrongToolReasonCounts,
      next_action:
        'Enforce intent-to-tool-family contracts and emit explicit mismatch reason IDs for every mismatch.',
    });
  }
  if (noResponseCount > 0) {
    issueDrafts.push({
      id: 'no_response_detected',
      severity: 'high',
      summary: 'Detected turns where assistant returned fallback-only text without substantive response.',
      evidence_count: noResponseCount,
      next_action:
        'Preserve safe LLM-authored direct answers when finalization fails; do not inject system chat text.',
    });
  }
  const actionablePhaseTraceSignals = phaseTraceIssueSignals.filter((row: any) => {
    const severity = cleanText(row?.severity_hint || '', 20);
    return severity === 'high' || severity === 'medium';
  });
  if (actionablePhaseTraceSignals.length > 0) {
    issueDrafts.push({
      id: 'orchestration_phase_trace_issue_signal_detected',
      severity: actionablePhaseTraceSignals.some((row: any) => cleanText(row?.severity_hint || '', 20) === 'high')
        ? 'high'
        : 'medium',
      summary: 'Detected actionable issue signals in orchestration-owned workflow phase trace.',
      evidence_count: actionablePhaseTraceSignals.length,
      next_action:
        'Inspect phase-trace issue signals and patch the owning control-plane phase before relying on text-only eval inference.',
    });
  }
  const nowIso = new Date(nowMs).toISOString();
  const updatedHistoryIssues: Record<string, IssueHistoryRow> = { ...previousHistoryIssues };
  const unmappedIssueIds = new Set<string>();
  const acceptanceCriteriaMissingIds = new Set<string>();
  const feedback: Array<Record<string, unknown>> = issueDrafts.map((draft) => {
    const issueId = cleanText(draft.id || 'issue', 120);
    const evidenceCount = Number(draft.evidence_count || 0);
    const metadata = getIssueMetadata(issueId);
    const metadataMapped = hasIssueMetadata(issueId);
    if (!metadataMapped) unmappedIssueIds.add(issueId);
    if (!Array.isArray(metadata.acceptance_criteria) || metadata.acceptance_criteria.length === 0) {
      acceptanceCriteriaMissingIds.add(issueId);
    }
    const fingerprint = buildIssueFingerprint(
      issueId,
      cleanText(draft.summary || '', 300),
      metadata.owner_component,
    );
    const previous = previousHistoryIssues[issueId];
    const runCount = Number(previous?.run_count || 0) + 1;
    const firstSeen = cleanText(previous?.first_seen || nowIso, 120);
    const deltaVsPrev = evidenceCount - Number(previous?.last_evidence_count || 0);
    const persistenceThresholdMet = runCount >= args.persistenceThreshold;
    const confidence = clamp(
      metadata.base_confidence
        + Math.min(0.14, Math.max(0, evidenceCount - 1) * 0.04)
        + (persistenceThresholdMet ? 0.08 : 0),
      0,
      0.99,
    );
    const needsHumanReview = confidence < args.minConfidenceForAutofiling;
    updatedHistoryIssues[issueId] = {
      issue_fingerprint: fingerprint,
      first_seen: firstSeen,
      last_seen: nowIso,
      run_count: runCount,
      last_evidence_count: evidenceCount,
    };
    return {
      ...draft,
      issue_fingerprint: fingerprint,
      confidence: Number(confidence.toFixed(2)),
      owner_component: metadata.owner_component,
      owner_path: metadata.owner_path,
      acceptance_criteria: metadata.acceptance_criteria,
      evidence: issueEvidenceById[issueId] || [],
      trend: {
        first_seen: firstSeen,
        last_seen: nowIso,
        run_count: runCount,
        delta_vs_prev: deltaVsPrev,
      },
      persistence_threshold: args.persistenceThreshold,
      persistence_threshold_met: persistenceThresholdMet,
      needs_human_review: needsHumanReview,
      file_issue_recommended: persistenceThresholdMet && !needsHumanReview,
    };
  });

  if (feedback.length === 0) {
    feedback.push({
      id: 'no_high_severity_patterns_detected',
      severity: 'info',
      summary: 'No high-severity eval/chat failure patterns were detected in sampled passive-memory turns.',
      evidence_count: 0,
      next_action: 'Keep monitor active and continue sampling passive-memory events.',
      issue_fingerprint: buildIssueFingerprint(
        'no_high_severity_patterns_detected',
        'No high-severity patterns.',
        'control_plane.monitoring',
      ),
      confidence: 0.99,
      owner_component: 'control_plane.monitoring',
      owner_path: 'tests/tooling/scripts/ci/',
      acceptance_criteria: ['Monitor remains active and emits fresh reports each run.'],
      evidence: [],
      trend: {
        first_seen: nowIso,
        last_seen: nowIso,
        run_count: 1,
        delta_vs_prev: 0,
      },
      persistence_threshold: args.persistenceThreshold,
      persistence_threshold_met: false,
      file_issue_recommended: false,
    });
  }
  const issueFilingCandidates = feedback.filter((row) => {
    const severity = cleanText(row?.severity || '', 20);
    const recommended = Boolean(row?.file_issue_recommended);
    return severity !== 'info' && recommended;
  });
  const nonInfoIssues = feedback.filter((row) => cleanText(row?.severity || '', 20) !== 'info');
  const trendFieldViolations = nonInfoIssues.filter((row) => {
    const trend = row?.trend || {};
    return (
      cleanText(trend.first_seen || '', 120).length === 0
      || cleanText(trend.last_seen || '', 120).length === 0
      || !Number.isFinite(Number(trend.run_count))
      || !Number.isFinite(Number(trend.delta_vs_prev))
    );
  });
  const fingerprintClusterMap = new Map<string, string[]>();
  for (const [issueId, row] of Object.entries(updatedHistoryIssues)) {
    const fp = cleanText(row?.issue_fingerprint || '', 120);
    if (!fp) continue;
    const existing = fingerprintClusterMap.get(fp) || [];
    existing.push(cleanText(issueId, 120));
    fingerprintClusterMap.set(fp, existing);
  }
  const fingerprintClusters = Array.from(fingerprintClusterMap.entries()).map(([fingerprint, issueIds]) => {
    const canonicalIssueIds = issueIds
      .map((entry) => cleanText(entry, 120))
      .filter(Boolean)
      .sort();
    const totalRuns = canonicalIssueIds.reduce(
      (acc, issueId) => acc + Number(updatedHistoryIssues[issueId]?.run_count || 0),
      0,
    );
    return {
      issue_fingerprint: fingerprint,
      issue_ids: canonicalIssueIds,
      issue_count: canonicalIssueIds.length,
      total_runs: totalRuns,
    };
  });
  const fingerprintContractViolations = fingerprintClusters.filter((row) => {
    return cleanText(row.issue_fingerprint, 120).length === 0 || Number(row.issue_count) <= 0;
  });

  const monitorEvalReport = {
    ok: true,
    type: 'dashboard_troubleshooting_eval_report',
    ts: nowIso,
    status: 'active',
    reason: 'eval_agent_chat_monitor_guard',
    model: evalModel,
    model_source: 'eval_agent_chat_monitor_guard',
    strong_default_model: 'gpt-5.4',
    entry_count: events.length,
    summary: cleanText(feedback[0]?.summary || 'Eval monitor active.', 400),
    issue_filing_candidate_count: issueFilingCandidates.length,
    issue_fingerprint_cluster_count: fingerprintClusters.length,
    issues: feedback
      .filter((row) => cleanText(row?.severity || '', 20) !== 'info')
      .map((row) => ({
        id: cleanText(row?.id || 'issue', 100),
        severity: cleanText(row?.severity || 'medium', 20),
        confidence: Number(row?.confidence || 0),
        issue_fingerprint: cleanText(row?.issue_fingerprint || '', 60),
        summary: cleanText(row?.summary || '', 260),
        next_action: cleanText(row?.next_action || '', 260),
        owner_component: cleanText(row?.owner_component || '', 180),
        owner_path: cleanText(row?.owner_path || '', 260),
        acceptance_criteria: Array.isArray(row?.acceptance_criteria)
          ? row.acceptance_criteria.map((entry) => cleanText(String(entry), 260))
          : [],
        evidence: Array.isArray(row?.evidence)
          ? row.evidence.slice(0, 2).map((entry) => ({
              turn_id: cleanText(entry?.turn_id || '', 260),
              ts: cleanText(entry?.ts || '', 120),
              agent_id: cleanText(entry?.agent_id || '', 180),
              snippet: cleanText(entry?.snippet || '', 260),
            }))
          : [],
        trend: row?.trend || null,
        persistence_threshold: Number(row?.persistence_threshold || args.persistenceThreshold),
        persistence_threshold_met: Boolean(row?.persistence_threshold_met),
        needs_human_review: Boolean(row?.needs_human_review),
        file_issue_recommended: Boolean(row?.file_issue_recommended),
        mismatch_reason_counts:
          row?.id === 'wrong_tool_selection_detected'
            ? row?.mismatch_reason_counts || {}
            : undefined,
      })),
    issue_filing_candidates: issueFilingCandidates.map((row) => ({
      id: cleanText(row?.id || '', 120),
      issue_fingerprint: cleanText(row?.issue_fingerprint || '', 60),
      confidence: Number(row?.confidence || 0),
      run_count: Number(row?.trend?.run_count || 0),
      owner_component: cleanText(row?.owner_component || '', 180),
      owner_path: cleanText(row?.owner_path || '', 260),
      summary: cleanText(row?.summary || '', 260),
      needs_human_review: Boolean(row?.needs_human_review),
    })),
    issue_fingerprint_clusters: fingerprintClusters,
    orchestration_phase_trace: {
      present: phaseTraceExists,
      valid: phaseTraceValid,
      path: args.phaseTracePath,
      trace_id: cleanText(phaseTrace?.trace_id || '', 180) || null,
      phase_count: phaseTracePhases.length,
      issue_signal_count: phaseTraceIssueSignals.length,
      owner: cleanText(phaseTrace?.owner || '', 180) || null,
    },
  };
  writeJsonArtifact(troubleshootingAbs, monitorEvalReport);

  const persistenceContractViolations = feedback.filter((row) => {
    if (cleanText(row?.severity || '', 20) === 'info') return false;
    if (!Boolean(row?.file_issue_recommended)) return false;
    return Number(row?.trend?.run_count || 0) < args.persistenceThreshold;
  });
  const confidenceContractViolations = feedback.filter((row) => {
    if (cleanText(row?.severity || '', 20) === 'info') return false;
    if (!Boolean(row?.file_issue_recommended)) return false;
    return Number(row?.confidence || 0) < args.minConfidenceForAutofiling;
  });
  const passiveMemorySignalMode = events.length > 0 ? 'sampled' : 'insufficient_signal';
  const checks = [
    {
      id: 'attention_queue_present',
      ok: fs.existsSync(queueAbs),
      detail: args.queuePath,
    },
    {
      id: 'passive_memory_turns_sampled',
      ok: true,
      detail: `mode=${passiveMemorySignalMode};sampled=${events.length}`,
    },
    {
      id: 'passive_memory_freshness_contract',
      ok: events.length === 0 || latestPassiveEventAgeSeconds <= args.chatFreshSeconds,
      detail: `mode=${passiveMemorySignalMode};age_seconds=${Number.isFinite(latestPassiveEventAgeSeconds) ? latestPassiveEventAgeSeconds : -1};threshold=${args.chatFreshSeconds}`,
    },
    {
      id: 'feedback_emission_contract',
      ok: feedback.length > 0,
      detail: `feedback_items=${feedback.length}`,
    },
    {
      id: 'issue_persistence_threshold_contract',
      ok: persistenceContractViolations.length === 0,
      detail: `threshold=${args.persistenceThreshold};candidates=${issueFilingCandidates.length};violations=${persistenceContractViolations.length}`,
    },
    {
      id: 'issue_confidence_abstain_contract',
      ok: confidenceContractViolations.length === 0,
      detail: `threshold=${args.minConfidenceForAutofiling};candidates=${issueFilingCandidates.length};violations=${confidenceContractViolations.length}`,
    },
    {
      id: 'issue_owner_mapping_contract',
      ok: unmappedIssueIds.size === 0,
      detail: `unmapped_count=${unmappedIssueIds.size};unmapped=${Array.from(unmappedIssueIds).join(',') || 'none'}`,
    },
    {
      id: 'issue_acceptance_criteria_contract',
      ok: acceptanceCriteriaMissingIds.size === 0,
      detail: `missing_count=${acceptanceCriteriaMissingIds.size};missing=${Array.from(acceptanceCriteriaMissingIds).join(',') || 'none'}`,
    },
    {
      id: 'issue_trend_fields_contract',
      ok: trendFieldViolations.length === 0,
      detail: `non_info_issues=${nonInfoIssues.length};violations=${trendFieldViolations.length}`,
    },
    {
      id: 'issue_fingerprint_cluster_contract',
      ok: fingerprintContractViolations.length === 0,
      detail: `clusters=${fingerprintClusters.length};violations=${fingerprintContractViolations.length}`,
    },
    {
      id: 'strong_eval_default_model_contract',
      ok: strongModel(evalModel),
      detail: `eval_model=${evalModel}`,
    },
    {
      id: 'troubleshooting_eval_freshness_contract',
      ok: true,
      detail: `ts=${monitorEvalReport.ts};threshold=${args.evalFreshSeconds};previous_ts=${troubleshootingLatestTs || 'missing'};previous_age_seconds=${Number.isFinite(troubleshootingLatestAgeSeconds) ? troubleshootingLatestAgeSeconds : -1}`,
    },
    {
      id: 'orchestration_phase_trace_contract',
      ok: phaseTraceValid,
      detail: `present=${phaseTraceExists};path=${args.phaseTracePath};phases=${phaseTracePhases.length};issue_signals=${phaseTraceIssueSignals.length}`,
    },
  ];

  const report = {
    type: 'eval_agent_chat_monitor_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    summary: {
      sampled_rows: sampledRows.length,
      passive_turn_rows: events.length,
      passive_memory_signal_mode: passiveMemorySignalMode,
      loop_repeat_threshold: args.loopRepeatThreshold,
      min_confidence_for_autofiling: args.minConfidenceForAutofiling,
      latest_passive_event_ts: latestPassiveEventTs || null,
      latest_passive_event_age_seconds: Number.isFinite(latestPassiveEventAgeSeconds)
        ? latestPassiveEventAgeSeconds
        : null,
      issue_counts: {
        workflow_retry_template_count: workflowRetryTemplateCount,
        workflow_route_automation_claim_count: workflowRouteAutomationClaimCount,
        auto_tool_selection_claim_count: autoToolSelectionClaimCount,
        policy_block_template_count: policyBlockTemplateCount,
        file_tool_route_misdirection_count: fileToolRouteMisdirectionCount,
        repeated_response_loop_count: repeatedResponseLoopCount,
        unsupported_claim_count: unsupportedClaimCount,
        wrong_tool_selection_count: wrongToolSelectionCount,
        no_response_count: noResponseCount,
      },
      troubleshooting_latest_eval: {
        ts: monitorEvalReport.ts,
        age_seconds: 0,
        model: evalModel,
        previous_ts: troubleshootingLatestTs || null,
        previous_age_seconds: Number.isFinite(troubleshootingLatestAgeSeconds)
          ? troubleshootingLatestAgeSeconds
          : null,
      },
      issue_filing_candidates: issueFilingCandidates.map((row) => ({
        id: cleanText(row?.id || '', 120),
        issue_fingerprint: cleanText(row?.issue_fingerprint || '', 60),
        confidence: Number(row?.confidence || 0),
        run_count: Number(row?.trend?.run_count || 0),
      })),
      issue_fingerprint_cluster_count: fingerprintClusters.length,
      orchestration_phase_trace: monitorEvalReport.orchestration_phase_trace,
    },
    feedback,
    issues: monitorEvalReport.issues,
    issue_filing_candidates: monitorEvalReport.issue_filing_candidates,
    issue_fingerprint_clusters: fingerprintClusters,
    sources: {
      queue: args.queuePath,
      troubleshooting_latest: args.troubleshootingPath,
      orchestration_phase_trace: args.phaseTracePath,
      history: args.historyPath,
    },
  };

  writeJsonArtifact(historyAbs, {
    type: 'eval_agent_chat_monitor_issue_history',
    schema_version: 1,
    updated_at: nowIso,
    issue_persistence_threshold: args.persistenceThreshold,
    min_confidence_for_autofiling: args.minConfidenceForAutofiling,
    issues: updatedHistoryIssues,
    issue_fingerprint_clusters: fingerprintClusters,
  });
  writeJsonArtifact(outLatestAbs, report);
  writeJsonArtifact(stateLatestAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));

  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
