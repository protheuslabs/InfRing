#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::http-socket-runtime.
//
// Health-only adapter seam for external frameworks that expose a local
// dashboard/API/socket service. A command/path hit is useful install evidence,
// but it is not runtime readiness. These adapters only report `available`
// after a bounded HTTP health probe succeeds.

'use strict';

const { resolveEngineDiscovery, cleanString } = require('./discovery.ts');

function cleanEngineId(value) {
  return cleanString(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, '');
}

function cleanText(value, max = 500) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function baseEvent(ctx, type, engineId) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  return {
    type,
    trace_id: cleanText(message.trace_id, 240),
    request_id: cleanText(message.request_id, 240),
    engine_id: cleanEngineId(message.engine_id || engineId),
    session_id: cleanText(message.session_id, 240),
    turn_id: cleanText(message.turn_id, 240),
  };
}

function uniqueUrls(urls) {
  const out = [];
  const seen = new Set();
  for (const url of asArray(urls)) {
    const clean = cleanText(url, 1000).replace(/\/+$/, '');
    if (!clean || seen.has(clean)) continue;
    if (!/^https?:\/\//i.test(clean)) continue;
    seen.add(clean);
    out.push(clean);
  }
  return out;
}

function discoveryUrls(engine, discovery) {
  const engineDiscovery = engine && engine.discovery && typeof engine.discovery === 'object' ? engine.discovery : {};
  const urls = [];
  if (discovery && discovery.url) urls.push(discovery.url);
  urls.push(...asArray(engineDiscovery.default_urls));
  return uniqueUrls(urls);
}

async function fetchJsonCandidate(baseUrl, probe, timeoutMs) {
  const path = cleanText(probe && probe.path, 500) || '/health';
  const method = cleanText(probe && probe.method, 20).toUpperCase() || 'GET';
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  const started = Date.now();
  try {
    const res = await fetch(`${baseUrl}${path.startsWith('/') ? path : `/${path}`}`, {
      method,
      cache: 'no-store',
      signal: controller.signal,
    });
    const text = await res.text().catch(() => '');
    let json = null;
    try { json = text ? JSON.parse(text) : null; } catch {}
    const allowUnauthorized = probe && probe.allow_unauthorized === true && (res.status === 401 || res.status === 403);
    return {
      ok: res.ok || allowUnauthorized,
      status_code: res.status,
      latency_ms: Date.now() - started,
      body_json: json,
      body_preview: cleanText(text, 500),
      path,
    };
  } catch (error) {
    return {
      ok: false,
      status_code: 0,
      latency_ms: Date.now() - started,
      error: cleanText(error && error.message ? error.message : error, 240),
      path,
    };
  } finally {
    clearTimeout(timer);
  }
}

function versionPreviewFromProbe(probe) {
  const body = probe && probe.body_json && typeof probe.body_json === 'object' ? probe.body_json : {};
  return cleanText(
    body.version || body.app_version || body.runtime_version || body.status || body.name || body.message || probe?.body_preview,
    240,
  );
}

function createHttpSocketRuntimeEngineAdapter(options = {}) {
  const engineId = cleanEngineId(options.engineId || 'external_socket_engine');
  const engineKind = cleanText(options.engineKind || 'external_framework_adapter', 120);
  const downloadActionRef = cleanText(options.downloadActionRef || `agent_runtime_download/${engineId}`, 500);
  const contextTransportMode = cleanText(options.contextTransportMode || 'structured_json', 80);
  const structuredTransportTarget = cleanText(options.structuredTransportTarget || 'structured_json', 80);
  const transportMigrationStatus = cleanText(options.transportMigrationStatus || 'planned_structured_target', 120);
  const defaultHealthProbes = asArray(options.healthProbes).length
    ? asArray(options.healthProbes)
    : [{ method: 'GET', path: '/health' }, { method: 'GET', path: '/api/status' }, { method: 'GET', path: '/status' }];
  const timeoutMs = Math.max(250, Math.min(Number(options.timeoutMs) || 1500, 10000));

  async function healthCheck(ctx) {
    const engine = ctx && ctx.engine && typeof ctx.engine === 'object' ? ctx.engine : { engine_id: engineId };
    const discovery = resolveEngineDiscovery(engine, {
      url: options.url,
      command: options.command,
      config: options.config,
      env: options.env || process.env,
    });
    const urls = discoveryUrls(engine, discovery);
    const probes = asArray(engine.private_schema_health_probes).length ? asArray(engine.private_schema_health_probes) : defaultHealthProbes;
    let best = null;
    for (const url of urls) {
      for (const probeSpec of probes) {
        const probe = await fetchJsonCandidate(url, probeSpec, timeoutMs);
        if (!best || probe.ok || probe.status_code > best.status_code) best = { ...probe, url };
        if (probe.ok) {
          return {
            ...baseEvent(ctx, 'engine.health.result', engineId),
            status: 'available',
            engine_kind: engineKind,
            url,
            discovery_source: discovery.discovery_source || 'default_url_probe',
            custom_location_allowed: discovery.custom_location_allowed,
            download_available: false,
            download_action_ref: downloadActionRef,
            context_transport_mode: contextTransportMode,
            structured_transport_target: structuredTransportTarget,
            transport_migration_status: transportMigrationStatus,
            supports_live_steering: false,
            supports_next_turn_steering: true,
            steering_transport: 'gateway_next_turn_intervention',
            version_preview: versionPreviewFromProbe(probe),
            health_path: probe.path,
            health_status_code: probe.status_code,
            health_latency_ms: probe.latency_ms,
          };
        }
      }
    }
    const commandEvidence = discovery.command || discovery.resolved_path || '';
    const status = commandEvidence ? 'installed_not_running' : (discovery.status === 'configurable' ? 'not_connected' : discovery.status || 'not_downloaded');
    return {
      ...baseEvent(ctx, 'engine.health.result', engineId),
      status,
      engine_kind: engineKind,
      url: urls[0] || null,
      command: cleanText(commandEvidence, 500) || null,
      discovery_source: discovery.discovery_source || 'missing_installable',
      custom_location_allowed: discovery.custom_location_allowed,
      download_available: Boolean(discovery.download_available || downloadActionRef),
      download_action_ref: downloadActionRef,
      context_transport_mode: contextTransportMode,
      structured_transport_target: structuredTransportTarget,
      transport_migration_status: transportMigrationStatus,
      supports_live_steering: false,
      supports_next_turn_steering: true,
      steering_transport: 'gateway_next_turn_intervention',
      reason: best && best.error ? best.error : 'runtime_health_endpoint_unreachable',
      health_status_code: best ? best.status_code : 0,
      health_path: best ? best.path : '',
      health_latency_ms: best ? best.latency_ms : 0,
      version_preview: best ? versionPreviewFromProbe(best) : '',
    };
  }

  return {
    health_check: healthCheck,
    async start_session(ctx) {
      return {
        ...baseEvent(ctx, 'session.started', engineId),
        status: 'pending_live_adapter',
        reason: `${engineId} session bridge is not implemented until the private runtime protocol is attached.`,
      };
    },
    async submit_turn(ctx) {
      return {
        ...baseEvent(ctx, 'turn.complete', engineId),
        status: 'failed',
        error_code: `${engineId}_live_turn_adapter_not_attached`,
        reason: `${engineId} is health-detectable, but live turn submission is not attached yet.`,
        output_text: `${engineId} is available, but InfRing has not attached its live turn adapter yet.`,
        retryable: false,
      };
    },
    async stream_events(ctx) {
      return {
        ...baseEvent(ctx, 'stream.events', engineId),
        status: 'pending_live_adapter',
      };
    },
    async cancel_turn(ctx) {
      return {
        ...baseEvent(ctx, 'turn.cancelled', engineId),
        status: 'cancelled',
      };
    },
    async collect_artifacts(ctx) {
      return {
        ...baseEvent(ctx, 'artifact.collection', engineId),
        status: 'pending_live_adapter',
      };
    },
    async emit_receipts(ctx) {
      return {
        ...baseEvent(ctx, 'receipt.emitted', engineId),
        status: 'pending_live_adapter',
      };
    },
  };
}

module.exports = {
  createHttpSocketRuntimeEngineAdapter,
};

