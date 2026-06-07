#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::text-boundary.
//
// Gateway owns the bounded text normalization used at external request and
// projection boundaries. Legacy adapter/dashboard hosts may call these helpers,
// but must not define their own Gateway text hygiene primitives.

'use strict';

function stripGatewayTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}

function cleanGatewayText(value, maxLen = 200) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function cleanGatewayDisplayText(value, maxLen = 24000) {
  return stripGatewayTerminalControls(value)
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .trim()
    .slice(0, maxLen);
}

module.exports = {
  stripGatewayTerminalControls,
  cleanGatewayText,
  cleanGatewayDisplayText,
};
