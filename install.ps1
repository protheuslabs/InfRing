param(
  [switch]$Full,
  [switch]$Minimal,
  [switch]$Pure,
  [switch]$TinyMax,
  [switch]$Repair,
  [switch]$Offline,
  [switch]$StrictSmoke,
  [switch]$Json,
  [switch]$VerifyInstallSummaryContract,
  [switch]$Force,
  [string]$InstallDir,
  [string]$TmpDir
)

$ErrorActionPreference = "Stop"

$RepoOwner = "protheuslabs"
$RepoName = "InfRing"
$DefaultApi = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
$DefaultReleasesApi = "https://api.github.com/repos/$RepoOwner/$RepoName/releases?per_page=30"
$DefaultLatestUrl = "https://github.com/$RepoOwner/$RepoName/releases/latest"
$DefaultBase = "https://github.com/$RepoOwner/$RepoName/releases/download"
$ReadmeWindowsInstallCommand = '$tmp = Join-Path $env:TEMP "infring-install.ps1"; irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop; powershell.exe -NoProfile -ExecutionPolicy Bypass -File $tmp -Repair -Full; Remove-Item $tmp -Force -ErrorAction SilentlyContinue'

$InstallDir = if ($InstallDir) {
  $InstallDir
} elseif ($env:INFRING_INSTALL_DIR) {
  $env:INFRING_INSTALL_DIR
} else {
  Join-Path $HOME ".infring\bin"
}
$InstallDirExplicit = $PSBoundParameters.ContainsKey("InstallDir")
$legacyInstallDir = Join-Path $HOME ".infring\bin"
$canonicalInstallDir = Join-Path $HOME ".infring\bin"
$normalizedInstallDir = if ($InstallDir) { $InstallDir.TrimEnd([char]92, [char]47).ToLower() } else { "" }
$normalizedLegacyInstallDir = $legacyInstallDir.TrimEnd([char]92, [char]47).ToLower()
if (
  (-not $InstallDirExplicit) -and
  $InstallDir -and
  (
    $normalizedInstallDir -eq $normalizedLegacyInstallDir -or
    $normalizedInstallDir.EndsWith("\\.infring\\bin") -or
    $normalizedInstallDir.EndsWith("/.infring/bin")
  )
) {
  Write-Host "[infring install] detected legacy compatibility install dir ($InstallDir); migrating to canonical $canonicalInstallDir"
  $InstallDir = $canonicalInstallDir
}
$TmpDir = if ($TmpDir) {
  $TmpDir
} elseif ($env:INFRING_TMP_DIR) {
  $env:INFRING_TMP_DIR
} else {
  $null
}
$RequestedVersion = if ($env:INFRING_VERSION) { $env:INFRING_VERSION } else { "latest" }
$ApiUrl = if ($env:INFRING_RELEASE_API_URL) { $env:INFRING_RELEASE_API_URL } else { $DefaultApi }
$ReleasesApiUrl = if ($env:INFRING_RELEASES_API_URL) { $env:INFRING_RELEASES_API_URL } else { $DefaultReleasesApi }
$LatestUrl = if ($env:INFRING_RELEASE_LATEST_URL) { $env:INFRING_RELEASE_LATEST_URL } else { $DefaultLatestUrl }
$BaseUrl = if ($env:INFRING_RELEASE_BASE_URL) { $env:INFRING_RELEASE_BASE_URL } else { $DefaultBase }
$InstallFull = $false
if ($env:INFRING_INSTALL_FULL -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_FULL.ToLower()) {
  $InstallFull = $true
}
$InstallPure = $false
if ($env:INFRING_INSTALL_PURE -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_PURE.ToLower()) {
  $InstallPure = $true
}
$InstallTinyMax = $false
if ($env:INFRING_INSTALL_TINY_MAX -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_TINY_MAX.ToLower()) {
  $InstallTinyMax = $true
}
$InstallRepair = $false
if ($env:INFRING_INSTALL_REPAIR -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_REPAIR.ToLower()) {
  $InstallRepair = $true
}
$InstallOffline = $false
if ($env:INFRING_INSTALL_OFFLINE -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_OFFLINE.ToLower()) {
  $InstallOffline = $true
}
$InstallStrictSmoke = $false
if ($env:INFRING_INSTALL_STRICT_SMOKE -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_STRICT_SMOKE.ToLower()) {
  $InstallStrictSmoke = $true
}
if ($Full) { $InstallFull = $true }
if ($Minimal) { $InstallFull = $false }
if ($Pure) {
  $InstallPure = $true
  $InstallFull = $false
}
if ($TinyMax) {
  $InstallTinyMax = $true
  $InstallPure = $true
  $InstallFull = $false
}
if ($Repair) { $InstallRepair = $true }
if ($Offline) { $InstallOffline = $true }
if ($StrictSmoke) { $InstallStrictSmoke = $true }
if ($Force) {
  # Compatibility shim for operators accustomed to `-Force`.
  # Treat this as an explicit repair pass and bias to `-Full` unless the caller
  # already selected a constrained mode.
  $InstallRepair = $true
  if (-not ($Minimal -or $Pure -or $TinyMax)) {
    $InstallFull = $true
  }
}

$InstallJson = $false
if ($env:INFRING_INSTALL_JSON -and @("1", "true", "yes", "on") -contains $env:INFRING_INSTALL_JSON.ToLower()) {
  $InstallJson = $true
}
if ($Json) { $InstallJson = $true }
$InstallSummaryJsonPath = if ($env:INFRING_INSTALL_SUMMARY_JSON_FILE) {
  $env:INFRING_INSTALL_SUMMARY_JSON_FILE
} else {
  Join-Path $HOME ".infring\logs\last_install_summary.json"
}
$InstallSummaryTextPath = if ($env:INFRING_INSTALL_SUMMARY_FILE) {
  $env:INFRING_INSTALL_SUMMARY_FILE
} else {
  Join-Path $HOME ".infring\logs\last_install_summary.txt"
}
$InstallSmokeSummaryJsonPath = if ($env:INFRING_INSTALL_SMOKE_SUMMARY_JSON_FILE) {
  $env:INFRING_INSTALL_SMOKE_SUMMARY_JSON_FILE
} else {
  Join-Path $HOME ".infring\logs\last_install_smoke_summary.json"
}
$script:InstallAssetLockfile = if ($env:INFRING_INSTALL_ASSET_LOCKFILE) {
  [string]$env:INFRING_INSTALL_ASSET_LOCKFILE
} else {
  Join-Path $HOME ".infring\state\install_asset_lock_v1.tsv"
}
$script:InstallClientRuntimeMode = "not_installed"
$script:InstallRuntimeContractStatus = "not_checked"
$script:RuntimeManifestRel = "client/runtime/config/install_runtime_manifest_v1.txt"
$script:RuntimeNodeModuleManifestRel = if ($env:INFRING_RUNTIME_NODE_MODULE_MANIFEST_REL) {
  $env:INFRING_RUNTIME_NODE_MODULE_MANIFEST_REL
} else {
  "client/runtime/config/install_runtime_node_modules_v1.txt"
}
$script:RuntimeTier1RequiredEntrypoints = @(
  "client/runtime/systems/ops/infringd.ts",
  "client/runtime/systems/ops/infring_status_dashboard.ts",
  "client/runtime/systems/ops/infring_unknown_guard.ts"
)
$script:InstallToolchainPolicyRaw = if ($env:INFRING_INSTALL_TOOLCHAIN_POLICY) {
  [string]$env:INFRING_INSTALL_TOOLCHAIN_POLICY
} else {
  "auto"
}
$script:InstallToolchainPolicy = switch ($script:InstallToolchainPolicyRaw.ToLowerInvariant()) {
  "fail" { "fail_closed" }
  "fail_closed" { "fail_closed" }
  "strict" { "fail_closed" }
  default { "auto" }
}
$script:InstallAssetCache = $true
if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ASSET_CACHE)) {
  $script:InstallAssetCache = Installer-TruthyFlag $env:INFRING_INSTALL_ASSET_CACHE $true
}
if ($InstallOffline) {
  $script:InstallAssetCache = $true
}
$script:InstallSummaryFinalized = $false
$script:ResolvedInstallVersionForSummary = ""
$script:ResolvedInstallTripleForSummary = ""

$script:InstallScriptRoot = if ($PSScriptRoot) {
  $PSScriptRoot
} elseif ($MyInvocation.MyCommand.Path) {
  Split-Path -Parent $MyInvocation.MyCommand.Path
} else {
  [string](Get-Location)
}

$script:InstallModuleDir = if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_MODULE_DIR)) {
  [string]$env:INFRING_INSTALL_MODULE_DIR
} elseif (Test-Path -LiteralPath (Join-Path $script:InstallScriptRoot "install/modules")) {
  Join-Path $script:InstallScriptRoot "install/modules"
} else {
  Join-Path $script:InstallScriptRoot "modules"
}

function Test-InstallerModuleDispatchEnabled {
  $flag = [string]$env:INFRING_INSTALL_USE_MODULES
  if ([string]::IsNullOrWhiteSpace($flag)) {
    return $true
  }
  return @("1", "true", "yes", "on") -contains $flag.ToLowerInvariant()
}

function Import-InstallerModuleIfPresent {
  param([Parameter(Mandatory=$true)][string]$ModuleName)
  if (-not (Test-InstallerModuleDispatchEnabled)) {
    return $false
  }
  $modulePath = Join-Path $script:InstallModuleDir $ModuleName
  if (-not (Test-Path -LiteralPath $modulePath)) {
    return $false
  }
  . $modulePath
  return $true
}

$script:InstallerCompletionModuleLoaded = Import-InstallerModuleIfPresent -ModuleName "completion_card.ps1"
$script:InstallerWindowsWrappersModuleLoaded = Import-InstallerModuleIfPresent -ModuleName "windows_wrappers.ps1"

function Invoke-OptionalWindowsWrapperModule {
  param(
    [Parameter(Mandatory=$true)][string]$BinDir,
    [Parameter(Mandatory=$true)][object[]]$WrapperSpecs
  )
  if (-not (Test-InstallerModuleDispatchEnabled)) {
    return $false
  }
  $writer = Get-Command Write-InfringWindowsWrappers -ErrorAction SilentlyContinue
  if (-not $writer) {
    return $false
  }
  try {
    Write-InfringWindowsWrappers -BinDir $BinDir -WrapperSpecs $WrapperSpecs | Out-Null
    return $true
  } catch {
    Write-Host ("[infring install] installer module dispatch skipped: {0}" -f $_.Exception.Message)
    return $false
  }
}

function Write-InstallCompletionCard {
  param(
    [string]$Version,
    [string]$InstallDir,
    [bool]$RuntimeInstalled = $true,
    [string]$RuntimeMode = "",
    [string]$BootstrapOnlyReason = ""
  )

  $launcherPath = Join-Path $InstallDir "infring.cmd"
  if (-not (Test-Path -LiteralPath $launcherPath)) {
    $launcherPath = Join-Path $InstallDir "infring.ps1"
  }
  $completionWriter = Get-Command Write-InfringInstallCompletionCard -ErrorAction SilentlyContinue
  if ((Test-InstallerModuleDispatchEnabled) -and $completionWriter) {
    Write-Host ""
    Write-InfringInstallCompletionCard -Version ("{0}." -f $Version) -Location $launcherPath -Command "infring --help" -RuntimeInstalled $RuntimeInstalled -RuntimeMode $RuntimeMode -BootstrapOnlyReason $BootstrapOnlyReason
    return
  }

  Write-Host ""
  Write-Host "Setting up InfRing..."
  Write-Host ""
  if (-not $RuntimeInstalled) {
    Write-Host "BOOTSTRAP INSTALLED: InfRing runtime pending." -ForegroundColor DarkYellow
    Write-Host ""
    Write-Host -NoNewline "  Version: "
    Write-Host "$Version." -ForegroundColor DarkYellow
    Write-Host "  Location: $launcherPath"
    Write-Host -NoNewline "  Runtime: "
    Write-Host "$RuntimeMode (runtime binaries unavailable)" -ForegroundColor DarkYellow
    Write-Host ""
    Write-Host -NoNewline "  Next: Run "
    Write-Host -NoNewline "infring recover" -ForegroundColor DarkYellow
    Write-Host " after MSVC tools or Windows runtime assets are available."
    Write-Host ""
    Write-Host "Installation incomplete: runtime pending." -ForegroundColor DarkYellow
    return
  }
  Write-Host "SUCCESS: InfRing successfully installed!" -ForegroundColor Green
  Write-Host ""
  Write-Host -NoNewline "  Version: "
  Write-Host "$Version." -ForegroundColor DarkYellow
  Write-Host "  Location: $launcherPath"
  Write-Host ""
  Write-Host -NoNewline "  Next: Run "
  Write-Host -NoNewline "infring --help" -ForegroundColor DarkYellow
  Write-Host " to get started."
  Write-Host ""
  Write-Host "Installation complete!"
}

function Test-InstallSummarySuccessContract {
  param(
    [string]$SummaryPath
  )

  if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    Write-Host "[infring install] summary contract failed: summary path missing"
    return $false
  }
  if (-not (Test-Path $SummaryPath)) {
    Write-Host "[infring install] summary contract failed: missing $SummaryPath"
    return $false
  }

  $rows = @(Get-Content -Path $SummaryPath -ErrorAction SilentlyContinue | Where-Object {
      -not [string]::IsNullOrWhiteSpace([string]$_)
    })
  if ($rows.Count -eq 0) {
    Write-Host "[infring install] summary contract failed: empty summary file"
    return $false
  }

  $hasCompletedAt = $false
  $hasStatus = $false
  $hasWorkspaceRefreshRequired = $false
  $hasWorkspaceRefreshApplied = $false
  $hasWorkspaceReleaseTagWritten = $false
  $hasWorkspaceReleaseTagWriteVerified = $false
  foreach ($row in $rows) {
    $trimmed = ([string]$row).Trim()
    if ($trimmed.StartsWith("completed_at:")) { $hasCompletedAt = $true }
    if ($trimmed.StartsWith("status:")) { $hasStatus = $true }
    if ($trimmed.StartsWith("workspace_runtime_refresh_required:")) { $hasWorkspaceRefreshRequired = $true }
    if ($trimmed.StartsWith("workspace_runtime_refresh_applied:")) { $hasWorkspaceRefreshApplied = $true }
    if ($trimmed.StartsWith("workspace_release_tag_written:")) { $hasWorkspaceReleaseTagWritten = $true }
    if ($trimmed.StartsWith("workspace_release_tag_write_verified:")) { $hasWorkspaceReleaseTagWriteVerified = $true }
  }

  if (-not $hasCompletedAt) {
    Write-Host "[infring install] summary contract failed: completed_at missing"
    return $false
  }
  if (-not $hasStatus) {
    Write-Host "[infring install] summary contract failed: status missing"
    return $false
  }
  if (-not $hasWorkspaceRefreshRequired) {
    Write-Host "[infring install] summary contract failed: workspace_runtime_refresh_required missing"
    return $false
  }
  if (-not $hasWorkspaceRefreshApplied) {
    Write-Host "[infring install] summary contract failed: workspace_runtime_refresh_applied missing"
    return $false
  }
  if (-not $hasWorkspaceReleaseTagWritten) {
    Write-Host "[infring install] summary contract failed: workspace_release_tag_written missing"
    return $false
  }
  if (-not $hasWorkspaceReleaseTagWriteVerified) {
    Write-Host "[infring install] summary contract failed: workspace_release_tag_write_verified missing"
    return $false
  }

  $lastLine = ([string]$rows[$rows.Count - 1]).Trim()
  if (($lastLine -ne "status: success") -and ($lastLine -ne "status: runtime_pending")) {
    Write-Host "[infring install] summary contract failed: status is not terminal success/runtime_pending line"
    return $false
  }

  Write-Host "[infring install] summary contract: ok"
  return $true
}

function Write-InstallFailureSummary {
  param(
    [string]$FailureReason,
    [string]$ExitCode = "1"
  )

  if ([bool]$script:InstallSummaryFinalized) {
    return
  }

  function Ensure-InstallFailureBootstrapWrappers {
    param([string]$InstallRoot)

    if ([string]::IsNullOrWhiteSpace([string]$InstallRoot)) {
      return $false
    }

    try {
      New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
    } catch {
      return $false
    }

    $cmdTemplate = @'
@echo off
setlocal
set "_BOOTSTRAP_ACTION=%~n0"
set "_OPS=%~dp0infring-ops.exe"
set "_DAEMON=%~dp0infringd.exe"
set "_CONDUIT=%~dp0conduit_daemon.exe"

if /I "%_BOOTSTRAP_ACTION%"=="infring" (
  if exist "%_OPS%" (
    "%_OPS%" infringctl %*
    exit /b %ERRORLEVEL%
  )
  goto :bootstrap
)

if /I "%_BOOTSTRAP_ACTION%"=="infringctl" (
  if exist "%_OPS%" (
    "%_OPS%" infringctl %*
    exit /b %ERRORLEVEL%
  )
  goto :bootstrap
)

if /I "%_BOOTSTRAP_ACTION%"=="infringd" (
  if exist "%_DAEMON%" (
    "%_DAEMON%" %*
    exit /b %ERRORLEVEL%
  )
  if exist "%_CONDUIT%" (
    "%_CONDUIT%" %*
    exit /b %ERRORLEVEL%
  )
  if exist "%_OPS%" (
    "%_OPS%" spine %*
    exit /b %ERRORLEVEL%
  )
  goto :bootstrap
)

:bootstrap
echo [infring bootstrap] runtime binaries are not installed on this machine yet.
echo [infring bootstrap] run: install.ps1 -Repair -Full
exit /b 0
'@

    try {
      $ps1Template = @'
param([Parameter(ValueFromRemainingArguments=$true)][string[]]$CommandArgs)
$target = Join-Path $PSScriptRoot "__TARGET__"
if (-not (Test-Path -LiteralPath $target)) {
  Write-Warning "[infring shim] bootstrap wrapper missing: $target"
  Write-Host "[infring bootstrap] run: install.ps1 -Repair -Full"
  exit 0
}
& $target @CommandArgs
exit $LASTEXITCODE
'@
      $wrapperMap = @(
        @{ cmd = "infring.cmd"; ps1 = "infring.ps1" },
        @{ cmd = "infringctl.cmd"; ps1 = "infringctl.ps1" },
        @{ cmd = "infringd.cmd"; ps1 = "infringd.ps1" }
      )
      foreach ($item in $wrapperMap) {
        $cmdPath = Join-Path $InstallRoot ([string]$item.cmd)
        $cmdContent = $cmdTemplate
        Set-Content -LiteralPath $cmdPath -Value $cmdContent -Encoding ASCII -Force
        $ps1Path = Join-Path $InstallRoot ([string]$item.ps1)
        $psContent = $ps1Template.Replace("__TARGET__", [string]$item.cmd)
        Set-Content -LiteralPath $ps1Path -Value $psContent -Encoding UTF8 -Force
      }
      Remove-StaleWindowsCommandShims -ShimInstallDir $InstallRoot
      return $true
    } catch {
      return $false
    }
  }

  try {
    $summaryTextDir = Split-Path -Parent $InstallSummaryTextPath
    if (-not [string]::IsNullOrWhiteSpace($summaryTextDir)) {
      New-Item -ItemType Directory -Force -Path $summaryTextDir | Out-Null
    }
    $failedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $resolvedVersion = if ([string]::IsNullOrWhiteSpace([string]$script:ResolvedInstallVersionForSummary)) { "unknown" } else { [string]$script:ResolvedInstallVersionForSummary }
    $resolvedTriple = if ([string]::IsNullOrWhiteSpace([string]$script:ResolvedInstallTripleForSummary)) { "unknown" } else { [string]$script:ResolvedInstallTripleForSummary }
    $cleanReason = if ([string]::IsNullOrWhiteSpace($FailureReason)) { "installer_exception" } else { ([string]$FailureReason -replace "[\\r\\n]+", " ").Trim() }
    $bootstrapWrappersWritten = Ensure-InstallFailureBootstrapWrappers -InstallRoot $InstallDir
    if ($bootstrapWrappersWritten) {
      Write-Host "[infring install] wrote bootstrap wrappers for failed install recovery"
    }

    $failureRows = @(
      "infring_install_summary_v1",
      "version: $resolvedVersion",
      "triple: $resolvedTriple",
      "install_mode_full: $([string][bool]$InstallFull).ToLower()",
      "install_mode_pure: $([string][bool]$InstallPure).ToLower()",
      "install_mode_tiny_max: $([string][bool]$InstallTinyMax).ToLower()",
      "install_mode_repair: $([string][bool]$InstallRepair).ToLower()",
      "install_mode_offline: $([string][bool]$InstallOffline).ToLower()",
      "install_mode_strict_smoke: $([string][bool]$InstallStrictSmoke).ToLower()",
      "asset_cache_enabled: $([string][bool]$script:InstallAssetCache).ToLower()",
      "toolchain_policy: $([string]$script:InstallToolchainPolicy)",
      "dashboard_smoke: not_run",
      "dashboard_runtime_persistence: not_started",
      "failed_at: $failedAt",
      "exit_code: $ExitCode",
      "failure_reason: $cleanReason",
      "bootstrap_wrappers_written: $([string][bool]$bootstrapWrappersWritten).ToLower()",
      "launcher: infring gateway",
      "recovery: install.ps1 -Repair -Full",
      "summary_json: $InstallSummaryJsonPath",
      "smoke_summary_json: $InstallSmokeSummaryJsonPath",
      "status: failed"
    )
    $failureRows | Set-Content -Path $InstallSummaryTextPath -Encoding UTF8
    Write-Host "[infring install] summary text: $InstallSummaryTextPath"

    $failureSummaryDir = Split-Path -Parent $InstallSummaryJsonPath
    if (-not [string]::IsNullOrWhiteSpace($failureSummaryDir)) {
      New-Item -ItemType Directory -Force -Path $failureSummaryDir | Out-Null
    }
    $failurePayload = @{
      ok = $false
      type = "infring_install_failure_summary"
      version = $resolvedVersion
      triple = $resolvedTriple
      install_mode = @{
        full = [bool]$InstallFull
        pure = [bool]$InstallPure
        tiny_max = [bool]$InstallTinyMax
        repair = [bool]$InstallRepair
        offline = [bool]$InstallOffline
        strict_smoke = [bool]$InstallStrictSmoke
      }
      verification = @{
        toolchain_policy = [string]$script:InstallToolchainPolicy
        asset_cache_enabled = [bool]$script:InstallAssetCache
        dashboard_smoke = "not_run"
        dashboard_runtime_persistence = "not_started"
        failed_at = $failedAt
        exit_code = [string]$ExitCode
        failure_reason = $cleanReason
        bootstrap_wrappers_written = [bool]$bootstrapWrappersWritten
      }
      commands = @{
        launcher = "infring gateway"
        verify = "infring doctor --json"
        recovery = "install.ps1 -Repair -Full"
      }
      summary_files = @{
        text = $InstallSummaryTextPath
        json = $InstallSummaryJsonPath
        smoke_json = $InstallSmokeSummaryJsonPath
      }
    }
    $failurePayload | ConvertTo-Json -Depth 8 | Set-Content -Path $InstallSummaryJsonPath -Encoding UTF8
    Write-Host "[infring install] summary json: $InstallSummaryJsonPath"
    $script:InstallSummaryFinalized = $true
  } catch {
    # fail-closed: do not hide original failure if summary write itself fails
  }
}

trap {
  $reason = "installer_exception"
  try {
    if ($_ -and $_.Exception -and $_.Exception.Message) {
      $reason = [string]$_.Exception.Message
    }
  } catch {}
  try {
    Write-Host ("[infring install] failure reason: {0}" -f $reason)
  } catch {}
  Write-InstallFailureSummary -FailureReason $reason -ExitCode "1"
  throw
}

if ($TmpDir) {
  New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null
  $env:TMPDIR = $TmpDir
  $env:TEMP = $TmpDir
  $env:TMP = $TmpDir
}

$script:SourceFallbackDir = $null
$script:SourceFallbackTmp = $null
$script:LastBinaryInstallFailure = $null
$script:LastBinaryInstallFailureReason = ""
$script:WindowsInstallPreflight = $null
$script:WindowsMsvcBootstrapAttempted = $false
$script:WindowsMsvcBootstrapSucceeded = $false
$script:ChecksumManifestVersion = ""
$script:ChecksumManifestPath = $null
$script:ChecksumManifestAssetName = ""
$script:ChecksumManifestTmpDir = $null
$script:RepairArchiveRun = ""
$script:RepairRemovedCount = 0
$script:RepairPreservedCount = 0
$script:WorkspaceRuntimeRefreshReason = ""
$script:WorkspaceRuntimeRefreshRequired = $false
$script:WorkspaceRuntimeRefreshApplied = $false
$script:WorkspaceRuntimeTagStateMissing = $false
$script:WorkspaceReleaseTagPrevious = ""
$script:WorkspaceReleaseTagCurrent = ""
$script:WorkspaceReleaseTagWriteApplied = $false
$script:WorkspaceReleaseTagWriteVerified = $false

function Installer-TruthyFlag([string]$RawValue, [bool]$DefaultValue = $false) {
  if ([string]::IsNullOrWhiteSpace($RawValue)) {
    return $DefaultValue
  }
  return @("1", "true", "yes", "on") -contains $RawValue.ToLower()
}

function Install-AutoRustupEnabled {
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_AUTO_RUSTUP)) {
    return Installer-TruthyFlag $env:INFRING_INSTALL_AUTO_RUSTUP $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_AUTO_RUSTUP)) {
    return Installer-TruthyFlag $env:INFRING_AUTO_RUSTUP $true
  }
  return $true
}

function Install-AutoMsvcBootstrapEnabled {
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_AUTO_MSVC)) {
    return Installer-TruthyFlag $env:INFRING_INSTALL_AUTO_MSVC $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_AUTO_MSVC_BOOTSTRAP)) {
    return Installer-TruthyFlag $env:INFRING_AUTO_MSVC_BOOTSTRAP $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_AUTO_MSVC)) {
    return Installer-TruthyFlag $env:INFRING_AUTO_MSVC $true
  }
  return $true
}

function Install-AllowDirectMsvcBootstrapEnabled {
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP)) {
    return Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_ALLOW_DIRECT_MSVC_BOOTSTRAP)) {
    return Installer-TruthyFlag $env:INFRING_ALLOW_DIRECT_MSVC_BOOTSTRAP $true
  }
  return $true
}

function Install-AllowNoMsvcSourceFallback {
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK)) {
    return Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_ALLOW_NO_MSVC_SOURCE_FALLBACK)) {
    return Installer-TruthyFlag $env:INFRING_ALLOW_NO_MSVC_SOURCE_FALLBACK $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_ALLOW_NO_MSVC)) {
    return Installer-TruthyFlag $env:INFRING_ALLOW_NO_MSVC $true
  }
  return $true
}

function Install-AllowCompatibleReleaseFallback {
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK)) {
    return Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK $true
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_ALLOW_COMPATIBLE_RELEASE_FALLBACK)) {
    return Installer-TruthyFlag $env:INFRING_ALLOW_COMPATIBLE_RELEASE_FALLBACK $true
  }
  return $true
}

function Install-AllowPinnedVersionCompatibleFallback {
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK)) {
    return Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK $false
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK)) {
    return Installer-TruthyFlag $env:INFRING_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK $false
  }
  return $false
}

function Resolve-Arch {
  $archRaw = if ($env:PROCESSOR_ARCHITECTURE) { $env:PROCESSOR_ARCHITECTURE } else { [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() }
  switch ($archRaw.ToLower()) {
    "amd64" { "x86_64" }
    "arm64" { "aarch64" }
    default { throw "Unsupported architecture: $archRaw" }
  }
}

function Resolve-HostOsFlags {
  $runtime = [System.Runtime.InteropServices.RuntimeInformation]
  $osPlatform = [System.Runtime.InteropServices.OSPlatform]
  $isWindowsRuntime = $runtime::IsOSPlatform($osPlatform::Windows)
  $isLinuxRuntime = $runtime::IsOSPlatform($osPlatform::Linux)
  $isMacRuntime = $runtime::IsOSPlatform($osPlatform::OSX)

  # PowerShell 6+ exposes $IsWindows/$IsLinux/$IsMacOS.
  # Windows PowerShell 5.1 does not, so runtime probing must remain the source of truth.
  $isWindows = if (Get-Variable -Name IsWindows -Scope Global -ErrorAction SilentlyContinue) {
    [bool](Get-Variable -Name IsWindows -Scope Global -ErrorAction SilentlyContinue).Value
  } else {
    $isWindowsRuntime
  }
  $isLinux = if (Get-Variable -Name IsLinux -Scope Global -ErrorAction SilentlyContinue) {
    [bool](Get-Variable -Name IsLinux -Scope Global -ErrorAction SilentlyContinue).Value
  } else {
    $isLinuxRuntime
  }
  $isMacOS = if (Get-Variable -Name IsMacOS -Scope Global -ErrorAction SilentlyContinue) {
    [bool](Get-Variable -Name IsMacOS -Scope Global -ErrorAction SilentlyContinue).Value
  } else {
    $isMacRuntime
  }

  if (-not ($isWindows -or $isLinux -or $isMacOS)) {
    $platformDescription = [string]$runtime::OSDescription
    throw "Unsupported OS for installer (detected: $platformDescription)"
  }

  return @{
    IsWindows = $isWindows
    IsLinux = $isLinux
    IsMacOS = $isMacOS
  }
}

function Normalize-WindowsPathEntry([string]$value) {
  if ([string]::IsNullOrWhiteSpace($value)) {
    return ""
  }
  $trimmed = $value.Trim().Trim('"')
  if ($trimmed.EndsWith([string][char]92)) {
    $trimmed = $trimmed.TrimEnd([char]92)
  }
  return $trimmed.ToLowerInvariant()
}

function Ensure-WindowsPathContains([string]$pathValue, [string]$entry, [switch]$PreferFront, [string[]]$RemoveEntries = @()) {
  $parts = @()
  if (-not [string]::IsNullOrWhiteSpace($pathValue)) {
    $parts = $pathValue.Split(";") |
      ForEach-Object { [string]$_ } |
      ForEach-Object { $_.Trim().Trim('"') } |
      Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
  }

  $entryClean = [string]$entry
  $entryNorm = Normalize-WindowsPathEntry $entryClean
  $removeNorms = @{}
  foreach ($removeEntry in $RemoveEntries) {
    $removeNorm = Normalize-WindowsPathEntry $removeEntry
    if (-not [string]::IsNullOrWhiteSpace($removeNorm)) {
      $removeNorms[$removeNorm] = $true
    }
  }
  $seen = @{}
  $deduped = New-Object System.Collections.Generic.List[string]
  $containsEntry = $false

  foreach ($part in $parts) {
    $norm = Normalize-WindowsPathEntry $part
    if ([string]::IsNullOrWhiteSpace($norm)) {
      continue
    }
    if ($removeNorms.ContainsKey($norm) -and $norm -ne $entryNorm) {
      continue
    }
    if ($norm -eq $entryNorm) {
      $containsEntry = $true
      if ($PreferFront) {
        continue
      }
    }
    if (-not $seen.ContainsKey($norm)) {
      $deduped.Add($part)
      $seen[$norm] = $true
    }
  }

  if ($PreferFront) {
    $deduped.Insert(0, $entryClean)
  } elseif (-not $containsEntry) {
    $deduped.Add($entryClean)
  }

  $joined = ($deduped -join ";")
  return @{
    Value = $joined
    Added = (-not $containsEntry)
    Changed = ($joined -ne [string]$pathValue)
  }
}

function Invoke-SourceFallbackCleanup {
  if (-not ($script:SourceFallbackTmp -and (Test-Path $script:SourceFallbackTmp.FullName))) {
    return
  }

  $cleanupRoot = $script:SourceFallbackTmp.FullName
  $script:SourceFallbackTmp = $null
  $script:SourceFallbackDir = $null

  if ($HostIsWindows) {
    try {
      $doubleQuote = [string][char]34
      $safeCleanupRoot = [string]$cleanupRoot
      if ($safeCleanupRoot.IndexOf([char]34) -ge 0) {
        $safeCleanupRoot = $safeCleanupRoot.Replace($doubleQuote, "")
      }
      $cleanupCommand = [string]::Concat("rmdir /s /q ", $doubleQuote, $safeCleanupRoot, $doubleQuote)
      Start-Process -FilePath "cmd.exe" -ArgumentList @("/d", "/c", $cleanupCommand) -WindowStyle Hidden | Out-Null
      Write-Host "[infring install] scheduled background cleanup of source fallback temp dir: $cleanupRoot"
      return
    } catch {
      Write-Host "[infring install] warning: background temp cleanup scheduling failed; falling back to synchronous cleanup"
    }
  }

  Remove-Item -Force -Recurse $cleanupRoot
}

function Remove-StaleWindowsCommandShims {
  param(
    [string]$ShimInstallDir
  )

  if (-not $HostIsWindows) {
    return
  }
  if ([string]::IsNullOrWhiteSpace($ShimInstallDir)) {
    return
  }
  if (-not (Test-Path $ShimInstallDir)) {
    return
  }

  $shimTargets = @(
    "infring.ps1",
    "infringctl.ps1",
    "infringd.ps1"
  )
  foreach ($shimTarget in $shimTargets) {
    $shimPath = Join-Path $ShimInstallDir $shimTarget
    if (Test-Path $shimPath) {
      Remove-Item -Force $shimPath
      Write-Host "[infring install] removed PowerShell command shim so restricted hosts resolve .cmd first: $shimPath"
    }
  }
}

function Resolve-Version {
  function Normalize-Version([string]$RawVersion) {
    if ($RawVersion.StartsWith("v")) { return $RawVersion }
    return "v$RawVersion"
  }

  function Resolve-VersionFromApi {
    try {
      $release = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
      if ($release.tag_name) {
        return Normalize-Version ([string]$release.tag_name)
      }
    } catch {
      return $null
    }
    return $null
  }

  function Resolve-VersionFromRedirect {
    try {
      $response = Invoke-WebRequest -Uri $LatestUrl -Method Head -MaximumRedirection 10 -UseBasicParsing
      $finalUrl = $response.BaseResponse.ResponseUri.AbsoluteUri
      if (-not $finalUrl) { return $null }
      if ($finalUrl -match "/releases/tag/(v[^/?#]+)") {
        return $Matches[1]
      }
    } catch {
      return $null
    }
    return $null
  }

  if ($RequestedVersion -ne "latest") {
    return Normalize-Version $RequestedVersion
  }

  if ($InstallOffline) {
    throw "Offline install mode requires an explicit release tag. Set INFRING_VERSION=vX.Y.Z and rerun with -Offline."
  }

  $version = Resolve-VersionFromApi
  if ($version) { return $version }

  $version = Resolve-VersionFromRedirect
  if ($version) {
    Write-Host "[infring install] GitHub API unavailable; resolved latest tag via releases/latest redirect: $version"
    return $version
  }

  $fallback = if ($env:INFRING_FALLBACK_VERSION) { $env:INFRING_FALLBACK_VERSION } else { $null }
  if ($fallback) {
    $fallbackVersion = Normalize-Version ([string]$fallback)
    Write-Host "[infring install] using fallback version: $fallbackVersion"
    return $fallbackVersion
  }

  throw "Failed to resolve latest release tag (GitHub API + releases/latest redirect). Set INFRING_VERSION=vX.Y.Z and retry."
}

function Get-ReleasesFromApi {
  try {
    $releases = Invoke-RestMethod -Uri $ReleasesApiUrl -UseBasicParsing
    if ($releases -is [System.Array]) {
      return @($releases)
    }
    return @()
  } catch {
    return @()
  }
}

function Get-BinaryStemAliases([string]$Stem) {
  switch ($Stem) {
    "infring-ops" { return @("infring-ops", "infring-ops") }
    "infringd" { return @("infringd", "infringd") }
    "infringd-tiny-max" { return @("infringd-tiny-max", "infringd-tiny-max", "infringd", "infringd") }
    "infring-pure-workspace" { return @("infring-pure-workspace", "infring-pure-workspace") }
    "infring-pure-workspace-tiny-max" { return @("infring-pure-workspace-tiny-max", "infring-pure-workspace-tiny-max", "infring-pure-workspace", "infring-pure-workspace") }
    default { return @($Stem) }
  }
}

function Get-BinaryStemForms([string]$Stem) {
  $forms = New-Object System.Collections.Generic.List[string]
  foreach ($alias in (Get-BinaryStemAliases $Stem)) {
    if ([string]::IsNullOrWhiteSpace([string]$alias)) { continue }
    if (-not $forms.Contains([string]$alias)) {
      $forms.Add([string]$alias) | Out-Null
    }
    $underscoreAlias = ([string]$alias) -replace "-", "_"
    if (-not [string]::IsNullOrWhiteSpace([string]$underscoreAlias) -and (-not $forms.Contains([string]$underscoreAlias))) {
      $forms.Add([string]$underscoreAlias) | Out-Null
    }
  }
  return @($forms)
}

function Get-InstallTripleAliases([string]$Triple) {
  if ([string]::IsNullOrWhiteSpace([string]$Triple)) {
    return @()
  }
  $aliases = New-Object System.Collections.Generic.List[string]
  $aliases.Add([string]$Triple) | Out-Null
  if ($Triple -like "x86_64-*") {
    $x64Triple = $Triple -replace "^x86_64-", "x64-"
    if (-not $aliases.Contains($x64Triple)) {
      $aliases.Add($x64Triple) | Out-Null
    }
  } elseif ($Triple -like "x64-*") {
    $x86Triple = $Triple -replace "^x64-", "x86_64-"
    if (-not $aliases.Contains($x86Triple)) {
      $aliases.Add($x86Triple) | Out-Null
    }
  }
  if ($Triple -like "aarch64-*") {
    $arm64Triple = $Triple -replace "^aarch64-", "arm64-"
    if (-not $aliases.Contains($arm64Triple)) {
      $aliases.Add($arm64Triple) | Out-Null
    }
  } elseif ($Triple -like "arm64-*") {
    $aarch64Triple = $Triple -replace "^arm64-", "aarch64-"
    if (-not $aliases.Contains($aarch64Triple)) {
      $aliases.Add($aarch64Triple) | Out-Null
    }
  }
  if ($Triple -like "*-pc-windows-msvc") {
    $gnuTriple = $Triple -replace "-pc-windows-msvc$", "-pc-windows-gnu"
    if (-not $aliases.Contains($gnuTriple)) {
      $aliases.Add($gnuTriple) | Out-Null
    }
  } elseif ($Triple -like "*-pc-windows-gnu") {
    $msvcTriple = $Triple -replace "-pc-windows-gnu$", "-pc-windows-msvc"
    if (-not $aliases.Contains($msvcTriple)) {
      $aliases.Add($msvcTriple) | Out-Null
    }
  }
  return @($aliases)
}

function Get-BinaryAssetCandidates([string]$Triple, [string]$Stem) {
  $variants = New-Object System.Collections.Generic.List[string]
  $tripleAliases = Get-InstallTripleAliases $Triple
  foreach ($alias in (Get-BinaryStemForms $Stem)) {
    foreach ($candidateTriple in $tripleAliases) {
      foreach ($candidate in @(
        "$alias-$candidateTriple.exe",
        "$alias-$candidateTriple",
        "$alias-$candidateTriple.bin",
        "$alias-$candidateTriple.zip",
        "$alias-$candidateTriple.tgz",
        "$alias-$candidateTriple.txz",
        "$alias-$candidateTriple.tzst",
        "$alias-$candidateTriple.tbz2",
        "$alias-$candidateTriple.tar.bz2",
        "$alias-$candidateTriple.tar.zst",
        "$alias-$candidateTriple.tar.xz",
        "$alias-$candidateTriple.tar.gz",
        "$alias-$candidateTriple.tar"
      )) {
        if (-not $variants.Contains([string]$candidate)) {
          $variants.Add([string]$candidate) | Out-Null
        }
      }
    }
    foreach ($candidate in @(
      "$alias.exe",
      "$alias",
      "$alias.zip",
      "$alias.tgz",
      "$alias.txz",
      "$alias.tzst",
      "$alias.tbz2",
      "$alias.tar.bz2",
      "$alias.tar.zst",
      "$alias.tar.xz",
      "$alias.tar.gz",
      "$alias.tar"
    )) {
      if (-not $variants.Contains([string]$candidate)) {
        $variants.Add([string]$candidate) | Out-Null
      }
    }
  }
  return @($variants)
}

function Release-HasAnyAsset([object]$Release, [string[]]$AssetCandidates) {
  if (-not $Release) { return $false }
  $assets = @()
  if ($Release.assets -is [System.Array]) {
    $assets = @($Release.assets | ForEach-Object { [string]$_.name })
  }
  if ($assets.Count -eq 0) { return $false }
  foreach ($candidate in $AssetCandidates) {
    if ($assets -contains $candidate) {
      return $true
    }
  }
  return $false
}

function Resolve-AssetCompatibleVersionForTriple([string]$Triple, [string[]]$Stems) {
  if (-not (Install-AllowCompatibleReleaseFallback)) {
    return $null
  }
  if (($RequestedVersion -ne "latest") -and (-not (Install-AllowPinnedVersionCompatibleFallback))) {
    return $null
  }
  $releases = Get-ReleasesFromApi
  if ($releases.Count -eq 0) {
    return $null
  }
  foreach ($release in $releases) {
    if (-not $release) { continue }
    if ([bool]$release.draft) { continue }
    if (-not $release.tag_name) { continue }
    $allPresent = $true
    foreach ($stem in $Stems) {
      $assetCandidates = Get-BinaryAssetCandidates $Triple $stem
      if (-not (Release-HasAnyAsset $release $assetCandidates)) {
        $allPresent = $false
        break
      }
    }
    if ($allPresent) {
      return [string]$release.tag_name
    }
  }
  return $null
}

function Resolve-ReleaseByTag([string]$VersionTag) {
  if ([string]::IsNullOrWhiteSpace($VersionTag)) {
    return $null
  }
  $releases = Get-ReleasesFromApi
  if ($releases.Count -eq 0) {
    return $null
  }
  $normalized = [string]$VersionTag
  foreach ($release in $releases) {
    if (-not $release) { continue }
    $tag = [string]$release.tag_name
    if ([string]::IsNullOrWhiteSpace($tag)) { continue }
    if ($tag -eq $normalized -or $tag.TrimStart("v") -eq $normalized.TrimStart("v")) {
      return $release
    }
  }
  return $null
}

function Probe-ReleaseAssetReachability([string]$VersionTag, [string]$AssetName) {
  $url = "$BaseUrl/$VersionTag/$AssetName"
  try {
    Invoke-WebRequest -Uri $url -Method Head -UseBasicParsing -TimeoutSec 20 | Out-Null
    return @{
      reachable = $true
      status = "head_ok"
      url = $url
    }
  } catch {
    try {
      Invoke-WebRequest -Uri $url -Method Get -Headers @{ Range = "bytes=0-0" } -UseBasicParsing -TimeoutSec 20 | Out-Null
      return @{
        reachable = $true
        status = "range_get_ok"
        url = $url
      }
    } catch {
      $status = "request_failed"
      try {
        $status = [string][int]$_.Exception.Response.StatusCode.value__
      } catch {
      }
      return @{
        reachable = $false
        status = $status
        url = $url
      }
    }
  }
}

function Resolve-ReleaseAssetProbe([string]$VersionTag, [string]$Triple, [string]$Stem) {
  $release = Resolve-ReleaseByTag $VersionTag
  $candidates = Get-BinaryAssetCandidates $Triple $Stem
  $tripleAliases = Get-InstallTripleAliases $Triple
  if (-not $release) {
    return @{
      stem = $Stem
      version = $VersionTag
      selected_asset = ""
      asset_found = $false
      reachable = $false
      reachability_status = "release_metadata_unavailable"
      candidate_triples = $tripleAliases
      candidates = $candidates
    }
  }
  $assetNames = @()
  if ($release.assets -is [System.Array]) {
    $assetNames = @($release.assets | ForEach-Object { [string]$_.name })
  }
  if ($assetNames.Count -eq 0) {
    return @{
      stem = $Stem
      version = $VersionTag
      selected_asset = ""
      asset_found = $false
      reachable = $false
      reachability_status = "release_asset_bundle_empty"
      candidate_triples = $tripleAliases
      candidates = $candidates
    }
  }
  $selected = ""
  foreach ($candidate in $candidates) {
    if ($assetNames -contains $candidate) {
      $selected = $candidate
      break
    }
  }
  if ([string]::IsNullOrWhiteSpace($selected)) {
    return @{
      stem = $Stem
      version = $VersionTag
      selected_asset = ""
      asset_found = $false
      reachable = $false
      reachability_status = "asset_not_listed_in_release"
      candidate_triples = $tripleAliases
      candidates = $candidates
    }
  }
  $selectedTriple = ""
  foreach ($candidateTriple in $tripleAliases) {
    if ($selected -like "*$candidateTriple*") {
      $selectedTriple = $candidateTriple
      break
    }
  }
  $reachability = Probe-ReleaseAssetReachability $VersionTag $selected
  return @{
    stem = $Stem
    version = $VersionTag
    selected_asset = $selected
    selected_triple = $selectedTriple
    asset_found = $true
    reachable = [bool]$reachability.reachable
    reachability_status = [string]$reachability.status
    reachability_url = [string]$reachability.url
    candidate_triples = $tripleAliases
    candidates = $candidates
  }
}

function Get-WindowsBuildToolSummary {
  $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
  $rustcCmd = Get-Command rustc -ErrorAction SilentlyContinue
  $clCmd = Get-Command cl.exe -ErrorAction SilentlyContinue
  $vswhereCmd = Get-Command vswhere.exe -ErrorAction SilentlyContinue
  $tarCmd = Get-Command tar -ErrorAction SilentlyContinue
  $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
  $vsInstallDetected = $false
  if ($vswhereCmd) {
    try {
      $vsPath = & $vswhereCmd.Source -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
      if (-not [string]::IsNullOrWhiteSpace([string]$vsPath)) {
        $vsInstallDetected = $true
      }
    } catch {
    }
  }
  return @{
    cargo_present = [bool]$cargoCmd
    rustc_present = [bool]$rustcCmd
    cl_present = [bool]$clCmd
    tar_present = [bool]$tarCmd
    winget_present = [bool]$wingetCmd
    vs_install_detected = [bool]$vsInstallDetected
    msvc_tools_present = [bool]$clCmd -or [bool]$vsInstallDetected
  }
}

function Get-WindowsBuildToolsInstallHint {
  return "Install Visual Studio Build Tools (MSVC+C++) via winget: winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override ""--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"" ; fallback (no winget): `$vs = Join-Path `$env:TEMP ""vs_BuildTools.exe""; irm https://aka.ms/vs/17/release/vs_BuildTools.exe -OutFile `$vs; Start-Process -FilePath `$vs -ArgumentList ""--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"" -Wait"
}

function Get-WindowsToolInstallHint([string]$ToolName) {
  $name = if ([string]::IsNullOrWhiteSpace([string]$ToolName)) { "" } else { [string]$ToolName.ToLowerInvariant() }
  switch ($name) {
    "cargo" {
      return "Install Rust toolchain: winget install --id Rustlang.Rustup -e ; rustup default stable"
    }
    "rustc" {
      return "Install Rust toolchain: winget install --id Rustlang.Rustup -e ; rustup default stable"
    }
    "tar" {
      return "Install tar tooling via Git for Windows: winget install --id Git.Git -e"
    }
    "winget" {
      return "Install App Installer from Microsoft Store (winget), or keep direct bootstrap fallback enabled (INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP=1)"
    }
    default {
      return ""
    }
  }
}

function Invoke-WindowsInstallerPreflight([string]$VersionTag, [string]$Triple, [string[]]$RequiredStems) {
  if (-not $HostIsWindows) {
    return
  }
  $dedupedStems = @($RequiredStems | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
  if ($dedupedStems.Count -eq 0) {
    return
  }
  $toolchain = Get-WindowsBuildToolSummary
  $assetProbes = @()
  foreach ($stem in $dedupedStems) {
    $assetProbes += Resolve-ReleaseAssetProbe $VersionTag $Triple $stem
  }
  $script:WindowsInstallPreflight = @{
    version = $VersionTag
    triple = $Triple
    required_stems = $dedupedStems
    toolchain = $toolchain
    assets = $assetProbes
  }
  Write-Host ("[infring install] preflight windows toolchain: cargo={0}; rustc={1}; msvc_tools={2}; tar={3}; winget={4}" -f `
      ([string]$toolchain.cargo_present).ToLower(), `
      ([string]$toolchain.rustc_present).ToLower(), `
      ([string]$toolchain.msvc_tools_present).ToLower(), `
      ([string]$toolchain.tar_present).ToLower(), `
      ([string]$toolchain.winget_present).ToLower())
  if (-not [bool]$toolchain.cargo_present) {
    Write-Host ("[infring install] preflight fix (cargo): {0}" -f (Get-WindowsToolInstallHint "cargo"))
  }
  if (-not [bool]$toolchain.rustc_present) {
    Write-Host ("[infring install] preflight fix (rustc): {0}" -f (Get-WindowsToolInstallHint "rustc"))
  }
  if (-not [bool]$toolchain.tar_present) {
    Write-Host ("[infring install] preflight fix (tar): {0}" -f (Get-WindowsToolInstallHint "tar"))
  }
  if (-not [bool]$toolchain.winget_present) {
    Write-Host ("[infring install] preflight fix (winget): {0}" -f (Get-WindowsToolInstallHint "winget"))
  }
  if (-not [bool]$toolchain.msvc_tools_present) {
    Write-Host ("[infring install] preflight fix (msvc): {0}" -f (Get-WindowsBuildToolsInstallHint))
  }
  Write-Host ("[infring install] preflight triple candidates: {0}" -f ((Get-InstallTripleAliases $Triple) -join ","))
  foreach ($probe in $assetProbes) {
    if ([bool]$probe.asset_found) {
      Write-Host ("[infring install] preflight asset probe ({0}): found {1}; reachable={2} ({3})" -f `
          [string]$probe.stem, `
          [string]$probe.selected_asset, `
          ([string][bool]$probe.reachable).ToLower(), `
          [string]$probe.reachability_status)
      if (-not [string]::IsNullOrWhiteSpace([string]$probe.selected_triple) -and ([string]$probe.selected_triple -ne [string]$Triple)) {
        Write-Host ("[infring install] preflight note: using compatible Windows triple asset variant {0} for requested {1}" -f `
            [string]$probe.selected_triple, `
            [string]$Triple)
      }
    } else {
      Write-Host ("[infring install] preflight asset probe ({0}): missing prebuilt in release metadata ({1})" -f `
          [string]$probe.stem, `
          [string]$probe.reachability_status)
    }
  }
  Write-Host ("[infring install] preflight policy: allow_no_msvc_source_fallback={0}; compatible_release_fallback={1}; pinned_version_compatible_fallback={2}" -f `
      ([string][bool](Install-AllowNoMsvcSourceFallback)).ToLower(), `
      ([string][bool](Install-AllowCompatibleReleaseFallback)).ToLower(), `
      ([string][bool](Install-AllowPinnedVersionCompatibleFallback)).ToLower())
  $assetGaps = @($assetProbes | Where-Object {
      (-not [bool]$_.asset_found) -or
      (([bool]$_.asset_found) -and (-not [bool]$_.reachable))
    })
  $autoRustup = Install-AutoRustupEnabled
  if ($assetGaps.Count -gt 0 -and (-not [bool]$toolchain.cargo_present) -and (-not $autoRustup)) {
    if ($RequestedVersion -eq "latest") {
      Write-Host "[infring install] preflight warning: current latest tag has Windows asset gaps and source fallback prerequisites are limited; installer will still try compatible-tag fallback before failing."
      return
    }
    $gapSummary = ($assetGaps | ForEach-Object { [string]$_.stem }) -join ", "
    throw "Windows installer preflight failed: prebuilt asset gaps detected for [$gapSummary], Cargo is unavailable, and auto Rust bootstrap is disabled (INFRING_INSTALL_AUTO_RUSTUP=0 or INFRING_AUTO_RUSTUP=0). Install Rust + MSVC build tools or publish missing Windows release assets."
  }
  if ($assetGaps.Count -gt 0 -and (-not [bool]$toolchain.cargo_present) -and $autoRustup) {
    Write-Host "[infring install] preflight note: Cargo missing but auto Rust bootstrap is enabled; installer will attempt toolchain bootstrap during source fallback."
  }
  if ($assetGaps.Count -gt 0 -and (-not [bool]$toolchain.msvc_tools_present)) {
    Write-Host "[infring install] preflight warning: MSVC build tools were not detected; source fallback may fail if Windows prebuilt assets are unavailable."
    if (Install-AutoMsvcBootstrapEnabled) {
      Write-Host "[infring install] preflight note: auto MSVC bootstrap is enabled (INFRING_INSTALL_AUTO_MSVC=1 default); installer will attempt winget bootstrap first and direct bootstrapper fallback if needed."
      if (-not [bool]$toolchain.winget_present) {
        if (Install-AllowDirectMsvcBootstrapEnabled) {
          Write-Host "[infring install] preflight note: winget is unavailable; installer will attempt direct Build Tools bootstrapper download during source fallback."
        } else {
          Write-Host "[infring install] preflight warning: winget is unavailable and direct bootstrap fallback is disabled; install Build Tools manually."
        }
      }
    } else {
      Write-Host "[infring install] preflight note: auto MSVC bootstrap is disabled (set INFRING_INSTALL_AUTO_MSVC=1 to enable automatic Build Tools install attempts)."
    }
  }
  if ($assetGaps.Count -gt 0 -and (-not [bool]$toolchain.tar_present)) {
    Write-Host "[infring install] preflight warning: tar was not detected; archive prebuilt extraction and some source fallback paths may fail."
  }
}

function Format-BinaryInstallFailureHint([string]$Stem, [string]$Triple, [string]$VersionTag) {
  $parts = New-Object System.Collections.Generic.List[string]
  $failure = $script:LastBinaryInstallFailure
  if ($failure -and ([string]$failure.stem -eq [string]$Stem)) {
    if ($failure.asset_probe) {
      $assetProbe = $failure.asset_probe
      if ([bool]$assetProbe.asset_found) {
        $parts.Add(("asset_probe={0};reachable={1};status={2}" -f `
            [string]$assetProbe.selected_asset, `
            ([string][bool]$assetProbe.reachable).ToLower(), `
            [string]$assetProbe.reachability_status))
        if (-not [string]::IsNullOrWhiteSpace([string]$assetProbe.selected_triple)) {
          $parts.Add(("asset_probe_triple={0}" -f [string]$assetProbe.selected_triple))
        }
      } else {
        $parts.Add(("asset_probe=missing;status={0}" -f [string]$assetProbe.reachability_status))
      }
      if ($assetProbe.candidate_triples) {
        $parts.Add(("asset_probe_triple_candidates={0}" -f ((@($assetProbe.candidate_triples) -join ","))))
      }
    }
    $attemptedAssets = @($failure.attempted_assets)
    if ($attemptedAssets.Count -gt 0) {
      $parts.Add(("attempted_assets={0}" -f ($attemptedAssets -join ",")))
    }
    $parts.Add(("source_fallback_attempted={0}" -f ([string][bool]$failure.source_fallback_attempted).ToLower()))
    $sourceFallbackVersions = @($failure.source_fallback_versions)
    if ($sourceFallbackVersions.Count -gt 0) {
      $parts.Add(("source_fallback_versions={0}" -f ($sourceFallbackVersions -join ",")))
    }
    if (-not [string]::IsNullOrWhiteSpace([string]$failure.source_fallback_reason)) {
      $parts.Add(("source_fallback_reason={0}" -f [string]$failure.source_fallback_reason))
    }
    if ($null -ne $failure.preflight_no_reachable_prebuilt_with_missing_msvc) {
      $parts.Add(
        ("preflight_no_reachable_prebuilt_with_missing_msvc={0}" -f `
            ([string][bool]$failure.preflight_no_reachable_prebuilt_with_missing_msvc).ToLower())
      )
    }
    $sourceFallbackPlan = @($failure.source_fallback_plan)
    if ($sourceFallbackPlan.Count -gt 0) {
      $parts.Add(("source_fallback_plan={0}" -f ($sourceFallbackPlan -join ",")))
    }
    if ($null -ne $failure.auto_msvc_bootstrap_enabled) {
      $parts.Add(("auto_msvc_bootstrap_enabled={0}" -f ([string][bool]$failure.auto_msvc_bootstrap_enabled).ToLower()))
    }
    if ($null -ne $failure.main_last_resort_fallback) {
      $parts.Add(("main_last_resort_fallback={0}" -f ([string][bool]$failure.main_last_resort_fallback).ToLower()))
    }
  }
  if ($HostIsWindows -and $script:WindowsInstallPreflight) {
    $toolchain = $script:WindowsInstallPreflight.toolchain
    if ($toolchain) {
      $parts.Add(("toolchain:cargo={0};rustc={1};msvc_tools={2};tar={3};winget={4}" -f `
          ([string][bool]$toolchain.cargo_present).ToLower(), `
          ([string][bool]$toolchain.rustc_present).ToLower(), `
          ([string][bool]$toolchain.msvc_tools_present).ToLower(), `
          ([string][bool]$toolchain.tar_present).ToLower(), `
          ([string][bool]$toolchain.winget_present).ToLower()))
    }
    $parts.Add(("auto_bootstrap:auto_rustup={0};auto_msvc={1}" -f `
        ([string][bool](Install-AutoRustupEnabled)).ToLower(), `
        ([string][bool](Install-AutoMsvcBootstrapEnabled)).ToLower()))
    $parts.Add(("auto_bootstrap:direct_msvc={0}" -f `
        ([string][bool](Install-AllowDirectMsvcBootstrapEnabled)).ToLower()))
    $parts.Add(("install_policy:allow_no_msvc_source_fallback={0};compatible_release_fallback={1};pinned_version_compatible_fallback={2}" -f `
        ([string][bool](Install-AllowNoMsvcSourceFallback)).ToLower(), `
        ([string][bool](Install-AllowCompatibleReleaseFallback)).ToLower(), `
        ([string][bool](Install-AllowPinnedVersionCompatibleFallback)).ToLower()))
  }
  if ($parts.Count -eq 0) {
    return "No additional diagnostics captured."
  }
  return ($parts -join " | ")
}

function Download-Asset($Version, $Asset, $OutPath) {
  function Is-PrereleaseVersionTag([string]$VersionTag) {
    $normalized = [string]$VersionTag
    return $normalized.Contains("-")
  }

  function Resolve-InstallVerifyAssetsEnabled {
    if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_VERIFY_ASSETS)) {
      return Installer-TruthyFlag $env:INFRING_INSTALL_VERIFY_ASSETS $true
    }
    return $true
  }

  function Resolve-AllowUnverifiedAssets {
    if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS)) {
      return Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS $false
    }
    return $false
  }

  function Resolve-StrictPrereleaseChecksum {
    if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_STRICT_PRERELEASE_CHECKSUM)) {
      return Installer-TruthyFlag $env:INFRING_INSTALL_STRICT_PRERELEASE_CHECKSUM $false
    }
    return $false
  }

  function Resolve-Sha256Hex([string]$TargetPath) {
    try {
      $hash = Get-FileHash -Path $TargetPath -Algorithm SHA256 -ErrorAction Stop
      if ($hash -and $hash.Hash) {
        return ([string]$hash.Hash).ToLowerInvariant()
      }
    } catch {}
    return $null
  }

  function Get-ExpectedAssetSha256([string]$ManifestPath, [string]$AssetName) {
    if (-not (Test-Path $ManifestPath)) {
      return $null
    }
    $target = [string]$AssetName
    foreach ($raw in Get-Content -Path $ManifestPath -ErrorAction SilentlyContinue) {
      $line = ([string]$raw).Trim()
      if ([string]::IsNullOrWhiteSpace($line)) { continue }

      $bsdPattern = "^SHA256\\s+\\(([^)]+)\\)\\s*=\\s*([a-fA-F0-9]{64})$"
      $bsd = [System.Text.RegularExpressions.Regex]::Match(
        $line,
        $bsdPattern,
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
      )
      if ($bsd.Success) {
        $file = [string]$bsd.Groups[1].Value
        $digest = ([string]$bsd.Groups[2].Value).ToLowerInvariant()
        if ($file -eq $target) {
          return $digest
        }
      }

      $gnu = [System.Text.RegularExpressions.Regex]::Match(
        $line,
        '^([a-fA-F0-9]{64})\s+\*?(.+)$',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
      )
      if ($gnu.Success) {
        $digest = ([string]$gnu.Groups[1].Value).ToLowerInvariant()
        $file = ([string]$gnu.Groups[2].Value).Trim().TrimStart(".").TrimStart("/")
        if ($file -eq $target) {
          return $digest
        }
      }
    }
    return $null
  }

  function Load-ReleaseChecksumManifest([string]$VersionTag) {
    if (
      $script:ChecksumManifestVersion -eq $VersionTag -and
      -not [string]::IsNullOrWhiteSpace([string]$script:ChecksumManifestPath) -and
      (Test-Path $script:ChecksumManifestPath)
    ) {
      return $true
    }
    if ($script:ChecksumManifestTmpDir -and (Test-Path $script:ChecksumManifestTmpDir)) {
      try { Remove-Item -Force -Recurse $script:ChecksumManifestTmpDir } catch {}
      $script:ChecksumManifestTmpDir = $null
    }
    $cacheRoot = Join-Path $HOME (".infring\cache\install-assets\" + [string]$VersionTag)
    if ([bool]$script:InstallAssetCache) {
      foreach ($candidate in @("SHA256SUMS", "SHA256SUMS.txt", "checksums.txt", "checksums.sha256")) {
        $cachedManifest = Join-Path $cacheRoot $candidate
        if (Test-Path $cachedManifest) {
          $script:ChecksumManifestVersion = $VersionTag
          $script:ChecksumManifestPath = $cachedManifest
          $script:ChecksumManifestAssetName = $candidate
          Write-Host "[infring install] checksum manifest: $candidate (cache)"
          return $true
        }
      }
    }
    if ($InstallOffline) {
      return $false
    }

    $tmpRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("infring-checksum-manifest-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $tmpRoot | Out-Null
    $script:ChecksumManifestTmpDir = $tmpRoot
    $script:ChecksumManifestPath = $null
    $script:ChecksumManifestAssetName = ""
    foreach ($candidate in @("SHA256SUMS", "SHA256SUMS.txt", "checksums.txt", "checksums.sha256")) {
      $path = Join-Path $tmpRoot $candidate
      $url = "$BaseUrl/$VersionTag/$candidate"
      try {
        Invoke-WebRequest -Uri $url -OutFile $path -UseBasicParsing | Out-Null
        if (Test-Path $path) {
          $script:ChecksumManifestVersion = $VersionTag
          $script:ChecksumManifestPath = $path
          $script:ChecksumManifestAssetName = $candidate
          if ([bool]$script:InstallAssetCache) {
            try {
              New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null
              Copy-Item -Force $path (Join-Path $cacheRoot $candidate)
            } catch {}
          }
          Write-Host "[infring install] checksum manifest: $candidate"
          return $true
        }
      } catch {}
    }
    return $false
  }

  function Verify-DownloadedAsset([string]$VersionTag, [string]$AssetName, [string]$AssetPath) {
    function Record-VerifiedAssetDigest([string]$VersionTagInner, [string]$AssetNameInner, [string]$DigestInner, [string]$AssetPathInner) {
      if ([string]::IsNullOrWhiteSpace($VersionTagInner) -or [string]::IsNullOrWhiteSpace($AssetNameInner) -or [string]::IsNullOrWhiteSpace($DigestInner)) {
        return
      }
      $lockPath = [string]$script:InstallAssetLockfile
      if ([string]::IsNullOrWhiteSpace($lockPath)) {
        return
      }
      $lockDir = Split-Path -Parent $lockPath
      if (-not [string]::IsNullOrWhiteSpace($lockDir)) {
        New-Item -ItemType Directory -Force -Path $lockDir | Out-Null
      }
      $rows = New-Object System.Collections.Generic.List[string]
      if (Test-Path $lockPath) {
        foreach ($line in Get-Content -Path $lockPath -ErrorAction SilentlyContinue) {
          if ([string]::IsNullOrWhiteSpace([string]$line)) { continue }
          if ([string]$line -eq "infring_install_asset_lock_v1") { continue }
          $parts = ([string]$line).Split("`t")
          if ($parts.Count -lt 3) { continue }
          if ($parts[0] -eq $VersionTagInner -and $parts[1] -eq $AssetNameInner) { continue }
          $rows.Add([string]$line) | Out-Null
        }
      }
      $stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
      $newRow = "$VersionTagInner`t$AssetNameInner`t$DigestInner`t$stamp`t$AssetPathInner"
      $payload = @("infring_install_asset_lock_v1", $newRow) + @($rows)
      Set-Content -Path $lockPath -Value $payload -Encoding UTF8
      Write-Host "[infring install] asset lockfile updated: $lockPath"
    }

    if (-not (Resolve-InstallVerifyAssetsEnabled)) {
      return $true
    }
    $allowUnverified = Resolve-AllowUnverifiedAssets
    $allowPrereleaseWithoutStrict = (Is-PrereleaseVersionTag $VersionTag) -and (-not (Resolve-StrictPrereleaseChecksum))
    if (-not (Load-ReleaseChecksumManifest $VersionTag)) {
      if ($allowUnverified -or $allowPrereleaseWithoutStrict) {
        Write-Host "[infring install] warning: checksum manifest missing for $VersionTag; continuing with unverified asset $AssetName."
        return $true
      }
      Write-Host "[infring install] asset verification failed: checksum manifest unavailable for $VersionTag"
      Write-Host "[infring install] fix: publish release checksum manifest (SHA256SUMS) or set INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS=1"
      return $false
    }
    $expected = Get-ExpectedAssetSha256 $script:ChecksumManifestPath $AssetName
    if ([string]::IsNullOrWhiteSpace([string]$expected)) {
      if ($allowUnverified -or $allowPrereleaseWithoutStrict) {
        Write-Host "[infring install] warning: no checksum entry for $AssetName; continuing unverified."
        return $true
      }
      Write-Host "[infring install] asset verification failed: missing checksum entry for $AssetName in $($script:ChecksumManifestAssetName)"
      return $false
    }
    $actual = Resolve-Sha256Hex $AssetPath
    if ([string]::IsNullOrWhiteSpace([string]$actual)) {
      Write-Host "[infring install] asset verification failed: unable to hash $AssetName"
      return $false
    }
    if ($actual -ne $expected) {
      Write-Host "[infring install] asset verification failed: checksum mismatch for $AssetName"
      Write-Host "[infring install] expected: $expected"
      Write-Host "[infring install] actual:   $actual"
      Write-Host "[infring install] fix: clear local temp/cache and rerun install from a clean shell."
      return $false
    }
    Write-Host "[infring install] verified $AssetName sha256:$actual"
    Record-VerifiedAssetDigest -VersionTagInner $VersionTag -AssetNameInner $AssetName -DigestInner $actual -AssetPathInner $AssetPath
    return $true
  }

  $cacheDir = Join-Path $HOME (".infring\cache\install-assets\" + [string]$Version)
  $cacheFile = Join-Path $cacheDir [string]$Asset
  if ([bool]$script:InstallAssetCache -and (Test-Path $cacheFile)) {
    try {
      Copy-Item -Force $cacheFile $OutPath
      if (Verify-DownloadedAsset $Version $Asset $OutPath) {
        Write-Host "[infring install] downloaded $Asset (cache hit)"
        return $true
      }
      Remove-Item -Force $cacheFile -ErrorAction SilentlyContinue
      if ($InstallOffline) {
        Write-Host "[infring install] offline cache invalid for $Asset; cannot refetch in offline mode."
        Write-Host "[infring install] fix: rerun once without -Offline to refresh cache for $Version."
        return $false
      }
      Write-Host "[infring install] cache invalid for $Asset; refetching"
    } catch {
      if ($InstallOffline) {
        Write-Host "[infring install] offline cache read failed for $Asset; cannot refetch in offline mode."
        return $false
      }
    }
  }

  if ($InstallOffline) {
    Write-Host "[infring install] offline cache miss for $Asset under $cacheDir"
    Write-Host "[infring install] fix: rerun once without -Offline to hydrate cache for $Version."
    return $false
  }

  $url = "$BaseUrl/$Version/$Asset"
  try {
    Invoke-WebRequest -Uri $url -OutFile $OutPath -UseBasicParsing | Out-Null
    if (-not (Verify-DownloadedAsset $Version $Asset $OutPath)) {
      try { Remove-Item -Force $OutPath -ErrorAction SilentlyContinue } catch {}
      return $false
    }
    if ([bool]$script:InstallAssetCache) {
      try {
        New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
        Copy-Item -Force $OutPath $cacheFile
      } catch {}
    }
    Write-Host "[infring install] downloaded $Asset"
    return $true
  } catch {
    return $false
  }
}

function Install-Binary($Version, $Triple, $Stem, $OutPath) {
  function Resolve-SourceBinName([string]$StemName) {
    switch ($StemName) {
      "infring-ops" { return "infring-ops" }
      "infringd" { return "infringd" }
      "infringd-tiny-max" { return "infringd" }
      "conduit_daemon" { return "conduit_daemon" }
      "infring-pure-workspace" { return "infring-pure-workspace" }
      "infring-pure-workspace-tiny-max" { return "infring-pure-workspace" }
      default { return $null }
    }
  }

  function Ensure-CargoToolchainForSourceFallback {
    function Test-CargoRunnable {
      $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
      if ($null -eq $cargoCmd) {
        return $false
      }
      try {
        & $cargoCmd.Source --version *> $null
        return ($LASTEXITCODE -eq 0)
      } catch {
        return $false
      }
    }

    if (Test-CargoRunnable) {
      return $true
    }

    $cargoExists = [bool](Get-Command cargo -ErrorAction SilentlyContinue)
    $autoRustup = Install-AutoRustupEnabled
    if ($cargoExists) {
      $script:LastBinaryInstallFailureReason = "cargo_present_but_unusable"
      if (-not $autoRustup) {
        $script:LastBinaryInstallFailureReason = "cargo_unusable_auto_rustup_disabled"
        return $false
      }
      $rustupCmd = Get-Command rustup -ErrorAction SilentlyContinue
      if ($null -eq $rustupCmd) {
        $script:LastBinaryInstallFailureReason = "rustup_missing_for_cargo_repair"
        return $false
      }
      Write-Host "[infring install] cargo detected but not runnable; attempting rustup default toolchain repair"
      try {
        & $rustupCmd.Source default stable *> $null
      } catch {
      }
      $cargoBin = Join-Path $HOME ".cargo\bin"
      if (Test-Path $cargoBin) {
        if (-not $env:Path.ToLower().Contains($cargoBin.ToLower())) {
          $env:Path = "$cargoBin;$env:Path"
        }
      }
      if (Test-CargoRunnable) {
        Write-Host "[infring install] cargo recovered via rustup default stable"
        return $true
      }
      $script:LastBinaryInstallFailureReason = "cargo_unusable_after_rustup_default"
      return $false
    }

    $script:LastBinaryInstallFailureReason = "cargo_missing"
    if (-not $HostIsWindows) {
      $script:LastBinaryInstallFailureReason = "cargo_missing_non_windows_source_fallback_unavailable"
      return $false
    }
    if (-not $autoRustup) {
      $script:LastBinaryInstallFailureReason = "cargo_missing_auto_rustup_disabled"
      return $false
    }
    Write-Host "[infring install] prebuilt binary not available; attempting Rust toolchain bootstrap for source fallback"
    $rustupExe = Join-Path ([System.IO.Path]::GetTempPath()) "rustup-init.exe"
    try {
      Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupExe -UseBasicParsing | Out-Null
      $proc = Start-Process -FilePath $rustupExe -ArgumentList "-y --profile minimal --default-toolchain stable" -Wait -PassThru
      if ($proc.ExitCode -ne 0) {
        $script:LastBinaryInstallFailureReason = "rustup_bootstrap_failed"
        return $false
      }
      $cargoBin = Join-Path $HOME ".cargo\bin"
      if (Test-Path $cargoBin) {
        if (-not $env:Path.ToLower().Contains($cargoBin.ToLower())) {
          $env:Path = "$cargoBin;$env:Path"
        }
      }
      $cargoPresent = [bool](Get-Command cargo -ErrorAction SilentlyContinue)
      if (-not $cargoPresent) {
        $script:LastBinaryInstallFailureReason = "cargo_still_missing_after_rustup"
      }
      return $cargoPresent
    } catch {
      $script:LastBinaryInstallFailureReason = "rustup_bootstrap_transport_error"
      return $false
    }
  }

  function Prepare-SourceFallbackRepo([string]$VersionTag) {
    if ($script:SourceFallbackDir -and (Test-Path $script:SourceFallbackDir)) {
      return $script:SourceFallbackDir
    }
    if (-not (Ensure-CargoToolchainForSourceFallback)) {
      return $null
    }

    $script:SourceFallbackTmp = New-TemporaryFile
    Remove-Item $script:SourceFallbackTmp.FullName -Force
    New-Item -ItemType Directory -Path $script:SourceFallbackTmp.FullName | Out-Null
    $script:SourceFallbackDir = Join-Path $script:SourceFallbackTmp.FullName "repo"
    $repoUrl = "https://github.com/$RepoOwner/$RepoName.git"

    $archivePath = Join-Path $script:SourceFallbackTmp.FullName "source.zip"
    $extractRoot = Join-Path $script:SourceFallbackTmp.FullName "extract"
    New-Item -ItemType Directory -Path $extractRoot | Out-Null
    $archiveUrls = @(
      "https://github.com/$RepoOwner/$RepoName/archive/refs/tags/$VersionTag.zip",
      "https://github.com/$RepoOwner/$RepoName/archive/refs/heads/main.zip"
    )
    foreach ($archiveUrl in $archiveUrls) {
      try {
        Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath -UseBasicParsing | Out-Null
        Expand-Archive -Path $archivePath -DestinationPath $extractRoot -Force
        $sourceDir = Get-ChildItem -Path $extractRoot -Directory | Select-Object -First 1
        if ($sourceDir) {
          Copy-Item -Recurse -Force (Join-Path $sourceDir.FullName "*") $script:SourceFallbackDir
          return $script:SourceFallbackDir
        }
      } catch {
      }
    }

    if (Get-Command git -ErrorAction SilentlyContinue) {
      try {
        git clone --quiet --depth 1 --branch main $repoUrl $script:SourceFallbackDir 2>$null | Out-Null
        if (-not [string]::IsNullOrWhiteSpace($VersionTag) -and $VersionTag -ne "main") {
          try {
            git -C $script:SourceFallbackDir fetch --quiet --depth 1 origin ("refs/tags/$VersionTag^{}") 2>$null | Out-Null
            git -c advice.detachedHead=false -C $script:SourceFallbackDir checkout --quiet --detach FETCH_HEAD 2>$null | Out-Null
          } catch {
            try {
              git -c advice.detachedHead=false -C $script:SourceFallbackDir checkout --quiet --detach $VersionTag 2>$null | Out-Null
            } catch {
            }
          }
        }
        return $script:SourceFallbackDir
      } catch {
      }
    }

    $script:LastBinaryInstallFailureReason = "source_repo_unavailable"
    return $null
  }

  function Install-BinaryFromSourceFallback([string]$VersionTag, [string]$StemName, [string]$OutBinaryPath) {
    function Ensure-WindowsBuildToolsForSourceFallback {
      if (-not $HostIsWindows) {
        return $true
      }
      $toolchain = Get-WindowsBuildToolSummary
      if ([bool]$toolchain.msvc_tools_present) {
        $script:WindowsMsvcBootstrapSucceeded = $true
        return $true
      }
      if (-not (Install-AutoMsvcBootstrapEnabled)) {
        $script:LastBinaryInstallFailureReason = "msvc_tools_missing_auto_bootstrap_disabled"
        return $false
      }
      if ($script:WindowsMsvcBootstrapAttempted) {
        $toolchainAfterPriorAttempt = Get-WindowsBuildToolSummary
        if ([bool]$toolchainAfterPriorAttempt.msvc_tools_present) {
          $script:WindowsMsvcBootstrapSucceeded = $true
          return $true
        }
        $script:LastBinaryInstallFailureReason = "msvc_tools_still_missing_after_bootstrap"
        return $false
      }
      $script:WindowsMsvcBootstrapAttempted = $true
      $bootstrapped = $false
      $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
      if ($wingetCmd) {
        Write-Host "[infring install] attempting automatic MSVC Build Tools bootstrap via winget"
        try {
          $proc = Start-Process -FilePath $wingetCmd.Source -ArgumentList @(
              "install",
              "--id",
              "Microsoft.VisualStudio.2022.BuildTools",
              "-e",
              "--override",
              "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools",
              "--accept-package-agreements",
              "--accept-source-agreements"
            ) -Wait -PassThru -WindowStyle Hidden
          if ($proc.ExitCode -eq 0) {
            $bootstrapped = $true
          } else {
            Write-Host ("[infring install] winget MSVC bootstrap failed (exit={0}); attempting direct bootstrapper fallback" -f [string]$proc.ExitCode)
            $script:LastBinaryInstallFailureReason = ("msvc_bootstrap_winget_failed_exit_{0}" -f [string]$proc.ExitCode)
          }
        } catch {
          Write-Host "[infring install] winget MSVC bootstrap transport failed; attempting direct bootstrapper fallback"
          $script:LastBinaryInstallFailureReason = "msvc_bootstrap_winget_transport_error"
        }
      } else {
        Write-Host "[infring install] winget unavailable; attempting direct MSVC Build Tools bootstrapper fallback"
        $script:LastBinaryInstallFailureReason = "msvc_bootstrap_winget_unavailable"
      }
      if ((-not $bootstrapped) -and (Install-AllowDirectMsvcBootstrapEnabled)) {
        $bootstrapperPath = Join-Path ([System.IO.Path]::GetTempPath()) "infring-vs_BuildTools.exe"
        try {
          Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_BuildTools.exe" -OutFile $bootstrapperPath -UseBasicParsing | Out-Null
          $directProc = Start-Process -FilePath $bootstrapperPath -ArgumentList @(
              "--quiet",
              "--wait",
              "--norestart",
              "--nocache",
              "--add",
              "Microsoft.VisualStudio.Workload.VCTools",
              "--includeRecommended"
            ) -Wait -PassThru -WindowStyle Hidden
          if ($directProc.ExitCode -eq 0) {
            $bootstrapped = $true
          } else {
            $script:LastBinaryInstallFailureReason = ("msvc_bootstrap_direct_failed_exit_{0}" -f [string]$directProc.ExitCode)
          }
        } catch {
          $script:LastBinaryInstallFailureReason = "msvc_bootstrap_direct_transport_error"
        }
      } elseif (-not $bootstrapped) {
        $script:LastBinaryInstallFailureReason = "msvc_bootstrap_direct_disabled"
      }
      if (-not $bootstrapped) {
        $script:WindowsMsvcBootstrapSucceeded = $false
        return $false
      }
      $postBootstrapToolchain = Get-WindowsBuildToolSummary
      if (-not [bool]$postBootstrapToolchain.msvc_tools_present) {
        $script:WindowsMsvcBootstrapSucceeded = $false
        $script:LastBinaryInstallFailureReason = "msvc_tools_still_missing_after_bootstrap"
        return $false
      }
      $script:WindowsMsvcBootstrapSucceeded = $true
      Write-Host "[infring install] MSVC Build Tools detected after bootstrap"
      return $true
    }

    $binName = Resolve-SourceBinName $StemName
    if (-not $binName) {
      $script:LastBinaryInstallFailureReason = "unsupported_stem_for_source_fallback"
      return $false
    }

    if (-not (Ensure-WindowsBuildToolsForSourceFallback)) {
      return $false
    }

    $repoDir = Prepare-SourceFallbackRepo $VersionTag
    if (-not $repoDir) {
      if ([string]::IsNullOrWhiteSpace($script:LastBinaryInstallFailureReason)) {
        $script:LastBinaryInstallFailureReason = "source_repo_prepare_failed"
      }
      return $false
    }

    $manifest = Join-Path $repoDir "core/layer0/ops/Cargo.toml"
    try {
      & cargo build --release --manifest-path $manifest --bin $binName | Out-Null
    } catch {
      $script:LastBinaryInstallFailureReason = "cargo_build_failed"
      return $false
    }
    if ($LASTEXITCODE -ne 0) {
      $script:LastBinaryInstallFailureReason = "cargo_build_failed_exit_$LASTEXITCODE"
      return $false
    }

    $built = Join-Path $repoDir "target/release/$binName.exe"
    if (-not (Test-Path $built)) {
      $targetReleaseDir = Join-Path $repoDir "target/release"
      if (Test-Path $targetReleaseDir) {
        $builtCandidates = @(Get-ChildItem -Path $targetReleaseDir -File -ErrorAction SilentlyContinue)
        if ($builtCandidates.Count -gt 0) {
          $candidateNames = New-Object System.Collections.Generic.List[string]
          foreach ($stemForm in (Get-BinaryStemForms $StemName)) {
            foreach ($name in @("$stemForm.exe", $stemForm)) {
              if (-not $candidateNames.Contains([string]$name)) {
                $candidateNames.Add([string]$name) | Out-Null
              }
            }
          }
          $selectedBuilt = $null
          foreach ($candidate in $candidateNames) {
            $match = $builtCandidates | Where-Object { [string]$_.Name -ieq [string]$candidate } | Select-Object -First 1
            if ($match) {
              $selectedBuilt = $match
              break
            }
          }
          if (-not $selectedBuilt) {
            foreach ($stemForm in (Get-BinaryStemForms $StemName)) {
              $match = $builtCandidates | Where-Object {
                ([string]$_.Name -like "$stemForm*.exe") -or ([string]$_.Name -like "$stemForm*")
              } | Select-Object -First 1
              if ($match) {
                $selectedBuilt = $match
                break
              }
            }
          }
          if ($selectedBuilt) {
            Copy-Item -Force $selectedBuilt.FullName $OutBinaryPath
            Write-Host ("[infring install] built {0} from source fallback (discovered in target/release)" -f [string]$selectedBuilt.Name)
            $script:LastBinaryInstallFailureReason = ""
            return $true
          }
        }
      }
      $script:LastBinaryInstallFailureReason = "source_build_output_missing"
      return $false
    }
    Copy-Item -Force $built $OutBinaryPath
    Write-Host "[infring install] built $binName from source fallback"
    $script:LastBinaryInstallFailureReason = ""
    return $true
  }

  function Install-BinaryFromDownloadedAsset([string]$DownloadedPath, [string]$AssetName, [string]$StemName, [string]$OutBinaryPath, [string]$TmpRoot) {
    $assetLower = [string]$AssetName
    if ($assetLower.EndsWith(".zip") -or $assetLower.EndsWith(".tgz") -or $assetLower.EndsWith(".txz") -or $assetLower.EndsWith(".tzst") -or $assetLower.EndsWith(".tbz2") -or $assetLower.EndsWith(".tar.bz2") -or $assetLower.EndsWith(".tar.zst") -or $assetLower.EndsWith(".tar.xz") -or $assetLower.EndsWith(".tar.gz") -or $assetLower.EndsWith(".tar")) {
      $extractDir = Join-Path $TmpRoot ("extract-" + [System.IO.Path]::GetRandomFileName())
      New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
      try {
        if ($assetLower.EndsWith(".zip")) {
          Expand-Archive -Path $DownloadedPath -DestinationPath $extractDir -Force
        } elseif ($assetLower.EndsWith(".tzst") -or $assetLower.EndsWith(".tar.zst")) {
          if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
            $script:LastBinaryInstallFailureReason = "asset_archive_tar_unavailable"
            return $false
          }
          try {
            tar --zstd -xf $DownloadedPath -C $extractDir
          } catch {
            if (Get-Command zstd -ErrorAction SilentlyContinue) {
              $tarPath = [System.IO.Path]::ChangeExtension($DownloadedPath, ".tar")
              zstd -d --stdout $DownloadedPath > $tarPath
              tar -xf $tarPath -C $extractDir
            } else {
              $script:LastBinaryInstallFailureReason = "asset_archive_zstd_unavailable"
              return $false
            }
          }
        } elseif ($assetLower.EndsWith(".tbz2") -or $assetLower.EndsWith(".tar.bz2")) {
          if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
            $script:LastBinaryInstallFailureReason = "asset_archive_tar_unavailable"
            return $false
          }
          tar -xjf $DownloadedPath -C $extractDir
        } elseif ($assetLower.EndsWith(".txz") -or $assetLower.EndsWith(".tar.xz")) {
          if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
            $script:LastBinaryInstallFailureReason = "asset_archive_tar_unavailable"
            return $false
          }
          tar -xJf $DownloadedPath -C $extractDir
        } elseif ($assetLower.EndsWith(".tgz") -or $assetLower.EndsWith(".tar.gz")) {
          if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
            $script:LastBinaryInstallFailureReason = "asset_archive_tar_unavailable"
            return $false
          }
          tar -xzf $DownloadedPath -C $extractDir
        } else {
          if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
            $script:LastBinaryInstallFailureReason = "asset_archive_tar_unavailable"
            return $false
          }
          tar -xf $DownloadedPath -C $extractDir
        }
      } catch {
        $script:LastBinaryInstallFailureReason = "asset_archive_extract_failed"
        return $false
      }
      $files = @(Get-ChildItem -Path $extractDir -Recurse -File -ErrorAction SilentlyContinue)
      if ($files.Count -eq 0) {
        $script:LastBinaryInstallFailureReason = "asset_archive_empty"
        return $false
      }
      $nameCandidates = New-Object System.Collections.Generic.List[string]
      foreach ($stemForm in (Get-BinaryStemForms $StemName)) {
        foreach ($name in @("$stemForm.exe", $stemForm)) {
          if (-not $nameCandidates.Contains([string]$name)) {
            $nameCandidates.Add([string]$name) | Out-Null
          }
        }
      }
      $selected = $null
      foreach ($candidate in $nameCandidates) {
        $match = $files | Where-Object { [string]$_.Name -ieq [string]$candidate } | Select-Object -First 1
        if ($match) {
          $selected = $match
          break
        }
      }
      if (-not $selected) {
        foreach ($stemForm in (Get-BinaryStemForms $StemName)) {
          $match = $files | Where-Object {
            ([string]$_.Name -like "$stemForm*.exe") -or ([string]$_.Name -like "$stemForm*")
          } | Select-Object -First 1
          if ($match) {
            $selected = $match
            break
          }
        }
      }
      if (-not $selected) {
        $script:LastBinaryInstallFailureReason = "asset_archive_binary_not_found"
        return $false
      }
      Copy-Item -Force $selected.FullName $OutBinaryPath
      Write-Host ("[infring install] extracted {0} from archive asset {1}" -f [string]$selected.Name, [string]$AssetName)
      $script:LastBinaryInstallFailureReason = ""
      return $true
    }
    Move-Item -Force $DownloadedPath $OutBinaryPath
    $script:LastBinaryInstallFailureReason = ""
    return $true
  }

  $tmp = New-TemporaryFile
  Remove-Item $tmp.FullName -Force
  New-Item -ItemType Directory -Path $tmp.FullName | Out-Null

  $assetProbe = Resolve-ReleaseAssetProbe $Version $Triple $Stem
  $attemptedAssets = New-Object System.Collections.Generic.List[string]
  $noReachablePrebuiltWithMissingMsvc = $false
  $raw = Join-Path $tmp.FullName "$Stem.download"
  $assetCandidates = Get-BinaryAssetCandidates $Triple $Stem
  foreach ($assetName in $assetCandidates) {
    $attemptedAssets.Add([string]$assetName)
    if (Download-Asset $Version $assetName $raw) {
      if (Install-BinaryFromDownloadedAsset $raw $assetName $Stem $OutPath $tmp.FullName) {
        $script:LastBinaryInstallFailure = @{
          stem = $Stem
          triple = $Triple
          version = $Version
          attempted_assets = @($attemptedAssets)
          source_fallback_attempted = $false
          source_fallback_plan = @()
          source_fallback_reason = ""
          auto_msvc_bootstrap_enabled = [bool](Install-AutoMsvcBootstrapEnabled)
          main_last_resort_fallback = $null
          preflight_no_reachable_prebuilt_with_missing_msvc = [bool]$noReachablePrebuiltWithMissingMsvc
          asset_probe = $assetProbe
        }
        return $true
      }
    }
  }

  if ($InstallOffline) {
    $script:LastBinaryInstallFailureReason = "offline_asset_cache_miss"
    $script:LastBinaryInstallFailure = @{
      stem = $Stem
      triple = $Triple
      version = $Version
      attempted_assets = @($attemptedAssets)
      source_fallback_attempted = $false
      source_fallback_plan = @()
      source_fallback_reason = [string]$script:LastBinaryInstallFailureReason
      auto_msvc_bootstrap_enabled = [bool](Install-AutoMsvcBootstrapEnabled)
      main_last_resort_fallback = $null
      preflight_no_reachable_prebuilt_with_missing_msvc = $false
      asset_probe = $assetProbe
    }
    return $false
  }

  $allowNoMsvcSourceFallback = Install-AllowNoMsvcSourceFallback
  if (
    $HostIsWindows -and
    $script:WindowsInstallPreflight -and
    (-not [bool]$script:WindowsInstallPreflight.toolchain.msvc_tools_present) -and
    $allowNoMsvcSourceFallback
  ) {
    Write-Host "[infring install] override enabled: proceeding with source fallback despite missing MSVC tools (set INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK=0 to disable)"
  }
  if (
    $HostIsWindows -and
    $script:WindowsInstallPreflight -and
    (-not [bool]$script:WindowsInstallPreflight.toolchain.msvc_tools_present) -and
    (
      (-not [bool]$assetProbe.asset_found) -or
      (([bool]$assetProbe.asset_found) -and (-not [bool]$assetProbe.reachable))
    )
  ) {
    $noReachablePrebuiltWithMissingMsvc = $true
    if (-not $allowNoMsvcSourceFallback) {
      if (-not (Install-AutoMsvcBootstrapEnabled)) {
        $script:LastBinaryInstallFailureReason = "msvc_tools_missing_no_reachable_prebuilt_asset"
        $script:LastBinaryInstallFailure = @{
          stem = $Stem
          triple = $Triple
          version = $Version
          attempted_assets = @($attemptedAssets)
          source_fallback_attempted = $false
          source_fallback_plan = @()
          source_fallback_reason = [string]$script:LastBinaryInstallFailureReason
          auto_msvc_bootstrap_enabled = [bool](Install-AutoMsvcBootstrapEnabled)
          main_last_resort_fallback = $null
          preflight_no_reachable_prebuilt_with_missing_msvc = $true
          asset_probe = $assetProbe
        }
        return $false
      }
      Write-Host "[infring install] preflight note: no reachable Windows prebuilt + MSVC tools missing; forcing best-effort source fallback despite INFRING_INSTALL_ALLOW_NO_MSVC_SOURCE_FALLBACK=0"
    } else {
      Write-Host "[infring install] preflight note: no reachable Windows prebuilt and MSVC tools missing; attempting best-effort source fallback"
    }
    Write-Host '[infring install] recommended fix: winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"'
  }

  $script:LastBinaryInstallFailureReason = ""
  $sourceFallbackVersions = @()
  $sourceFallbackPlan = New-Object System.Collections.Generic.List[string]
  $assetMissing = $assetProbe -and (-not [bool]$assetProbe.asset_found)
  $allowMainLastResortFallback = $true
  if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_ALLOW_MAIN_LAST_RESORT_SOURCE_FALLBACK)) {
    $allowMainLastResortFallback = Installer-TruthyFlag $env:INFRING_INSTALL_ALLOW_MAIN_LAST_RESORT_SOURCE_FALLBACK $true
  } elseif (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_ALLOW_MAIN_LAST_RESORT_SOURCE_FALLBACK)) {
    $allowMainLastResortFallback = Installer-TruthyFlag $env:INFRING_ALLOW_MAIN_LAST_RESORT_SOURCE_FALLBACK $true
  }
  if ($assetMissing -and $Version -ne "main") {
    Write-Host ("[infring install] source fallback policy: main_last_resort_fallback={0}" -f ([string][bool]$allowMainLastResortFallback).ToLower())
  }
  $preferMainSourceFallback = (
    ($RequestedVersion -eq "latest") -and
    ($Version -ne "main") -and
    $assetMissing
  )
  if ($preferMainSourceFallback) {
    $sourceFallbackPlan.Add("main") | Out-Null
    $sourceFallbackPlan.Add([string]$Version) | Out-Null
  } else {
    $sourceFallbackPlan.Add([string]$Version) | Out-Null
    if (
      ($RequestedVersion -eq "latest") -and
      ($Version -ne "main")
    ) {
      $sourceFallbackPlan.Add("main") | Out-Null
    } elseif (
      $allowMainLastResortFallback -and
      $assetMissing -and
      ($Version -ne "main")
    ) {
      # Non-latest installs can still encounter releases missing Windows prebuilts.
      # Keep `main` as a last-resort source fallback to reduce dead-end installs.
      $sourceFallbackPlan.Add("main") | Out-Null
    }
  }
  $fallbackOk = $false
  $sourceFallbackIndex = 0
  while ($sourceFallbackIndex -lt $sourceFallbackPlan.Count) {
    $sourceFallbackVersion = [string]$sourceFallbackPlan[$sourceFallbackIndex]
    $sourceFallbackVersions += [string]$sourceFallbackVersion
    if (
      $preferMainSourceFallback -and
      ($sourceFallbackVersion -eq "main")
    ) {
      Write-Host "[infring install] source fallback using main first (missing prebuilt asset metadata for $Stem on $Triple)"
    } elseif (
      ($sourceFallbackVersion -eq "main") -and
      ($sourceFallbackVersions.Count -gt 1)
    ) {
      Write-Host "[infring install] source fallback for release $Version failed ($script:LastBinaryInstallFailureReason); retrying from main branch"
    }
    $fallbackOk = Install-BinaryFromSourceFallback $sourceFallbackVersion $Stem $OutPath
    if ($fallbackOk) {
      break
    }
    $sourceFallbackReason = [string]$script:LastBinaryInstallFailureReason
    $mainRetryEligible = (
      $allowMainLastResortFallback -and
      ($sourceFallbackVersion -ne "main") -and
      ($Version -ne "main") -and
      (-not @($sourceFallbackPlan).Contains("main")) -and
      (
        $sourceFallbackReason.StartsWith("cargo_build_failed") -or
        ($sourceFallbackReason -eq "source_build_output_missing")
      )
    )
    if ($mainRetryEligible) {
      $sourceFallbackPlan.Add("main") | Out-Null
      Write-Host ("[infring install] source fallback for {0} failed ({1}); appending main as last-resort source retry" -f [string]$sourceFallbackVersion, $sourceFallbackReason)
    }
    $sourceFallbackIndex += 1
  }
  if ($sourceFallbackPlan.Count -gt 0) {
    Write-Host ("[infring install] source fallback plan: {0}" -f (@($sourceFallbackPlan) -join ","))
  }
  $script:LastBinaryInstallFailure = @{
    stem = $Stem
    triple = $Triple
    version = $Version
    attempted_assets = @($attemptedAssets)
    source_fallback_attempted = $true
    source_fallback_versions = @($sourceFallbackVersions)
    source_fallback_plan = @($sourceFallbackPlan)
    source_fallback_reason = [string]$script:LastBinaryInstallFailureReason
    auto_msvc_bootstrap_enabled = [bool](Install-AutoMsvcBootstrapEnabled)
    main_last_resort_fallback = [bool]$allowMainLastResortFallback
    preflight_no_reachable_prebuilt_with_missing_msvc = [bool]$noReachablePrebuiltWithMissingMsvc
    asset_probe = $assetProbe
  }
  return $fallbackOk
}

function Install-ClientBundle($Version, $Triple, $OutDir) {
  New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
  $tmp = New-TemporaryFile
  Remove-Item $tmp.FullName -Force
  New-Item -ItemType Directory -Path $tmp.FullName | Out-Null
  $archive = Join-Path $tmp.FullName "client-runtime.bundle"
  function Expand-ClientArchive($ArchivePath, $Destination, $AssetName = $null) {
    if (-not $AssetName) { $AssetName = $ArchivePath }
    if ($AssetName.EndsWith(".tar.zst")) {
      try {
        tar -xf $ArchivePath -C $Destination
        return $true
      } catch {
        if (Get-Command zstd -ErrorAction SilentlyContinue) {
          $tarPath = [System.IO.Path]::ChangeExtension($ArchivePath, ".tar")
          zstd -d --stdout $ArchivePath > $tarPath
          tar -xf $tarPath -C $Destination
          return $true
        }
        Write-Host "[infring install] skipping .tar.zst bundle (zstd unavailable); falling back to .tar.gz assets"
        return $false
      }
    }
    if ($AssetName.EndsWith(".tar.gz")) {
      tar -xzf $ArchivePath -C $Destination
      return $true
    }
    try {
      tar -xzf $ArchivePath -C $Destination
      return $true
    } catch {
      if (Get-Command zstd -ErrorAction SilentlyContinue) {
        $tarPath = [System.IO.Path]::ChangeExtension($ArchivePath, ".tar")
        zstd -d --stdout $ArchivePath > $tarPath
        tar -xf $tarPath -C $Destination
        return $true
      }
    }
    return $false
  }
  $assets = @(
    "infring-client-runtime-$Triple.tar.zst",
    "infring-client-runtime.tar.zst",
    "infring-client-$Triple.tar.zst",
    "infring-client.tar.zst",
    "infring-client-runtime-$Triple.tar.gz",
    "infring-client-runtime.tar.gz",
    "infring-client-$Triple.tar.gz",
    "infring-client.tar.gz"
  )
  foreach ($asset in $assets) {
    if (Download-Asset $Version $asset $archive) {
      if (Expand-ClientArchive $archive $OutDir $asset) {
        Write-Host "[infring install] installed optional client runtime bundle"
        return $true
      }
    }
  }
  return $false
}

function Install-ClientBundleFromSourceFallback($OutDir) {
  if (-not ($script:SourceFallbackDir -and (Test-Path $script:SourceFallbackDir))) {
    return $false
  }

  $repoDir = $script:SourceFallbackDir
  $runtimeSource = Join-Path $repoDir "client/runtime"
  if (-not (Test-Path $runtimeSource)) {
    return $false
  }

  if (Test-Path $OutDir) {
    Remove-Item -Force -Recurse $OutDir
  }
  $clientRoot = Join-Path $OutDir "client"
  New-Item -ItemType Directory -Force -Path $clientRoot | Out-Null
  Copy-Item -Recurse -Force $runtimeSource (Join-Path $clientRoot "runtime")
  Write-Host "[infring install] installed client runtime from source fallback"
  return $true
}

function Test-RuntimeEntrypointForMode {
  param(
    [string]$RuntimeRoot,
    [string]$RelativePath,
    [string]$RuntimeMode = "source"
  )

  $targetPath = Join-Path $RuntimeRoot $RelativePath
  if (Test-Path $targetPath) {
    return $true
  }
  if ($RuntimeMode -ne "source") {
    return $false
  }
  if ($RelativePath.EndsWith(".js")) {
    $tsRel = $RelativePath.Substring(0, $RelativePath.Length - 3) + ".ts"
    return (Test-Path (Join-Path $RuntimeRoot $tsRel))
  }
  if ($RelativePath.EndsWith(".ts")) {
    $jsRel = $RelativePath.Substring(0, $RelativePath.Length - 3) + ".js"
    return (Test-Path (Join-Path $RuntimeRoot $jsRel))
  }
  return $false
}

function Test-InstallRuntimeManifestContract {
  param(
    [string]$RuntimeRoot,
    [string]$RuntimeMode = "source",
    [string]$ContextLabel = "runtime"
  )

  if ([string]::IsNullOrWhiteSpace($RuntimeRoot)) {
    return $false
  }
  $manifestRel = [string]$script:RuntimeManifestRel
  $manifestPath = Join-Path $RuntimeRoot $manifestRel
  if (-not (Test-Path $manifestPath)) {
    Write-Host "[infring install] runtime integrity check failed ($ContextLabel): manifest missing"
    Write-Host "[infring install] missing: $manifestRel"
    return $false
  }
  $manifestEntries = New-Object -TypeName 'System.Collections.Generic.HashSet[string]' -ArgumentList ([System.StringComparer]::OrdinalIgnoreCase)
  foreach ($row in Get-Content -Path $manifestPath) {
    $entry = [string]$row
    if ([string]::IsNullOrWhiteSpace($entry)) { continue }
    $entry = $entry.Trim()
    if ([string]::IsNullOrWhiteSpace($entry) -or $entry.StartsWith("#")) { continue }
    [void]$manifestEntries.Add($entry)
  }
  $missingManifestEntries = New-Object System.Collections.Generic.List[string]
  foreach ($requiredRel in $script:RuntimeTier1RequiredEntrypoints) {
    if (-not $manifestEntries.Contains([string]$requiredRel)) {
      $missingManifestEntries.Add([string]$requiredRel) | Out-Null
    }
  }
  if ($missingManifestEntries.Count -gt 0) {
    Write-Host "[infring install] runtime integrity check failed ($ContextLabel mode=$RuntimeMode): manifest missing required Tier-1 runtime entries"
    foreach ($rel in $missingManifestEntries) {
      Write-Host "[infring install] manifest-missing: $rel"
    }
    Write-Host "[infring install] manifest: $manifestRel"
    return $false
  }
  $missing = New-Object System.Collections.Generic.List[string]
  foreach ($row in Get-Content -Path $manifestPath) {
    $rel = [string]$row
    if ([string]::IsNullOrWhiteSpace($rel)) { continue }
    $rel = $rel.Trim()
    if ([string]::IsNullOrWhiteSpace($rel) -or $rel.StartsWith("#")) { continue }
    if (-not (Test-RuntimeEntrypointForMode -RuntimeRoot $RuntimeRoot -RelativePath $rel -RuntimeMode $RuntimeMode)) {
      $missing.Add($rel) | Out-Null
    }
  }
  if ($missing.Count -gt 0) {
    Write-Host "[infring install] runtime integrity check failed ($ContextLabel mode=$RuntimeMode): required command entrypoints are missing"
    foreach ($rel in $missing) {
      Write-Host "[infring install] missing: $rel"
    }
    Write-Host "[infring install] manifest: $manifestRel"
    return $false
  }
  Write-Host "[infring install] runtime integrity check: manifest verified ($manifestRel) [$ContextLabel mode=$RuntimeMode]"
  return $true
}

function Get-RuntimeNodeRequiredModules {
  param([string]$RuntimeRoot)

  $manifestRel = [string]$script:RuntimeNodeModuleManifestRel
  $manifestPath = Join-Path $RuntimeRoot $manifestRel
  if (-not (Test-Path $manifestPath)) {
    Write-Host "[infring install] node module closure failed: dependency manifest missing ($manifestRel)"
    return $null
  }
  $modules = New-Object System.Collections.Generic.List[string]
  foreach ($row in Get-Content -Path $manifestPath) {
    $module = [string]$row
    if ([string]::IsNullOrWhiteSpace($module)) { continue }
    $module = $module.Trim()
    if ([string]::IsNullOrWhiteSpace($module) -or $module.StartsWith("#")) { continue }
    if (-not $modules.Contains($module)) {
      $modules.Add($module) | Out-Null
    }
  }
  if ($modules.Count -eq 0) {
    $fallbackRaw = if ($env:INFRING_RUNTIME_NODE_REQUIRED_MODULES) {
      $env:INFRING_RUNTIME_NODE_REQUIRED_MODULES
    } else {
      "typescript ws"
    }
    foreach ($module in ($fallbackRaw -split "\s+")) {
      $value = [string]$module
      if ([string]::IsNullOrWhiteSpace($value)) { continue }
      if (-not $modules.Contains($value)) {
        $modules.Add($value) | Out-Null
      }
    }
  }
  return $modules
}

function Test-NodeModuleResolvable {
  param(
    [string]$RuntimeRoot,
    [string]$ModuleName,
    [string]$NodeExecutable,
    [string]$NodePathPrefix
  )

  $nodeExec = if ([string]::IsNullOrWhiteSpace($NodeExecutable)) { "node" } else { $NodeExecutable }
  $escapedModule = [string]$ModuleName -replace "'", "''"
  $probe = "try{require.resolve('$escapedModule');process.exit(0);}catch(_e){process.exit(1);}"
  $previousPath = $env:Path
  if (-not [string]::IsNullOrWhiteSpace($NodePathPrefix)) {
    $env:Path = "$NodePathPrefix;$env:Path"
  }
  Push-Location $RuntimeRoot
  try {
    & $nodeExec -e $probe *> $null
    return ($LASTEXITCODE -eq 0)
  } finally {
    Pop-Location
    if (-not [string]::IsNullOrWhiteSpace($NodePathPrefix)) {
      $env:Path = $previousPath
    }
  }
}

function Resolve-NodeToolchainCommands {
  $nodeCmd = Get-Command node -ErrorAction SilentlyContinue
  $npmCmd = Get-Command npm -ErrorAction SilentlyContinue
  $nodePath = ""
  $npmPath = ""
  $nodeDir = ""
  if ($nodeCmd) {
    $nodePath = [string]$nodeCmd.Source
    try {
      $nodeDir = Split-Path -Parent $nodePath
    } catch {
      $nodeDir = ""
    }
  }
  if ($npmCmd) {
    $npmPath = [string]$npmCmd.Source
  }
  return @{
    node_present = [bool]$nodeCmd
    npm_present = [bool]$npmCmd
    node_path = $nodePath
    npm_path = $npmPath
    node_dir = $nodeDir
  }
}

function Ensure-RuntimeNodeModuleClosure {
  param([string]$RuntimeRoot)

  $toolchain = Resolve-NodeToolchainCommands
  if (-not [bool]$toolchain.node_present) {
    Write-Host "[infring install] node module closure failed: node runtime unavailable"
    return $false
  }
  if (-not [bool]$toolchain.npm_present) {
    Write-Host "[infring install] node module closure failed: npm unavailable"
    return $false
  }
  Write-Host ("[infring install] node closure toolchain: node={0}; npm={1}" -f [string]$toolchain.node_path, [string]$toolchain.npm_path)
  $requiredModules = Get-RuntimeNodeRequiredModules -RuntimeRoot $RuntimeRoot
  if (-not $requiredModules -or $requiredModules.Count -eq 0) {
    Write-Host "[infring install] node module closure failed: required-module list is empty"
    return $false
  }
  $missing = New-Object System.Collections.Generic.List[string]
  foreach ($module in $requiredModules) {
    if (-not (Test-NodeModuleResolvable -RuntimeRoot $RuntimeRoot -ModuleName $module -NodeExecutable ([string]$toolchain.node_path) -NodePathPrefix ([string]$toolchain.node_dir))) {
      $missing.Add([string]$module) | Out-Null
    }
  }
  if ($missing.Count -eq 0) {
    Write-Host "[infring install] node module closure: satisfied"
    return $true
  }
  if (-not (Test-Path (Join-Path $RuntimeRoot "package.json"))) {
    Write-Host ("[infring install] node module closure failed: package.json missing in runtime root (missing:{0})" -f ($missing -join " "))
    return $false
  }
  Write-Host ("[infring install] installing runtime node module closure: {0}" -f ($missing -join " "))
  $previousPath = $env:Path
  if (-not [string]::IsNullOrWhiteSpace([string]$toolchain.node_dir)) {
    $env:Path = "$([string]$toolchain.node_dir);$env:Path"
  }
  Push-Location $RuntimeRoot
  try {
    & ([string]$toolchain.npm_path) install --silent --no-audit --no-fund --no-save @($missing.ToArray())
    if ($LASTEXITCODE -ne 0) {
      Write-Host "[infring install] node module closure install failed"
      return $false
    }
  } finally {
    Pop-Location
    if (-not [string]::IsNullOrWhiteSpace([string]$toolchain.node_dir)) {
      $env:Path = $previousPath
    }
  }
  $stillMissing = New-Object System.Collections.Generic.List[string]
  foreach ($module in $requiredModules) {
    if (-not (Test-NodeModuleResolvable -RuntimeRoot $RuntimeRoot -ModuleName $module -NodeExecutable ([string]$toolchain.node_path) -NodePathPrefix ([string]$toolchain.node_dir))) {
      $stillMissing.Add([string]$module) | Out-Null
    }
  }
  if ($stillMissing.Count -gt 0) {
    Write-Host ("[infring install] node module closure verification failed: {0}" -f ($stillMissing -join " "))
    return $false
  }
  Write-Host "[infring install] node module closure: installed and verified"
  return $true
}

function Resolve-WorkspaceRootForRepair {
  $candidates = @(
    $env:INFRING_WORKSPACE_ROOT,
    # Legacy compatibility only; canonical workspace root env is INFRING_WORKSPACE_ROOT.
    $env:INFRING_WORKSPACE_ROOT,
    (Get-Location).Path,
    (Join-Path $HOME ".infring/workspace"),
    # Legacy compatibility path.
    (Join-Path $HOME ".infring/workspace")
  )
  foreach ($candidate in $candidates) {
    if (-not $candidate) { continue }
    $manifest = Join-Path $candidate "core/layer0/ops/Cargo.toml"
    $runtimeDir = Join-Path $candidate "client/runtime"
    if ((Test-Path $manifest) -and (Test-Path $runtimeDir)) {
      return $candidate
    }
  }
  return $null
}

function Get-WorkspaceInstallReleaseTag {
  param(
    [string]$WorkspaceRoot
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    return ""
  }
  $statePath = Join-Path $WorkspaceRoot "local/state/ops/install_release_tag.txt"
  if (-not (Test-Path $statePath)) {
    return ""
  }
  try {
    return ([string](Get-Content -Path $statePath -TotalCount 1 -ErrorAction Stop)).Trim()
  } catch {
    return ""
  }
}

function Set-WorkspaceInstallReleaseTag {
  param(
    [string]$WorkspaceRoot,
    [string]$VersionTag
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot) -or [string]::IsNullOrWhiteSpace($VersionTag)) {
    return $false
  }
  $stateDir = Join-Path $WorkspaceRoot "local/state/ops"
  try {
    New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
    $statePath = Join-Path $stateDir "install_release_tag.txt"
    Set-Content -Path $statePath -Value $VersionTag -Encoding UTF8
    $writtenTag = Get-WorkspaceInstallReleaseTag -WorkspaceRoot $WorkspaceRoot
    if ([string]::IsNullOrWhiteSpace($writtenTag) -or ($writtenTag -ne $VersionTag)) {
      return $false
    }
    return $true
  } catch {
    return $false
  }
}

function Write-WorkspaceRuntimeReleaseTagState {
  param(
    [string]$WorkspaceRoot,
    [string]$VersionTag
  )

  return [bool](Set-WorkspaceInstallReleaseTag -WorkspaceRoot $WorkspaceRoot -VersionTag $VersionTag)
}

function Assert-WorkspaceRuntimeReleaseTagState {
  param(
    [string]$WorkspaceRoot,
    [string]$VersionTag
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot) -or [string]::IsNullOrWhiteSpace($VersionTag)) {
    return $false
  }
  $writtenTag = [string](Get-WorkspaceInstallReleaseTag -WorkspaceRoot $WorkspaceRoot)
  return ((-not [string]::IsNullOrWhiteSpace($writtenTag)) -and ($writtenTag -eq $VersionTag))
}

function Resolve-WorkspaceRuntimeRefreshTarget {
  param(
    [string]$WorkspaceRoot
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    return $null
  }
  $primary = Join-Path $WorkspaceRoot "client/runtime"
  if (Test-Path (Join-Path $WorkspaceRoot "client")) {
    return $primary
  }
  $alt = Join-Path $WorkspaceRoot "infring-client/client/runtime"
  if (Test-Path (Join-Path $WorkspaceRoot "infring-client/client")) {
    return $alt
  }
  return $primary
}

function Resolve-WorkspaceRuntimeRefreshSource {
  param(
    [string]$InstallDir,
    [string]$SourceFallbackDir
  )

  $candidates = New-Object System.Collections.Generic.List[string]
  if (-not [string]::IsNullOrWhiteSpace($InstallDir)) {
    $candidates.Add((Join-Path $InstallDir "infring-client/client/runtime")) | Out-Null
  }
  if (-not [string]::IsNullOrWhiteSpace($SourceFallbackDir)) {
    $candidates.Add((Join-Path $SourceFallbackDir "client/runtime")) | Out-Null
    $candidates.Add((Join-Path $SourceFallbackDir "infring-client/client/runtime")) | Out-Null
  }
  foreach ($candidate in $candidates) {
    if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path $candidate -PathType Container)) {
      return $candidate
    }
  }
  return $null
}

function Resolve-WorkspaceRuntimeRefreshDecision {
  param(
    [string]$WorkspaceRoot,
    [string]$VersionTag,
    [bool]$Repair
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    return @{
      refresh_required = $false
      reason = ""
      previous_tag = ""
      target = ""
      runtime_exists = $false
      tag_state_missing = $false
    }
  }

  $target = Resolve-WorkspaceRuntimeRefreshTarget -WorkspaceRoot $WorkspaceRoot
  $runtimeExists = $false
  if (-not [string]::IsNullOrWhiteSpace($target)) {
    $runtimeExists = Test-Path $target -PathType Container
  }
  $previousTag = Get-WorkspaceInstallReleaseTag -WorkspaceRoot $WorkspaceRoot
  $tagStateMissing = [string]::IsNullOrWhiteSpace($previousTag)
  $reason = ""
  if ($Repair) {
    $reason = "repair_mode"
  } elseif (-not $runtimeExists) {
    $reason = "runtime_missing"
  } elseif ($tagStateMissing) {
    $reason = "tag_state_missing"
  } elseif ($previousTag -ne $VersionTag) {
    $reason = "release_tag_changed"
  }

  return @{
    refresh_required = (-not [string]::IsNullOrWhiteSpace($reason))
    reason = $reason
    previous_tag = $previousTag
    target = $target
    runtime_exists = [bool]$runtimeExists
    tag_state_missing = [bool]$tagStateMissing
  }
}

function Invoke-WorkspaceRuntimeRefresh {
  param(
    [string]$WorkspaceRoot,
    [string]$InstallDir,
    [string]$SourceFallbackDir,
    [string]$Reason
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    return $false
  }
  $source = Resolve-WorkspaceRuntimeRefreshSource -InstallDir $InstallDir -SourceFallbackDir $SourceFallbackDir
  if ([string]::IsNullOrWhiteSpace($source)) {
    Write-Host "[infring install] workspace runtime refresh skipped (source runtime missing)"
    return $false
  }
  $target = Resolve-WorkspaceRuntimeRefreshTarget -WorkspaceRoot $WorkspaceRoot
  if ([string]::IsNullOrWhiteSpace($target)) {
    Write-Host "[infring install] workspace runtime refresh skipped (target path unresolved)"
    return $false
  }

  $sourceNorm = ""
  $targetNorm = ""
  try {
    $sourceNorm = (Resolve-Path -LiteralPath $source -ErrorAction Stop).Path.ToLowerInvariant()
  } catch {
    $sourceNorm = [string]$source
  }
  try {
    if (Test-Path $target) {
      $targetNorm = (Resolve-Path -LiteralPath $target -ErrorAction Stop).Path.ToLowerInvariant()
    }
  } catch {
    $targetNorm = [string]$target
  }
  if (-not [string]::IsNullOrWhiteSpace($targetNorm) -and $sourceNorm -eq $targetNorm) {
    Write-Host "[infring install] workspace runtime refresh: source and target already aligned"
    return $true
  }

  $targetParent = Split-Path -Parent $target
  New-Item -ItemType Directory -Force -Path $targetParent | Out-Null
  $targetName = Split-Path -Leaf $target
  $staging = Join-Path $targetParent ($targetName + ".__install_tmp_" + [guid]::NewGuid().ToString("N"))
  try {
    if (Test-Path $staging) {
      Remove-Item -Force -Recurse $staging
    }
    Copy-Item -Force -Recurse $source $staging
    if (Test-Path $target) {
      Remove-Item -Force -Recurse $target
    }
    Move-Item -Path $staging -Destination $target
    Write-Host "[infring install] workspace runtime refreshed: reason=$Reason source=$source target=$target"
    return $true
  } catch {
    Write-Host "[infring install] workspace runtime refresh failed: $($_.Exception.Message)"
    try {
      if (Test-Path $staging) { Remove-Item -Force -Recurse $staging }
    } catch {}
    return $false
  }
}

function Ensure-WorkspaceRuntimeContract {
  param(
    [string]$WorkspaceRoot,
    [string]$InstallDir,
    [string]$SourceFallbackDir
  )

  if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    return $true
  }
  $runtimeRoot = Resolve-WorkspaceRuntimeRefreshTarget -WorkspaceRoot $WorkspaceRoot
  if ([string]::IsNullOrWhiteSpace($runtimeRoot)) {
    return $false
  }
  if (Test-InstallRuntimeManifestContract -RuntimeRoot $runtimeRoot -RuntimeMode "source" -ContextLabel "workspace_runtime") {
    return (Ensure-RuntimeNodeModuleClosure -RuntimeRoot $runtimeRoot)
  }
  Write-Host "[infring install] workspace runtime contract failed; attempting self-heal refresh"
  if (-not (Invoke-WorkspaceRuntimeRefresh -WorkspaceRoot $WorkspaceRoot -InstallDir $InstallDir -SourceFallbackDir $SourceFallbackDir -Reason "runtime_contract_failed")) {
    return $false
  }
  if (-not (Test-InstallRuntimeManifestContract -RuntimeRoot $runtimeRoot -RuntimeMode "source" -ContextLabel "workspace_runtime_post_heal")) {
    return $false
  }
  return (Ensure-RuntimeNodeModuleClosure -RuntimeRoot $runtimeRoot)
}

function Test-RepairArtifactBroken {
  param(
    [string]$InstallPath,
    [string]$ArtifactName
  )

  if ($ArtifactName -like "infring*") {
    return $true
  }
  if (-not (Test-Path $InstallPath)) {
    return $true
  }
  if (Test-Path $InstallPath -PathType Container) {
    if ($ArtifactName -eq "infring-client") {
      return (-not (Test-Path (Join-Path $InstallPath "client/runtime/config/install_runtime_manifest_v1.txt")))
    }
    return $false
  }
  $item = Get-Item -LiteralPath $InstallPath -ErrorAction SilentlyContinue
  if ($null -eq $item -or $item.Length -le 0) {
    return $true
  }
  if ($ArtifactName.ToLowerInvariant().EndsWith(".cmd")) {
    $content = (Get-Content -LiteralPath $InstallPath -TotalCount 120 -ErrorAction SilentlyContinue) -join "`n"
    if ([string]::IsNullOrWhiteSpace($content)) {
      return $true
    }
    $contentLower = $content.ToLowerInvariant()
    if ($content.Contains("<<<<<<<") -or $content.Contains("=======") -or $content.Contains(">>>>>>>")) {
      return $true
    }
    if ($content.Contains("__PS1__") -or $content.Contains("__TARGET__")) {
      return $true
    }
    $hasDispatchOrBootstrapEntry = $contentLower.Contains(":_dispatch") -or $contentLower.Contains("_bootstrap_action")
    $hasPowerShellDispatch = $contentLower.Contains("powershell.exe -noprofile -executionpolicy bypass -file")
    $hasBootstrapRecoveryCopy = $contentLower.Contains("[infring bootstrap] runtime binaries are not installed on this machine yet.")
    $containsLegacyThrowOnlyTemplate = $contentLower.Contains("missing command wrapper:") -and (-not $hasBootstrapRecoveryCopy)
    if ($containsLegacyThrowOnlyTemplate) {
      return $true
    }
    return (-not (($hasDispatchOrBootstrapEntry -or $hasPowerShellDispatch) -and $hasBootstrapRecoveryCopy))
  }
  if ($ArtifactName.ToLowerInvariant().EndsWith(".ps1")) {
    $content = (Get-Content -LiteralPath $InstallPath -TotalCount 120 -ErrorAction SilentlyContinue) -join "`n"
    if ([string]::IsNullOrWhiteSpace($content)) {
      return $true
    }
    $contentLower = $content.ToLowerInvariant()
    if ($content.Contains("<<<<<<<") -or $content.Contains("=======") -or $content.Contains(">>>>>>>")) {
      return $true
    }
    if ($content.Contains("__TARGET__") -or $content.Contains("__PS1__")) {
      return $true
    }
    if ($content.Contains('Join-Path\\s+\\$PSScriptRoot') -or $content.Contains('Join-Path\\s+\\')) {
      return $true
    }
    $hasJoinPathReference = $contentLower.Contains("join-path $psscriptroot") -or $contentLower.Contains("join-path ${psscriptroot}")
    if (-not $hasJoinPathReference) {
      return $true
    }
    $hasCommandArgsSignature = $contentLower.Contains("valuefromremainingarguments=$true") -and $contentLower.Contains("commandargs")
    if (-not $hasCommandArgsSignature) {
      return $true
    }
    if ($contentLower.Contains("missing command wrapper") -and $contentLower.Contains("throw ")) {
      return $true
    }
    $hasBootstrapRecoveryCopy = $contentLower.Contains("[infring bootstrap] run: install.ps1 -repair -full") -or $contentLower.Contains("[infring shim] bootstrap wrapper missing")
    if (-not $hasBootstrapRecoveryCopy) {
      return $true
    }
    return $false
  }
  return $false
}

function Ensure-RepairBootstrapWrapperFloor {
  param(
    [string]$InstallDir
  )

  $cmdTemplate = @'
@echo off
setlocal
set "_BOOTSTRAP_ACTION=%~n0"
set "_OPS=%~dp0infring-ops.exe"
set "_DAEMON=%~dp0infringd.exe"
set "_CONDUIT=%~dp0conduit_daemon.exe"

if /I "%_BOOTSTRAP_ACTION%"=="infring" (
  if exist "%_OPS%" (
    "%_OPS%" infringctl %*
    exit /b %ERRORLEVEL%
  )
  goto :bootstrap
)

if /I "%_BOOTSTRAP_ACTION%"=="infringctl" (
  if exist "%_OPS%" (
    "%_OPS%" infringctl %*
    exit /b %ERRORLEVEL%
  )
  goto :bootstrap
)

if /I "%_BOOTSTRAP_ACTION%"=="infringd" (
  if exist "%_DAEMON%" (
    "%_DAEMON%" %*
    exit /b %ERRORLEVEL%
  )
  if exist "%_CONDUIT%" (
    "%_CONDUIT%" %*
    exit /b %ERRORLEVEL%
  )
  if exist "%_OPS%" (
    "%_OPS%" spine %*
    exit /b %ERRORLEVEL%
  )
  goto :bootstrap
)

:bootstrap
echo [infring bootstrap] runtime binaries are not installed on this machine yet.
echo [infring bootstrap] rerun with execution-policy bypass:
echo [infring bootstrap] powershell.exe -NoProfile -ExecutionPolicy Bypass -File install.ps1 -Repair -Full
exit /b %ERRORLEVEL%
'@
  $ps1Template = @'
param([Parameter(ValueFromRemainingArguments=$true)][string[]]$CommandArgs)
$target = Join-Path $PSScriptRoot "__TARGET__"
if (-not (Test-Path -LiteralPath $target)) {
  Write-Warning "[infring shim] bootstrap wrapper missing: $target"
  Write-Host "[infring bootstrap] run: install.ps1 -Repair -Full"
  exit 0
}
& $target @CommandArgs
exit $LASTEXITCODE
'@
  $wrapperNames = @(
    @{ cmd = "infring.cmd"; ps1 = "infring.ps1" },
    @{ cmd = "infringctl.cmd"; ps1 = "infringctl.ps1" },
    @{ cmd = "infringd.cmd"; ps1 = "infringd.ps1" }
  )

  $wrapperSpecs = @(
    foreach ($wrapper in $wrapperNames) {
      $wrapperName = [string]$wrapper.cmd
      @{
        cmd = $wrapperName
        ps1 = [string]$wrapper.ps1
        cmd_body = $cmdTemplate
        ps1_body = $ps1Template.Replace("__TARGET__", $wrapperName)
      }
    }
  )
  if (Invoke-OptionalWindowsWrapperModule -BinDir $InstallDir -WrapperSpecs $wrapperSpecs) {
    Remove-StaleWindowsCommandShims -ShimInstallDir $InstallDir
    return
  }

  foreach ($wrapper in $wrapperNames) {
    $wrapperName = [string]$wrapper.cmd
    $cmdPath = Join-Path $InstallDir ([string]$wrapperName)
    $rewriteCmd = $false
    if (Test-Path $cmdPath) {
      $rewriteCmd = [bool](Test-RepairArtifactBroken -InstallPath $cmdPath -ArtifactName ([string]$wrapperName))
    }
    if ((-not (Test-Path $cmdPath)) -or $rewriteCmd) {
      try {
        $cmdContent = $cmdTemplate
        Set-Content -LiteralPath $cmdPath -Value $cmdContent -Encoding ASCII -Force
        if ($rewriteCmd) {
          Write-Host "[infring install] repair rewrote broken command wrapper: $cmdPath"
        } else {
          Write-Host "[infring install] repair bootstrapped command wrapper: $cmdPath"
        }
      } catch {
        Write-Host "[infring install] repair warning: failed to bootstrap command wrapper: $cmdPath"
      }
    }
    $ps1Name = [string]$wrapper.ps1
    $ps1Path = Join-Path $InstallDir $ps1Name
    $rewritePs1 = $false
    if (Test-Path $ps1Path) {
      $rewritePs1 = [bool](Test-RepairArtifactBroken -InstallPath $ps1Path -ArtifactName $ps1Name)
    }
    if ((-not (Test-Path $ps1Path)) -or $rewritePs1) {
      try {
        $psContent = $ps1Template.Replace("__TARGET__", $wrapperName)
        Set-Content -LiteralPath $ps1Path -Value $psContent -Encoding UTF8 -Force
        if ($rewritePs1) {
          Write-Host "[infring install] repair rewrote broken PowerShell shim: $ps1Path"
        } else {
          Write-Host "[infring install] repair bootstrapped PowerShell shim: $ps1Path"
        }
      } catch {
        Write-Host "[infring install] repair warning: failed to bootstrap PowerShell shim: $ps1Path"
      }
    }
  }
  Remove-StaleWindowsCommandShims -ShimInstallDir $InstallDir
}

function Invoke-RepairInstallDir {
  $legacyWrapperTargets = @()
  $targets = @(
    "infring.cmd", "infringctl.cmd", "infringd.cmd",
    "infring.ps1", "infringctl.ps1", "infringd.ps1",
    "infring-ops.exe", "infring-pure-workspace.exe",
    "infringd.exe", "conduit_daemon.exe", "infring-client"
  )
  $repairArchiveRoot = Join-Path $InstallDir "_repair_archive"
  $repairArchiveRun = Join-Path $repairArchiveRoot (Get-Date -Format "yyyyMMddTHHmmss")
  New-Item -ItemType Directory -Force -Path $repairArchiveRun | Out-Null
  $repairRemoved = 0
  $repairPreserved = 0
  foreach ($target in $targets) {
    $path = Join-Path $InstallDir $target
    if (Test-Path $path) {
      if ($legacyWrapperTargets -contains $target) {
        Remove-Item -Force -Recurse $path
        $repairRemoved += 1
        Write-Host "[infring install] repair removed stale legacy command wrapper: $path"
        continue
      }
      $artifactBroken = Test-RepairArtifactBroken -InstallPath $path -ArtifactName $target
      if ($artifactBroken) {
        Remove-Item -Force -Recurse $path
        $repairRemoved += 1
        Write-Host "[infring install] repair removed broken install artifact: $path"
        continue
      }
      try {
        Copy-Item -Force -Recurse $path (Join-Path $repairArchiveRun $target)
        Write-Host "[infring install] repair archived healthy install artifact: $path"
      } catch {
        Write-Host "[infring install] repair warning: failed to archive healthy install artifact: $path"
      }
      $repairPreserved += 1
      Write-Host "[infring install] repair preserved healthy install artifact: $path"
    }
  }
  Ensure-RepairBootstrapWrapperFloor -InstallDir $InstallDir
  $requiredWrappers = @(
    "infring.cmd", "infringctl.cmd", "infringd.cmd"
  )
  $missingWrappers = @($requiredWrappers | Where-Object {
      -not (Test-Path (Join-Path $InstallDir $_))
    })
  if ($missingWrappers.Count -gt 0) {
    Write-Host "[infring install] repair warning: bootstrap wrapper floor incomplete; retrying wrapper regeneration"
    Ensure-RepairBootstrapWrapperFloor -InstallDir $InstallDir
    $missingWrappers = @($requiredWrappers | Where-Object {
        -not (Test-Path (Join-Path $InstallDir $_))
      })
  }
  if ($missingWrappers.Count -gt 0) {
    throw "repair wrapper floor failed; missing wrappers: $($missingWrappers -join ', ')"
  }
  $script:RepairArchiveRun = [string]$repairArchiveRun
  $script:RepairRemovedCount = [int]$repairRemoved
  $script:RepairPreservedCount = [int]$repairPreserved
  Write-Host "[infring install] repair summary: removed=$repairRemoved preserved=$repairPreserved archive=$repairArchiveRun"
}

function Invoke-RepairWorkspaceState {
  $workspaceRoot = Resolve-WorkspaceRootForRepair
  if (-not $workspaceRoot) {
    Write-Host "[infring install] repair skipped workspace cleanup (workspace root not detected)"
    return
  }
  $timestamp = Get-Date -Format "yyyyMMddTHHmmssZ"
  $archiveDir = Join-Path $workspaceRoot "local/workspace/archive/install-repair"
  New-Item -ItemType Directory -Force -Path $archiveDir | Out-Null

  $memoryPath = Join-Path $workspaceRoot "local/workspace/memory"
  if (Test-Path $memoryPath) {
    $memoryArchive = Join-Path $archiveDir "memory-$timestamp.zip"
    try {
      Compress-Archive -Path $memoryPath -DestinationPath $memoryArchive -Force
      Write-Host "[infring install] repair archived local/workspace/memory to $memoryArchive"
    } catch {
      Write-Host "[infring install] repair warning: failed to archive memory path ($memoryPath)"
    }
  }

  $statePath = Join-Path $workspaceRoot "local/state"
  if (Test-Path $statePath) {
    $stateArchive = Join-Path $archiveDir "state-$timestamp.zip"
    try {
      Compress-Archive -Path $statePath -DestinationPath $stateArchive -Force
      Write-Host "[infring install] repair archived local/state to $stateArchive"
    } catch {
      Write-Host "[infring install] repair warning: failed to archive state path ($statePath)"
    }
  }

  $cleanup = @("client/runtime/local", "client/tmp", "core/local/tmp", "local/state")
  foreach ($rel in $cleanup) {
    $abs = Join-Path $workspaceRoot $rel
    if (Test-Path -LiteralPath $abs) {
      try {
        Remove-Item -Force -Recurse -LiteralPath $abs -ErrorAction Stop
        Write-Host "[infring install] repair removed stale runtime path: $rel"
      } catch {
        $cleanupReason = "unknown"
        try {
          if ($_ -and $_.Exception -and $_.Exception.Message) {
            $cleanupReason = [string]$_.Exception.Message
          }
        } catch {}
        Write-Host "[infring install] repair warning: failed to remove stale runtime path: $rel ($cleanupReason)"
      }
    }
  }
  try {
    New-Item -ItemType Directory -Force -Path (Join-Path $workspaceRoot "local/state") -ErrorAction Stop | Out-Null
  } catch {
    $stateRecreateReason = "unknown"
    try {
      if ($_.Exception -and $_.Exception.Message) { $stateRecreateReason = [string]$_.Exception.Message }
    } catch {}
    Write-Host ("[infring install] repair warning: failed to recreate local/state ({0})" -f $stateRecreateReason)
  }
}

if ($VerifyInstallSummaryContract) {
  if (Test-InstallSummarySuccessContract -SummaryPath $InstallSummaryTextPath) {
    exit 0
  }
  exit 1
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
if ($InstallRepair) {
  Write-Host "[infring install] repair mode enabled"
  Invoke-RepairInstallDir
  Invoke-RepairWorkspaceState
}
$arch = Resolve-Arch
$osFlags = Resolve-HostOsFlags
$HostIsWindows = [bool]$osFlags.IsWindows
$HostIsLinux = [bool]$osFlags.IsLinux
$HostIsMacOS = [bool]$osFlags.IsMacOS
  $triple = if ($HostIsWindows) {
  "$arch-pc-windows-msvc"
} elseif ($HostIsLinux) {
  "$arch-unknown-linux-gnu"
} elseif ($HostIsMacOS) {
  "$arch-apple-darwin"
} else {
  throw "Unsupported OS for installer"
}
  $version = Resolve-Version
$script:ResolvedInstallVersionForSummary = [string]$version
$script:ResolvedInstallTripleForSummary = [string]$triple
$resolvedVersionLabel = $version
$script:InstallBootstrapOnlyMode = $false
$script:InstallBootstrapOnlyReason = ""

Write-Host "[infring install] version: $version"
Write-Host "[infring install] platform: $triple"
Write-Host "[infring install] install dir: $InstallDir"
if ($InstallOffline) {
  Write-Host "[infring install] mode: offline (network disabled; cached artifacts only)"
}

$opsBin = Join-Path $InstallDir "infring-ops.exe"
$pureBin = Join-Path $InstallDir "infring-pure-workspace.exe"
$infringdBin = Join-Path $InstallDir "infringd.exe"
$daemonBin = Join-Path $InstallDir "conduit_daemon.exe"
$preferredDaemonTriple = if ($HostIsLinux -and $arch -eq "x86_64") { "x86_64-unknown-linux-musl" } else { $triple }

if ($HostIsWindows) {
  # Required stems are only install-critical binaries.
  # Daemon binaries are optional at install time (installer can run in spine mode),
  # so they must not block compatible-tag selection on Windows.
  $requiredWindowsStems = @()
  if ($InstallPure) {
    $requiredWindowsStems += "infring-pure-workspace"
  } else {
    $requiredWindowsStems += "infring-ops"
  }
  Invoke-WindowsInstallerPreflight -VersionTag $version -Triple $triple -RequiredStems $requiredWindowsStems
  $allowCompatibleWindowsFallback = Install-AllowCompatibleReleaseFallback
  $allowPinnedCompatibleWindowsFallback = Install-AllowPinnedVersionCompatibleFallback
  $preflightWindowsAssetGaps = @()
  if ($script:WindowsInstallPreflight -and $script:WindowsInstallPreflight.assets) {
    $preflightWindowsAssetGaps = @($script:WindowsInstallPreflight.assets | Where-Object {
        (-not [bool]$_.asset_found) -or
        (([bool]$_.asset_found) -and (-not [bool]$_.reachable))
      })
  }
  if (
    ($RequestedVersion -ne "latest") -and
    (-not $allowPinnedCompatibleWindowsFallback) -and
    ($preflightWindowsAssetGaps.Count -gt 0)
  ) {
    Write-Host "[infring install] pinned Windows compatible-release fallback is disabled; set INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK=1 to allow compatible prebuilt selection when pinned tag assets are unavailable."
  }
  if (($RequestedVersion -eq "latest") -or $allowPinnedCompatibleWindowsFallback) {
    if (-not $allowCompatibleWindowsFallback) {
      Write-Host "[infring install] compatible Windows release fallback is disabled (set INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK=1 to enable alternate-tag prebuilt scanning)."
    } else {
      $compatibleWindows = Resolve-AssetCompatibleVersionForTriple $triple $requiredWindowsStems
      if ($compatibleWindows -and ($compatibleWindows -ne $version)) {
        if ($RequestedVersion -eq "latest") {
          Write-Host "[infring install] latest release $version is missing one or more required Windows prebuilts for $triple; using compatible release $compatibleWindows"
        } else {
          Write-Host "[infring install] pinned release $version is missing one or more required Windows prebuilts for $triple; using compatible release $compatibleWindows (disable with INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK=0)"
        }
        $version = $compatibleWindows
        $resolvedVersionLabel = $compatibleWindows
        Invoke-WindowsInstallerPreflight -VersionTag $version -Triple $triple -RequiredStems $requiredWindowsStems
      } elseif (-not $compatibleWindows) {
        Write-Host "[infring install] no compatible Windows prebuilt release found for required stems; source fallback remains a backup path only."
        if (Install-AutoMsvcBootstrapEnabled) {
          Write-Host "[infring install] auto MSVC bootstrap is enabled; installer will attempt Build Tools install during source fallback if needed."
        } else {
          Write-Host "[infring install] auto MSVC bootstrap is disabled; enable with INFRING_INSTALL_AUTO_MSVC=1 for best-effort source fallback repair."
        }
      }
    }
    if (-not $allowCompatibleWindowsFallback) {
      if (Install-AutoMsvcBootstrapEnabled) {
        Write-Host "[infring install] auto MSVC bootstrap is enabled; installer will attempt Build Tools install during source fallback if needed."
      } else {
        Write-Host "[infring install] auto MSVC bootstrap is disabled; enable with INFRING_INSTALL_AUTO_MSVC=1 for best-effort source fallback repair."
      }
    }
  }
}

if ($InstallRepair -and $HostIsWindows) {
  Remove-StaleWindowsCommandShims -ShimInstallDir $InstallDir
}

if ($InstallPure) {
  if (($RequestedVersion -eq "latest") -and (-not $HostIsWindows)) {
    $compatiblePure = Resolve-AssetCompatibleVersionForTriple $triple @("infring-pure-workspace")
    if ($compatiblePure -and ($compatiblePure -ne $version)) {
      Write-Host "[infring install] latest release $version does not publish pure prebuilt assets for $triple; using compatible release $compatiblePure"
      $version = $compatiblePure
      $resolvedVersionLabel = $compatiblePure
    }
  }
  $pureInstalled = $false
  if ($InstallTinyMax) {
    $pureInstalled = Install-Binary $version $triple "infring-pure-workspace-tiny-max" $pureBin
  }
  if (-not $pureInstalled) {
    $pureInstalled = Install-Binary $version $triple "infring-pure-workspace" $pureBin
  }
  if (-not $pureInstalled) {
    $failureHint = Format-BinaryInstallFailureHint -Stem "infring-pure-workspace" -Triple $triple -VersionTag $version
    $windowsToolsHint = if ($HostIsWindows) { (Get-WindowsBuildToolsInstallHint) } else { "" }
    throw "Failed to install pure workspace binary for $triple ($resolvedVersionLabel). No compatible prebuilt asset was found and source fallback did not complete. Diagnostic: $failureHint Install Rust toolchain + C++ build tools, then rerun the README Windows install command: $ReadmeWindowsInstallCommand $windowsToolsHint"
  }
  if ($InstallTinyMax) {
    Write-Host "[infring install] tiny-max pure mode selected: Rust-only tiny profile installed"
  } else {
    Write-Host "[infring install] pure mode selected: Rust-only client installed"
  }
} else {
  if (($RequestedVersion -eq "latest") -and (-not $HostIsWindows)) {
    $compatibleOps = Resolve-AssetCompatibleVersionForTriple $triple @("infring-ops")
    if ($compatibleOps -and ($compatibleOps -ne $version)) {
      Write-Host "[infring install] latest release $version does not publish core ops runtime prebuilt assets for $triple; using compatible release $compatibleOps"
      $version = $compatibleOps
      $resolvedVersionLabel = $compatibleOps
    }
  }
  $opsInstalled = Install-Binary $version $triple "infring-ops" $opsBin
  if (-not $opsInstalled) {
    $opsFailureReason = [string]$script:LastBinaryInstallFailureReason
    $opsFailure = $script:LastBinaryInstallFailure
    $missingPrebuiltWithNoMsvc = $false
    if ($opsFailure -and $opsFailure.ContainsKey("preflight_no_reachable_prebuilt_with_missing_msvc")) {
      $missingPrebuiltWithNoMsvc = [bool]$opsFailure.preflight_no_reachable_prebuilt_with_missing_msvc
    }
    $allowBootstrapOnlyFallback = (
      $HostIsWindows -and
      $InstallFull -and
      (-not $InstallPure) -and
      (
        $missingPrebuiltWithNoMsvc -or
        ($opsFailureReason -eq "msvc_tools_still_missing_after_bootstrap") -or
        ($opsFailureReason -eq "msvc_tools_missing_auto_bootstrap_disabled") -or
        ($opsFailureReason -eq "msvc_tools_missing_no_reachable_prebuilt_asset") -or
        ($opsFailureReason -eq "cargo_unusable_after_rustup_default") -or
        ($opsFailureReason -eq "cargo_unusable_auto_rustup_disabled") -or
        ($opsFailureReason -eq "rustup_missing_for_cargo_repair")
      )
    )
    if ($allowBootstrapOnlyFallback) {
      if ([string]::IsNullOrWhiteSpace($opsFailureReason)) {
        $opsFailureReason = "ops_runtime_unavailable"
      }
      $script:InstallBootstrapOnlyMode = $true
      $script:InstallBootstrapOnlyReason = [string]$opsFailureReason
      Write-Host "[infring install] full-mode onboarding fallback enabled: continuing without local Rust/MSVC runtime build."
      Write-Host "[infring install] onboarding fallback reason: $($script:InstallBootstrapOnlyReason)"
      Write-Host "[infring install] onboarding fallback note: `infring`, `infringctl`, and `infring gateway` will run in bootstrap-only mode until runtime binaries are installed."
    } else {
      $failureHint = Format-BinaryInstallFailureHint -Stem "infring-ops" -Triple $triple -VersionTag $version
      $windowsToolsHint = if ($HostIsWindows) { (Get-WindowsBuildToolsInstallHint) } else { "" }
      throw "Failed to install core ops runtime for $triple ($resolvedVersionLabel). Prebuilt asset download failed and source fallback did not complete. Diagnostic: $failureHint Install Rust toolchain + C++ build tools, then rerun the README Windows install command: $ReadmeWindowsInstallCommand $windowsToolsHint"
    }
  }
}

$daemonMode = "spine"
if ([bool]$script:InstallBootstrapOnlyMode) {
  $daemonMode = "bootstrap"
  Write-Host "[infring install] onboarding fallback: using bootstrap-only gateway shim (runtime binaries unavailable)."
} elseif ($InstallTinyMax -and (Install-Binary $version $preferredDaemonTriple "infringd-tiny-max" $infringdBin)) {
  $daemonMode = "infringd"
  Write-Host "[infring install] using tiny-max daemon runtime"
} elseif (Install-Binary $version $preferredDaemonTriple "infringd" $infringdBin) {
  $daemonMode = "infringd"
  if ($preferredDaemonTriple -eq "x86_64-unknown-linux-musl") {
    Write-Host "[infring install] using static musl daemon runtime (embedded-minimal-core)"
  } else {
    Write-Host "[infring install] using daemon runtime"
  }
} elseif ($preferredDaemonTriple -ne $triple -and (Install-Binary $version $triple "infringd" $infringdBin)) {
  $daemonMode = "infringd"
  Write-Host "[infring install] using native daemon runtime fallback"
} elseif (Install-Binary $version $triple "conduit_daemon" $daemonBin) {
  $daemonMode = "conduit"
  Write-Host "[infring install] using conduit_daemon compatibility fallback"
} else {
  Write-Host "[infring install] no dedicated daemon binary found; falling back to spine mode via core ops runtime"
}

$wrapperPrelude = @'
@echo off
setlocal EnableExtensions EnableDelayedExpansion
set "_infring_root="
if defined INFRING_WORKSPACE_ROOT call :_check_candidate "%INFRING_WORKSPACE_ROOT%"
if not defined _infring_root call :_search_up "%CD%"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring\workspace"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring\workspace"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring"
if not defined _infring_root call :_check_candidate "%USERPROFILE%\.infring"
if defined _infring_root (
  set "INFRING_WORKSPACE_ROOT=%_infring_root%"
  cd /d "%_infring_root%" >nul 2>&1
)
goto :_dispatch

:_check_candidate
set "_candidate=%~1"
if "%_candidate%"=="" goto :eof
if exist "%_candidate%\core\layer0\ops\Cargo.toml" if exist "%_candidate%\client\runtime" set "_infring_root=%_candidate%"
goto :eof

:_search_up
set "_probe=%~1"
:_search_up_loop
if "!_probe!"=="" goto :eof
call :_check_candidate "!_probe!"
if defined _infring_root goto :eof
for %%I in ("!_probe!") do set "_parent=%%~dpI"
if not defined _parent goto :eof
if "!_parent:~-1!"=="\" set "_parent=!_parent:~0,-1!"
if /I "!_parent!"=="!_probe!" goto :eof
set "_probe=!_parent!"
goto :_search_up_loop
'@

$gatewayDispatchTemplate = @'
:_dispatch
if /I "%~1"=="recover" (
  shift
  call :_recover_dispatch %*
  set "_recover_rc=!ERRORLEVEL!"
  exit /b !_recover_rc!
)
if /I "%~1"=="gateway" (
  shift
  call :_gateway_dispatch %*
  set "_gateway_rc=!ERRORLEVEL!"
  exit /b !_gateway_rc!
)
if not exist "%~dp0infring-ops.exe" if not exist "%~dp0infring-pure-workspace.exe" (
  call :_bootstrap_dispatch %*
  set "_bootstrap_rc=!ERRORLEVEL!"
  exit /b !_bootstrap_rc!
)
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!

:_bootstrap_help
echo [infring bootstrap] full-mode onboarding fallback active (runtime binaries unavailable).
echo [infring bootstrap] install Visual Studio Build Tools, then rerun install.ps1 -Repair -Full.
echo [infring bootstrap] available commands now: infring --help, infring status, infring setup status --json, infring gateway status
exit /b 0

:_bootstrap_dispatch
set "_bootstrap_cmd=%~1"
if "%_bootstrap_cmd%"=="" goto :_bootstrap_help
if /I "%_bootstrap_cmd%"=="--help" goto :_bootstrap_help
if /I "%_bootstrap_cmd%"=="-h" goto :_bootstrap_help
if /I "%_bootstrap_cmd%"=="help" goto :_bootstrap_help
if /I "%_bootstrap_cmd%"=="status" (
  echo [infring bootstrap] runtime binaries unavailable; onboarding fallback active.
  exit /b 0
)
if /I "%_bootstrap_cmd%"=="setup" (
  if /I "%~2"=="status" (
    echo {"ok":true,"type":"infring_setup_status","mode":"bootstrap_only","runtime_installed":false,"next_action":"install_msvc_and_rerun_repair_full"}
    exit /b 0
  )
  echo [infring bootstrap] setup accepted in bootstrap-only mode.
  exit /b 0
)
echo [infring bootstrap] command requires runtime binaries: %_bootstrap_cmd%
echo [infring bootstrap] run install.ps1 -Repair -Full after installing Visual Studio Build Tools.
exit /b 0

:_recover_usage
echo Usage: infring recover [--dashboard-host=127.0.0.1] [--dashboard-port=4173] [--wait-max=90]
exit /b 0

:_recover_dispatch
set "_recover_host=127.0.0.1"
set "_recover_port=4173"
set "_recover_wait=90"
:_recover_parse
if "%~1"=="" goto :_recover_run
if /I "%~1"=="--help" goto :_recover_usage
if /I "%~1"=="-h" goto :_recover_usage
if /I "%~1"=="help" goto :_recover_usage
for /f "tokens=1,* delims==" %%A in ("%~1") do (
  if /I "%%~A"=="--dashboard-host" set "_recover_host=%%~B"
  if /I "%%~A"=="--dashboard-port" set "_recover_port=%%~B"
  if /I "%%~A"=="--wait-max" set "_recover_wait=%%~B"
)
shift
goto :_recover_parse

:_recover_run
echo [infring recover] stopping runtime
call :_gateway_dispatch stop --dashboard-host=!_recover_host! --dashboard-port=!_recover_port! --dashboard-open=0 >nul 2>&1
echo [infring recover] starting runtime
call :_gateway_dispatch start --dashboard-host=!_recover_host! --dashboard-port=!_recover_port! --dashboard-open=0
if not "!ERRORLEVEL!"=="0" (
  echo [infring recover] gateway start failed 1>&2
  exit /b 1
)
echo [infring recover] checking runtime status
call :_gateway_dispatch status --dashboard-host=!_recover_host! --dashboard-port=!_recover_port! --dashboard-open=0 >nul 2>&1
call "%~dp0infringctl.cmd" verify-install --json >nul 2>&1
if not "!ERRORLEVEL!"=="0" (
  echo [infring recover] verify-install failed 1>&2
  exit /b 1
)
echo [infring recover] complete
exit /b 0

:_gateway_usage
echo Usage: infring gateway [start^|stop^|restart^|status^|attach^|subscribe^|tick^|diagnostics] [flags]
echo   default action is 'start'
echo   add --dashboard-open=0 to skip browser auto-open on start
exit /b 0

:_gateway_dispatch
set "_gateway_arg1=%~1"
set "_gateway_action="
set "_gateway_shift=0"
if "%_gateway_arg1%"=="" set "_gateway_action=start"
if /I "%_gateway_arg1%"=="start" set "_gateway_action=start" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="boot" set "_gateway_action=start" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="stop" set "_gateway_action=stop" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="restart" set "_gateway_action=restart" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="status" set "_gateway_action=status" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="attach" set "_gateway_action=attach" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="subscribe" set "_gateway_action=subscribe" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="tick" set "_gateway_action=tick" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="diagnostics" set "_gateway_action=diagnostics" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="efficiency-status" set "_gateway_action=efficiency-status" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="embedded-core-status" set "_gateway_action=embedded-core-status" & set "_gateway_shift=1"
if /I "%_gateway_arg1%"=="--help" goto :_gateway_usage
if /I "%_gateway_arg1%"=="-h" goto :_gateway_usage
if /I "%_gateway_arg1%"=="help" goto :_gateway_usage
if not defined _gateway_action set "_gateway_action=start"
if "!_gateway_shift!"=="1" shift

set "_gateway_tmp=%TEMP%\infring-gateway-%RANDOM%-%RANDOM%.log"
call "%~dp0infringd.cmd" "!_gateway_action!" %* > "!_gateway_tmp!" 2>&1
set "_gateway_status=!ERRORLEVEL!"
if not "!_gateway_status!"=="0" (
  if exist "!_gateway_tmp!" type "!_gateway_tmp!" 1>&2
  echo [infring gateway] !_gateway_action! failed 1>&2
  if exist "!_gateway_tmp!" del /q "!_gateway_tmp!" >nul 2>&1
  exit /b !_gateway_status!
)

set "_gateway_raw=0"
if /I "%INFRING_GATEWAY_RAW%"=="1" set "_gateway_raw=1"
if "!_gateway_raw!"=="1" if exist "!_gateway_tmp!" type "!_gateway_tmp!"

if /I "!_gateway_action!"=="start" (
  set "_dashboard_url=%INFRING_DASHBOARD_URL%"
  if "!_dashboard_url!"=="" set "_dashboard_url=http://127.0.0.1:4173/dashboard#chat"
  set "_dashboard_open=1"
  if /I "%INFRING_NO_BROWSER%"=="1" set "_dashboard_open=0"
  for %%A in (%*) do (
    if /I "%%~A"=="--dashboard-open=0" set "_dashboard_open=0"
    if /I "%%~A"=="--dashboard-open=1" set "_dashboard_open=1"
    if /I "%%~A"=="--no-browser" set "_dashboard_open=0"
  )
  if "!_dashboard_open!"=="1" start "" "!_dashboard_url!" >nul 2>&1
  echo P o w e r  T o  T h e  U s e r s
  echo [infring gateway] runtime started
  echo [infring gateway] dashboard: !_dashboard_url!
  if defined INFRING_WORKSPACE_ROOT echo [infring gateway] workspace: !INFRING_WORKSPACE_ROOT!
) else if /I "!_gateway_action!"=="stop" (
  echo [infring gateway] runtime stopped
) else if /I "!_gateway_action!"=="status" (
  echo [infring gateway] runtime status received
  if defined INFRING_WORKSPACE_ROOT echo [infring gateway] workspace: !INFRING_WORKSPACE_ROOT!
) else if /I "!_gateway_action!"=="restart" (
  echo P o w e r  T o  T h e  U s e r s
  echo [infring gateway] runtime restarted
) else (
  echo [infring gateway] action complete: !_gateway_action!
)
if exist "!_gateway_tmp!" del /q "!_gateway_tmp!" >nul 2>&1
exit /b 0
'@

$plainDispatchTemplate = @'
:_dispatch
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!
'@

$daemonCompatDispatchTemplate = @'
:_dispatch
set "_daemon_cmd=%~1"
if /I "%_daemon_cmd%"=="start" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="stop" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="restart" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="status" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="attach" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="subscribe" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="tick" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="diagnostics" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="efficiency-status" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="embedded-core-status" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="watchdog" goto :_daemon_control_dispatch
if /I "%_daemon_cmd%"=="daemon-control" goto :_compat_dispatch
if /I "%_daemon_cmd%"=="dashboard-ui" goto :_compat_dispatch
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!

:_daemon_control_dispatch
if exist "%~dp0infring-ops.exe" (
  set "_needs_node_hint=0"
  if /I "%_daemon_cmd%"=="start" set "_needs_node_hint=1"
  if /I "%_daemon_cmd%"=="restart" set "_needs_node_hint=1"
  if /I "%_daemon_cmd%"=="watchdog" set "_needs_node_hint=1"
  if "!_needs_node_hint!"=="1" (
    set "_has_node_flag=0"
    for %%A in (%*) do (
      set "_arg=%%~A"
      if /I "!_arg:~0,14!"=="--node-binary=" set "_has_node_flag=1"
    )
    if "!_has_node_flag!"=="0" (
      set "_node_hint=!INFRING_NODE_BINARY!"
      if not defined _node_hint for %%N in (node.exe) do set "_node_hint=%%~$PATH:N"
      if defined _node_hint (
        call "%~dp0infring-ops.exe" daemon-control %* "--node-binary=!_node_hint!"
        set "_cmd_rc=!ERRORLEVEL!"
        exit /b !_cmd_rc!
      )
    )
  )
  call "%~dp0infring-ops.exe" daemon-control %*
  set "_cmd_rc=!ERRORLEVEL!"
  exit /b !_cmd_rc!
)
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!

:_compat_dispatch
set "_ops_domain=%INFRING_OPS_DOMAIN%"
if not defined _ops_domain set "_ops_domain=infringctl"
if exist "%~dp0infring-ops.exe" (
  call "%~dp0infring-ops.exe" "!_ops_domain!" %*
  set "_cmd_rc=!ERRORLEVEL!"
  exit /b !_cmd_rc!
)
call __ENTRY__ __ENTRY_ARGS__ %*
set "_cmd_rc=!ERRORLEVEL!"
exit /b !_cmd_rc!
'@

$bootstrapGatewayDispatchTemplate = @'
:_dispatch
set "_bootstrap_action=%~1"
if "%_bootstrap_action%"=="" set "_bootstrap_action=start"
if /I "%_bootstrap_action%"=="--help" goto :_bootstrap_usage
if /I "%_bootstrap_action%"=="-h" goto :_bootstrap_usage
if /I "%_bootstrap_action%"=="help" goto :_bootstrap_usage
if /I "%_bootstrap_action%"=="start" (
  echo P o w e r  T o  T h e  U s e r s
  echo [infring gateway] bootstrap-only mode active
  echo [infring gateway] runtime binaries are not installed on this machine yet
  echo [infring gateway] next: install Visual Studio Build Tools, then run install.ps1 -Repair -Full
  exit /b 0
)
if /I "%_bootstrap_action%"=="restart" (
  echo P o w e r  T o  T h e  U s e r s
  echo [infring gateway] bootstrap-only mode active
  echo [infring gateway] runtime restart deferred until runtime binaries are installed
  exit /b 0
)
if /I "%_bootstrap_action%"=="status" (
  echo [infring gateway] bootstrap-only mode active (runtime not installed)
  exit /b 0
)
if /I "%_bootstrap_action%"=="stop" (
  echo [infring gateway] bootstrap-only mode active; nothing to stop
  exit /b 0
)
echo [infring gateway] bootstrap-only mode action complete: %_bootstrap_action%
exit /b 0

:_bootstrap_usage
echo Usage: infringd [start^|stop^|restart^|status]
echo [infring gateway] bootstrap-only mode active (runtime binaries unavailable)
exit /b 0
'@

function Write-CmdWrapper {
  param(
    [string]$Path,
    [string]$Entry,
    [string]$EntryArgs,
    [switch]$Gateway
  )

  $dispatch = if ($Gateway) { $gatewayDispatchTemplate } else { $plainDispatchTemplate }
  $dispatch = $dispatch.Replace("__ENTRY__", $Entry)
  if ([string]::IsNullOrWhiteSpace($EntryArgs)) {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", "")
  } else {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", $EntryArgs)
  }

  $content = $wrapperPrelude + "`r`n" + $dispatch + "`r`n"
  Set-Content -Path $Path -Value $content -Encoding ASCII
}

function Write-DaemonCmdWrapper {
  param(
    [string]$Path,
    [string]$Entry,
    [string]$EntryArgs
  )

  $dispatch = $daemonCompatDispatchTemplate.Replace("__ENTRY__", $Entry)
  if ([string]::IsNullOrWhiteSpace($EntryArgs)) {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", "")
  } else {
    $dispatch = $dispatch.Replace("__ENTRY_ARGS__", $EntryArgs)
  }

  $content = $wrapperPrelude + "`r`n" + $dispatch + "`r`n"
  Set-Content -Path $Path -Value $content -Encoding ASCII
}

function Write-BootstrapGatewayCmdWrapper {
  param(
    [string]$Path
  )

  $content = $wrapperPrelude + "`r`n" + $bootstrapGatewayDispatchTemplate + "`r`n"
  Set-Content -Path $Path -Value $content -Encoding ASCII
}

function Resolve-WorkspaceRootForSmoke {
  return Resolve-WorkspaceRootForRepair
}

function Show-DashboardFailureLogs {
  param(
    [string]$WorkspaceRoot
  )

  $root = if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    Resolve-WorkspaceRootForSmoke
  } else {
    $WorkspaceRoot
  }
  if ([string]::IsNullOrWhiteSpace($root)) {
    return
  }
  $stateDir = Join-Path $root "local/state/ops/daemon_control"
  foreach ($name in @("dashboard_ui.log", "dashboard_watchdog.log")) {
    $path = Join-Path $stateDir $name
    if (-not (Test-Path $path)) { continue }
    Write-Host "[infring install] tail $path"
    Get-Content -Path $path -Tail 80 -ErrorAction SilentlyContinue
  }
}

function Test-DashboardHealthSmoke {
  param(
    [string]$InstallDir,
    [string]$DashboardHost = "127.0.0.1",
    [int]$Port = 4173
  )

  $workspaceRoot = Resolve-WorkspaceRootForSmoke
  $healthLog = New-InstallSmokeLogPath -CheckName "dashboard_healthz"

  $null = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "stop", "--dashboard-host=$DashboardHost", "--dashboard-port=$Port", "--dashboard-open=0") -TimeoutSec 20

  $startResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "start", "--dashboard-host=$DashboardHost", "--dashboard-port=$Port", "--dashboard-open=0", "--gateway-persist=0") -TimeoutSec 45 -LogPath $healthLog
  if (-not [bool]$startResult.Ok) {
    $errorCode = "gateway_start_failed"
    if ([bool]$startResult.TimedOut) {
      $errorCode = "gateway_start_timeout"
      Write-Host "[infring install] smoke dashboard_health: failed (gateway start timeout)"
    } else {
      Write-Host "[infring install] smoke dashboard_health: failed (gateway start)"
    }
    if ([bool]$startResult.LogPath -and (Test-Path $startResult.LogPath)) {
      Get-Content -Path $startResult.LogPath -Tail 120 -ErrorAction SilentlyContinue
    }
    if ([bool]$startResult.ErrPath -and (Test-Path $startResult.ErrPath)) {
      Get-Content -Path $startResult.ErrPath -Tail 120 -ErrorAction SilentlyContinue
    }
    Show-DashboardFailureLogs -WorkspaceRoot $workspaceRoot
    return @{
      Ok = $false
      ExitCode = if ($null -ne $startResult.ExitCode) { $startResult.ExitCode } else { 1 }
      TimedOut = [bool]$startResult.TimedOut
      Error = $errorCode
      LogPath = $healthLog
      ErrPath = $startResult.ErrPath
      HealthzUrl = "http://$DashboardHost`:$Port/healthz"
    }
  }

  $ready = $false
  for ($i = 0; $i -lt 45; $i++) {
    try {
      Invoke-WebRequest -Uri "http://$DashboardHost`:$Port/healthz" -UseBasicParsing -TimeoutSec 2 | Out-Null
      $ready = $true
      break
    } catch {}
    Start-Sleep -Seconds 1
  }

  $null = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -Arguments @("gateway", "stop", "--dashboard-host=$DashboardHost", "--dashboard-port=$Port", "--dashboard-open=0") -TimeoutSec 20

  if (-not $ready) {
    Write-Host "[infring install] smoke dashboard_health: failed (healthz timeout)"
    if (Test-Path $healthLog) { Get-Content -Path $healthLog -Tail 120 -ErrorAction SilentlyContinue }
    Show-DashboardFailureLogs -WorkspaceRoot $workspaceRoot
    return @{
      Ok = $false
      ExitCode = 1
      TimedOut = $false
      Error = "healthz_timeout"
      LogPath = $healthLog
      ErrPath = $startResult.ErrPath
      HealthzUrl = "http://$DashboardHost`:$Port/healthz"
    }
  }

  Write-Host "[infring install] smoke dashboard_health: ok"
  return @{
    Ok = $true
    ExitCode = 0
    TimedOut = $false
    Error = $null
    LogPath = $healthLog
    ErrPath = $startResult.ErrPath
    HealthzUrl = "http://$DashboardHost`:$Port/healthz"
  }
}

function Invoke-InfringCmdWithTimeout {
  param(
    [string]$InstallDir,
    [string[]]$Arguments,
    [string]$WrapperName = "infring",
    [int]$TimeoutSec = 25,
    [string]$LogPath
  )

  $wrapper = if ([string]::IsNullOrWhiteSpace($WrapperName)) { "infring" } else { $WrapperName }
  $cmdPath = Join-Path $InstallDir "$wrapper.cmd"
  $psPath = Join-Path $InstallDir "$wrapper.ps1"
  $launcher = "cmd"
  if (-not (Test-Path $cmdPath)) {
    if (Test-Path $psPath) {
      $launcher = "powershell"
    } else {
      return @{
        Ok = $false
        ExitCode = 1
        TimedOut = $false
        Error = "missing_${wrapper}_cmd"
        LogPath = $null
        ErrPath = $null
      }
    }
  }

  if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $LogPath = Join-Path ([System.IO.Path]::GetTempPath()) ("infring-install-smoke-" + [guid]::NewGuid().ToString("N") + ".log")
  }
  $errPath = "$LogPath.err"

  $quotedArgs = @()
  foreach ($arg in $Arguments) {
    $escaped = [string]$arg
    $escaped = $escaped.Replace('"', '""')
    $quotedArgs += "`"$escaped`""
  }
  try {
    if ($launcher -eq "powershell") {
      $proc = Start-Process -FilePath "powershell.exe" -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $psPath) + $Arguments -PassThru -WindowStyle Hidden -RedirectStandardOutput $LogPath -RedirectStandardError $errPath
    } else {
      $commandLine = "call `"$cmdPath`""
      if ($quotedArgs.Count -gt 0) {
        $commandLine = "$commandLine " + ($quotedArgs -join " ")
      }
      $proc = Start-Process -FilePath "cmd.exe" -ArgumentList @("/d", "/s", "/c", $commandLine) -PassThru -WindowStyle Hidden -RedirectStandardOutput $LogPath -RedirectStandardError $errPath
    }
  } catch {
    return @{
      Ok = $false
      ExitCode = 1
      TimedOut = $false
      Error = $_.Exception.Message
      LogPath = $LogPath
      ErrPath = $errPath
    }
  }

  $finished = $proc.WaitForExit($TimeoutSec * 1000)
  if (-not $finished) {
    try { $proc.Kill() } catch {}
    return @{
      Ok = $false
      ExitCode = $null
      TimedOut = $true
      Error = "timeout_${TimeoutSec}s"
      LogPath = $LogPath
      ErrPath = $errPath
    }
  }

  return @{
    Ok = ($proc.ExitCode -eq 0)
    ExitCode = $proc.ExitCode
    TimedOut = $false
    Error = $null
    LogPath = $LogPath
    ErrPath = $errPath
  }
}

function New-InstallSmokeLogPath {
  param(
    [string]$CheckName
  )

  $normalized = if ([string]::IsNullOrWhiteSpace($CheckName)) { "check" } else { $CheckName }
  $safe = ($normalized -replace '[^a-zA-Z0-9_\-]', '_').ToLowerInvariant()
  $dir = Join-Path $HOME ".infring\logs\install-smoke"
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  return Join-Path $dir ("$safe-" + [guid]::NewGuid().ToString("N") + ".log")
}

function New-InstallerSmokeCheckRecord {
  param(
    [string]$CheckName,
    [string]$Command,
    [hashtable]$Result,
    [bool]$Required = $true
  )

  $ok = [bool]$Result.Ok
  $errorCode = ""
  if ($ok) {
    $errorCode = ""
  } elseif ([bool]$Result.TimedOut) {
    $errorCode = "timeout"
  } elseif ($null -ne $Result.ExitCode) {
    $errorCode = "exit_code_$($Result.ExitCode)"
  } elseif (-not [string]::IsNullOrWhiteSpace([string]$Result.Error)) {
    $errorCode = [string]$Result.Error
  } else {
    $errorCode = "unknown"
  }
  return @{
    name = $CheckName
    command = $Command
    required = [bool]$Required
    ok = $ok
    status = if ($ok) { "passed" } else { "failed" }
    error_code = $errorCode
    timed_out = [bool]$Result.TimedOut
    exit_code = $Result.ExitCode
    log_path = $Result.LogPath
    err_path = $Result.ErrPath
  }
}

function Write-InstallerSmokeResult {
  param(
    [hashtable]$Record
  )

  $name = [string]$Record.name
  $status = [string]$Record.status
  if ($status -like "skipped*") {
    Write-Host "[infring install] smoke ${name}: skipped"
    return
  }
  if ([bool]$Record.ok) {
    Write-Host "[infring install] smoke ${name}: ok"
    return
  }
  $err = [string]$Record.error_code
  if ([string]::IsNullOrWhiteSpace($err)) { $err = "unknown" }
  Write-Host "[infring install] smoke ${name}: failed ($err)"
  if ([bool]$Record.log_path -and (Test-Path $Record.log_path)) {
    Get-Content -Path $Record.log_path -Tail 80 -ErrorAction SilentlyContinue
  }
  if ([bool]$Record.err_path -and (Test-Path $Record.err_path)) {
    Get-Content -Path $Record.err_path -Tail 80 -ErrorAction SilentlyContinue
  }
}

function Test-RustupDefaultToolchainMissing {
  $rustup = Get-Command rustup -ErrorAction SilentlyContinue
  if ($null -eq $rustup) { return $false }
  $cargo = Get-Command cargo -ErrorAction SilentlyContinue
  if ($null -ne $cargo) {
    try {
      & $cargo.Source --version *> $null
      if ($LASTEXITCODE -eq 0) { return $false }
    } catch {}
    return $true
  }
  try {
    & $rustup.Source default *> $null
    if ($LASTEXITCODE -eq 0) { return $false }
  } catch {}
  return $true
}

$powerShellShimTemplate = @'
param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$CommandArgs
)
$target = Join-Path $PSScriptRoot "__TARGET__"
$shimName = [System.IO.Path]::GetFileNameWithoutExtension($PSCommandPath).ToLowerInvariant()
$opsExe = Join-Path $PSScriptRoot "infring-ops.exe"
$daemonExe = Join-Path $PSScriptRoot "infringd.exe"
$conduitExe = Join-Path $PSScriptRoot "conduit_daemon.exe"
if (-not (Test-Path $target)) {
  $fallbackInvoked = $false
  $gatewayAction = "start"
  $gatewayArgs = @()
  if ($shimName -eq "infring" -and $CommandArgs.Count -gt 0) {
    $first = [string]$CommandArgs[0]
    if ($first.ToLowerInvariant() -eq "gateway") {
      if ($CommandArgs.Count -gt 1) {
        $candidate = [string]$CommandArgs[1]
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and -not $candidate.StartsWith("-")) {
          $gatewayAction = $candidate
          if ($CommandArgs.Count -gt 2) {
            $gatewayArgs = $CommandArgs | Select-Object -Skip 2
          }
        } else {
          $gatewayArgs = $CommandArgs | Select-Object -Skip 1
        }
      }
      if (Test-Path $daemonExe) {
        Write-Warning "[infring shim] missing wrapper $target; using infringd.exe gateway fallback."
        & $daemonExe $gatewayAction @gatewayArgs
        $fallbackInvoked = $true
      } elseif (Test-Path $conduitExe) {
        Write-Warning "[infring shim] missing wrapper $target; using conduit_daemon.exe gateway fallback."
        & $conduitExe $gatewayAction @gatewayArgs
        $fallbackInvoked = $true
      }
    }
  }
  if ($shimName -eq "infring" -and (Test-Path $opsExe)) {
    Write-Warning "[infring shim] missing wrapper $target; using infring-ops.exe infringctl fallback."
    & $opsExe "infringctl" @CommandArgs
    $fallbackInvoked = $true
  } elseif ($shimName -eq "infringctl" -and (Test-Path $opsExe)) {
    Write-Warning "[infring shim] missing wrapper $target; using infring-ops.exe infringctl fallback."
    & $opsExe "infringctl" @CommandArgs
    $fallbackInvoked = $true
  } elseif ($shimName -eq "infringd") {
    if (Test-Path $daemonExe) {
      Write-Warning "[infring shim] missing wrapper $target; using infringd.exe fallback."
      & $daemonExe @CommandArgs
      $fallbackInvoked = $true
    } elseif (Test-Path $conduitExe) {
      Write-Warning "[infring shim] missing wrapper $target; using conduit_daemon.exe fallback."
      & $conduitExe @CommandArgs
      $fallbackInvoked = $true
    } elseif (Test-Path $opsExe) {
      Write-Warning "[infring shim] missing wrapper $target; using infring-ops.exe spine fallback."
      & $opsExe "spine" @CommandArgs
      $fallbackInvoked = $true
    }
  }
  if (-not $fallbackInvoked -and $shimName -eq "infring") {
    $bootstrapCmd = ""
    if ($CommandArgs.Count -gt 0) {
      $bootstrapCmd = [string]$CommandArgs[0]
    }
    $bootstrapCmdLower = $bootstrapCmd.ToLowerInvariant()
    if ([string]::IsNullOrWhiteSpace($bootstrapCmdLower) -or $bootstrapCmdLower -eq "--help" -or $bootstrapCmdLower -eq "-h" -or $bootstrapCmdLower -eq "help") {
      Write-Warning "[infring shim] missing wrapper $target; using bootstrap-only fallback."
      Write-Host "[infring bootstrap] full-mode onboarding fallback active (runtime binaries unavailable)."
      Write-Host "[infring bootstrap] install Visual Studio Build Tools, then rerun install.ps1 -Repair -Full."
      Write-Host "[infring bootstrap] available commands now: infring --help, infring status, infring setup status --json, infring gateway status"
      exit 0
    }
    if ($bootstrapCmdLower -eq "status") {
      Write-Warning "[infring shim] missing wrapper $target; using bootstrap-only fallback."
      Write-Host "[infring bootstrap] runtime binaries unavailable; onboarding fallback active."
      exit 0
    }
    if ($bootstrapCmdLower -eq "setup") {
      $setupSubcmd = ""
      if ($CommandArgs.Count -gt 1) {
        $setupSubcmd = [string]$CommandArgs[1]
      }
      if ($setupSubcmd.ToLowerInvariant() -eq "status") {
        Write-Warning "[infring shim] missing wrapper $target; using bootstrap-only fallback."
        Write-Output '{"ok":true,"type":"infring_setup_status","mode":"bootstrap_only","runtime_installed":false,"next_action":"install_msvc_and_rerun_repair_full"}'
        exit 0
      }
      Write-Warning "[infring shim] missing wrapper $target; using bootstrap-only fallback."
      Write-Host "[infring bootstrap] setup accepted in bootstrap-only mode."
      exit 0
    }
    if ($bootstrapCmdLower -eq "gateway") {
      $gatewaySubcmd = "start"
      if ($CommandArgs.Count -gt 1) {
        $candidate = [string]$CommandArgs[1]
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and -not $candidate.StartsWith("-")) {
          $gatewaySubcmd = $candidate.ToLowerInvariant()
        }
      }
      Write-Warning "[infring shim] missing wrapper $target; using bootstrap-only gateway fallback."
      if ($gatewaySubcmd -eq "start") {
        Write-Host "[infring gateway] bootstrap-only mode active"
        Write-Host "[infring gateway] runtime binaries are not installed on this machine yet"
        Write-Host "[infring gateway] next: install Visual Studio Build Tools, then run install.ps1 -Repair -Full"
        exit 0
      }
      if ($gatewaySubcmd -eq "restart") {
        Write-Host "[infring gateway] bootstrap-only mode active"
        Write-Host "[infring gateway] runtime restart deferred until runtime binaries are installed"
        exit 0
      }
      if ($gatewaySubcmd -eq "status") {
        Write-Host "[infring gateway] bootstrap-only mode active (runtime not installed)"
        exit 0
      }
      if ($gatewaySubcmd -eq "stop") {
        Write-Host "[infring gateway] bootstrap-only mode active; nothing to stop"
        exit 0
      }
      Write-Host "[infring gateway] bootstrap-only mode action complete: $gatewaySubcmd"
      exit 0
    }
  }
  if ($fallbackInvoked) {
    exit $LASTEXITCODE
  }
  Write-Warning "[infring shim] deterministic recovery: infring setup status --json"
  Write-Warning "[infring shim] deterministic recovery: infring doctor --json"
  Write-Warning "[infring shim] deterministic recovery: rerun install.ps1 with -Repair -Full"
  Write-Host "[infring bootstrap] runtime binaries are not installed on this machine yet."
  Write-Host "[infring bootstrap] run: install.ps1 -Repair -Full"
  exit 0
}
__DEPRECATION__
& $target @CommandArgs
exit $LASTEXITCODE
'@

function Write-PowerShellShim {
  param(
    [string]$Path,
    [string]$TargetCmd,
    [string]$DeprecationMessage
  )

  $content = $powerShellShimTemplate.Replace("__TARGET__", $TargetCmd)
  $deprecationLine = ""
  if (-not [string]::IsNullOrWhiteSpace($DeprecationMessage)) {
    $deprecationEscaped = $DeprecationMessage.Replace('"', '""')
    $deprecationLine = "Write-Warning `"$deprecationEscaped`""
  }
  $content = $content.Replace("__DEPRECATION__", $deprecationLine)
  Set-Content -Path $Path -Value $content -Encoding UTF8
}

$infringCmd = Join-Path $InstallDir "infring.cmd"
$infringctlCmd = Join-Path $InstallDir "infringctl.cmd"
$infringdCmd = Join-Path $InstallDir "infringd.cmd"

if ($InstallPure) {
  if ($InstallTinyMax) {
    Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0infring-pure-workspace.exe"' -EntryArgs '--tiny-max=1' -Gateway
  } else {
    Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0infring-pure-workspace.exe"' -EntryArgs '' -Gateway
  }
  Write-CmdWrapper -Path $infringctlCmd -Entry '"%~dp0infring-pure-workspace.exe"' -EntryArgs 'conduit' -Gateway
} else {
  Write-CmdWrapper -Path $infringCmd -Entry '"%~dp0infring-ops.exe"' -EntryArgs 'infringctl' -Gateway
  Write-CmdWrapper -Path $infringctlCmd -Entry '"%~dp0infring-ops.exe"' -EntryArgs 'infringctl' -Gateway
}

if ($daemonMode -eq "infringd") {
  Write-DaemonCmdWrapper -Path $infringdCmd -Entry '"%~dp0infringd.exe"' -EntryArgs ''
} elseif ($daemonMode -eq "conduit") {
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0conduit_daemon.exe"' -EntryArgs ''
} elseif ($daemonMode -eq "bootstrap") {
  Write-BootstrapGatewayCmdWrapper -Path $infringdCmd
} else {
  if ($InstallPure) {
    throw "No daemon binary available for pure mode"
  }
  Write-CmdWrapper -Path $infringdCmd -Entry '"%~dp0infring-ops.exe"' -EntryArgs 'spine'
}

$infringPs1 = Join-Path $InstallDir "infring.ps1"
$infringctlPs1 = Join-Path $InstallDir "infringctl.ps1"
$infringdPs1 = Join-Path $InstallDir "infringd.ps1"

Write-PowerShellShim -Path $infringPs1 -TargetCmd "infring.cmd" -DeprecationMessage "PowerShell shim is compatibility-only; prefer infring.cmd on locked-down Windows hosts."
Write-PowerShellShim -Path $infringctlPs1 -TargetCmd "infringctl.cmd" -DeprecationMessage "PowerShell shim is compatibility-only; prefer infringctl.cmd on locked-down Windows hosts."
Write-PowerShellShim -Path $infringdPs1 -TargetCmd "infringd.cmd" -DeprecationMessage "PowerShell shim is compatibility-only; prefer infringd.cmd on locked-down Windows hosts."
Remove-StaleWindowsCommandShims -ShimInstallDir $InstallDir
Ensure-RepairBootstrapWrapperFloor -InstallDir $InstallDir

if ($InstallPure) {
  Write-Host "[infring install] pure mode: skipping Infring client bundle"
  $script:InstallClientRuntimeMode = "pure_profile"
  $script:InstallRuntimeContractStatus = "pure_profile"
} elseif ([bool]$script:InstallBootstrapOnlyMode) {
  Write-Host "[infring install] bootstrap-only mode: skipping client runtime bundle until runtime binaries are installed"
  $script:InstallClientRuntimeMode = "bootstrap_only_profile"
  $script:InstallRuntimeContractStatus = "bootstrap_only_profile"
} elseif ($InstallFull) {
  $clientDir = Join-Path $InstallDir "infring-client"
  if (Install-ClientBundle $version $triple $clientDir) {
    $script:InstallClientRuntimeMode = "dist"
    Write-Host "[infring install] full mode enabled: client runtime installed at $clientDir"
  } elseif (Install-ClientBundleFromSourceFallback $clientDir) {
    $script:InstallClientRuntimeMode = "source"
    Write-Host "[infring install] full mode enabled: client runtime installed from source fallback at $clientDir"
  } else {
    throw "Full mode requested but no client runtime bundle is available for $triple ($version), and source fallback runtime copy was unavailable."
  }
  if (-not (Test-InstallRuntimeManifestContract -RuntimeRoot $clientDir -RuntimeMode $script:InstallClientRuntimeMode -ContextLabel "install_dir_client_runtime")) {
    throw "Full mode runtime contract validation failed for $clientDir ($($script:InstallClientRuntimeMode))."
  }
  if (-not (Ensure-RuntimeNodeModuleClosure -RuntimeRoot $clientDir)) {
    throw "Full mode runtime node-module closure verification failed for $clientDir."
  }
  $script:InstallRuntimeContractStatus = "verified"
} else {
  Write-Host "[infring install] lazy mode: skipping TS systems/eyes client bundle (use -Full to include)"
  $script:InstallClientRuntimeMode = "minimal_profile"
  $script:InstallRuntimeContractStatus = "minimal_profile"
}

$workspaceRootForState = Resolve-WorkspaceRootForRepair
$workspaceRefreshDecision = Resolve-WorkspaceRuntimeRefreshDecision -WorkspaceRoot $workspaceRootForState -VersionTag $version -Repair ([bool]$InstallRepair)
$script:WorkspaceRuntimeRefreshRequired = [bool]$workspaceRefreshDecision.refresh_required
$script:WorkspaceRuntimeRefreshReason = [string]$workspaceRefreshDecision.reason
$script:WorkspaceRuntimeTagStateMissing = [bool]$workspaceRefreshDecision.tag_state_missing
$script:WorkspaceReleaseTagPrevious = [string]$workspaceRefreshDecision.previous_tag
$script:WorkspaceReleaseTagCurrent = [string]$version
$script:WorkspaceRuntimeRefreshApplied = $false
$script:WorkspaceReleaseTagWriteApplied = $false
$script:WorkspaceReleaseTagWriteVerified = $false
if ([bool]$script:WorkspaceRuntimeRefreshRequired) {
  Write-Host "[infring install] workspace runtime refresh required: $($workspaceRefreshDecision.reason)"
  $script:WorkspaceRuntimeRefreshApplied = [bool](Invoke-WorkspaceRuntimeRefresh -WorkspaceRoot $workspaceRootForState -InstallDir $InstallDir -SourceFallbackDir $script:SourceFallbackDir -Reason ([string]$workspaceRefreshDecision.reason))
}
if ([bool]$script:WorkspaceRuntimeRefreshRequired -and (-not [bool]$script:WorkspaceRuntimeRefreshApplied)) {
  throw "Workspace runtime refresh required but not applied (reason=$([string]$script:WorkspaceRuntimeRefreshReason)); refusing release-tag state update."
}
if ((-not $InstallPure) -and (-not [string]::IsNullOrWhiteSpace($workspaceRootForState))) {
  if (-not (Ensure-WorkspaceRuntimeContract -WorkspaceRoot $workspaceRootForState -InstallDir $InstallDir -SourceFallbackDir $script:SourceFallbackDir)) {
    throw "Workspace runtime contract validation failed after refresh/self-heal."
  }
}
if (-not [string]::IsNullOrWhiteSpace($workspaceRootForState)) {
  $script:WorkspaceReleaseTagWriteApplied = [bool](Write-WorkspaceRuntimeReleaseTagState -WorkspaceRoot $workspaceRootForState -VersionTag ([string]$version))
  if ([bool]$script:WorkspaceReleaseTagWriteApplied) {
    $script:WorkspaceReleaseTagWriteVerified = [bool](Assert-WorkspaceRuntimeReleaseTagState -WorkspaceRoot $workspaceRootForState -VersionTag ([string]$version))
    if (-not [bool]$script:WorkspaceReleaseTagWriteVerified) {
      throw "Workspace release tag state verification failed for $workspaceRootForState."
    }
    Write-Host "[infring install] workspace release tag state updated: $version"
  } else {
    throw "Workspace release tag state update failed for $workspaceRootForState."
  }
}

$machinePath = [Environment]::GetEnvironmentVariable("Path", "User")
$userPathResult = Ensure-WindowsPathContains $machinePath $InstallDir -PreferFront -RemoveEntries @($legacyInstallDir)
if ([bool]$userPathResult.Changed) {
  [Environment]::SetEnvironmentVariable("Path", [string]$userPathResult.Value, "User")
  if ([bool]$userPathResult.Added) {
    Write-Host "[infring install] added install dir to user PATH"
  } else {
    Write-Host "[infring install] normalized user PATH entries"
  }
}
$sessionPathResult = Ensure-WindowsPathContains $env:Path $InstallDir -PreferFront -RemoveEntries @($legacyInstallDir)
$env:Path = [string]$sessionPathResult.Value

$resolvedInfring = Get-Command infring -ErrorAction SilentlyContinue
if ($null -ne $resolvedInfring) {
  Write-Host "[infring install] shell command resolves to: $($resolvedInfring.Source)"
  $resolvedNorm = Normalize-WindowsPathEntry $resolvedInfring.Source
  $installNorm = Normalize-WindowsPathEntry $InstallDir
  if ($installNorm -and (-not $resolvedNorm.StartsWith($installNorm))) {
    Write-Host "[infring install] warning: current shell still prefers a non-canonical infring shim; use direct path fallback or start a new PowerShell session."
    Write-Host "[infring install] direct-path setup status: $InstallDir\\infring.cmd setup status --json"
    Write-Host "[infring install] direct-path gateway status: $InstallDir\\infring.cmd gateway status"
  }
} else {
  Write-Host "[infring install] warning: shell command resolution for 'infring' not ready in this session; use direct path fallback."
  Write-Host "[infring install] direct-path setup status: $InstallDir\\infring.cmd setup status --json"
  Write-Host "[infring install] direct-path gateway status: $InstallDir\\infring.cmd gateway status"
}

$smokeChecks = @()
$infringHelpResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -WrapperName "infring" -Arguments @("--help") -TimeoutSec 20 -LogPath (New-InstallSmokeLogPath -CheckName "infring_help")
$infringHelpCheck = New-InstallerSmokeCheckRecord -CheckName "infring_help" -Command "infring --help" -Result $infringHelpResult -Required $true
$smokeChecks += $infringHelpCheck
Write-InstallerSmokeResult -Record $infringHelpCheck

$infringctlHelpLogPath = New-InstallSmokeLogPath -CheckName "infringctl_help"
if (Test-RustupDefaultToolchainMissing) {
  if ($script:InstallToolchainPolicy -eq "fail_closed") {
    @(
      "failed (toolchain policy fail_closed): missing rustup default toolchain",
      "fix: run 'rustup default stable'"
    ) | Set-Content -Path $infringctlHelpLogPath -Encoding UTF8
    $infringctlHelpCheck = @{
      name = "infringctl_help"
      command = "infringctl --help"
      required = $true
      ok = $false
      status = "failed_policy_toolchain"
      error_code = "rustup_default_toolchain_missing"
      timed_out = $false
      exit_code = $null
      log_path = $infringctlHelpLogPath
      err_path = ""
    }
    Write-Host "[infring install] smoke infringctl_help: failed (toolchain policy fail_closed; missing rustup default toolchain)"
    Write-Host "[infring install] smoke infringctl_help: run 'rustup default stable' and rerun install."
  } else {
    @(
      "skipped (missing rustup default toolchain)",
      "help: run 'rustup default stable' to download the latest stable release of Rust and set it as your default toolchain."
    ) | Set-Content -Path $infringctlHelpLogPath -Encoding UTF8
    $infringctlHelpCheck = @{
      name = "infringctl_help"
      command = "infringctl --help"
      required = $false
      ok = $true
      status = "skipped_toolchain"
      error_code = ""
      timed_out = $false
      exit_code = 0
      log_path = $infringctlHelpLogPath
      err_path = ""
    }
    Write-Host "[infring install] smoke infringctl_help: skipped (missing rustup default toolchain; policy=auto)"
    Write-Host "[infring install] smoke infringctl_help: run 'rustup default stable' to enable this check."
  }
} else {
  $infringctlHelpResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -WrapperName "infringctl" -Arguments @("--help") -TimeoutSec 20 -LogPath $infringctlHelpLogPath
  $infringctlHelpCheck = New-InstallerSmokeCheckRecord -CheckName "infringctl_help" -Command "infringctl --help" -Result $infringctlHelpResult -Required $true
}
$smokeChecks += $infringctlHelpCheck
Write-InstallerSmokeResult -Record $infringctlHelpCheck

$infringStatusResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -WrapperName "infring" -Arguments @("status") -TimeoutSec 25 -LogPath (New-InstallSmokeLogPath -CheckName "infring_status")
$infringStatusCheck = New-InstallerSmokeCheckRecord -CheckName "infring_status" -Command "infring status" -Result $infringStatusResult -Required $true
$smokeChecks += $infringStatusCheck
Write-InstallerSmokeResult -Record $infringStatusCheck

$gatewaySmokeResult = Invoke-InfringCmdWithTimeout -InstallDir $InstallDir -WrapperName "infring" -Arguments @("gateway", "status", "--auto-heal=0", "--dashboard-open=0") -TimeoutSec 25 -LogPath (New-InstallSmokeLogPath -CheckName "gateway_status")
$gatewayStatusCheck = New-InstallerSmokeCheckRecord -CheckName "gateway_status" -Command "infring gateway status --auto-heal=0 --dashboard-open=0" -Result $gatewaySmokeResult -Required $true
$smokeChecks += $gatewayStatusCheck
Write-InstallerSmokeResult -Record $gatewayStatusCheck

$dashboardSmokeRequired = $InstallFull -and (-not [bool]$script:InstallBootstrapOnlyMode)
if ($InstallStrictSmoke) {
  $dashboardSmokeRequired = $true
}
if ($dashboardSmokeRequired) {
  $smokePort = 4400 + (Get-Random -Minimum 0 -Maximum 1000)
  $dashboardSmokeResult = Test-DashboardHealthSmoke -InstallDir $InstallDir -DashboardHost "127.0.0.1" -Port $smokePort
  $dashboardSmokeCheck = New-InstallerSmokeCheckRecord -CheckName "dashboard_healthz" -Command "GET http://127.0.0.1:$smokePort/healthz" -Result $dashboardSmokeResult -Required $true
  $smokeChecks += $dashboardSmokeCheck
  if (-not [bool]$dashboardSmokeCheck.ok) {
    throw "Full install failed dashboard health smoke."
  }
} else {
  Write-Host "[infring install] smoke dashboard_health: skipped (set INFRING_INSTALL_STRICT_SMOKE=1 or use -Full to enforce)"
  $dashboardSmokeCheck = @{
    name = "dashboard_healthz"
    command = "GET http://127.0.0.1:4173/healthz"
    required = $false
    ok = $true
    status = "skipped"
    error_code = ""
    timed_out = $false
    exit_code = 0
    log_path = ""
    err_path = ""
  }
  $smokeChecks += $dashboardSmokeCheck
}
$failedSmokeRequired = @($smokeChecks | Where-Object { [bool]$_.required -and -not [bool]$_.ok })
$gatewaySmokeOk = [bool]$gatewayStatusCheck.ok
$gatewaySmokeError = [string]$gatewayStatusCheck.error_code
$dashboardSmokeStatus = if ([string]$dashboardSmokeCheck.status -eq "skipped") { "skipped" } elseif ([bool]$dashboardSmokeCheck.ok) { "passed" } else { "failed:$([string]$dashboardSmokeCheck.error_code)" }
$gatewaySmokeStatus = if ($gatewaySmokeOk) { "passed" } else { "failed:$gatewaySmokeError" }
$runtimeContractMode = [string]$script:InstallClientRuntimeMode
$runtimeContractOk = @("verified", "pure_profile", "minimal_profile", "bootstrap_only_profile") -contains [string]$script:InstallRuntimeContractStatus
$binaryInstallStatus = if ([bool]$script:InstallBootstrapOnlyMode) { "bootstrap_fallback" } else { "ok" }
$runtimeInstalled = -not [bool]$script:InstallBootstrapOnlyMode
$installOutcome = if ($runtimeInstalled) { "success" } else { "runtime_pending" }
$verificationConfidence = "high"
if (-not $runtimeContractOk -or $failedSmokeRequired.Count -gt 0) {
  $verificationConfidence = "medium"
}
if (-not $runtimeInstalled) {
  $verificationConfidence = "runtime_pending"
}
if ($dashboardSmokeRequired -and $dashboardSmokeStatus -ne "passed") {
  $verificationConfidence = "medium"
}
$launcherCommand = "infring gateway"
$restartCommand = "infring gateway restart"
$recoveryCommand = "infring recover"

Write-Host "[infring install] install summary: status=$installOutcome binaries=$binaryInstallStatus runtime=$runtimeContractMode launcher=$launcherCommand restart=$restartCommand verification_confidence=$verificationConfidence"
Write-Host "[infring install] smoke summary: gateway_smoke=$gatewaySmokeStatus dashboard_smoke=$dashboardSmokeStatus recovery=$recoveryCommand"
if (-not $runtimeInstalled) {
  Write-Host "[infring install] runtime pending reason: $($script:InstallBootstrapOnlyReason)"
}

$summaryPayload = @{
  ok = $true
  type = if ($runtimeInstalled) { "infring_install_success_summary" } else { "infring_install_runtime_pending_summary" }
  status = $installOutcome
  version = [string]$version
  triple = [string]$triple
  runtime_installed = [bool]$runtimeInstalled
  install_mode = @{
    full = [bool]$InstallFull
    pure = [bool]$InstallPure
    tiny_max = [bool]$InstallTinyMax
    repair = [bool]$InstallRepair
    offline = [bool]$InstallOffline
  }
  verification = @{
    confidence = $verificationConfidence
    runtime_contract_ok = [bool]$runtimeContractOk
    runtime_contract_mode = $runtimeContractMode
    bootstrap_only_mode = [bool]$script:InstallBootstrapOnlyMode
    bootstrap_only_reason = [string]$script:InstallBootstrapOnlyReason
    gateway_smoke = $gatewaySmokeStatus
    dashboard_smoke = $dashboardSmokeStatus
    smoke_required_failed = $failedSmokeRequired.Count
    smoke_checks = $smokeChecks
    asset_checksum_verification = @{
      enabled = if (-not [string]::IsNullOrWhiteSpace([string]$env:INFRING_INSTALL_VERIFY_ASSETS)) { Installer-TruthyFlag $env:INFRING_INSTALL_VERIFY_ASSETS $true } else { $true }
      manifest_version = [string]$script:ChecksumManifestVersion
      manifest_asset = [string]$script:ChecksumManifestAssetName
      manifest_path = if ($script:ChecksumManifestPath) { [string]$script:ChecksumManifestPath } else { "" }
      lockfile_path = [string]$script:InstallAssetLockfile
    }
    repair_summary = @{
      archive_path = [string]$script:RepairArchiveRun
      removed = [int]$script:RepairRemovedCount
      preserved = [int]$script:RepairPreservedCount
    }
    "workspace_runtime_refresh" = [ordered]@{
      required = [bool]$script:WorkspaceRuntimeRefreshRequired
      reason = [string]$script:WorkspaceRuntimeRefreshReason
      tag_state_missing = [bool]$script:WorkspaceRuntimeTagStateMissing
      applied = [bool]$script:WorkspaceRuntimeRefreshApplied
      release_tag_write_applied = [bool]$script:WorkspaceReleaseTagWriteApplied
      release_tag_write_verified = [bool]$script:WorkspaceReleaseTagWriteVerified
      previous_release_tag = [string]$script:WorkspaceReleaseTagPrevious
      current_release_tag = [string]$script:WorkspaceReleaseTagCurrent
    }
  }
  commands = @{
    launcher = $launcherCommand
    restart = $restartCommand
    recovery = $recoveryCommand
  }
  summary_files = @{
    text = $InstallSummaryTextPath
    json = $InstallSummaryJsonPath
    smoke_json = $InstallSmokeSummaryJsonPath
  }
}
$summaryTextDir = Split-Path -Parent $InstallSummaryTextPath
if (-not [string]::IsNullOrWhiteSpace($summaryTextDir)) {
  New-Item -ItemType Directory -Force -Path $summaryTextDir | Out-Null
}
$summaryCompletedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$summaryTextRows = @(
  "infring_install_summary_v1",
  "version: $version",
  "triple: $triple",
  "install_mode_full: $([string][bool]$InstallFull).ToLower()",
  "install_mode_pure: $([string][bool]$InstallPure).ToLower()",
  "install_mode_tiny_max: $([string][bool]$InstallTinyMax).ToLower()",
  "install_mode_repair: $([string][bool]$InstallRepair).ToLower()",
  "install_mode_offline: $([string][bool]$InstallOffline).ToLower()",
  "toolchain_policy: $([string]$script:InstallToolchainPolicy)",
  "binary_status: $binaryInstallStatus",
  "runtime_mode: $runtimeContractMode",
  "workspace_runtime_refresh_required: $([string][bool]$script:WorkspaceRuntimeRefreshRequired).ToLower()",
  "workspace_runtime_refresh_applied: $([string][bool]$script:WorkspaceRuntimeRefreshApplied).ToLower()",
  "workspace_runtime_tag_state_missing: $([string][bool]$script:WorkspaceRuntimeTagStateMissing).ToLower()",
  "workspace_runtime_refresh_reason: $([string]$script:WorkspaceRuntimeRefreshReason)",
  "workspace_release_tag_previous: $([string]$script:WorkspaceReleaseTagPrevious)",
  "workspace_release_tag_current: $([string]$script:WorkspaceReleaseTagCurrent)",
  "workspace_release_tag_written: $([string][bool]$script:WorkspaceReleaseTagWriteApplied).ToLower()",
  "workspace_release_tag_write_verified: $([string][bool]$script:WorkspaceReleaseTagWriteVerified).ToLower()",
  "install_status: $installOutcome",
  "runtime_installed: $([string][bool]$runtimeInstalled).ToLower()",
  "bootstrap_only_mode: $([string][bool]$script:InstallBootstrapOnlyMode).ToLower()",
  "bootstrap_only_reason: $([string]$script:InstallBootstrapOnlyReason)",
  "verification_confidence: $verificationConfidence",
  "gateway_smoke: $gatewaySmokeStatus",
  "dashboard_smoke: $dashboardSmokeStatus",
  "launcher: $launcherCommand",
  "restart: $restartCommand",
  "recovery: $recoveryCommand",
  "summary_json: $InstallSummaryJsonPath",
  "smoke_summary_json: $InstallSmokeSummaryJsonPath",
  "completed_at: $summaryCompletedAt",
  "status: $installOutcome"
)
$summaryTextRows | Set-Content -Path $InstallSummaryTextPath -Encoding UTF8
Write-Host "[infring install] summary text: $InstallSummaryTextPath"
$script:InstallSummaryFinalized = $true

$summaryDir = Split-Path -Parent $InstallSummaryJsonPath
if (-not [string]::IsNullOrWhiteSpace($summaryDir)) {
  New-Item -ItemType Directory -Force -Path $summaryDir | Out-Null
}
$summaryPayload | ConvertTo-Json -Depth 8 | Set-Content -Path $InstallSummaryJsonPath -Encoding UTF8
Write-Host "[infring install] summary json: $InstallSummaryJsonPath"

$smokeSummaryPayload = @{
  ok = ($failedSmokeRequired.Count -eq 0)
  type = "infring_install_smoke_summary"
  version = [string]$version
  triple = [string]$triple
  required_failed = $failedSmokeRequired.Count
  toolchain_policy = [string]$script:InstallToolchainPolicy
  checks = $smokeChecks
}
$smokeSummaryDir = Split-Path -Parent $InstallSmokeSummaryJsonPath
if (-not [string]::IsNullOrWhiteSpace($smokeSummaryDir)) {
  New-Item -ItemType Directory -Force -Path $smokeSummaryDir | Out-Null
}
$smokeSummaryPayload | ConvertTo-Json -Depth 8 | Set-Content -Path $InstallSmokeSummaryJsonPath -Encoding UTF8
Write-Host "[infring install] smoke summary json: $InstallSmokeSummaryJsonPath"
if ($InstallJson) {
  $summaryPayload | ConvertTo-Json -Depth 8 -Compress | Write-Output
}

Write-InstallCompletionCard -Version $version -InstallDir $InstallDir -RuntimeInstalled $runtimeInstalled -RuntimeMode $runtimeContractMode -BootstrapOnlyReason $script:InstallBootstrapOnlyReason
if ($runtimeInstalled) {
  Write-Host "[infring install] installed commands: infring, infringctl, infringd"
  Write-Host "[infring install] run now (direct path): $InstallDir\\infring.cmd --help"
  Write-Host "[infring install] quickstart now (direct path): $InstallDir\\infring.cmd gateway"
  Write-Host "[infring install] run in this shell: infring --help"
  Write-Host "[infring install] quickstart: infring gateway"
  Write-Host "[infring install] stop: infring gateway stop"
  if ($dashboardSmokeStatus -eq "passed") {
    Write-Host "[infring install] dashboard smoke passed (ephemeral check). Use 'infring gateway' for persistent runtime."
  } elseif ($dashboardSmokeStatus -eq "skipped") {
    Write-Host "[infring install] dashboard smoke skipped in this install mode. Use 'infring gateway' to launch persistent runtime."
  }
  Write-Host "[infring install] if command isn't found immediately, run: $InstallDir\\infring.cmd --help"
} else {
  Write-Host "[infring install] installed bootstrap commands: infring, infringctl, infringd"
  Write-Host "[infring install] runtime pending: gateway/dashboard quickstart is deferred until runtime binaries are installed."
  Write-Host "[infring install] recovery: infring recover"
  Write-Host "[infring install] repair rerun: powershell.exe -NoProfile -ExecutionPolicy Bypass -File install.ps1 -Repair -Full"
  Write-Host "[infring install] status now (direct path): $InstallDir\\infring.cmd setup status --json"
}
Write-Host "[infring install] if `Remove-Item` prints nothing, that's expected success behavior in PowerShell."
Write-Host "[infring install] README Windows install command: $ReadmeWindowsInstallCommand"

Invoke-SourceFallbackCleanup
