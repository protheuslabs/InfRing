#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::artifacts.
//
// Gateway owns bounded local artifact helpers used by compatibility hosts for
// status snapshots, troubleshooting seed data, JSONL append streams, and
// deterministic receipt hashes.

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

function ensureGatewayDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function writeGatewayJson(filePath, value) {
  ensureGatewayDir(path.dirname(filePath));
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function writeGatewayJsonIfMissing(filePath, value) {
  if (fs.existsSync(filePath)) return false;
  writeGatewayJson(filePath, value);
  return true;
}

function appendGatewayJsonl(filePath, value) {
  ensureGatewayDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function appendBoundedGatewayJsonl(filePath, value, maxRows) {
  ensureGatewayDir(path.dirname(filePath));
  let rows = [];
  try {
    rows = fs.readFileSync(filePath, 'utf8').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  } catch {}
  rows.push(JSON.stringify(value));
  fs.writeFileSync(filePath, `${rows.slice(-Math.max(1, maxRows || 1)).join('\n')}\n`, 'utf8');
}

function deterministicGatewayReceiptHash(value) {
  try {
    return createHash('sha256').update(JSON.stringify(value)).digest('hex');
  } catch {
    return '';
  }
}

module.exports = {
  ensureGatewayDir,
  writeGatewayJson,
  writeGatewayJsonIfMissing,
  appendGatewayJsonl,
  appendBoundedGatewayJsonl,
  deterministicGatewayReceiptHash,
};
