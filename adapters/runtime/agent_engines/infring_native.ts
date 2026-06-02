#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::infring-native.
//
// This adapter treats orchestration/** as InfRing's native swappable agent
// runtime engine without moving or rewriting orchestration itself. Live Gateway
// wiring can inject an orchestrationClient later; until then the adapter reports
// a bounded not_connected health state instead of fabricating runtime behavior.

'use strict';

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function baseEvent(ctx, type) {
  const message = (ctx && ctx.message) || {};
  const engine = (ctx && ctx.engine) || {};
  return {
    type,
    trace_id: cleanString(message.trace_id, 200),
    request_id: cleanString(message.request_id, 200),
    engine_id: cleanString(message.engine_id || engine.engine_id || 'infring_native', 120),
    session_id: cleanString(message.session_id, 200),
    turn_id: cleanString(message.turn_id, 200),
  };
}

function errorEvent(ctx, code, reason) {
  return {
    ...baseEvent(ctx, 'error'),
    error_code: cleanString(code, 120),
    reason: cleanString(reason, 1000),
    retryable: false,
  };
}

function stableRef(prefix, ctx) {
  const event = baseEvent(ctx, 'ref');
  const trace = event.trace_id || 'missing-trace';
  const session = event.session_id || 'no-session';
  const turn = event.turn_id || event.request_id || 'no-turn';
  return `${prefix}/${event.engine_id}/${trace}/${session}/${turn}`;
}

function clientMethod(options, name) {
  const client = options && options.orchestrationClient;
  return client && typeof client[name] === 'function' ? client[name].bind(client) : null;
}

async function callOrchestration(options, ctx, methodName, fallbackType) {
  const method = clientMethod(options, methodName);
  if (!method) {
    return errorEvent(
      ctx,
      'infring_native_orchestration_client_unavailable',
      `infring_native adapter requires orchestrationClient.${methodName} before live dispatch.`,
    );
  }
  const result = await method({
    trace_id: cleanString(ctx && ctx.message && ctx.message.trace_id, 200),
    session_id: cleanString(ctx && ctx.message && ctx.message.session_id, 200),
    turn_id: cleanString(ctx && ctx.message && ctx.message.turn_id, 200),
    request_id: cleanString(ctx && ctx.message && ctx.message.request_id, 200),
    message: ctx && ctx.message,
    engine: ctx && ctx.engine,
  });
  return {
    ...baseEvent(ctx, fallbackType),
    ...(result && typeof result === 'object' ? result : {}),
  };
}

function createInfringNativeEngineAdapter(options = {}) {
  return {
    async health_check(ctx) {
      const health = clientMethod(options, 'healthCheck');
      if (!health) {
        return {
          ...baseEvent(ctx, 'engine.health.result'),
          status: 'not_connected',
          readiness: 'adapter_seam_ready',
          engine_kind: 'native_orchestration',
          implementation_path: 'orchestration/**',
          reason: 'orchestration_client_not_attached',
        };
      }
      const result = await health({
        trace_id: cleanString(ctx && ctx.message && ctx.message.trace_id, 200),
        engine: ctx && ctx.engine,
      });
      return {
        ...baseEvent(ctx, 'engine.health.result'),
        status: 'available',
        engine_kind: 'native_orchestration',
        implementation_path: 'orchestration/**',
        ...(result && typeof result === 'object' ? result : {}),
      };
    },

    async start_session(ctx) {
      return callOrchestration(options, ctx, 'startSession', 'session.started');
    },

    async submit_turn(ctx) {
      return callOrchestration(options, ctx, 'submitTurn', 'turn.complete');
    },

    async stream_events(ctx) {
      return callOrchestration(options, ctx, 'streamEvents', 'heartbeat');
    },

    async cancel_turn(ctx) {
      return callOrchestration(options, ctx, 'cancelTurn', 'turn.cancelled');
    },

    async collect_artifacts(ctx) {
      const method = clientMethod(options, 'collectArtifacts');
      if (!method) {
        return {
          ...baseEvent(ctx, 'artifact.created'),
          artifact_ref: stableRef('artifact/pending', ctx),
          artifact_kind: 'orchestration_artifact_projection',
          status: 'not_connected',
        };
      }
      const result = await method({
        trace_id: cleanString(ctx && ctx.message && ctx.message.trace_id, 200),
        session_id: cleanString(ctx && ctx.message && ctx.message.session_id, 200),
        turn_id: cleanString(ctx && ctx.message && ctx.message.turn_id, 200),
        engine: ctx && ctx.engine,
      });
      return {
        ...baseEvent(ctx, 'artifact.created'),
        artifact_ref: stableRef('artifact/native', ctx),
        artifact_kind: 'orchestration_artifact_projection',
        ...(result && typeof result === 'object' ? result : {}),
      };
    },

    async emit_receipts(ctx) {
      const method = clientMethod(options, 'emitReceipts');
      if (!method) {
        return {
          ...baseEvent(ctx, 'receipt.created'),
          receipt_ref: stableRef('receipt/pending', ctx),
          receipt_kind: 'infring_native_adapter_seam',
          status: 'not_connected',
        };
      }
      const result = await method({
        trace_id: cleanString(ctx && ctx.message && ctx.message.trace_id, 200),
        session_id: cleanString(ctx && ctx.message && ctx.message.session_id, 200),
        turn_id: cleanString(ctx && ctx.message && ctx.message.turn_id, 200),
        engine: ctx && ctx.engine,
      });
      return {
        ...baseEvent(ctx, 'receipt.created'),
        receipt_ref: stableRef('receipt/native', ctx),
        receipt_kind: 'orchestration_runtime_receipt',
        ...(result && typeof result === 'object' ? result : {}),
      };
    },
  };
}

module.exports = {
  createInfringNativeEngineAdapter,
};
