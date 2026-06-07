#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime-route-assembly.
//
// Gateway owns Agent Runtime route/store/projection assembly. The legacy
// dashboard host may serve HTTP, but it should not be the place where Agent
// Runtime route policy and context/projection stores are composed.

'use strict';

const path = require('node:path');
const { loadAgentRuntimeEngineRegistry } = require('./agent_runtime_router.ts');
const { createGatewayAgentRuntimeRouterAssembly } = require('./agent_runtime_router_assembly.ts');
const {
  ingestAgentRuntimeContextProjection,
  appendAgentRuntimeTurnAtoms,
  materializeAgentRuntimeContextPack,
  loadAgentRuntimeContextRows,
} = require('./agent_runtime_context_store.ts');
const { materializeKernelAgentRuntimeContextPack } = require('./agent_runtime_kernel_context_bridge.ts');
const { buildUniversalToolGrants } = require('./universal_core_tools.ts');
const {
  createShellSocketAgentRuntimeOverlayRouteHandler,
} = require('../sockets/shell_socket/shell_socket_agent_runtime_overlay_routes.ts');
const { normalizeAgentRuntimeTurnInput } = require('../agent_runtime_input_normalizer.ts');
const {
  createAgentRuntimeWorkspaceStore,
} = require('./agent_runtime_workspace.ts');
const {
  createAgentRuntimeWorkspaceRouteHandler,
} = require('./agent_runtime_workspace_routes.ts');
const {
  createAgentRuntimeApprovalStore,
} = require('./agent_runtime_approvals.ts');
const {
  createAgentRuntimeApprovalRouteHandler,
} = require('./agent_runtime_approval_routes.ts');
const {
  createAgentRuntimeReceiptStore,
} = require('./agent_runtime_receipts.ts');
const {
  createAgentRuntimeTranscriptStore,
} = require('./agent_runtime_transcripts.ts');
const {
  createAgentRuntimeSessionStateStore,
} = require('./agent_runtime_session_state.ts');
const {
  createAgentRuntimeEngineProjectionStore,
  findAgentRuntimeEngine,
} = require('./agent_runtime_engine_projections.ts');
const {
  createAgentRuntimeEngineRouteHandler,
} = require('./agent_runtime_engine_routes.ts');
const {
  createAgentRuntimeTurnProjectionStore,
  sanitizeAgentRuntimeActivityEvent,
} = require('./agent_runtime_turn_projection.ts');
const {
  createAgentRuntimeTurnRouteHandler,
} = require('./agent_runtime_turn_routes.ts');
const {
  createAgentRuntimeContextPreviewProjectionStore,
} = require('./agent_runtime_context_preview.ts');
const {
  AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
  buildAgentRuntimeContextPack,
} = require('./agent_runtime_context_pack.ts');
const {
  createGatewayNativeOrchestrationClient,
} = require('../gateway_native_orchestration_client.ts');

function createGatewayAgentRuntimeRouteAssembly(options = {}) {
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
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  const fetchBackendJson = options.fetchBackendJson;
  const adapterFactories = options.adapterFactories || {};
  const materializeKernelContextPack = typeof options.materializeKernelAgentRuntimeContextPack === 'function'
    ? options.materializeKernelAgentRuntimeContextPack
    : materializeKernelAgentRuntimeContextPack;
  const materializeGatewayContextPack = typeof options.materializeAgentRuntimeContextPack === 'function'
    ? options.materializeAgentRuntimeContextPack
    : materializeAgentRuntimeContextPack;
  const createNativeOrchestrationClient = typeof options.createNativeOrchestrationClient === 'function'
    ? options.createNativeOrchestrationClient
    : createGatewayNativeOrchestrationClient;

  const agentRuntimeWorkspaceStore = createAgentRuntimeWorkspaceStore({ root, statusDir });
  const {
    normalizeAgentRuntimeWorkspacePath,
    loadAgentRuntimeWorkspace,
  } = agentRuntimeWorkspaceStore;
  const {
    createAdapterMap,
    createRouter,
  } = createGatewayAgentRuntimeRouterAssembly({
    root,
    normalizeWorkspacePath: normalizeAgentRuntimeWorkspacePath,
    adapterFactories,
  });
  const {
    handleAgentRuntimeWorkspaceRoute,
  } = createAgentRuntimeWorkspaceRouteHandler({
    workspaceStore: agentRuntimeWorkspaceStore,
    readJsonBody,
    sendJson,
  });

  const agentRuntimeApprovalStore = createAgentRuntimeApprovalStore({ root });
  const {
    sanitizeAgentRuntimeProposalArguments,
    recordAgentRuntimePendingApproval,
    mergeAgentRuntimeApprovalPermissionPolicy,
  } = agentRuntimeApprovalStore;
  const {
    handleAgentRuntimeApprovalRoute,
  } = createAgentRuntimeApprovalRouteHandler({
    approvalStore: agentRuntimeApprovalStore,
    readJsonBody,
    sendJson,
  });

  const agentRuntimeReceiptStore = createAgentRuntimeReceiptStore({ root });
  const {
    recordAgentRuntimeTurnReceipts,
  } = agentRuntimeReceiptStore;
  const agentRuntimeTranscriptStore = createAgentRuntimeTranscriptStore({ statusDir });
  const {
    appendAgentRuntimeTranscriptTurn,
  } = agentRuntimeTranscriptStore;
  const {
    handleShellSocketAgentRuntimeOverlayRoute,
  } = createShellSocketAgentRuntimeOverlayRouteHandler({
    transcriptStore: agentRuntimeTranscriptStore,
    fetchBackendJson,
    sendJson,
  });

  const agentRuntimeSessionStateStore = createAgentRuntimeSessionStateStore({
    statusDir,
    loadRegistry: () => loadAgentRuntimeEngineRegistry(root),
    findEngine: findAgentRuntimeEngine,
  });
  const {
    loadAgentRuntimeSelection,
    saveAgentRuntimeSelection,
    agentRuntimeSteerProjection,
    drainAgentRuntimeSteeringInterventions,
  } = agentRuntimeSessionStateStore;
  const agentRuntimeEngineProjectionStore = createAgentRuntimeEngineProjectionStore({
    root,
    loadRegistry: () => loadAgentRuntimeEngineRegistry(root),
    createAdapterMap,
    loadSelection: loadAgentRuntimeSelection,
    saveSelection: saveAgentRuntimeSelection,
  });
  const {
    handleAgentRuntimeEngineRoute,
  } = createAgentRuntimeEngineRouteHandler({
    engineProjectionStore: agentRuntimeEngineProjectionStore,
    selectEngine: agentRuntimeEngineProjectionStore.agentRuntimeSelectionProjection,
    readJsonBody,
    sendJson,
  });

  const agentRuntimeTurnProjectionStore = createAgentRuntimeTurnProjectionStore({
    root,
    contextFanoutTarget: AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
    normalizeAgentRuntimeTurnInput,
    loadAgentRuntimeEngineRegistry: () => loadAgentRuntimeEngineRegistry(root),
    findAgentRuntimeEngine,
    loadAgentRuntimeWorkspace,
    createRouter,
    sanitizeAgentRuntimeActivityEvent,
    appendAgentRuntimeTranscriptTurn,
    appendAgentRuntimeTurnAtoms,
    ingestAgentRuntimeContextProjection,
    loadAgentRuntimeContextRows,
    materializeKernelAgentRuntimeContextPack: materializeKernelContextPack,
    materializeAgentRuntimeContextPack: materializeGatewayContextPack,
    buildAgentRuntimeContextPack,
    mergeAgentRuntimeApprovalPermissionPolicy,
    buildUniversalToolGrants,
    drainAgentRuntimeSteeringInterventions,
    sanitizeAgentRuntimeProposalArguments,
    recordAgentRuntimePendingApproval,
    recordAgentRuntimeTurnReceipts,
  });
  const agentRuntimeContextPreviewProjectionStore = createAgentRuntimeContextPreviewProjectionStore({
    root,
    loadAgentRuntimeContextRows,
    materializeKernelAgentRuntimeContextPack: materializeKernelContextPack,
    materializeAgentRuntimeContextPack: materializeGatewayContextPack,
    buildAgentRuntimeContextPack,
    buildUniversalToolGrants,
  });
  const {
    handleAgentRuntimeTurnRoute,
  } = createAgentRuntimeTurnRouteHandler({
    turnProjectionStore: agentRuntimeTurnProjectionStore,
    contextPreviewProjectionStore: agentRuntimeContextPreviewProjectionStore,
    steer: agentRuntimeSteerProjection,
    createNativeOrchestrationClient,
    readJsonBody,
    sendJson,
  });

  return {
    agentRuntimeWorkspaceStore,
    agentRuntimeApprovalStore,
    agentRuntimeReceiptStore,
    agentRuntimeTranscriptStore,
    agentRuntimeSessionStateStore,
    agentRuntimeEngineProjectionStore,
    agentRuntimeTurnProjectionStore,
    agentRuntimeContextPreviewProjectionStore,
    handleAgentRuntimeWorkspaceRoute,
    handleAgentRuntimeApprovalRoute,
    handleShellSocketAgentRuntimeOverlayRoute,
    handleAgentRuntimeEngineRoute,
    handleAgentRuntimeTurnRoute,
  };
}

module.exports = {
  createGatewayAgentRuntimeRouteAssembly,
};
