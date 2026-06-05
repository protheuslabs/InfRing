#!/usr/bin/env tsx

// Compatibility shim only. Canonical Gateway Agent Runtime context storage lives
// in gateway/runtime/agent_runtime/agent_runtime_context_store.ts.
'use strict';

module.exports = require('../../../gateway/runtime/agent_runtime/agent_runtime_context_store.ts');
