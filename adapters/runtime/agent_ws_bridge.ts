#!/usr/bin/env tsx
// Compatibility shim: canonical Gateway WebSocket bridge lives under gateway/**.
module.exports = require('../../gateway/runtime/sockets/agent_ws/agent_ws_bridge.ts');
