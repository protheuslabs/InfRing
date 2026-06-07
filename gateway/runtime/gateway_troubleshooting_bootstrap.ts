#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::troubleshooting-bootstrap.
//
// Gateway owns compatibility bootstrap projection for dashboard troubleshooting
// snapshots. Concrete host process/output capabilities are injected by the
// legacy dashboard host so this module does not become a process authority.

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {
  writeGatewayJson: writeJson,
  writeGatewayJsonIfMissing: writeJsonIfMissing,
  appendGatewayJsonl: appendJsonl,
  deterministicGatewayReceiptHash: deterministicReceiptHash,
} = require('./gateway_artifacts.ts');
const { gatewayNowIso: nowIso } = require('./gateway_timing.ts');
const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

const DEFAULT_EVAL_MODEL = 'gpt-5.4';
const DEFAULT_MAX_RECENT = 10;

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function createGatewayTroubleshootingBootstrap(options = {}) {
  const root = options.root || process.cwd();
  const statusDir = options.statusDir || path.resolve(
    root,
    'client',
    'runtime',
    'local',
    'state',
    'ui',
    'infring_dashboard',
  );
  const maxRecent = Number.isFinite(Number(options.maxRecent))
    ? Math.max(1, Number(options.maxRecent))
    : DEFAULT_MAX_RECENT;
  const evalModel = cleanText(options.evalModel || DEFAULT_EVAL_MODEL, 80) || DEFAULT_EVAL_MODEL;
  const invokeBridge = typeof options.invokeInfringOpsViaBridge === 'function'
    ? options.invokeInfringOpsViaBridge
    : null;
  const runOps = typeof options.runInfringOps === 'function' ? options.runInfringOps : () => 1;
  const stdout = options.stdout && typeof options.stdout.write === 'function' ? options.stdout : process.stdout;
  const stderr = options.stderr && typeof options.stderr.write === 'function' ? options.stderr : process.stderr;

  const statusSnapshotPath = path.resolve(statusDir, 'latest_snapshot.json');
  const troubleshootingDir = path.resolve(statusDir, 'troubleshooting');
  const recentWorkflowsPath = path.resolve(troubleshootingDir, 'recent_workflows.json');
  const evalQueuePath = path.resolve(troubleshootingDir, 'eval_queue.json');
  const issueOutboxPath = path.resolve(troubleshootingDir, 'issue_outbox.json');
  const latestSnapshotPath = path.resolve(troubleshootingDir, 'latest_snapshot.json');
  const snapshotHistoryPath = path.resolve(troubleshootingDir, 'snapshot_history.jsonl');
  const latestEvalReportPath = path.resolve(troubleshootingDir, 'latest_eval_report.json');

  function readRecentActionRows(limit = maxRecent) {
    const historyPath = path.resolve(statusDir, 'actions', 'history.jsonl');
    let raw = '';
    try {
      raw = fs.readFileSync(historyPath, 'utf8');
    } catch {
      return [];
    }
    const lines = raw
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    if (!lines.length) return [];
    const out = [];
    for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
      let parsed = null;
      try {
        parsed = JSON.parse(lines[idx]);
      } catch {
        parsed = null;
      }
      if (!parsed || cleanText(parsed.action || '', 80) !== 'app.chat') continue;
      out.push(parsed);
      if (out.length >= limit) break;
    }
    return out.reverse();
  }

  function summarizeBootstrapActionRow(actionRow, previousSummary) {
    const laneOk = actionRow?.ok === true;
    const laneStatus = Number.isFinite(Number(actionRow?.lane_status))
      ? Number(actionRow.lane_status)
      : laneOk
        ? 0
        : 1;
    const payload = actionRow && typeof actionRow.payload === 'object' && actionRow.payload
      ? actionRow.payload
      : {};
    const input = cleanText(payload.input || payload.message || payload.prompt || '', 240);
    const tools = Array.isArray(actionRow?.tool_receipts)
      ? actionRow.tool_receipts
      : Array.isArray(payload?.tool_receipts)
        ? payload.tool_receipts
        : [];
    const toolSummary = tools
      .slice(0, 3)
      .map((row) => cleanText(row?.name || row?.tool || '', 40))
      .filter(Boolean)
      .join(',');
    const laneLabel = laneOk ? 'lane_ok' : `lane_fail(${laneStatus})`;
    const inputLabel = input ? `input:${cleanText(input, 64)}` : 'input:empty';
    const toolLabel = toolSummary ? `tools:${toolSummary}` : 'tools:none';
    const summary = `${laneLabel};${inputLabel};${toolLabel}`;
    if (!previousSummary) return summary;
    return `${summary};prev:${cleanText(previousSummary, 120)}`;
  }

  function bootstrapRecentWorkflowEntries() {
    const actionRows = readRecentActionRows(maxRecent);
    const entries = [];
    let previousSummary = '';
    for (let index = 0; index < actionRows.length; index += 1) {
      const row = actionRows[index] || {};
      const payload = row && typeof row.payload === 'object' && row.payload ? row.payload : {};
      const laneOk = row?.ok === true;
      const laneStatus = Number.isFinite(Number(row?.lane_status))
        ? Number(row.lane_status)
        : laneOk
          ? 0
          : 1;
      const summary = summarizeBootstrapActionRow(row, previousSummary);
      previousSummary = summary;
      const entry = {
        workflow_id: cleanText(row?.id || `wf_${index + 1}`, 120) || `wf_${index + 1}`,
        source_sequence: index + 1,
        ts: cleanText(row?.ts || nowIso(), 80),
        lane_ok: laneOk,
        lane_status: laneStatus,
        error_code: cleanText(row?.error_code || row?.error || '', 120).toLowerCase(),
        exchange: {
          user: cleanText(payload.input || payload.message || payload.prompt || '', 1600),
          assistant: cleanText(row?.response || payload.response || '', 2000),
          tool_receipts: Array.isArray(row?.tool_receipts)
            ? row.tool_receipts.slice(0, 12)
            : [],
        },
        process_summary: {
          previous: cleanText(index === 0 ? '' : entries[index - 1]?.process_summary?.current || '', 360),
          current: cleanText(summary, 360),
          source: 'snapshot_compat_bootstrap',
        },
        metadata: {
          source: 'snapshot_compat_bootstrap',
        },
      };
      entry.receipt_hash = deterministicReceiptHash(entry);
      entries.push(entry);
    }
    return entries;
  }

  function writeBridgeOutput(out) {
    if (!out || typeof out !== 'object') return 1;
    if (out.stdout) stdout.write(String(out.stdout));
    if (out.stderr) stderr.write(String(out.stderr));
    if (out.payload && !out.stdout) stdout.write(`${JSON.stringify(out.payload)}\n`);
    const status = Number(out.status);
    return Number.isFinite(status) ? status : 1;
  }

  function bootstrapTroubleshootingFromSnapshot(snapshotPayload) {
    const payload = snapshotPayload && typeof snapshotPayload === 'object' ? snapshotPayload : {};
    const seededEntries = bootstrapRecentWorkflowEntries();
    writeJsonIfMissing(recentWorkflowsPath, {
      ok: true,
      type: 'dashboard_troubleshooting_recent_workflows',
      ts: nowIso(),
      entries: seededEntries,
      receipt_hash: deterministicReceiptHash({
        entries: seededEntries,
        type: 'dashboard_troubleshooting_recent_workflows',
      }),
    });
    writeJsonIfMissing(evalQueuePath, {
      ok: true,
      type: 'dashboard_troubleshooting_eval_queue',
      ts: nowIso(),
      items: [],
      receipt_hash: deterministicReceiptHash({
        items: [],
        type: 'dashboard_troubleshooting_eval_queue',
      }),
    });
    writeJsonIfMissing(issueOutboxPath, {
      ok: true,
      type: 'dashboard_troubleshooting_issue_outbox',
      ts: nowIso(),
      items: [],
      receipt_hash: deterministicReceiptHash({
        items: [],
        type: 'dashboard_troubleshooting_issue_outbox',
      }),
    });
    if (!fs.existsSync(latestSnapshotPath)) {
      const failureCount = seededEntries.filter((row) => row?.lane_ok !== true).length;
      const snapshot = {
        ok: true,
        type: 'dashboard_troubleshooting_snapshot',
        snapshot_id: `snap_${Date.now().toString(36)}`,
        trigger: 'runtime_bootstrap_compat',
        ts: nowIso(),
        failure_count: failureCount,
        entry_count: seededEntries.length,
        entries: seededEntries,
        metadata: {
          source: 'dashboard_snapshot_compat_bootstrap',
          snapshot_receipt_hash: cleanText(payload.receipt_hash || '', 160),
        },
      };
      snapshot.receipt_hash = deterministicReceiptHash(snapshot);
      writeJson(latestSnapshotPath, snapshot);
      appendJsonl(snapshotHistoryPath, snapshot);
    }
    writeJsonIfMissing(latestEvalReportPath, {
      ok: true,
      type: 'dashboard_troubleshooting_eval_report',
      ts: nowIso(),
      status: 'idle',
      reason: 'runtime_bootstrap_compat',
      model: evalModel,
      model_source: 'strong_default_bootstrap',
      strong_default_model: evalModel,
      entry_count: seededEntries.length,
      issues: [],
      summary: 'Eval runtime is initialized and waiting for failure snapshots.',
      receipt_hash: deterministicReceiptHash({
        status: 'idle',
        model: evalModel,
        entry_count: seededEntries.length,
        type: 'dashboard_troubleshooting_eval_report',
      }),
    });
  }

  function runSnapshotWithCompatBootstrap(args, runOptions) {
    const out = invokeBridge ? invokeBridge(['dashboard-ui', ...args], runOptions) : null;
    if (!out) {
      const status = runOps(['dashboard-ui', ...args], runOptions);
      if (Number(status) === 0 && fs.existsSync(statusSnapshotPath)) {
        try {
          const fallbackPayload = JSON.parse(fs.readFileSync(statusSnapshotPath, 'utf8'));
          if (fallbackPayload && typeof fallbackPayload === 'object') {
            bootstrapTroubleshootingFromSnapshot(fallbackPayload);
          }
        } catch {}
      }
      return status;
    }
    const parsedPayload = out.payload && typeof out.payload === 'object'
      ? out.payload
      : parseLastJson(out.stdout || '');
    if (parsedPayload && typeof parsedPayload === 'object') {
      bootstrapTroubleshootingFromSnapshot(parsedPayload);
      if (!out.payload) out.payload = parsedPayload;
    }
    return writeBridgeOutput(out);
  }

  return {
    readRecentActionRows,
    summarizeBootstrapActionRow,
    bootstrapRecentWorkflowEntries,
    bootstrapTroubleshootingFromSnapshot,
    runSnapshotWithCompatBootstrap,
  };
}

module.exports = {
  createGatewayTroubleshootingBootstrap,
  parseGatewayTroubleshootingBridgePayload: parseLastJson,
};
