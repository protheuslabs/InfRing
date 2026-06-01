# InfRing

[![License: Dual](https://img.shields.io/badge/license-dual%20(NC%20%2B%20Apache--2.0)-orange.svg)](LICENSE_SCOPE.md)
[![Architecture](https://img.shields.io/badge/architecture-three--plane%20metakernel-0A7A5E)](planes/README.md)
![Coverage](docs/client/badges/coverage.svg)

InfRing is a deterministic, receipt-first autonomous runtime built on a three-plane metakernel.
It is designed for verifiable execution, fail-closed safety, and reproducible operator workflows.

Canonical long-term vision: [InfRing Vision: Adaptive Intelligence Under User Authority](VISION.md).

Kernel authority is Rust-first (`core/**` path compatibility).
Orchestration Control Plane coordination lives in `orchestration/**` (non-canonical, contract-driven coordination).
Shell/runtime surfaces remain thin presentation wrappers around policy-governed Kernel lanes (`client/**` path compatibility).

## Canonical Terminology (Enforced)

- **Kernel** is the authority layer. `core/**` is the repository path for Kernel implementation and compatibility wording only; `Core` is not a second authority.
- **Orchestration Control Plane** is the coordination layer. `orchestration/**` is its repository path. `Tower` is not a canonical architecture term.
- **Gateways** are the external boundary layer. `adapters/**` is the implementation path and `Adapters` is compatibility wording only.
- **Shell** is the presentation layer. `client/**` is the repository path for Shell implementation and compatibility wording only; `Client` is not a conceptual owner.
- **Validation**, **Observability**, and **Governance** are Assurance domains, not Kernel, Orchestration, or Shell subfeatures.
- When a document refers to a filesystem path, use the path form (`core/**`, `client/**`, `adapters/**`). When it refers to ownership, use the concept form (**Kernel**, **Shell**, **Gateways**).
- Alias retirement state is governed by:
  - `docs/workspace/policy/release_terminology_transition_policy.md`
  - `client/runtime/config/terminology_transition_deprecation_tracker.json`

## Why InfRing

- Deterministic execution with evidence-backed receipts.
- Fail-closed safety and policy enforcement by default.
- Rust-authoritative kernel with explicit thin-shell boundaries.
- Multi-profile runtime strategy: rich, pure, and tiny-max.
- Operator-first CLI and gateway operator surface (presentation + orchestration ingress).

## Architecture At A Glance

| Plane | Role |
|---|---|
| Safety Plane | Deterministic guardrails, invariants, fail-closed behavior |
| Cognition Plane | Historical metakernel plane label for non-authoritative coordination and presentation; active owners are Orchestration Control Plane and Shell |
| Substrate Plane | Runtime integration, execution surfaces, system bridges |

Runtime split inside cognition:

- Authoritative Kernel: `core/**` path compatibility
- Orchestration Control Plane: `orchestration/**`
- Presentation Shell: `client/**` path compatibility
- Gateway Layer: `adapters/**` path compatibility

See [planes/README.md](planes/README.md) for the canonical architecture contract.
See [docs/client/PUBLIC_OPERATOR_PROFILE.md](docs/client/PUBLIC_OPERATOR_PROFILE.md) for the public operator-facing surface and support expectations.

## Current State (April 2026)

What is true in this repository today:

- Primary operator entrypoint is `infring` (with `infringctl` and `infringd` wrappers).
- Main dashboard is served by the gateway at `http://127.0.0.1:4173/dashboard#chat`.
- Gateway health endpoint is `http://127.0.0.1:4173/healthz`.
- Gateway persistence is enabled by default (auto-restart + reboot supervision unless disabled).
- Pure profiles (`--pure`, `--tiny-max`) are Rust-only and intentionally do not expose the rich `gateway` UI surface.
- Full command surface still requires Node.js 22+; Node-free fallback remains available for kernel operations.
- Production release channels are resident-IPC authoritative: process transport fallbacks are blocked (`process_transport_forbidden_in_production` / `process_fallback_forbidden_in_production`).
- Release-closure evidence now includes topology diagnostics, live stateful upgrade/rollback rehearsal, recovery rehearsal, numeric release scorecards, and support-bundle export.
- Runtime proof now supports dual-track evidence (`synthetic` canary + `empirical` live artifact track) with profile-scoped empirical sample budgets and required source/metric completeness enforced in release gating.
- Runtime proof verify now publishes split artifact streams (`runtime_proof_synthetic_canary_current.json` and `runtime_proof_empirical_release_evidence_current.json`) plus empirical trend deltas (`runtime_proof_empirical_trends_current.json`).
- Gateway release readiness includes manifest-backed graduation checks (hooks + chaos scenarios) plus staged roadmap-gateway tracking under the same graduation manifest.
- Layer2 parity guard requires every listed lane to be explicitly marked `complete`; provisional lanes are release blockers.
- Release proof packs are assembled as grouped, checksummed artifacts under `validation/release_gates/proof_packs/<version>/`.
- Dashboard runtime blocks now carry explicit freshness metadata fields (`source`, `source_sequence`, `age_seconds`, `stale`) and are guarded by the dashboard surface authority contract.
- Critical operator-path Node dependency is now inventoried as a first-class artifact (`ops:node-critical-path:inventory`) with non-regression checks.
- Rust-native agent surfaces are now guarded by a support-level status manifest (`ops:agent-surface:status:guard`) so release-required vs experimental lanes stay explicit.

## Production Support Contract

- Canonical production profile: rich
- Constrained profiles: `--pure`, `--tiny-max`
- Experimental lanes (explicit opt-in): `assimilate`
- Resident IPC is the only supported production topology; the legacy process runner is dev-only.
- Release entrypoints quarantine the legacy runner under `adapters/runtime/dev_only/**`.
- Legacy runner deletion target: remove `adapters/runtime/dev_only/legacy_process_runner.ts` by `v0.3.11-stable` / `2026-05-15` unless an explicit open release blocker depends on it.
- Operator topology diagnostic: `npm run -s ops:production-topology:status`
- Single operator production truth command: `npm run -s ops:status:production`.
- Transport spawn audit: `npm run -s ops:transport:spawn-audit`
- Assimilation v1 support guard: `npm run -s ops:assimilation:v1:support:guard`
- Frozen assimilation v1 slice: one ingress -> orchestration -> assimilation-kernel -> receipt-output path is hardened; broader assimilation surfaces remain experimental.
- Assimilation v1 can graduate only through candidate-build evidence; no new assimilation surface is added during hardening.
- Numeric release thresholds are enforced by `npm run -s ops:release:scorecard:gate` and re-checked directly by `npm run -s ops:production-closure:gate`.
- Release evidence is staged: `ops:release:scorecard:gate` is pre-bundle, and `ops:production-closure:gate` is the final-seal gate after `npm run -s ops:support-bundle:export`.
- Release scorecard compares against the previous release scorecard when a baseline is provided.
- Runtime proof release gate accepts `--proof-track=synthetic|empirical|dual` (default: `dual`) and enforces profile-scoped empirical proof budgets (`rich`, `pure`, `tiny-max`).
- Proof-pack assembly command: `npm run -s ops:release:proof-pack -- --version <release-tag-or-date>`.
- Release verdict artifact: `npm run -s ops:release:verdict`.
- Release-candidate dress rehearsal: `npm run -s ops:release:rc-rehearsal`.
- Release-candidate recovery rehearsal is required every cycle through `npm run -s ops:release:rc-rehearsal`.
- Release-candidate rehearsal also requires chaos, replay, and orchestration hidden-state proofs.
- Shell authority regression guard: `npm run -s ops:shell-layer:boundary`.
- Support bundle is the single incident truth package for release closure.
- Internal/maintenance lanes are not part of the public production SLA.
- Operator diagnostics and incident export: `npm run -s ops:support-bundle:export`
- Terminology canonical policy: `docs/workspace/policy/release_terminology_transition_policy.md`.

## Quick Start

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full
infring setup --yes --defaults
infring setup status --json
infring gateway
infring gateway status
```

Optional Node bootstrap for full command surface:

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node
infring setup --yes --defaults
infring setup status --json
infring gateway
infring gateway status
```

Optional machine-readable install summary (JSON):

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --json
```

### Windows (PowerShell)

Canonical install command (single-line):

`$tmp = Join-Path $env:TEMP "infring-install.ps1"; irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop; powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full; Remove-Item $tmp -Force -ErrorAction SilentlyContinue`

```powershell
# Use the child PowerShell process with -ExecutionPolicy Bypass so locked-down
# execution policies and typo-prone Set-ExecutionPolicy commands do not block install.
$tmp = Join-Path $env:TEMP "infring-install.ps1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full
Remove-Item $tmp -Force -ErrorAction SilentlyContinue
# Confirm command resolution in this shell; if unresolved, use direct-path fallback below.
Get-Command infring -ErrorAction SilentlyContinue
infring setup --yes --defaults
infring setup status --json
infring gateway
infring gateway status
```

Optional machine-readable install summary (JSON):

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full -Json
```

Optional offline/cached reinstall (PowerShell):

```powershell
# Hydrate local cache once for this version.
$env:INFRING_VERSION = "v0.3.12"
$env:INFRING_INSTALL_ASSET_CACHE = "1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full

# Repeat install without network.
$env:INFRING_INSTALL_OFFLINE = "1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full -Offline -ReleaseVersion v0.3.12

# Optional cleanup.
Remove-Item Env:INFRING_INSTALL_OFFLINE -ErrorAction SilentlyContinue
Remove-Item Env:INFRING_VERSION -ErrorAction SilentlyContinue
```

If script execution is still restricted in your environment, use a no-file fallback:

```powershell
$env:INFRING_INSTALL_REPAIR = "1"
$env:INFRING_INSTALL_FULL = "1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 | iex
infring setup --yes --defaults
infring setup status --json
infring gateway
infring gateway status
```

If command resolution has not propagated yet in the same session, run directly:

```powershell
$HOME\.infring\bin\infring.cmd setup --yes --defaults
$HOME\.infring\bin\infring.cmd setup status --json
$HOME\.infring\bin\infring.cmd gateway
$HOME\.infring\bin\infring.cmd gateway status
```

If a release does not publish Windows prebuilt binaries for your architecture, the installer now attempts source fallback automatically. On fresh Windows machines, install prerequisites first if needed:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
# Optional but often required for MSVC source builds:
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"
```

The installer now attempts MSVC Build Tools bootstrap automatically during source fallback when missing (`INFRING_INSTALL_AUTO_MSVC=1` by default; legacy aliases `INFRING_AUTO_MSVC` and `INFRING_AUTO_MSVC_BOOTSTRAP` are also honored; set to `0` to disable).
When `winget` is unavailable or fails, installer fallback now also tries the direct Visual Studio bootstrapper (`https://aka.ms/vs/17/release/vs_BuildTools.exe`) unless `INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP=0`.
Installer diagnostics now also report `winget` availability and auto-bootstrap policy in failure hints to speed remote triage.
Source fallback now also performs target-directory binary discovery when exact binary naming differs, reducing false `source_build_output_missing` failures.
If you must install a pinned version that is missing required Windows prebuilts, opt in to compatible-release prebuilt fallback with `INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK=1`. To disable all compatible-release fallback behavior, set `INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK=0`.

For locked-down environments, you can explicitly disable auto-bootstrap and rely on manual prerequisites:

```powershell
$env:INFRING_INSTALL_AUTO_MSVC = "0"
$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"
$env:INFRING_INSTALL_AUTO_RUSTUP = "0"
$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK = "0"
$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "0"
```

### Verify CLI Is Globally Available

```bash
infring --help
infring list
infring gateway status
```

Canonical wrappers:

- Primary commands: `infring`, `infringctl`, `infringd`

First-launch diagnostics contract (`infring gateway` start/restart):

1. `startup-checkpoint: env_ready`
2. `startup-checkpoint: runtime_contract_state=preflight`
3. `startup-checkpoint: gateway_command_accepted=<start|restart>`
4. `startup-checkpoint: dashboard_status=pending`
5. `startup-checkpoint: next_action=infring gateway status`

Success path:
- `startup-checkpoint: runtime_contract_state=accepted`
- `startup-checkpoint: dashboard_status=running_or_bootstrapping`

Failure path:
- `startup-checkpoint: runtime_contract_state=failed(code=...)`
- `startup-checkpoint: dashboard_status=failed`
- `startup-checkpoint: next_action=infring doctor --json`
- `startup-checkpoint: escalation=infring recover`

If your shell has not reloaded `PATH` yet:

```bash
. "$HOME/.infring/env.sh"
hash -r 2>/dev/null || true
infring --help
```

Fallback (direct path):

```bash
"$HOME/.infring/bin/infring" --help
```

Shell-specific activation snippets:

```bash
# zsh / bash
. "$HOME/.infring/env.sh" && hash -r 2>/dev/null || true && infring --help

# fish
set -gx PATH "$HOME/.infring/bin" $PATH; and command -q rehash; and rehash; and infring --help
```

```powershell
# PowerShell
$env:Path = "$HOME/.infring/bin;$env:Path"; infring --help
```

## Repository Workflows

For local repository work, use the canonical workspace entrypoints:

- Inspect the script surface: `npm run -s workspace:commands`
- Inspect the governed tooling registry: `npm run -s tooling:list`
- Start the canonical local dev loop: `npm run -s workspace:dev`
- Run the canonical full verification path: `npm run -s workspace:verify`
- Run the fast tooling profile: `npm run -s tooling:profile -- --id=fast`
- Run any registered tooling gate: `npm run -s tooling:run -- --id=ops:arch:conformance`
- Inspect indexed lane inventory: `npm run -s lane:list -- --json=1`
- Run any registered lane: `npm run -s lane:run -- --id=<ID>`
- Run lane-specific regression coverage: `npm run -s test:lane:run -- --id=<ID>`

`workspace:verify` delegates to the root [`verify.sh`](/Users/jay/.openclaw/workspace/verify.sh) pipeline, which now reads the manifest-driven tooling profile in [`tests/tooling/config/verify_profiles.json`](/Users/jay/.openclaw/workspace/tests/tooling/config/verify_profiles.json) through the shared runner at [`tests/tooling/scripts/ci/tooling_registry_runner.ts`](/Users/jay/.openclaw/workspace/tests/tooling/scripts/ci/tooling_registry_runner.ts). `workspace:ci` is the canonical CI-equivalent alias for the same path.

Installer behavior:

- Persists `PATH` to shell startup file(s)
- Writes activation script at `~/.infring/env.sh`
- Applies command shims when install dir is not already on `PATH`
- Supports privileged shim fallback for stricter environments (`INFRING_INSTALL_SUDO_SHIMS=auto|off`)

## Install Modes

| Mode | Flag | Purpose |
|---|---|---|
| Minimal (default) | `--minimal` | CLI + daemon wrappers |
| Full | `--full` | Minimal plus workspace runtime bootstrap (release bundle or source fallback) |
| Pure | `--pure` | Rust-only runtime surface (no Node/TS runtime dependency) |
| Tiny-Max | `--tiny-max` | Lowest-footprint pure profile for constrained hardware |
| Repair | `--repair` | Removes stale wrappers/runtime artifacts before reinstall |

## Runtime Surface Contract (Manifest-Backed)

Manifest: `client/runtime/config/install_runtime_manifest_v1.txt`
Node module closure manifest: `client/runtime/config/install_runtime_node_modules_v1.txt`

Required runtime entries:

- `client/runtime/systems/ops/infringd.ts`
- `client/runtime/systems/ops/infring_status_dashboard.ts`
- `client/runtime/systems/ops/infring_unknown_guard.ts`

Required runtime node modules:

- `typescript`
- `ws`

Mode matrix:

- `--full`: required manifest surfaces + full command surface (Node-assisted paths available)
- `--pure` / `--tiny-max`: Rust-only constrained surfaces with optional rich command lanes disabled

If a command is unavailable in your installed mode, the unknown-command guard (`infring_unknown_guard`) returns deterministic wrapper-first recovery guidance (`infring` first).

Examples:

```bash
# Pure
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --pure

# Tiny-max
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --tiny-max

# Repair + full
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --repair --full

# In-place update from an existing install
infring update --repair --full

# Offline update from cached release artifacts (must pin version)
infring update --offline --version v0.3.1-alpha --full
```

## Gateway + Dashboard Operations

```bash
# Start runtime + dashboard
infring gateway

# Check runtime + dashboard status
infring gateway status

# Stop runtime + dashboard
infring gateway stop

# Restart
infring gateway restart
```

Default behavior:

- Auto-opens dashboard on launch
- Supervises runtime and dashboard
- Keeps gateway persistent unless explicitly disabled (`--gateway-persist=0`)

## Command Surfaces

### Rust Fallback Surface (No Node.js)

When Node.js is unavailable, `infring` exposes a reduced but operational command set:

- `gateway [start|stop|restart|status]`
- `update [--repair] [--full|--minimal|--pure|--tiny-max] [--version vX.Y.Z]`
- `verify-gateway [--dashboard-host=127.0.0.1] [--dashboard-port=4173]`
- `start`, `stop`, `restart`
- `dashboard`, `status`
- `session <status|register|resume|send|list>`
- `rag <status|search|chat|memory>`
- `memory <status|search>`
- `adaptive <status|propose|shadow-train|prioritize|graduate>`
- `enterprise-hardening <run|status|export-compliance|identity-surface|certify-scale|dashboard>`
- `benchmark <run|status>`
- `alpha-check [--strict=1|0] [--run-gates=1|0]`
- `research <status|diagnostics|fetch>`
- `help`, `list`, `version`

Not available in Node-free fallback:

- `assimilate` (experimental runtime lane; requires Node.js 22+ full surface)

Install Node.js 22+ to unlock full JS-assisted command surfaces.

### Experimental Runtime Surface (Node.js 22+ Required)

- `assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1]`

Behavior:

- Known targets route to governed Kernel bridge lanes.
- Unknown targets fail as `unadmitted` by default.
- Local simulation mode is test-only and must be explicitly enabled via `--allow-local-simulation=1`.
- Use `--plan-only=1` to emit the canonical assimilation planning chain without executing bridge mutations.

### Release-Closure Diagnostics

- `npm run -s ops:production-topology:status`
- `npm run -s ops:legacy-runner:release-guard`
- `npm run -s ops:stateful-upgrade-rollback:gate`
- `npm run -s dr:gameday`
- `npm run -s dr:gameday:gate`
- `npm run -s ops:support-bundle:export`

### Local Source Workflow

```bash
npm ci
npm run local:init
npm run build
npm run test:ci
npm run gateway
```

<!-- BEGIN: benchmark-snapshot -->
## Performance Snapshot (Latest Artifact)

Latest benchmark source:

- [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

Canonical throughput metric (kernel/shared workload): `kernel_shared_workload_ops_per_sec`
Rich end-to-end command-path throughput metric: `rich_end_to_end_command_path_ops_per_sec`

Current measured rows in that artifact:

| Metric | Rich | Pure (`InfRing (pure)`) | Tiny-Max (`InfRing (tiny-max)`) |
|---|---:|---:|---:|
| Readiness latency (status-path; not zero-boot) | 0.005 ms | 2.262 ms | 2.197 ms |
| Cold start (engine init micro) | 0.005 ms | n/a | n/a |
| Cold start (orchestration component) | 0.000 ms | n/a | n/a |
| Kernel ready | 0.005 ms | n/a | n/a |
| Gateway ready | 0.005 ms | n/a | n/a |
| Dashboard interactive | 0.005 ms | n/a | n/a |
| Idle memory | 7.156 MB | 1.516 MB | 1.516 MB |
| Install artifact size | 29.027 MB | 12.039 MB | 0.631 MB |
| Throughput (kernel/shared workload) | 247,748.44 ops/sec | 247,748.44 ops/sec | 247,748.44 ops/sec |
| Throughput (rich end-to-end command path) | 3.72 ops/sec | n/a | n/a |
| Security systems | 83 | 83 | 83 |
| Channel gateways | 6 | 0 | 0 |
| LLM providers | 3 | 0 | 0 |
| Data channels | 4 | 0 | 0 |
| Plugin marketplace checks | 4 | 0 | 0 |

Preflight metadata in the same artifact:

- `benchmark_preflight.enabled = true`
- `benchmark_validation.ok = true`
- `sample_cv_pct = 1.85` (tolerance `18.75`)
- Artifact timestamp: `2026-04-17T21:12:21.784Z`

Current nuance:

- Public benchmark summaries are generated from the canonical artifact during refresh and verified by `ops:benchmark:public-audit`.
- Reproducibility commands are listed below; claims should match the linked JSON artifact exactly.
- `kernel_shared_workload_ops_per_sec` is a shared kernel workload metric; treat it separately from end-to-end runtime throughput.
- `rich_end_to_end_command_path_ops_per_sec` is the rich command-path throughput metric measured through the governed command bridge.
- `cold_start_ms` in this matrix is a status-path readiness metric, not a full stopped-from-zero dashboard boot benchmark.

### Competitor Comparison (Latest Matrix)

Source: [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

| Project | Cold Start (ms) | Idle Memory (MB) | Install Size (MB) | Throughput (ops/sec) |
|---|---:|---:|---:|---:|
| Infring | 0.005 | 7.156 | 29.027 | 247,748.44 |
| AutoGen | 4000.000 | 250.000 | 200.000 | 0.00 |
| CrewAI | 3000.000 | 200.000 | 100.000 | 0.00 |
| OpenHands | 1300.000 | 150.000 | 95.500 | 0.00 |
| Workflow Graph | 2500.000 | 180.000 | 150.000 | 0.00 |

Refresh commands:

```bash
npm run -s ops:benchmark:refresh
npm run -s ops:benchmark:sanity
npm run -s ops:benchmark:public-audit
npm run -s ops:benchmark:repro
```
<!-- END: benchmark-snapshot -->

### Benchmark Metric Classes (Interpretation Contract)

Use benchmark values by class, not as interchangeable numbers:

- `readiness`: status-path availability timing only (for example, `cold_start_ms` in public matrix rows).
- `kernel_shared_throughput`: synthetic/shared workload throughput (`kernel_shared_workload_ops_per_sec`) for kernel-level comparative efficiency.
- `end_to_end_command_throughput`: governed rich-runtime command-path throughput (`rich_end_to_end_command_path_ops_per_sec`) for operator-facing runtime behavior.

Operator caveats:

- Do not interpret readiness latency as full stopped-from-zero dashboard boot time.
- Do not use kernel/shared throughput as a proxy for full command-path throughput.
- Use the latest artifact and reproducibility commands together:
  - `docs/client/reports/benchmark_matrix_run_latest.json`
  - `npm run -s ops:benchmark:refresh`
  - `npm run -s ops:benchmark:public-audit`
















## What Ships Today

- Rust-authoritative Kernel control and policy lanes under `core/layer0`, `core/layer1`, and `core/layer2`
- Gateway runtime with dashboard serving and health probes
- Agent/session/memory/rag/research command surfaces
- Signed policy/config registry surfaces under `client/runtime/config/**`
- Regression and governance gates in `tests/**` and `verify.sh`

## Public SDK

- TypeScript SDK package: `packages/infring-sdk` (`@infring/sdk`)
- Stable contract methods:
  - `submitTask`
  - `inspectReceipts`
  - `queryMemory`
  - `reviewEvidence`
  - `runAssimilation`
  - `attachPolicies`
- Reference app build proof:
  - `examples/apps/reference-task-submit`
  - `examples/apps/reference-receipts-memory`
  - `examples/apps/reference-assimilation-policy`
  - `npm run -s ops:sdk:surface:build`

## Repository Map

| Path | Responsibility |
|---|---|
| `core/` | Rust authority layers and runtime Kernel |
| `client/runtime/systems/` | Shell runtime wrappers and operator surfaces |
| `client/runtime/config/` | Policy manifests, registries, and guardrails |
| `adapters/` | Integration bridges |
| `apps/` | Runnable app surfaces and examples |
| `tests/` | Regression, governance, and toolchain validation |
| `docs/` | Runbooks, architecture, onboarding, and policies |
| `planes/` | Three-plane architecture contract definitions |
| `install.sh`, `install.ps1` | Cross-platform installers |

## Documentation

- [Getting Started](docs/client/GETTING_STARTED.md)
- [Onboarding Playbook](docs/client/ONBOARDING_PLAYBOOK.md)
- [Developer Lane Quickstart](docs/client/DEVELOPER_LANE_QUICKSTART.md)
- [Operator Runbook](docs/client/OPERATOR_RUNBOOK.md)
- [Security Posture](docs/client/SECURITY_POSTURE.md)
- [Backlog Governance](docs/client/BACKLOG_GOVERNANCE.md)
- [Architecture](ARCHITECTURE.md)
- [Roadmap](roadmap.md)
- [Glossary](glossary.md)

## Contributing

1. Read [CONTRIBUTING.md](docs/workspace/CONTRIBUTING.md).
2. Run tests and required gates for touched surfaces.
3. Keep claims evidence-backed (diff + test output + runtime proof).
4. Update [CHANGELOG.md](docs/workspace/CHANGELOG.md) for user-visible changes.

## Security

- Disclosure policy: [SECURITY.md](SECURITY.md)
- Runtime security docs: [docs/client/SECURITY.md](docs/client/SECURITY.md)

## License

InfRing uses dual licensing:

- Apache-2.0 for open Kernel scope: [LICENSE-APACHE-2.0](LICENSE-APACHE-2.0)
- LicenseRef-InfRing-NC-1.0 for default NC scope: [LICENSE-INFRING-NC-1.0](LICENSE-INFRING-NC-1.0)

Canonical SPDX matrix: [LICENSE_MATRIX.json](LICENSE_MATRIX.json)
Human-readable path scope: [LICENSE_SCOPE.md](LICENSE_SCOPE.md)
