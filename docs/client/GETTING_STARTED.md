# Getting Started

InfRing runs with a Rust core and a thin TypeScript surface routed through conduit.

## 1) Install in <2 minutes

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full
infring setup --yes --defaults
infring setup status --json
infring gateway
infring gateway status
```

If PATH has not refreshed in the same shell, run directly: `~/.infring/bin/infring gateway`.

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
# Remove-Item is silent on success in PowerShell.
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
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full -Offline -ReleaseVersion $env:INFRING_VERSION

# Optional cleanup.
Remove-Item Env:INFRING_INSTALL_OFFLINE -ErrorAction SilentlyContinue
Remove-Item Env:INFRING_VERSION -ErrorAction SilentlyContinue
```

If PATH has not refreshed in the same shell, run directly: `$HOME\.infring\bin\infring.cmd gateway`.

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

If a release has no Windows prebuilt binary for your architecture, the installer falls back to building from source. Install prerequisites first on fresh machines:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
# Optional but often required for MSVC source builds:
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"
```

When `winget` is unavailable or fails, the installer now attempts a direct Build Tools bootstrapper fallback by default. Disable that path only in locked-down environments:

```powershell
$env:INFRING_INSTALL_AUTO_MSVC = "0"
$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"
$env:INFRING_INSTALL_AUTO_RUSTUP = "0"
$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK = "0"
$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "0"
```

For pinned installs with missing Windows prebuilts, opt in to compatible-release prebuilt fallback with:
`$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "1"`.

### Optional: Python Wrapper (`pipx`)

```bash
pipx install infring-cli-wrapper
infring --help
```

## 2) Verify binaries

```bash
infring --help
infringctl --help
infringd --help
```

Legacy aliases (`infring`, `infringctl`, `infringd`) are compatibility-only and should not be used in new automation.

## Runtime manifest and mode contract

Install/runtime required entries are defined in:

- `client/runtime/config/install_runtime_manifest_v1.txt`
- `client/runtime/config/install_runtime_node_modules_v1.txt`

Current required runtime entries:

- `client/runtime/systems/ops/infringd.ts`
- `client/runtime/systems/ops/infring_status_dashboard.ts`
- `client/runtime/systems/ops/infring_unknown_guard.ts`

Current required runtime node modules:

- `typescript`
- `ws`

Mode behavior:

- `--full`: full command surface, dashboard and setup flow available.
- `--pure` / `--tiny-max`: constrained Rust-first runtime surface; optional rich lanes are intentionally limited.
- `--minimal`: install-light profile; optional onboarding/dashboard surfaces may require explicit setup/opt-in.

Mode contract (operator-facing):

| Mode | Gateway | Dashboard/UI surface | Setup interaction default | Notes |
| --- | --- | --- | --- | --- |
| `full` | Available | Available | Interactive on TTY, deterministic defaults in non-interactive | Recommended for full operator workflows |
| `pure` | Available | Limited/optional | Non-interactive defaults stay conservative | Rust-first path for constrained environments |
| `tiny-max` | Available | Limited/optional | Non-interactive defaults stay conservative | Minimal footprint profile |
| `minimal` | Available | Optional/limited | Setup may require explicit invocation | Install-light compatibility profile |

Required vs optional command surfaces (release-mode contract):

| Surface | Required in all modes | `full` | `minimal` | `pure` / `tiny-max` |
| --- | --- | --- | --- | --- |
| Wrappers (`infring`, `infringctl`, `infringd`) | Yes | Yes | Yes | Yes |
| Setup lane (`infring setup`, `infring setup status --json`) | Yes | Yes | Yes | Yes |
| Gateway status (`infring gateway status`) | Yes | Yes | Yes | Yes |
| Rich gateway launch (`infring gateway`) | Optional | Available | Available (may require explicit setup) | Limited/optional by design |

## First-launch diagnostics contract

For `infring gateway` start/restart, expect deterministic checkpoint output:

1. `startup-checkpoint: env_ready`
2. `startup-checkpoint: runtime_contract_state=preflight`
3. `startup-checkpoint: gateway_command_accepted=<start|restart>`
4. `startup-checkpoint: dashboard_status=pending`
5. `startup-checkpoint: next_action=infring gateway status`

On success:
- `startup-checkpoint: runtime_contract_state=accepted`
- `startup-checkpoint: dashboard_status=running_or_bootstrapping`

On failure:
- `startup-checkpoint: runtime_contract_state=failed(code=...)`
- `startup-checkpoint: dashboard_status=failed`
- `startup-checkpoint: next_action=infring doctor --json`
- `startup-checkpoint: escalation=infring recover`

## 3) Start core surfaces

```bash
infringd
infring status
infring contract-check
```

## 4) Run mandatory gates

```bash
cargo run --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops -- contract-check
NODE_PATH=$PWD/node_modules npm run -s formal:invariants:run
```

## Notes

- Rust is the source of truth for kernel logic (primitives, constitution, policy, receipts).
- TypeScript is limited to thin shell surfaces and extension/UI workflows via conduit.
- Python is optional and only provides a thin CLI wrapper that forwards to Rust.
