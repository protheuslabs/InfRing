#!/usr/bin/env node
// source_domain: validation; owner_domain: validation.installers
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const INSTALL_SCRIPT_PATH = 'install.ps1';
const INSTALL_SCRIPT_SH_PATH = 'install.sh';
const VERSION_BUMP_WORKFLOW_PATH = '.github/workflows/version-bump.yml';
const RELEASE_WORKFLOW_PATH = '.github/workflows/release.yml';
const DEFAULT_OUT_JSON = 'core/local/artifacts/windows_installer_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/WINDOWS_INSTALLER_CONTRACT_GUARD_CURRENT.md';
const DEFAULT_RELIABILITY_OUT_JSON = 'core/local/artifacts/windows_install_reliability_current.json';
const DEFAULT_RELIABILITY_OUT_ALIAS_JSON = 'artifacts/windows_install_reliability_latest.json';
const DEFAULT_RELIABILITY_OUT_MARKDOWN =
  'local/workspace/reports/WINDOWS_INSTALL_RELIABILITY_CURRENT.md';

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
  };
}

function countOccurrences(source: string, pattern: RegExp): number {
  const matches = source.match(pattern);
  return Array.isArray(matches) ? matches.length : 0;
}

function countLiteral(source: string, literal: string): number {
  return literal.length === 0 ? 0 : source.split(literal).length - 1;
}

function extractUnique(source: string, pattern: RegExp, captureIndex = 1): string[] {
  const values = new Set<string>();
  let match: RegExpExecArray | null = null;
  while ((match = pattern.exec(source)) != null) {
    const raw = cleanText(String(match[captureIndex] || ''), 200);
    if (raw.length > 0) values.add(raw);
  }
  return [...values].sort((a, b) => a.localeCompare(b));
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# WINDOWS INSTALLER CONTRACT GUARD');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Checks');
  for (const row of payload.checks || []) {
    lines.push(`- [${row.ok ? 'x' : ' '}] \`${row.id}\` — ${row.detail}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function toReliabilityMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# WINDOWS INSTALL RELIABILITY');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- prebuilt_probe_declared: ${payload.summary.prebuilt_probe_declared}`);
  lines.push(`- source_fallback_reason_token_present: ${payload.summary.source_fallback_reason_token_present}`);
  lines.push(`- wrapper_status_contract_pass: ${payload.summary.wrapper_status_contract_pass}`);
  lines.push(`- parser_safety_contract_pass: ${payload.summary.parser_safety_contract_pass}`);
  lines.push(`- ps1_release_tag_contract_pass: ${payload.summary.ps1_release_tag_contract_pass}`);
  lines.push(`- cross_shell_release_tag_contract_pass: ${payload.summary.cross_shell_release_tag_contract_pass}`);
  lines.push('');
  lines.push('## Fallback Reasons');
  if (!Array.isArray(payload.source_fallback_reasons) || payload.source_fallback_reasons.length === 0) {
    lines.push('- none');
  } else {
    for (const row of payload.source_fallback_reasons) lines.push(`- ${row}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);
  const source = fs.readFileSync(path.resolve(ROOT, INSTALL_SCRIPT_PATH), 'utf8');
  const sourceSh = fs.readFileSync(path.resolve(ROOT, INSTALL_SCRIPT_SH_PATH), 'utf8');
  const versionBumpWorkflow = fs.readFileSync(
    path.resolve(ROOT, VERSION_BUMP_WORKFLOW_PATH),
    'utf8',
  );
  const releaseWorkflow = fs.readFileSync(path.resolve(ROOT, RELEASE_WORKFLOW_PATH), 'utf8');

  const checks: CheckRow[] = [];
  const asciiWrapperWriteCount = countOccurrences(
    source,
    /Set-Content\s+-Path \$Path -Value \$content -Encoding ASCII/g,
  );
  const parserUnsafeJoinPathRegexCount = countOccurrences(
    source,
    /-match\s+["']Join-Path\\\\s\+\\\\/g,
  );
  const parserUnsafeWindowsEscapeMatchCount = countOccurrences(
    source,
    /-match\s+"Join-Path\\\\s\+\\\\[A-Za-z]:/g,
  );
  const parserUnsafeVariableInterpolatedJoinPathRegexCount = countOccurrences(
    source,
    /-match\s+"Join-Path\\\\s\+\\\\\$[A-Za-z_][A-Za-z0-9_]*/g,
  );
  const tempPathJoinPathCount = countOccurrences(
    source,
    /Join-Path\s+\(\[System\.IO\.Path\]::GetTempPath\(\)\)/g,
  );
  const parserUnsafeTrimEndBackslashStringCount = countLiteral(
    source,
    String.raw`.TrimEnd("\", "/")`,
  );
  const parserUnsafeEndsWithBackslashStringCount = countLiteral(
    source,
    String.raw`.EndsWith("\")`,
  );
  const parserUnsafeInterpolatedRmdirCommandCount = countLiteral(
    source,
    'rmdir /s /q `"$cleanupRoot`"',
  );
  const parserUnsafeEscapedQuoteHelpTextCount = countLiteral(
    source,
    String.raw`--override \"--quiet`,
  );
  const powershellNonAsciiGlyphCount = countOccurrences(source, /[^\x00-\x7F]/g);

  checks.push({
    id: 'windows_install_script_no_merge_conflict_markers',
    ok:
      countOccurrences(source, /^\s*<{7}(?!<)/gm) === 0
      && countOccurrences(source, /^\s*={7}(?!=)/gm) === 0
      && countOccurrences(source, /^\s*>{7}(?!>)/gm) === 0,
    detail: 'install.ps1 must not contain unresolved merge markers',
  });

  checks.push({
    id: 'windows_install_script_single_repair_function_definition',
    ok: countOccurrences(source, /function\s+Invoke-RepairInstallDir\s*\{/g) === 1,
    detail: 'Invoke-RepairInstallDir must be declared exactly once',
  });

  checks.push({
    id: 'windows_install_script_single_repair_target_table',
    ok: countOccurrences(source, /\$targets\s*=\s*@\(/g) === 1,
    detail: 'repair target list must have one canonical declaration to prevent parser drift',
  });

  checks.push({
    id: 'windows_install_script_no_double_quoted_psscriptroot_regex',
    ok: !source.includes('-match "Join-Path\\\\s+\\\\$PSScriptRoot"'),
    detail: 'regex checks for $PSScriptRoot must avoid double-quoted interpolation hazards',
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_join_path_regex_sequences',
    ok: parserUnsafeJoinPathRegexCount === 0,
    detail: `install.ps1 must not contain parser-unsafe escaped Join-Path regex sequences (count=${parserUnsafeJoinPathRegexCount})`,
  });

  checks.push({
    id: 'windows_install_script_cmd_wrappers_ascii_encoded',
    ok: asciiWrapperWriteCount >= 3,
    detail: `cmd wrapper writers must emit ASCII wrappers for Windows cmd compatibility (ascii_writes=${asciiWrapperWriteCount})`,
  });

  checks.push({
    id: 'windows_install_script_powershell_shims_utf8_encoded',
    ok: source.includes('Set-Content -Path $Path -Value $content -Encoding UTF8'),
    detail: 'PowerShell shim writer must emit UTF8 content deterministically',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_floor_invoked_during_repair_and_post_write',
    ok: countOccurrences(source, /Ensure-RepairBootstrapWrapperFloor\s+-InstallDir\s+\$InstallDir/g) >= 2,
    detail: 'wrapper floor must run during repair and after wrapper generation',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_help_copy_present',
    ok:
      source.includes('[infring bootstrap] runtime binaries are not installed on this machine yet.')
      && source.includes('[infring bootstrap] run: install.ps1 -Repair -Full'),
    detail: 'installer wrappers must include deterministic bootstrap recovery guidance',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_action_cmd_fallback_present',
    ok: source.includes('_BOOTSTRAP_ACTION') && source.includes('goto :bootstrap'),
    detail: 'failure bootstrap cmd wrapper must include deterministic action dispatch fallback',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_only_is_runtime_pending_outcome',
    ok:
      source.includes('$script:InstallBootstrapOnlyMode = $true')
      && source.includes('bootstrap_only_profile')
      && source.includes('bootstrap-only mode: skipping client runtime bundle until runtime binaries are installed')
      && source.includes('"bootstrap_only_profile") -contains [string]$script:InstallRuntimeContractStatus')
      && source.includes('status=$installOutcome')
      && source.includes('runtime_installed = [bool]$runtimeInstalled')
      && source.includes('Installation incomplete: runtime pending.')
      && source.includes('cargo_unusable_after_rustup_default')
      && source.includes('cargo_unusable_auto_rustup_disabled')
      && source.includes('rustup_missing_for_cargo_repair'),
    detail:
      'Windows full-mode onboarding fallback must complete as bootstrap-only runtime-pending outcome instead of claiming full runtime success',
  });

  checks.push({
    id: 'windows_install_script_empty_release_bundle_diagnostic_present',
    ok: source.includes('release_asset_bundle_empty'),
    detail:
      'installer must distinguish an empty published GitHub release from a normal missing-platform asset so Windows bootstrap-only installs point at the real release packaging failure',
  });

  checks.push({
    id: 'version_bump_workflow_does_not_create_assetless_github_releases',
    ok:
      !/\bgh\s+release\s+(create|edit)\b/.test(versionBumpWorkflow)
      && versionBumpWorkflow.includes('release_publication_owner')
      && versionBumpWorkflow.includes('.github/workflows/release.yml'),
    detail:
      'version-bump may create/preserve the tag, but GitHub release publication must remain owned by release.yml so latest releases cannot be published without runtime assets',
  });

  checks.push({
    id: 'release_workflow_verifies_windows_runtime_asset_contract',
    ok:
      releaseWorkflow.includes('Verify published release asset contract')
      && releaseWorkflow.includes('release_asset_contract_failed')
      && releaseWorkflow.includes('infring-ops-x86_64-pc-windows-msvc.exe')
      && releaseWorkflow.includes('infringd-x86_64-pc-windows-msvc.exe')
      && releaseWorkflow.includes('conduit_daemon-x86_64-pc-windows-msvc.exe'),
    detail:
      'asset-owning release workflow must fail if the published release is missing required Windows runtime assets',
  });

  checks.push({
    id: 'release_workflow_runs_from_version_bump_tag_handoff',
    ok:
      releaseWorkflow.includes('tags:')
      && releaseWorkflow.includes('"v*"')
      && releaseWorkflow.includes("mode: 'tag_handoff'")
      && releaseWorkflow.includes('github.ref_type')
      && releaseWorkflow.includes('github.ref_name'),
    detail:
      'release workflow must consume version-bump tag handoffs so version bumping works without allowing version-bump to publish assetless releases',
  });

  checks.push({
    id: 'windows_install_script_runtime_pending_does_not_print_quickstart',
    ok:
      source.includes('if ($runtimeInstalled) {')
      && source.includes('quickstart now (direct path): $InstallDir\\\\infring.cmd gateway')
      && source.includes('runtime pending: gateway/dashboard quickstart is deferred until runtime binaries are installed.')
      && source.includes('installed bootstrap commands: infring, infringctl, infringd')
      && source.includes('status now (direct path): $InstallDir\\\\infring.cmd setup status --json'),
    detail:
      'bootstrap-only installs must not print gateway quickstart as if the runtime is installed; they must print runtime-pending recovery/status guidance',
  });

  checks.push({
    id: 'windows_install_script_repair_wrapper_map_covers_three_entrypoints',
    ok:
      source.includes('@{ cmd = "infring.cmd"; ps1 = "infring.ps1" }')
      && source.includes('@{ cmd = "infringctl.cmd"; ps1 = "infringctl.ps1" }')
      && source.includes('@{ cmd = "infringd.cmd"; ps1 = "infringd.ps1" }'),
    detail: 'repair/failure wrapper maps must preserve all three canonical entrypoint wrapper pairs',
  });

  checks.push({
    id: 'windows_install_script_wrapper_ps1_to_cmd_continuity_contract',
    ok:
      source.includes('Write-PowerShellShim -Path $infringPs1 -TargetCmd "infring.cmd"')
      && source.includes('Write-PowerShellShim -Path $infringctlPs1 -TargetCmd "infringctl.cmd"')
      && source.includes('Write-PowerShellShim -Path $infringdPs1 -TargetCmd "infringd.cmd"')
      && source.includes('$target = Join-Path $PSScriptRoot "__TARGET__"')
      && source.includes('& $target @CommandArgs'),
    detail: 'PowerShell shims must preserve deterministic ps1->cmd continuity for all entrypoints',
  });

  checks.push({
    id: 'windows_install_script_failure_bootstrap_function_present',
    ok: source.includes('function Ensure-InstallFailureBootstrapWrappers'),
    detail: 'failure path must still emit bootstrap wrappers for deterministic recovery',
  });

  checks.push({
    id: 'windows_install_script_repair_detects_legacy_throw_wrappers',
    ok:
      source.includes("missing command wrapper")
      && source.includes('Test-RepairArtifactBroken')
      && source.includes('throw '),
    detail: 'repair logic must detect and replace legacy throw-based wrapper templates',
  });

  checks.push({
    id: 'windows_install_script_repair_cmd_write_uses_literalpath_force',
    ok: source.includes('Set-Content -LiteralPath $cmdPath -Value $cmdContent -Encoding ASCII -Force'),
    detail: 'repair cmd wrapper writes must use LiteralPath+Force for deterministic rewrite behavior',
  });

  checks.push({
    id: 'windows_install_script_repair_ps1_write_uses_literalpath_force',
    ok: source.includes('Set-Content -LiteralPath $ps1Path -Value $psContent -Encoding UTF8 -Force'),
    detail: 'repair PowerShell wrapper writes must use LiteralPath+Force for deterministic rewrite behavior',
  });

  checks.push({
    id: 'windows_install_script_repair_cmd_detects_placeholder_tokens',
    ok: source.includes('if ($content.Contains("__PS1__") -or $content.Contains("__TARGET__"))'),
    detail: 'repair cmd wrapper detector must fail closed on unresolved template placeholders',
  });

  checks.push({
    id: 'windows_install_script_repair_ps1_detects_placeholder_tokens',
    ok: source.includes('if ($content.Contains("__TARGET__") -or $content.Contains("__PS1__"))'),
    detail: 'repair ps1 wrapper detector must fail closed on unresolved template placeholders',
  });

  checks.push({
    id: 'windows_install_script_repair_archive_recovery_contract',
    ok:
      source.includes('$repairArchiveRoot = Join-Path $InstallDir "_repair_archive"')
      && source.includes('Copy-Item -Force -Recurse $path (Join-Path $repairArchiveRun $target)')
      && source.includes('repair archived healthy install artifact')
      && source.includes('repair preserved healthy install artifact'),
    detail: 'repair mode must archive healthy artifacts and preserve recovery visibility',
  });

  checks.push({
    id: 'windows_install_script_repair_workspace_cleanup_fail_soft_contract',
    ok:
      source.includes('Test-Path -LiteralPath $abs')
      && source.includes('Remove-Item -Force -Recurse -LiteralPath $abs -ErrorAction Stop')
      && source.includes('repair warning: failed to remove stale runtime path: $rel ($cleanupReason)')
      && source.includes('New-Item -ItemType Directory -Force -Path (Join-Path $workspaceRoot "local/state") -ErrorAction Stop')
      && source.includes('repair warning: failed to recreate local/state ({0})'),
    detail:
      'repair workspace cleanup and local/state recreation must be diagnostic and non-fatal when Windows refuses stale generated runtime paths',
  });

  checks.push({
    id: 'windows_install_script_exception_reason_visible_contract',
    ok:
      source.includes('[infring install] failure reason: {0}')
      && source.includes('Write-InstallFailureSummary -FailureReason $reason -ExitCode "1"'),
    detail:
      'installer exceptions must print a visible failure reason before the recovery summary so CI does not collapse to ScriptHalted',
  });

  checks.push({
    id: 'windows_install_script_repair_wrapper_floor_fail_closed_contract',
    ok:
      source.includes('$requiredWrappers = @(')
      && source.includes('"infring.cmd", "infringctl.cmd", "infringd.cmd"')
      && source.includes('"infring.ps1", "infringctl.ps1", "infringd.ps1"')
      && source.includes('throw "repair wrapper floor failed; missing wrappers: $($missingWrappers -join \', \')"'),
    detail: 'repair mode must fail closed when wrapper floor is incomplete',
  });

  checks.push({
    id: 'windows_install_script_parser_escape_guard_single_quoted_tokens',
    ok: source.includes("if ($content.Contains('Join-Path\\\\s+\\\\$PSScriptRoot') -or $content.Contains('Join-Path\\\\s+\\\\'))"),
    detail: 'parser safety checks must use single-quoted escape tokens for Join-Path regex-like fragments',
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_windows_escape_match_sequences',
    ok: parserUnsafeWindowsEscapeMatchCount === 0,
    detail: `install.ps1 must not contain parser-unsafe double-quoted -match patterns for Windows temp paths (count=${parserUnsafeWindowsEscapeMatchCount})`,
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_variable_interpolated_join_path_regex',
    ok: parserUnsafeVariableInterpolatedJoinPathRegexCount === 0,
    detail:
      `install.ps1 must not contain parser-unsafe variable-interpolated Join-Path regex patterns (count=${parserUnsafeVariableInterpolatedJoinPathRegexCount})`,
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_trimend_backslash_string',
    ok: parserUnsafeTrimEndBackslashStringCount === 0,
    detail:
      `install.ps1 must not pass bare backslash double-quoted strings into TrimEnd on Windows PowerShell (count=${parserUnsafeTrimEndBackslashStringCount})`,
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_endswith_backslash_string',
    ok: parserUnsafeEndsWithBackslashStringCount === 0,
    detail:
      `install.ps1 must not pass bare backslash double-quoted strings into EndsWith on Windows PowerShell (count=${parserUnsafeEndsWithBackslashStringCount})`,
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_interpolated_rmdir_backtick_quotes',
    ok: parserUnsafeInterpolatedRmdirCommandCount === 0,
    detail:
      `install.ps1 must not inline cleanupRoot inside backtick-quoted cmd rmdir strings (count=${parserUnsafeInterpolatedRmdirCommandCount})`,
  });

  checks.push({
    id: 'windows_install_script_cleanup_rmdir_command_is_prebuilt',
    ok:
      source.includes('$doubleQuote = [string][char]34')
      && source.includes('$cleanupCommand = [string]::Concat("rmdir /s /q ", $doubleQuote, $safeCleanupRoot, $doubleQuote)')
      && source.includes('Start-Process -FilePath "cmd.exe" -ArgumentList @("/d", "/c", $cleanupCommand)'),
    detail:
      'install.ps1 must prebuild the cmd cleanup command before Start-Process so Windows PowerShell does not parse nested backtick quotes',
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_escaped_quote_help_text',
    ok: parserUnsafeEscapedQuoteHelpTextCount === 0,
    detail:
      `install.ps1 help text must not use backslash-escaped quotes inside double-quoted PowerShell strings (count=${parserUnsafeEscapedQuoteHelpTextCount})`,
  });

  checks.push({
    id: 'windows_install_script_windows_path_trim_uses_char_literals',
    ok:
      source.includes('TrimEnd([char]92, [char]47)')
      && source.includes("EndsWith([string][char]92)")
      && source.includes('TrimEnd([char]92)'),
    detail:
      'install.ps1 must use numeric char literals for Windows path trimming rather than quote-sensitive slash strings',
  });

  checks.push({
    id: 'windows_install_script_cleanup_command_uses_char34_contract',
    ok:
      source.includes('$doubleQuote = [string][char]34')
      && source.includes('$safeCleanupRoot.IndexOf([char]34) -ge 0')
      && source.includes('$cleanupCommand = [string]::Concat("rmdir /s /q ", $doubleQuote, $safeCleanupRoot, $doubleQuote)'),
    detail:
      'install.ps1 must build the Windows cleanup command from char-code quote primitives rather than nested quote literals',
  });

  checks.push({
    id: 'windows_install_script_manifest_regex_uses_named_pattern_variable',
    ok:
      source.includes('$bsdPattern = "^SHA256\\\\s+\\\\(([^)]+)\\\\)\\\\s*=\\\\s*([a-fA-F0-9]{64})$"')
      && source.includes('$bsd = [System.Text.RegularExpressions.Regex]::Match(')
      && source.includes('$bsdPattern,'),
    detail:
      'install.ps1 must keep the SHA256 manifest regex in a dedicated pattern variable for Windows PowerShell parser stability',
  });

  checks.push({
    id: 'windows_install_script_powershell_ascii_only_completion_card',
    ok:
      powershellNonAsciiGlyphCount === 0
      && source.includes('SUCCESS: InfRing successfully installed!')
      && source.includes('BOOTSTRAP INSTALLED: InfRing runtime pending.')
      && source.includes('Installation complete!')
      && source.includes('Installation incomplete: runtime pending.')
      && !source.includes('✔ InfRing successfully installed!')
      && !source.includes('✅ Installation complete!'),
    detail:
      `install.ps1 must keep the PowerShell completion/runtime-pending cards ASCII-only for Windows PowerShell encoding compatibility (non_ascii_count=${powershellNonAsciiGlyphCount})`,
  });

  checks.push({
    id: 'windows_install_script_temp_path_joinpath_contract_present',
    ok: tempPathJoinPathCount >= 1,
    detail: `installer must retain Join-Path temp-path construction for parser-safe temp edge cases (count=${tempPathJoinPathCount})`,
  });

  checks.push({
    id: 'windows_install_script_install_diagnostic_tokens_present',
    ok:
      source.includes('asset_probe=')
      && source.includes('attempted_assets=')
      && source.includes('source_fallback_reason='),
    detail: 'installer diagnostics must include prebuilt reachability and source-fallback reason tokens',
  });

  checks.push({
    id: 'windows_install_script_workspace_refresh_summary_contract_ps1',
    ok:
      source.includes('workspace_runtime_refresh_required:')
      && source.includes('workspace_runtime_refresh_applied:')
      && source.includes('workspace_runtime_refresh_reason:')
      && source.includes('workspace_runtime_tag_state_missing:')
      && source.includes('workspace_release_tag_write_verified:'),
    detail:
      'install.ps1 summary notes must include required/applied/reason/tag_state_missing/release_tag_write_verified fields',
  });

  checks.push({
    id: 'windows_install_script_workspace_refresh_json_contract_ps1',
    ok:
      source.includes('"workspace_runtime_refresh" = [ordered]@{')
      && source.includes('required = [bool]$script:WorkspaceRuntimeRefreshRequired')
      && source.includes('applied = [bool]$script:WorkspaceRuntimeRefreshApplied')
      && source.includes('reason = [string]$script:WorkspaceRuntimeRefreshReason')
      && source.includes('tag_state_missing = [bool]$script:WorkspaceRuntimeTagStateMissing')
      && source.includes('release_tag_write_verified = [bool]$script:WorkspaceReleaseTagWriteVerified'),
    detail:
      'install.ps1 JSON success summary must include workspace runtime refresh contract keys (required/applied/reason/tag_state_missing/release_tag_write_verified)',
  });

  checks.push({
    id: 'windows_install_script_release_tag_write_readback_fail_closed_ps1',
    ok:
      (source.includes('if ($script:WorkspaceRuntimeRefreshRequired -and -not $script:WorkspaceRuntimeRefreshApplied)')
        || source.includes('if ([bool]$script:WorkspaceRuntimeRefreshRequired -and (-not [bool]$script:WorkspaceRuntimeRefreshApplied))'))
      && source.includes('Workspace runtime refresh required but not applied')
      && source.includes('Write-WorkspaceRuntimeReleaseTagState')
      && source.includes('Assert-WorkspaceRuntimeReleaseTagState')
      && source.includes('Workspace release tag state verification failed'),
    detail:
      'install.ps1 must fail closed when refresh is required but not applied and must verify release-tag write via explicit readback assertion',
  });

  checks.push({
    id: 'windows_install_script_cross_shell_workspace_refresh_summary_contract',
    ok:
      sourceSh.includes('workspace_runtime_refresh_required:')
      && sourceSh.includes('workspace_runtime_refresh_applied:')
      && sourceSh.includes('workspace_runtime_refresh_reason:')
      && sourceSh.includes('workspace_runtime_tag_state_missing:'),
    detail:
      'install.sh must emit workspace refresh summary fields (required/applied/reason/tag_state_missing) for cross-shell contract parity',
  });

  checks.push({
    id: 'windows_install_script_cross_shell_release_tag_summary_contract',
    ok:
      sourceSh.includes('workspace_release_tag_previous:')
      && sourceSh.includes('workspace_release_tag_current:')
      && sourceSh.includes('workspace_release_tag_written:')
      && sourceSh.includes('workspace_release_tag_write_verified:'),
    detail:
      'install.sh must emit release-tag summary fields (previous/current/written/write_verified) for cross-shell contract parity',
  });

  checks.push({
    id: 'windows_install_script_cross_shell_tag_state_missing_reason_contract',
    ok:
      sourceSh.includes('workspace_refresh_reason="tag_state_missing"')
      && sourceSh.includes('workspace_refresh_tag_state_missing=1'),
    detail:
      'install.sh must surface tag_state_missing reason parity when release-tag state is absent',
  });

  checks.push({
    id: 'windows_install_script_cross_shell_release_tag_write_readback_fail_closed',
    ok:
      sourceSh.includes('write_workspace_release_tag "$WORKSPACE_DIR" "$version" || exit 1')
      && sourceSh.includes('if workspace_release_tag_matches "$WORKSPACE_DIR" "$version"; then')
      && sourceSh.includes('workspace_release_tag_write_verified=1')
      && sourceSh.includes('workspace release-tag state write verification failed'),
    detail:
      'install.sh must fail closed unless release-tag write is followed by explicit readback verification success',
  });

  checks.push({
    id: 'windows_install_script_cross_shell_refresh_required_apply_fail_closed',
    ok:
      sourceSh.includes('if [ "$workspace_refresh_required" = "1" ] && [ "$workspace_refresh_applied" != "1" ]; then')
      && sourceSh.includes('workspace runtime refresh required but not applied'),
    detail:
      'install.sh must fail closed when workspace refresh is required but not applied before release-tag update',
  });

  checks.push({
    id: 'windows_install_script_cross_shell_json_workspace_refresh_contract',
    ok:
      sourceSh.includes('"workspace_runtime_refresh":{"required":')
      && sourceSh.includes('"tag_state_missing":')
      && sourceSh.includes('"release_tag_write_verified":'),
    detail:
      'install.sh JSON success summary must include refresh/tag-state/write-verification contract fields',
  });

  checks.push({
    id: 'installer_success_completion_card_contract',
    ok:
      source.includes('function Write-InstallCompletionCard')
      && source.includes('SUCCESS: InfRing successfully installed!')
      && source.includes('BOOTSTRAP INSTALLED: InfRing runtime pending.')
      && source.includes('Installation complete!')
      && source.includes('Installation incomplete: runtime pending.')
      && source.includes('-ForegroundColor Green')
      && source.includes('-ForegroundColor DarkYellow')
      && sourceSh.includes('emit_install_completion_card')
      && sourceSh.includes('✔ InfRing successfully installed!')
      && sourceSh.includes('✅ Installation complete!')
      && sourceSh.includes('38;5;208'),
    detail:
      'installers must retain Claude-style success completion card and must show a distinct runtime-pending card for bootstrap-only fallback',
  });

  const ok = checks.every((row) => row.ok);
  const sourceFallbackReasons = extractUnique(source, /source_fallback_reason=([a-z0-9_]+)/g, 1);
  const assetProbeStatuses = extractUnique(source, /asset_probe=([a-z_]+)/g, 1);
  const wrapperTargets = extractUnique(source, /"infring(?:ctl|d)?\.cmd"/g, 0).map((row) =>
    row.replace(/"/g, ''),
  );
  const wrapperPs1Targets = extractUnique(source, /"infring(?:ctl|d)?\.ps1"/g, 0).map((row) =>
    row.replace(/"/g, ''),
  );
  const reliabilityPayload = {
    type: 'windows_install_reliability',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    script_path: INSTALL_SCRIPT_PATH,
    script_sh_path: INSTALL_SCRIPT_SH_PATH,
    summary: {
      prebuilt_probe_declared: source.includes('preflight asset probe'),
      source_fallback_reason_token_present: source.includes('source_fallback_reason='),
      wrapper_status_contract_pass:
        wrapperTargets.length >= 3
        && wrapperPs1Targets.length >= 3
        && source.includes('Ensure-RepairBootstrapWrapperFloor -InstallDir $InstallDir'),
      parser_safety_contract_pass:
        parserUnsafeJoinPathRegexCount === 0
        && parserUnsafeWindowsEscapeMatchCount === 0
        && parserUnsafeVariableInterpolatedJoinPathRegexCount === 0
        && parserUnsafeTrimEndBackslashStringCount === 0
        && parserUnsafeEndsWithBackslashStringCount === 0
        && parserUnsafeInterpolatedRmdirCommandCount === 0
        && parserUnsafeEscapedQuoteHelpTextCount === 0
        && source.includes("if ($content.Contains('Join-Path\\\\s+\\\\$PSScriptRoot') -or $content.Contains('Join-Path\\\\s+\\\\'))"),
      cross_shell_release_tag_contract_pass:
        sourceSh.includes('workspace_runtime_refresh_required:')
        && sourceSh.includes('workspace_runtime_tag_state_missing:')
        && sourceSh.includes('workspace_release_tag_write_verified:')
        && sourceSh.includes('workspace_release_tag_matches "$WORKSPACE_DIR" "$version"'),
      ps1_release_tag_contract_pass:
        source.includes('workspace_runtime_refresh_required:')
        && source.includes('workspace_runtime_tag_state_missing:')
        && source.includes('workspace_release_tag_write_verified:')
        && source.includes('Assert-WorkspaceRuntimeReleaseTagState'),
      temp_path_joinpath_count: tempPathJoinPathCount,
    },
    prebuilt_reachability: {
      probe_log_declared: source.includes('preflight asset probe'),
      diagnostic_asset_probe_token_present: source.includes('asset_probe='),
      diagnostic_attempted_assets_token_present: source.includes('attempted_assets='),
      asset_probe_status_tokens: assetProbeStatuses,
    },
    source_fallback_reasons: sourceFallbackReasons,
    wrapper_status: {
      cmd_targets: wrapperTargets,
      powershell_targets: wrapperPs1Targets,
      repair_archive_enabled: source.includes('$repairArchiveRoot = Join-Path $InstallDir "_repair_archive"'),
      repair_floor_fail_closed: source.includes('repair wrapper floor failed; missing wrappers'),
    },
    parser_safety: {
      parser_unsafe_join_path_regex_count: parserUnsafeJoinPathRegexCount,
      parser_unsafe_windows_escape_match_count: parserUnsafeWindowsEscapeMatchCount,
      parser_unsafe_variable_interpolated_join_path_regex_count:
        parserUnsafeVariableInterpolatedJoinPathRegexCount,
      parser_unsafe_trimend_backslash_string_count: parserUnsafeTrimEndBackslashStringCount,
      parser_unsafe_endswith_backslash_string_count: parserUnsafeEndsWithBackslashStringCount,
      parser_unsafe_interpolated_rmdir_command_count: parserUnsafeInterpolatedRmdirCommandCount,
      parser_unsafe_escaped_quote_help_text_count: parserUnsafeEscapedQuoteHelpTextCount,
      temp_path_joinpath_count: tempPathJoinPathCount,
      single_quoted_escape_token_guard_present: source.includes(
        "if ($content.Contains('Join-Path\\\\s+\\\\$PSScriptRoot') -or $content.Contains('Join-Path\\\\s+\\\\'))",
      ),
    },
  };
  const payload = {
    ok,
    strict: args.strict,
    type: 'windows_installer_contract_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      script_path: INSTALL_SCRIPT_PATH,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary: {
      check_count: checks.length,
      failed_check_count: checks.filter((row) => !row.ok).length,
    },
    checks,
  };

  writeJsonArtifact(DEFAULT_RELIABILITY_OUT_JSON, reliabilityPayload);
  writeJsonArtifact(DEFAULT_RELIABILITY_OUT_ALIAS_JSON, reliabilityPayload);
  writeTextArtifact(DEFAULT_RELIABILITY_OUT_MARKDOWN, toReliabilityMarkdown({
    ...reliabilityPayload,
    ok:
      reliabilityPayload.summary.wrapper_status_contract_pass
      && reliabilityPayload.summary.parser_safety_contract_pass
      && reliabilityPayload.summary.prebuilt_probe_declared,
  }));
  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
