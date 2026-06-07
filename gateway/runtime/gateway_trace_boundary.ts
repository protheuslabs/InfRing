#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::trace-boundary.
//
// Gateway owns external trace admission and propagation. Legacy adapter hosts
// may serve HTTP while migration continues, but they must delegate trace ID
// minting, incoming header adoption, and boundary metadata to this module.

'use strict';

function sanitizeGatewayTraceId(value) {
  const raw = Array.isArray(value) ? value[0] : value;
  const clean = String(raw || '').trim();
  return /^[A-Za-z0-9_.:-]{8,160}$/.test(clean) ? clean : '';
}

function mintGatewayTraceId() {
  return `trace_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 14)}`;
}

function gatewayRequestTraceId(req) {
  if (!req || typeof req !== 'object') return mintGatewayTraceId();
  if (req.__infringTraceId) return req.__infringTraceId;
  const rawHeader = req.headers && (req.headers['x-infring-trace-id'] || req.headers.traceparent);
  const existing = sanitizeGatewayTraceId(rawHeader);
  req.__infringTraceId = existing || mintGatewayTraceId();
  req.__infringTraceBoundary = {
    trace_id: req.__infringTraceId,
    source: existing ? 'incoming_header' : 'gateway_minted',
    gateway_boundary: 'gateway.runtime.trace_boundary',
  };
  return req.__infringTraceId;
}

function gatewayRequestTraceBoundary(req) {
  const traceId = gatewayRequestTraceId(req);
  return (req && req.__infringTraceBoundary) || {
    trace_id: traceId,
    source: 'gateway_minted',
    gateway_boundary: 'gateway.runtime.trace_boundary',
  };
}

module.exports = {
  sanitizeGatewayTraceId,
  gatewayRequestTraceId,
  gatewayRequestTraceBoundary,
};
