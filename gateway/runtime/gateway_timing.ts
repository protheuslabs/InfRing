#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::timing.
//
// Gateway owns small timing primitives used by boundary wait/retry loops.
// Legacy adapter/dashboard hosts may call these helpers, but should not define
// their own Gateway lifecycle timing primitives.

'use strict';

async function sleepGatewayMs(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

function gatewayNowIso() {
  return new Date().toISOString();
}

module.exports = {
  sleepGatewayMs,
  gatewayNowIso,
};
