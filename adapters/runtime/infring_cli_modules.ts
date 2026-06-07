#!/usr/bin/env node
'use strict';

const { runInfringOps } = require('./run_infring_ops.ts');

function normalizeArgv(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function normalizeSubcommand(raw, fallback = 'status', aliases = {}) {
  const sub = String(raw || fallback).trim().toLowerCase() || fallback;
  return Object.prototype.hasOwnProperty.call(aliases, sub) ? String(aliases[sub]) : sub;
}

function createDomainCommandModule(spec = {}) {
  const domain = String(spec.domain || '').trim();
  const fallback = String(spec.defaultSubcommand || 'status').trim() || 'status';
  const aliases = spec.aliases && typeof spec.aliases === 'object' ? { ...spec.aliases } : {};
  const unknownDomainFallback = spec.unknownDomainFallback !== false;

  function localNormalizeSubcommand(raw) {
    return normalizeSubcommand(raw, fallback, aliases);
  }

  function run(argv = process.argv.slice(2)) {
    const args = normalizeArgv(argv);
    const sub = localNormalizeSubcommand(args[0]);
    return runInfringOps([domain, sub, ...args.slice(1)], { unknownDomainFallback });
  }

  return { run, normalizeSubcommand: localNormalizeSubcommand };
}

function createDirectCommandModule(spec = {}) {
  const buildArgs = typeof spec.buildArgs === 'function' ? spec.buildArgs : ((argv) => argv);
  const unknownDomainFallback = spec.unknownDomainFallback === true;

  function run(argv = process.argv.slice(2)) {
    return runInfringOps(buildArgs(normalizeArgv(argv)), { unknownDomainFallback });
  }

  return { run };
}

function createStatusDashboardModule() {
  function hasWebFlag(argv) {
    return Array.isArray(argv) && argv.some((arg) => arg === '--web' || arg === 'web');
  }

  function stripDashboardCompatFlags(argv) {
    return normalizeArgv(Array.isArray(argv) ? argv : []).filter(
      (arg) =>
        arg !== '--dashboard' &&
        arg !== 'dashboard' &&
        arg !== '--web' &&
        arg !== 'web'
    );
  }

  function runDashboardUi(argv = process.argv.slice(2)) {
    const forward = stripDashboardCompatFlags(argv);
    return runInfringOps(['daemon-control', 'start', ...forward], { unknownDomainFallback: true });
  }

  function run(argv = process.argv.slice(2)) {
    const args = normalizeArgv(argv);
    if (hasWebFlag(args)) {
      return runDashboardUi(args);
    }
    const passthrough = args.length ? args : ['daemon-control', 'status'];
    return runInfringOps(passthrough, { unknownDomainFallback: true });
  }

  return { run, runDashboardUi };
}

function createReplModule() {
  function run(argv = process.argv.slice(2)) {
    const args = normalizeArgv(argv);
    if (args.includes('--help') || args.includes('-h')) {
      process.stdout.write(
        'Usage: infring repl\n' +
          'Lightweight REPL bootstrap for constrained installs.\n',
      );
      return 0;
    }
    const status = runInfringOps(['command-list-kernel', '--mode=help'], {
      unknownDomainFallback: false,
    });
    if (status === 0 && process.stdin.isTTY) {
      process.stdout.write(
        '[infring repl] interactive shell is unavailable in slim runtime; showing command index.\n',
      );
    }
    return status;
  }

  return { run };
}

function createUnknownGuardModule() {
  function resolveRuntimeMode(env = process.env) {
    const raw = String(
      (env && (env.INFRING_INSTALL_MODE || env.INFRING_RUNTIME_MODE || env.INFRING_RUNTIME_MODE)) || ''
    )
      .trim()
      .toLowerCase();
    if (['full', 'minimal', 'pure', 'tiny-max'].includes(raw)) return raw;
    if (['1', 'true', 'yes', 'on'].includes(String((env && env.INFRING_TINY_MAX_MODE) || '').trim().toLowerCase())) return 'tiny-max';
    if (['1', 'true', 'yes', 'on'].includes(String((env && env.INFRING_PURE_MODE) || '').trim().toLowerCase())) return 'pure';
    return 'full';
  }

  function buildModeContract(mode) {
    const normalized = String(mode || '').trim().toLowerCase() || 'full';
    if (normalized === 'pure' || normalized === 'tiny-max') {
      return {
        mode: normalized,
        dashboard_surface: 'limited_optional',
        dashboard_opt_in_command: 'none',
        dashboard_opt_in_reason: 'constrained_mode_optional_dashboard_surfaces_limited',
        auto_open_dashboard_noninteractive: false,
        capability_reason: 'rust_first_mode_limits_optional_rich_surfaces',
      };
    }
    if (normalized === 'minimal') {
      return {
        mode: normalized,
        dashboard_surface: 'optional_limited',
        dashboard_opt_in_command: 'infring gateway start --dashboard-open=1',
        dashboard_opt_in_reason: 'minimal_mode_requires_explicit_dashboard_opt_in',
        auto_open_dashboard_noninteractive: false,
        capability_reason: 'minimal_mode_install_light_optional_surfaces_may_require_explicit_setup',
      };
    }
    return {
      mode: 'full',
      dashboard_surface: 'available',
      dashboard_opt_in_command: 'infring gateway start --dashboard-open=1',
      dashboard_opt_in_reason: 'full_mode_requires_explicit_opt_in_for_noninteractive_dashboard_open',
      auto_open_dashboard_noninteractive: false,
      capability_reason: 'full_mode_complete_operator_surface',
    };
  }

  function buildModeValidCommands(mode) {
    const normalized = String(mode || '').trim().toLowerCase() || 'full';
    if (normalized === 'pure' || normalized === 'tiny-max') {
      return {
        commands: [
          'infring help',
          'infring setup',
          'infring setup status --json',
          'infring gateway status',
          'infring doctor --json',
        ],
        reason: 'constrained_mode_optional_rich_surfaces_limited',
      };
    }
    if (normalized === 'minimal') {
      return {
        commands: [
          'infring help',
          'infring setup',
          'infring setup status --json',
          'infring gateway',
          'infring gateway status',
          'infring doctor --json',
        ],
        reason: 'minimal_mode_operator_surface_requires_explicit_setup_on_some_hosts',
      };
    }
    return {
      commands: [
        'infring help',
        'infring setup',
        'infring setup status --json',
        'infring gateway',
        'infring gateway status',
        'infring doctor --json',
      ],
      reason: 'full_mode_complete_operator_surface',
    };
  }

  function buildUnknownRecoveryContract(unknown) {
    const command = String(unknown || '').trim();
    const mode = resolveRuntimeMode(process.env);
    const modeHelp = buildModeValidCommands(mode);
    const modeContract = buildModeContract(mode);
    const requiredOptionalSurfaceMatrix = {
      wrappers: {
        required_all_modes: true,
        full: 'available',
        minimal: 'available',
        pure_tiny_max: 'available',
      },
      setup_lane: {
        required_all_modes: true,
        full: 'available',
        minimal: 'available',
        pure_tiny_max: 'available',
      },
      gateway_status: {
        required_all_modes: true,
        full: 'available',
        minimal: 'available',
        pure_tiny_max: 'available',
      },
      gateway_launch: {
        required_all_modes: false,
        full: 'available',
        minimal: 'available_with_explicit_setup_if_needed',
        pure_tiny_max: 'limited_optional_by_design',
      },
    };
    return {
      route: 'unknown_command_recovery_v1',
      recovery_contract_version: '1.1',
      unknown_command: command,
      mode_contract: modeContract,
      mode_help_reason: modeHelp.reason,
      mode_valid_commands: modeHelp.commands,
      runtime_manifest_rel: 'install_runtime_manifest_v1',
      required_optional_surface_matrix: requiredOptionalSurfaceMatrix,
      noninteractive_opt_in_required: true,
      noninteractive_opt_in_command: 'infring setup --yes --defaults',
      noninteractive_opt_in_expected_output:
        'setup profile saved with onboarding receipt and deterministic next action',
      retry_command: 'infring help',
      retry_expected_output: 'prints available commands and aliases',
      path_reload_command: '. "$HOME/.infring/env.sh" && hash -r 2>/dev/null || true',
      path_reload_expected_output: 'infring wrapper resolves in current shell',
      setup_retry_command: 'infring setup --yes --defaults',
      setup_retry_expected_output: 'saved profile confirmation with onboarding receipt',
      setup_status_command: 'infring setup status --json',
      setup_status_expected_output: 'returns onboarding_receipt with deterministic setup status',
      gateway_status_command: 'infring gateway status',
      gateway_status_expected_output: 'gateway status contract indicates running or exact failure code',
      gateway_restart_command: 'infring gateway restart',
      gateway_restart_expected_output: 'gateway restart contract confirms accepted restart action',
      diagnostics_command: 'infring doctor --json',
      diagnostics_expected_output: 'returns install/runtime diagnostics with explicit failure codes',
      dashboard_opt_in_command: modeContract.dashboard_opt_in_command || 'none',
      dashboard_opt_in_reason:
        modeContract.dashboard_opt_in_reason || 'explicit_opt_in_required_for_optional_surfaces',
      dashboard_opt_in_expected_output:
        modeContract.dashboard_opt_in_command && modeContract.dashboard_opt_in_command !== 'none'
          ? 'gateway accepts explicit dashboard opt-in request'
          : 'dashboard surface remains constrained in current mode',
      full_mode_repair_command:
        'curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node',
      full_mode_repair_expected_output:
        'installs full command surface and refreshes runtime wrappers',
      escalation_command: 'infring recover',
      escalation_expected_output: 'restarts runtime and re-validates gateway/install contracts',
      recovery_step_order: [
        'retry_command',
        'path_reload_command',
        'noninteractive_opt_in_command',
        'setup_retry_command',
        'setup_status_command',
        'gateway_status_command',
        'gateway_restart_command',
        'diagnostics_command',
        'full_mode_repair_command',
        'escalation_command'
      ]
    };
  }

  function isJsonMode(argv) {
    return argv.some((arg) => arg === '--json' || arg === '--json=1');
  }

  function firstUnknownCommand(argv) {
    for (const raw of argv) {
      const token = String(raw || '').trim();
      if (!token) continue;
      if (token === '--json' || token === '--json=1') continue;
      if (token === '--help' || token === '-h') continue;
      if (token.startsWith('-')) continue;
      return token;
    }
    return '';
  }

  function run(argv = process.argv.slice(2)) {
    const tokens = normalizeArgv(argv);
    const unknown = firstUnknownCommand(tokens);
    const json = isJsonMode(tokens);
    const recovery = buildUnknownRecoveryContract(unknown);
    if (json) {
      process.stdout.write(
        `${JSON.stringify({
          ok: false,
          type: 'infring_unknown_guard',
          error: 'unknown_command',
          command: unknown,
          hint: 'Run `infring help` to list available commands.',
          recovery,
        })}\n`,
      );
      return 2;
    }
    if (unknown) {
      process.stderr.write(`[infring] unknown command: ${unknown}\n`);
    } else {
      process.stderr.write('[infring] unknown command\n');
    }
    process.stderr.write('[infring] deterministic recovery path:\n');
    process.stderr.write(`[infring]   mode: ${recovery.mode_contract.mode} (dashboard surface: ${recovery.mode_contract.dashboard_surface})\n`);
    process.stderr.write(`[infring]   runtime-manifest: ${recovery.runtime_manifest_rel}\n`);
    process.stderr.write(`[infring]   mode-help: ${recovery.mode_help_reason}\n`);
    if (Array.isArray(recovery.mode_valid_commands) && recovery.mode_valid_commands.length > 0) {
      process.stderr.write('[infring]   mode-valid-commands:\n');
      for (const command of recovery.mode_valid_commands) {
        process.stderr.write(`[infring]     - ${command}\n`);
      }
    }
    process.stderr.write(`[infring]   retry: ${recovery.retry_command} (expect: ${recovery.retry_expected_output})\n`);
    process.stderr.write(`[infring]   path: ${recovery.path_reload_command} (expect: ${recovery.path_reload_expected_output})\n`);
    process.stderr.write(`[infring]   noninteractive-opt-in: ${recovery.noninteractive_opt_in_command} (expect: ${recovery.noninteractive_opt_in_expected_output})\n`);
    process.stderr.write(`[infring]   setup-retry: ${recovery.setup_retry_command} (expect: ${recovery.setup_retry_expected_output})\n`);
    process.stderr.write(`[infring]   setup: ${recovery.setup_status_command} (expect: ${recovery.setup_status_expected_output})\n`);
    process.stderr.write(`[infring]   gateway: ${recovery.gateway_status_command} (expect: ${recovery.gateway_status_expected_output})\n`);
    process.stderr.write(`[infring]   gateway-restart: ${recovery.gateway_restart_command} (expect: ${recovery.gateway_restart_expected_output})\n`);
    process.stderr.write(`[infring]   dashboard-opt-in: ${recovery.dashboard_opt_in_command} (expect: ${recovery.dashboard_opt_in_expected_output})\n`);
    process.stderr.write(`[infring]   dashboard-opt-in-reason: ${recovery.dashboard_opt_in_reason}\n`);
    process.stderr.write(`[infring]   doctor: ${recovery.diagnostics_command} (expect: ${recovery.diagnostics_expected_output})\n`);
    process.stderr.write(`[infring]   full-repair: ${recovery.full_mode_repair_command} (expect: ${recovery.full_mode_repair_expected_output})\n`);
    process.stderr.write(`[infring]   escalate: ${recovery.escalation_command} (expect: ${recovery.escalation_expected_output})\n`);
    runInfringOps(['command-list-kernel', '--mode=help'], {
      unknownDomainFallback: false,
    });
    return 2;
  }

  return { run };
}

const backlogGithubSync = createDomainCommandModule({
  domain: 'backlog-github-sync',
});

const backlogRegistry = createDomainCommandModule({
  domain: 'backlog-registry',
  aliases: {
    metrics: 'status',
    triage: 'status',
  },
});

const contractCheck = createDirectCommandModule({
  buildArgs(argv) {
    return argv.length ? ['contract-check', ...argv] : ['contract-check', 'status'];
  },
  unknownDomainFallback: false,
});

const infringControlPlane = createDomainCommandModule({
  domain: 'infring-control-plane',
  aliases: {
    audit: 'run',
    health: 'status',
    'job-submit': 'run',
  },
});

const infringRepl = createReplModule();
const infringStatusDashboard = createStatusDashboardModule();
const infringUnknownGuard = createUnknownGuardModule();

const rust50MigrationProgram = createDomainCommandModule({
  domain: 'rust50-migration-program',
});

const rustEnterpriseProductivityProgram = createDomainCommandModule({
  domain: 'rust-enterprise-productivity-program',
});

const venomContainmentLayer = createDomainCommandModule({
  domain: 'venom-containment-layer',
  aliases: {
    evolve: 'evaluate',
  },
});

module.exports = {
  backlogGithubSync,
  backlogRegistry,
  contractCheck,
  createDirectCommandModule,
  createDomainCommandModule,
  createReplModule,
  createStatusDashboardModule,
  createUnknownGuardModule,
  normalizeSubcommand,
  infringControlPlane,
  infringRepl,
  infringStatusDashboard,
  infringUnknownGuard,
  rust50MigrationProgram,
  rustEnterpriseProductivityProgram,
  venomContainmentLayer,
};
