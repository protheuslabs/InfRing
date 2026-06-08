import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, test } from 'vitest';

const ROOT = process.cwd();

type RowEvidence = {
  id: string;
  paths: string[];
};

const V10_RUNTIME_REGRESSION_EVIDENCE: RowEvidence[] = [
  {
    id: 'V10-TASK-RUNTIME-001.1',
    paths: [
      'core/layer0/ops/src/infringctl_routes_parts/010-command-routing.rs',
      'core/layer2/ops/src/workspace_gateway_runtime_parts/040-task-commands.rs',
    ],
  },
  {
    id: 'V10-TASK-RUNTIME-001.2',
    paths: [
      'core/layer2/ops/src/workspace_gateway_runtime_parts/020-task-state-and-receipts.rs',
      'core/layer2/ops/src/workspace_gateway_runtime_parts/040-task-commands.rs',
    ],
  },
  {
    id: 'V10-TASK-RUNTIME-001.3',
    paths: [
      'core/layer2/ops/src/workspace_gateway_runtime_parts/030-task-bus.rs',
      'core/layer2/ops/Cargo.toml',
    ],
  },
  {
    id: 'V10-MEMORY-030.1',
    paths: [
      'core/layer0/memory_runtime/src/main_parts/predictive_defrag_parts/010-types-and-policy.rs',
      'core/layer0/memory_runtime/src/main_parts/predictive_defrag_parts/020-monitor.rs',
    ],
  },
  {
    id: 'V10-MEMORY-030.2',
    paths: [
      'core/layer0/memory_runtime/src/main_parts/predictive_defrag_parts/030-stress.rs',
      'core/layer0/memory_runtime/src/main_parts/070-cli-entrypoint.rs',
    ],
  },
  {
    id: 'V10-INSTALL-CLI-002.1',
    paths: ['install.sh', 'tests/vitest/conduit_primitives_gap_closer.test.ts'],
  },
  {
    id: 'V10-DASH-004.1',
    paths: [
      'client/runtime/systems/ui/infring_dashboard.ts',
      'client/runtime/systems/ui/dashboard_static_asset_router.ts',
      'tests/client-memory-tools/infring_dashboard_ui.test.ts',
    ],
  },
  {
    id: 'V10-OPS-009.1',
    paths: [
      'core/layer0/ops/src/operator_tooling_kernel.rs',
      'core/layer0/ops/src/operator_tooling_kernel_parts/060_entry_tests.rs',
    ],
  },
  {
    id: 'V10-OPS-009.2',
    paths: [
      'client/runtime/systems/ops/infring_status_dashboard.ts',
      'core/layer0/ops/src/daemon_control_parts/030-start-dashboard-with-config.rs',
    ],
  },
  {
    id: 'V10-OPS-009.3',
    paths: [
      'core/layer0/ops/src/operator_tooling_kernel_parts/040_spawn.rs',
      'core/layer0/ops/src/operator_tooling_kernel_parts/060_entry_tests.rs',
    ],
  },
  {
    id: 'V10-OPS-009.4',
    paths: [
      'core/layer0/ops/src/operator_tooling_kernel_parts/030_memory_trace.rs',
      'core/layer0/ops/src/operator_tooling_kernel_parts/050_ops.rs',
    ],
  },
  {
    id: 'V10-OPS-009.5',
    paths: [
      'core/layer0/ops/src/operator_tooling_kernel_parts/050_ops.rs',
      'core/layer0/ops/src/operator_tooling_kernel_parts/060_entry_tests.rs',
    ],
  },
  {
    id: 'V10-OPS-009.6',
    paths: [
      'core/layer0/ops/src/operator_tooling_kernel_parts/040_spawn.rs',
      'core/layer0/ops/src/operator_tooling_kernel_parts/060_entry_tests.rs',
    ],
  },
  {
    id: 'V10-TASK-RUNTIME-001.4',
    paths: [
      'core/layer2/ops/src/workspace_gateway_runtime_parts/010-task-types-and-config.rs',
      'core/layer2/ops/src/workspace_gateway_runtime_parts/040-task-commands.rs',
    ],
  },
  {
    id: 'V10-MEMORY-031.1',
    paths: [
      'core/layer0/ops/src/dashboard_compat_api_parts/030-set-config-payload.rs',
      'core/layer0/ops/src/dashboard_compat_api_parts/config_payload_tests_parts/100-governance-and-semantic-memory.combined_parts/520-append-turn-message-captures-low-signal-web-tool-outcome-keyframe.rs',
    ],
  },
  {
    id: 'V10-DASH-004.2',
    paths: [
      'core/layer0/ops/src/dashboard_compat_api_parts/020-usage-from-state.rs',
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/192-slash-alias-and-alerts.ts',
    ],
  },
  {
    id: 'V10-DASH-004.3',
    paths: [
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/191-slash-continuity-and-alias-helpers.ts',
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/120-slash-and-agent-select.ts',
    ],
  },
  {
    id: 'V10-DASH-004.4',
    paths: [
      'core/layer0/ops/src/dashboard_compat_api_parts/020-usage-from-state.rs',
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/070-init-sequences-and-pointer.part01.ts',
    ],
  },
  {
    id: 'V10-MEMORY-031.2',
    paths: [
      'core/layer0/ops/src/dashboard_compat_api_parts/020-usage-from-state.rs',
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/020-init-roles-and-vibes.part01.ts',
    ],
  },
  {
    id: 'V10-DASH-004.5',
    paths: [
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/192-slash-alias-and-alerts.ts',
      'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/191-slash-continuity-and-alias-helpers.ts',
    ],
  },
];

describe('v10 runtime regression evidence contracts', () => {
  test.each(V10_RUNTIME_REGRESSION_EVIDENCE)('$id has non-backlog code/test evidence paths', ({ paths }) => {
    expect(Array.isArray(paths) && paths.length > 0).toBe(true);
    for (const rel of paths) {
      const full = path.join(ROOT, rel);
      expect(fs.existsSync(full)).toBe(true);
      expect(fs.statSync(full).isFile()).toBe(true);
    }
  });
});
