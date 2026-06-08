#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type GateCheck = {
  id: string;
  ok: boolean;
  detail: string;
};

function cleanText(value: unknown, maxLen = 5000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseBool(value: string | undefined, fallback = false): boolean {
  const raw = cleanText(value ?? '', 32).toLowerCase();
  if (!raw) return fallback;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function keysetDrift(
  value: unknown,
  requiredKeys: string[],
): { missing: string[]; unexpected: string[] } {
  const keys =
    value && typeof value === 'object' && !Array.isArray(value)
      ? Object.keys(value as Record<string, unknown>)
      : [];
  const required = new Set(requiredKeys);
  return {
    missing: requiredKeys.filter((key) => !keys.includes(key)),
    unexpected: keys.filter((key) => !required.has(key)),
  };
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/release_policy_gate_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
  };
}

function readJson(filePath: string, fallback: any = null): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function isCanonicalToken(value: string): boolean {
  return /^[a-z0-9_-]+$/.test(cleanText(value, 80));
}

function isCanonicalNameToken(value: string): boolean {
  return /^[a-z0-9_.-]+$/.test(cleanText(value, 120));
}

function readReleaseWorkflow(root: string): string {
  const rel = '.github/workflows/release.yml';
  return fs.readFileSync(path.resolve(root, rel), 'utf8');
}

function countRegex(source: string, pattern: RegExp): number {
  return source.match(pattern)?.length ?? 0;
}

function checkWorkflowNeedleSet(root: string, id: string, needles: string[]): GateCheck {
  const source = readReleaseWorkflow(root);
  const missing = needles.filter((needle) => !source.includes(needle));
  return {
    id,
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function parseNumericVersion(value: string): number[] | null {
  const raw = cleanText(value, 40);
  if (!raw) return null;
  const parts = raw.split('.').map((row) => Number(row));
  if (!parts.length || parts.some((row) => !Number.isFinite(row) || row < 0)) return null;
  while (parts.length < 3) parts.push(0);
  return parts.slice(0, 3);
}

function versionAtLeast(value: string, min: string): boolean {
  const lhs = parseNumericVersion(value);
  const rhs = parseNumericVersion(min);
  if (!lhs || !rhs) return false;
  for (let i = 0; i < 3; i += 1) {
    if (lhs[i]! > rhs[i]!) return true;
    if (lhs[i]! < rhs[i]!) return false;
  }
  return true;
}

function checkFileExists(root: string, relPath: string, id: string): GateCheck {
  const ok = fs.existsSync(path.resolve(root, relPath));
  return {
    id,
    ok,
    detail: ok ? `found:${relPath}` : `missing:${relPath}`,
  };
}

function checkReleaseChannelPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const allowed = Array.isArray(policy.channels) ? policy.channels : [];
  const rules = Array.isArray(policy.promotion_rules) ? policy.promotion_rules : [];
  const ok =
    allowed.includes('alpha') &&
    allowed.includes('beta') &&
    allowed.includes('stable') &&
    rules.length > 0;
  return {
    id: 'release_channel_policy',
    ok,
    detail: ok
      ? `channels=${allowed.join(',')};rules=${rules.length}`
      : `invalid policy at ${rel}`,
  };
}

function checkReleaseChannelPolicyTopLevelKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const drift = keysetDrift(policy, [
    'schema_id',
    'schema_version',
    'default_channel',
    'channels',
    'promotion_rules',
  ]);
  const violations: string[] = [];
  if (drift.missing.length > 0) violations.push(`missing=${drift.missing.join(',')}`);
  if (drift.unexpected.length > 0) violations.push(`unexpected=${drift.unexpected.join(',')}`);
  return {
    id: 'release_channel_policy_top_level_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkReleaseChannelPromotionRuleRowKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const violations: string[] = [];
  for (const row of rules) {
    const drift = keysetDrift(row, ['from', 'to']);
    if (drift.missing.length > 0 || drift.unexpected.length > 0) {
      const edge = `${cleanText((row as any)?.from ?? '', 80)}->${cleanText((row as any)?.to ?? '', 80)}` || '<missing>';
      violations.push(
        `${edge}:missing=${drift.missing.join(',') || 'none'};unexpected=${drift.unexpected.join(',') || 'none'}`,
      );
    }
  }
  return {
    id: 'release_channel_promotion_rule_row_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkReleaseChannelPolicyWhitespaceContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const defaultChannelRaw = String(policy?.default_channel ?? '');
  const channelsRaw = Array.isArray(policy?.channels)
    ? policy.channels.map((row: unknown) => String(row ?? ''))
    : [];
  const badDefault = /\s/.test(defaultChannelRaw);
  const badChannels = channelsRaw.filter((row) => /\s/.test(row));
  return {
    id: 'release_channel_policy_whitespace_contract',
    ok: !badDefault && badChannels.length === 0,
    detail:
      !badDefault && badChannels.length === 0
        ? 'ok'
        : `default_whitespace=${String(badDefault)};channels_whitespace=${badChannels.join(',') || 'none'}`,
  };
}

function checkReleaseChannelPolicySchema(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const schemaId = cleanText(policy?.schema_id ?? '', 80);
  const schemaVersion = cleanText(policy?.schema_version ?? '', 40);
  const ok = schemaId === 'release_channel_policy' && schemaVersion === '1.0';
  return {
    id: 'release_channel_policy_schema_contract',
    ok,
    detail: ok ? 'schema_id/version_ok' : `schema_id=${schemaId || 'missing'};schema_version=${schemaVersion || 'missing'}`,
  };
}

function checkReleaseChannelDefaultChannel(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channels = Array.isArray(policy?.channels)
    ? policy.channels.map((value: unknown) => cleanText(value, 80)).filter(Boolean)
    : [];
  const defaultChannel = cleanText(policy?.default_channel ?? '', 80);
  const ok =
    channels.length > 0 &&
    channels.every((channel) => isCanonicalToken(channel)) &&
    channels.includes(defaultChannel);
  return {
    id: 'release_channel_default_channel_contract',
    ok,
    detail: ok ? `default=${defaultChannel};channels=${channels.join(',')}` : `default=${defaultChannel || 'missing'};channels=${channels.join(',') || 'missing'}`,
  };
}

function checkReleaseChannelPromotionRules(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channels = new Set<string>(
    Array.isArray(policy?.channels)
      ? policy.channels.map((value: unknown) => cleanText(value, 80)).filter(Boolean)
      : [],
  );
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const normalizedRules = rules.map((row: any) => ({
    from: cleanText(row?.from ?? '', 80),
    to: cleanText(row?.to ?? '', 80),
  }));
  const duplicateEdges = normalizedRules
    .map((row) => `${row.from}->${row.to}`)
    .filter((edge, index, arr) => arr.indexOf(edge) !== index);
  const unknownEndpointRules = normalizedRules.filter(
    (row) => !channels.has(row.from) || !channels.has(row.to),
  );
  const selfLoopRules = normalizedRules.filter((row) => row.from === row.to);
  const ok =
    normalizedRules.length > 0 &&
    duplicateEdges.length === 0 &&
    unknownEndpointRules.length === 0 &&
    selfLoopRules.length === 0;
  return {
    id: 'release_channel_promotion_rules_contract',
    ok,
    detail: ok
      ? `rules=${normalizedRules.length}`
      : `duplicates=${Array.from(new Set(duplicateEdges)).join(',') || 'none'};unknown=${unknownEndpointRules.map((row) => `${row.from}->${row.to}`).join(',') || 'none'};self_loops=${selfLoopRules.map((row) => `${row.from}->${row.to}`).join(',') || 'none'}`,
  };
}

function checkReleaseChannelCanonicalOrder(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channels = Array.isArray(policy?.channels)
    ? policy.channels.map((value: unknown) => cleanText(value, 80))
    : [];
  const canonical = ['alpha', 'beta', 'stable'];
  const ok = channels.length === canonical.length && channels.every((row, idx) => row === canonical[idx]);
  return {
    id: 'release_channel_canonical_order_contract',
    ok,
    detail: ok ? channels.join(',') : `channels=${channels.join(',') || 'missing'}`,
  };
}

function checkReleaseChannelDefaultAlpha(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channel = cleanText(policy?.default_channel ?? '', 80);
  const ok = channel === 'alpha';
  return {
    id: 'release_channel_default_alpha_contract',
    ok,
    detail: ok ? 'default=alpha' : `default=${channel || 'missing'}`,
  };
}

function checkReleaseChannelPromotionCanonicalMatrix(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const edges = rules.map((row: any) => `${cleanText(row?.from ?? '', 80)}->${cleanText(row?.to ?? '', 80)}`);
  const canonical = ['alpha->beta', 'beta->stable', 'alpha->stable'];
  const ok = edges.length === canonical.length && edges.every((row, idx) => row === canonical[idx]);
  return {
    id: 'release_channel_promotion_canonical_matrix_contract',
    ok,
    detail: ok ? `edges=${edges.join(',')}` : `edges=${edges.join(',') || 'missing'}`,
  };
}

function checkReleaseChannelPolicyTokenHygiene(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channels = Array.isArray(policy?.channels)
    ? policy.channels.map((value: unknown) => cleanText(value, 80)).filter(Boolean)
    : [];
  const channelSet = new Set(channels);
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const invalidChannels = channels.filter((row) => !isCanonicalToken(row));
  const invalidRules = rules.filter((row: any) => {
    const from = cleanText(row?.from ?? '', 80);
    const to = cleanText(row?.to ?? '', 80);
    return (
      !from ||
      !to ||
      from === to ||
      !isCanonicalToken(from) ||
      !isCanonicalToken(to) ||
      !channelSet.has(from) ||
      !channelSet.has(to)
    );
  });
  const ok = invalidChannels.length === 0 && invalidRules.length === 0;
  return {
    id: 'release_channel_policy_token_hygiene_contract',
    ok,
    detail: ok ? 'ok' : `invalid_channels=${invalidChannels.join(',') || 'none'};invalid_rules=${invalidRules.length}`,
  };
}

function checkReleaseChannelPolicyObjectContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), null);
  const ok = !!policy && typeof policy === 'object' && !Array.isArray(policy);
  return {
    id: 'release_channel_policy_object_contract',
    ok,
    detail: ok ? 'ok' : `invalid_policy_type:${typeof policy}`,
  };
}

function checkReleaseChannelPolicyChannelsNoDuplicateContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channelsRaw = Array.isArray(policy?.channels) ? policy.channels : [];
  const channels = channelsRaw.map((row: unknown) => cleanText(row, 80)).filter(Boolean);
  const duplicates = channels.filter((row, idx, arr) => arr.indexOf(row) !== idx);
  return {
    id: 'release_channel_policy_channels_no_duplicate_contract',
    ok: duplicates.length === 0,
    detail: duplicates.length === 0 ? 'ok' : Array.from(new Set(duplicates)).join(','),
  };
}

function checkReleaseChannelPolicyChannelsLowercaseContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channelsRaw = Array.isArray(policy?.channels) ? policy.channels : [];
  const channels = channelsRaw.map((row: unknown) => cleanText(row, 80)).filter(Boolean);
  const bad = channels.filter((row) => row !== row.toLowerCase());
  return {
    id: 'release_channel_policy_channels_lowercase_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelPolicyChannelsTrimmedContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channelsRaw = Array.isArray(policy?.channels) ? policy.channels : [];
  const bad = channelsRaw
    .map((row: unknown) => String(row ?? ''))
    .filter((row) => row.trim() !== row);
  return {
    id: 'release_channel_policy_channels_trimmed_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelDefaultLowercaseContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const channel = cleanText(policy?.default_channel ?? '', 80);
  const ok = !!channel && channel === channel.toLowerCase();
  return {
    id: 'release_channel_default_lowercase_contract',
    ok,
    detail: ok ? channel : channel || 'missing',
  };
}

function checkReleaseChannelPromotionRuleTokenPresenceContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const bad = rules
    .map((row: any) => ({
      from: cleanText(row?.from ?? '', 80),
      to: cleanText(row?.to ?? '', 80),
    }))
    .filter((row) => !row.from || !row.to)
    .map((row) => `${row.from || '<missing>'}->${row.to || '<missing>'}`);
  return {
    id: 'release_channel_promotion_rule_token_presence_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelPromotionRuleLowercaseContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const bad = rules
    .map((row: any) => ({
      from: cleanText(row?.from ?? '', 80),
      to: cleanText(row?.to ?? '', 80),
    }))
    .filter((row) => row.from !== row.from.toLowerCase() || row.to !== row.to.toLowerCase())
    .map((row) => `${row.from}->${row.to}`);
  return {
    id: 'release_channel_promotion_rule_lowercase_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelPromotionRuleTrimmedContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const bad = rules
    .map((row: any) => ({
      fromRaw: String(row?.from ?? ''),
      toRaw: String(row?.to ?? ''),
    }))
    .filter((row) => row.fromRaw.trim() !== row.fromRaw || row.toRaw.trim() !== row.toRaw)
    .map((row) => `${cleanText(row.fromRaw, 80)}->${cleanText(row.toRaw, 80)}`);
  return {
    id: 'release_channel_promotion_rule_trimmed_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelPromotionRuleNoStableSourceContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const bad = rules
    .map((row: any) => ({
      from: cleanText(row?.from ?? '', 80),
      to: cleanText(row?.to ?? '', 80),
    }))
    .filter((row) => row.from === 'stable')
    .map((row) => `${row.from}->${row.to}`);
  return {
    id: 'release_channel_promotion_rule_no_stable_source_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelPromotionRuleNoAlphaTargetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const bad = rules
    .map((row: any) => ({
      from: cleanText(row?.from ?? '', 80),
      to: cleanText(row?.to ?? '', 80),
    }))
    .filter((row) => row.to === 'alpha')
    .map((row) => `${row.from}->${row.to}`);
  return {
    id: 'release_channel_promotion_rule_no_alpha_target_contract',
    ok: bad.length === 0,
    detail: bad.length === 0 ? 'ok' : bad.join(','),
  };
}

function checkReleaseChannelPromotionRuleSourceCoverageContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const sourceCounts: Record<string, number> = { alpha: 0, beta: 0, stable: 0 };
  for (const row of rules) {
    const from = cleanText((row as any)?.from ?? '', 80);
    if (Object.prototype.hasOwnProperty.call(sourceCounts, from)) sourceCounts[from] += 1;
  }
  const ok = sourceCounts.alpha === 2 && sourceCounts.beta === 1 && sourceCounts.stable === 0;
  return {
    id: 'release_channel_promotion_rule_source_coverage_contract',
    ok,
    detail: ok
      ? 'alpha=2;beta=1;stable=0'
      : `alpha=${sourceCounts.alpha};beta=${sourceCounts.beta};stable=${sourceCounts.stable}`,
  };
}

function checkReleaseChannelPromotionRuleTargetCoverageContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const rules = Array.isArray(policy?.promotion_rules) ? policy.promotion_rules : [];
  const targetCounts: Record<string, number> = { alpha: 0, beta: 0, stable: 0 };
  for (const row of rules) {
    const to = cleanText((row as any)?.to ?? '', 80);
    if (Object.prototype.hasOwnProperty.call(targetCounts, to)) targetCounts[to] += 1;
  }
  const ok = targetCounts.alpha === 0 && targetCounts.beta === 1 && targetCounts.stable === 2;
  return {
    id: 'release_channel_promotion_rule_target_coverage_contract',
    ok,
    detail: ok
      ? 'alpha=0;beta=1;stable=2'
      : `alpha=${targetCounts.alpha};beta=${targetCounts.beta};stable=${targetCounts.stable}`,
  };
}

function checkCompatibilityPolicyBooleanFlagTypeContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const migrationGuideFlag = policy?.require_migration_guide;
  const deprecationNoticeFlag = policy?.require_deprecation_notice;
  const ok = typeof migrationGuideFlag === 'boolean' && typeof deprecationNoticeFlag === 'boolean';
  return {
    id: 'compatibility_policy_boolean_flag_type_contract',
    ok,
    detail: ok
      ? 'ok'
      : `require_migration_guide=${typeof migrationGuideFlag};require_deprecation_notice=${typeof deprecationNoticeFlag}`,
  };
}

function checkCompatibilityPolicyRequiredDeprecationDaysIntegerContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const days = Number(policy?.required_deprecation_days);
  const ok = Number.isInteger(days) && days > 0;
  return {
    id: 'compatibility_policy_required_deprecation_days_integer_contract',
    ok,
    detail: ok ? `days=${days}` : `days=${String(policy?.required_deprecation_days ?? 'missing')}`,
  };
}

function checkCompatibilityPolicyRequiredDeprecationDaysUpperBoundContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const days = Number(policy?.required_deprecation_days);
  const ok = Number.isFinite(days) && days <= 3650;
  return {
    id: 'compatibility_policy_required_deprecation_days_upper_bound_contract',
    ok,
    detail: ok ? `days=${days}` : `days=${String(policy?.required_deprecation_days ?? 'missing')}`,
  };
}

function checkCompatibilityPolicyRegistryPathExistsContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const registryPath = cleanText(policy?.registry_path ?? '', 240);
  const abs = path.resolve(root, registryPath || '__missing__');
  const exists = !!registryPath && fs.existsSync(abs);
  const parses = exists && readJson(abs, null) !== null;
  return {
    id: 'compatibility_policy_registry_path_exists_contract',
    ok: exists && parses,
    detail:
      exists && parses
        ? registryPath
        : `registry_path=${registryPath || 'missing'};exists=${exists};json=${parses}`,
  };
}

function checkDependencyPolicySecurityPatchSlaIntegerContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const days = Number(policy?.security_patch_sla_days);
  const ok = Number.isInteger(days) && days > 0 && days <= 365;
  return {
    id: 'dependency_policy_security_patch_sla_integer_contract',
    ok,
    detail: ok ? `days=${days}` : `days=${String(policy?.security_patch_sla_days ?? 'missing')}`,
  };
}

function checkDependencyPolicyVulnerabilityBudgetsIntegerContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const critical = Number(policy?.max_critical_vulnerabilities);
  const high = Number(policy?.max_high_vulnerabilities);
  const ok =
    Number.isInteger(critical) &&
    Number.isInteger(high) &&
    critical >= 0 &&
    high >= 0 &&
    critical <= high;
  return {
    id: 'dependency_policy_vulnerability_budgets_integer_contract',
    ok,
    detail: ok
      ? `critical=${critical};high=${high}`
      : `critical=${String(policy?.max_critical_vulnerabilities ?? 'missing')};high=${String(policy?.max_high_vulnerabilities ?? 'missing')}`,
  };
}

function checkDependencyPolicyRequiredEcosystemUniquenessContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const ecosystems = Array.isArray(policy?.dependabot_required_ecosystems)
    ? policy.dependabot_required_ecosystems.map((row: unknown) => cleanText(row, 80)).filter(Boolean)
    : [];
  const duplicates = ecosystems.filter((row, idx, arr) => arr.indexOf(row) !== idx);
  return {
    id: 'dependency_policy_required_ecosystem_uniqueness_contract',
    ok: duplicates.length === 0,
    detail: duplicates.length === 0 ? 'ok' : Array.from(new Set(duplicates)).join(','),
  };
}

function checkDependencyPolicyRequiredEcosystemCanonicalOrderContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const ecosystems = Array.isArray(policy?.dependabot_required_ecosystems)
    ? policy.dependabot_required_ecosystems.map((row: unknown) => cleanText(row, 80))
    : [];
  const expected = ['npm', 'cargo', 'github-actions'];
  const ok = ecosystems.length === expected.length && ecosystems.every((row, idx) => row === expected[idx]);
  return {
    id: 'dependency_policy_required_ecosystem_canonical_order_contract',
    ok,
    detail: ok ? ecosystems.join(',') : ecosystems.join(',') || 'missing',
  };
}

function checkCompatibilityPolicyTopLevelKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const drift = keysetDrift(policy, [
    'schema_id',
    'schema_version',
    'required_deprecation_days',
    'require_migration_guide',
    'require_deprecation_notice',
    'registry_path',
  ]);
  const violations: string[] = [];
  if (drift.missing.length > 0) violations.push(`missing=${drift.missing.join(',')}`);
  if (drift.unexpected.length > 0) violations.push(`unexpected=${drift.unexpected.join(',')}`);
  return {
    id: 'compatibility_policy_top_level_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkCompatibilityPolicyRegistryPathShapeContract(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const registryPath = cleanText(policy?.registry_path ?? '', 240);
  const ok =
    registryPath.startsWith('client/runtime/config/') &&
    registryPath.endsWith('.json') &&
    !registryPath.includes('..') &&
    !/\s/.test(registryPath);
  return {
    id: 'compatibility_policy_registry_path_shape_contract',
    ok,
    detail: ok ? registryPath : registryPath || 'missing',
  };
}

function checkDeprecationPolicy(root: string): GateCheck {
  const registryRel = 'client/runtime/config/api_cli_contract_registry.json';
  const policyRel = 'client/runtime/config/release_compatibility_policy.json';
  const registry = readJson(path.resolve(root, registryRel), {});
  const policy = readJson(path.resolve(root, policyRel), {});
  const minDays = Number(policy.required_deprecation_days ?? 90);
  const requireGuide = policy.require_migration_guide !== false;
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const violations: string[] = [];
  for (const row of contracts) {
    const name = cleanText(row?.name ?? 'unknown', 120);
    const days = Number(row?.deprecation_window_days ?? 0);
    if (!Number.isFinite(days) || days < minDays) {
      violations.push(`${name}:deprecation_window_days_lt_${minDays}`);
    }
    const status = cleanText(row?.status ?? '', 40).toLowerCase();
    if (status === 'deprecated' && requireGuide) {
      const guide = cleanText(row?.migration_guide ?? '', 240);
      const notice = cleanText(row?.deprecation_notice ?? '', 240);
      if (!guide || !notice) violations.push(`${name}:deprecated_missing_migration_guide_or_notice`);
    }
  }
  return {
    id: 'compatibility_deprecation_policy',
    ok: violations.length === 0,
    detail: violations.length ? violations.join('; ') : 'ok',
  };
}

function checkCompatibilityRequiredDeprecationFloor(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const requiredDays = Number(policy?.required_deprecation_days ?? 0);
  const ok = Number.isInteger(requiredDays) && requiredDays >= 90 && requiredDays <= 365;
  return {
    id: 'compatibility_required_deprecation_floor_contract',
    ok,
    detail: ok ? `required_deprecation_days=${requiredDays}` : `required_deprecation_days=${String(policy?.required_deprecation_days ?? 'missing')}`,
  };
}

function checkCompatibilityRegistrySchema(root: string): GateCheck {
  const policyRel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, policyRel), {});
  const registryPath = cleanText(policy?.registry_path ?? '', 240);
  const registry = registryPath ? readJson(path.resolve(root, registryPath), null) : null;
  const ok =
    !!registry &&
    cleanText(registry?.schema_id ?? '', 120) === 'api_cli_contract_registry' &&
    cleanText(registry?.schema_version ?? '', 40) === '1.0';
  return {
    id: 'compatibility_registry_schema_contract',
    ok,
    detail: ok ? `registry=${registryPath}` : `registry=${registryPath || 'missing'};schema_id=${cleanText(registry?.schema_id ?? '', 120) || 'missing'}`,
  };
}

function checkCompatibilityRegistryNameUniqueness(root: string): GateCheck {
  const registryRel = 'client/runtime/config/api_cli_contract_registry.json';
  const registry = readJson(path.resolve(root, registryRel), {});
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const names = contracts.map((row) => cleanText(row?.name ?? '', 120));
  const duplicates = names.filter((value, index, arr) => value && arr.indexOf(value) !== index);
  const malformed = names.filter((value) => !value || !isCanonicalNameToken(value));
  const ok = duplicates.length === 0 && malformed.length === 0;
  return {
    id: 'compatibility_registry_name_uniqueness_contract',
    ok,
    detail: ok
      ? `contracts=${contracts.length}`
      : `duplicates=${Array.from(new Set(duplicates)).join(',') || 'none'};malformed=${Array.from(new Set(malformed)).join(',') || 'none'}`,
  };
}

function checkCompatibilityRegistryDeprecatedPayload(root: string): GateCheck {
  const registryRel = 'client/runtime/config/api_cli_contract_registry.json';
  const registry = readJson(path.resolve(root, registryRel), {});
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const deprecatedRows = contracts.filter((row) => cleanText(row?.status ?? '', 40).toLowerCase() === 'deprecated');
  const invalidRows = deprecatedRows.filter((row) => {
    const guide = cleanText(row?.migration_guide ?? '', 240);
    const guidePath = cleanText(guide.split('#')[0] ?? '', 240);
    const notice = cleanText(row?.deprecation_notice ?? '', 240);
    return !guide || !guidePath || !fs.existsSync(path.resolve(root, guidePath)) || !notice;
  });
  const ok = deprecatedRows.length > 0 && invalidRows.length === 0;
  return {
    id: 'compatibility_registry_deprecated_payload_contract',
    ok,
    detail: ok
      ? `deprecated_contracts=${deprecatedRows.length}`
      : `deprecated_contracts=${deprecatedRows.length};invalid=${invalidRows.map((row) => cleanText(row?.name ?? 'unknown', 120)).join(',') || 'none'}`,
  };
}

function checkCompatibilityRegistryDeprecationWindowFloor(root: string): GateCheck {
  const policyRel = 'client/runtime/config/release_compatibility_policy.json';
  const registryRel = 'client/runtime/config/api_cli_contract_registry.json';
  const policy = readJson(path.resolve(root, policyRel), {});
  const registry = readJson(path.resolve(root, registryRel), {});
  const floor = Number(policy?.required_deprecation_days ?? 0);
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const failing = contracts.filter((row) => {
    const days = Number(row?.deprecation_window_days ?? NaN);
    return !Number.isInteger(days) || days < floor;
  });
  const ok = Number.isInteger(floor) && floor > 0 && failing.length === 0;
  return {
    id: 'compatibility_registry_deprecation_window_floor_contract',
    ok,
    detail: ok ? `floor=${floor}` : `floor=${String(policy?.required_deprecation_days ?? 'missing')};violations=${failing.map((row) => cleanText(row?.name ?? 'unknown', 120)).join(',') || 'none'}`,
  };
}

function checkCompatibilityPolicySchema(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const schemaId = cleanText(policy?.schema_id ?? '', 80);
  const schemaVersion = cleanText(policy?.schema_version ?? '', 40);
  const ok = schemaId === 'release_compatibility_policy' && schemaVersion === '1.0';
  return {
    id: 'compatibility_policy_schema_contract',
    ok,
    detail: ok ? 'schema_id/version_ok' : `schema_id=${schemaId || 'missing'};schema_version=${schemaVersion || 'missing'}`,
  };
}

function checkCompatibilityPolicyRegistryPath(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const registryPath = cleanText(policy?.registry_path ?? '', 240);
  const hasNoTraversal = registryPath.length > 0 && !registryPath.includes('..') && !registryPath.startsWith('/');
  const exists = hasNoTraversal && fs.existsSync(path.resolve(root, registryPath));
  const matchesDefault = registryPath === 'client/runtime/config/api_cli_contract_registry.json';
  const ok = hasNoTraversal && exists && matchesDefault;
  return {
    id: 'compatibility_policy_registry_path_contract',
    ok,
    detail: ok ? registryPath : `registry_path=${registryPath || 'missing'};exists=${exists};matches_default=${matchesDefault}`,
  };
}

function checkCompatibilityPolicyNoticeFlag(root: string): GateCheck {
  const rel = 'client/runtime/config/release_compatibility_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const requireMigrationGuide = policy?.require_migration_guide === true;
  const requireDeprecationNotice = policy?.require_deprecation_notice === true;
  const ok = requireMigrationGuide && requireDeprecationNotice;
  return {
    id: 'compatibility_policy_notice_flag_contract',
    ok,
    detail: ok
      ? 'require_migration_guide_and_deprecation_notice_enabled'
      : `require_migration_guide=${String(requireMigrationGuide)};require_deprecation_notice=${String(requireDeprecationNotice)}`,
  };
}

function checkApiCliRegistryTopLevelKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/api_cli_contract_registry.json';
  const registry = readJson(path.resolve(root, rel), {});
  const drift = keysetDrift(registry, [
    'schema_id',
    'schema_version',
    'api_contracts',
    'cli_contracts',
  ]);
  const violations: string[] = [];
  if (drift.missing.length > 0) violations.push(`missing=${drift.missing.join(',')}`);
  if (drift.unexpected.length > 0) violations.push(`unexpected=${drift.unexpected.join(',')}`);
  return {
    id: 'api_cli_registry_top_level_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkApiCliRegistryContractVersionSemverContract(root: string): GateCheck {
  const rel = 'client/runtime/config/api_cli_contract_registry.json';
  const registry = readJson(path.resolve(root, rel), {});
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const invalid = contracts.filter((row) => !/^\d+\.\d+\.\d+$/.test(cleanText(row?.version ?? '', 40)));
  return {
    id: 'api_cli_registry_contract_version_semver_contract',
    ok: invalid.length === 0,
    detail: invalid.length === 0 ? 'ok' : invalid.map((row) => cleanText(row?.name ?? 'unknown', 120)).join(','),
  };
}

function checkApiCliRegistryContractTokenShapeContract(root: string): GateCheck {
  const rel = 'client/runtime/config/api_cli_contract_registry.json';
  const registry = readJson(path.resolve(root, rel), {});
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const invalidNameRows = contracts.filter((row) => !isCanonicalNameToken(cleanText(row?.name ?? '', 120)));
  const invalidStatusRows = contracts.filter((row) => {
    const status = cleanText(row?.status ?? '', 40).toLowerCase();
    return status !== 'active' && status !== 'deprecated';
  });
  return {
    id: 'api_cli_registry_contract_token_shape_contract',
    ok: invalidNameRows.length === 0 && invalidStatusRows.length === 0,
    detail:
      invalidNameRows.length === 0 && invalidStatusRows.length === 0
        ? 'ok'
        : `invalid_names=${invalidNameRows.map((row) => cleanText(row?.name ?? 'unknown', 120)).join(',') || 'none'};invalid_status=${invalidStatusRows.map((row) => cleanText(row?.name ?? 'unknown', 120)).join(',') || 'none'}`,
  };
}

function checkApiCliRegistryDeprecatedGuidePathContract(root: string): GateCheck {
  const rel = 'client/runtime/config/api_cli_contract_registry.json';
  const registry = readJson(path.resolve(root, rel), {});
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const deprecated = contracts.filter((row) => cleanText(row?.status ?? '', 40).toLowerCase() === 'deprecated');
  const invalid = deprecated.filter((row) => {
    const migrationGuide = cleanText(row?.migration_guide ?? '', 260);
    const guidePath = cleanText(migrationGuide.split('#')[0] ?? '', 260);
    return (
      !migrationGuide ||
      !guidePath ||
      !guidePath.startsWith('docs/client/') ||
      !fs.existsSync(path.resolve(root, guidePath))
    );
  });
  return {
    id: 'api_cli_registry_deprecated_guide_path_contract',
    ok: invalid.length === 0,
    detail: invalid.length === 0 ? 'ok' : invalid.map((row) => cleanText(row?.name ?? 'unknown', 120)).join(','),
  };
}

function checkSchemaVersioningTargetsCanonicalSet(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const ids = Array.isArray(policy?.targets)
    ? policy.targets.map((row: any) => cleanText(row?.id ?? '', 80))
    : [];
  const canonical = ['proposal_admission', 'system_budget'];
  const ok = ids.length === canonical.length && ids.every((row, idx) => row === canonical[idx]);
  return {
    id: 'schema_versioning_targets_canonical_set_contract',
    ok,
    detail: ok ? ids.join(',') : `ids=${ids.join(',') || 'missing'}`,
  };
}

function checkSchemaVersioningTargetsUniqueIds(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const ids = Array.isArray(policy?.targets)
    ? policy.targets.map((row: any) => cleanText(row?.id ?? '', 80)).filter(Boolean)
    : [];
  const duplicates = ids.filter((value, index, arr) => arr.indexOf(value) !== index);
  const ok = ids.length > 0 && duplicates.length === 0;
  return {
    id: 'schema_versioning_targets_unique_id_contract',
    ok,
    detail: ok ? `targets=${ids.length}` : `duplicates=${Array.from(new Set(duplicates)).join(',') || 'none'}`,
  };
}

function checkSchemaVersioningTargetsSchemaAlignment(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const targets = Array.isArray(policy?.targets) ? policy.targets : [];
  const invalid = targets.filter((row: any) => {
    const id = cleanText(row?.id ?? '', 80);
    const requiredSchemaId = cleanText(row?.required_schema_id ?? '', 80);
    const kind = cleanText(row?.kind ?? '', 40);
    return !id || id !== requiredSchemaId || kind !== 'json';
  });
  const ok = targets.length > 0 && invalid.length === 0;
  return {
    id: 'schema_versioning_targets_schema_alignment_contract',
    ok,
    detail: ok ? `targets=${targets.length}` : `invalid_rows=${invalid.map((row: any) => cleanText(row?.id ?? 'unknown', 80)).join(',') || 'none'}`,
  };
}

function checkSchemaVersioningTargetsMinVersion(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const targets = Array.isArray(policy?.targets) ? policy.targets : [];
  const invalid = targets.filter((row: any) => {
    const minVersion = cleanText(row?.min_schema_version ?? '', 40);
    return !versionAtLeast(minVersion, '1.0');
  });
  const ok = targets.length > 0 && invalid.length === 0;
  return {
    id: 'schema_versioning_targets_min_version_contract',
    ok,
    detail: ok ? `targets=${targets.length}` : `invalid_rows=${invalid.map((row: any) => cleanText(row?.id ?? 'unknown', 80)).join(',') || 'none'}`,
  };
}

function checkSchemaVersioningOutputsCanonicalPaths(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const latestPath = cleanText(policy?.outputs?.latest_path ?? '', 260);
  const historyPath = cleanText(policy?.outputs?.history_path ?? '', 260);
  const ok =
    latestPath === 'local/state/contracts/schema_versioning_gate/latest.json' &&
    historyPath === 'local/state/contracts/schema_versioning_gate/history.jsonl';
  return {
    id: 'schema_versioning_outputs_paths_contract',
    ok,
    detail: ok ? 'latest/history_paths_ok' : `latest=${latestPath || 'missing'};history=${historyPath || 'missing'}`,
  };
}

function checkSchemaVersioningPolicyTopLevelKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const drift = keysetDrift(policy, ['version', 'enabled', 'targets', 'migrations', 'outputs']);
  const violations: string[] = [];
  if (drift.missing.length > 0) violations.push(`missing=${drift.missing.join(',')}`);
  if (drift.unexpected.length > 0) violations.push(`unexpected=${drift.unexpected.join(',')}`);
  return {
    id: 'schema_versioning_policy_top_level_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkSchemaVersioningOutputsKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const outputs = policy?.outputs ?? {};
  const drift = keysetDrift(outputs, ['latest_path', 'history_path']);
  const violations: string[] = [];
  if (drift.missing.length > 0) violations.push(`missing=${drift.missing.join(',')}`);
  if (drift.unexpected.length > 0) violations.push(`unexpected=${drift.unexpected.join(',')}`);
  return {
    id: 'schema_versioning_outputs_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkSchemaVersioningTargetPathShapeContract(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const targets = Array.isArray(policy?.targets) ? policy.targets : [];
  const invalid = targets.filter((row: any) => {
    const p = cleanText(row?.path ?? '', 400);
    return (
      !p.startsWith('client/runtime/config/contracts/') ||
      !p.endsWith('.schema.json') ||
      p.includes('..') ||
      /\s/.test(p)
    );
  });
  return {
    id: 'schema_versioning_target_path_shape_contract',
    ok: invalid.length === 0,
    detail: invalid.length === 0 ? 'ok' : invalid.map((row: any) => cleanText(row?.id ?? 'unknown', 80)).join(','),
  };
}

function checkSchemaVersioningTargetRowKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const targets = Array.isArray(policy?.targets) ? policy.targets : [];
  const violations: string[] = [];
  for (const row of targets) {
    const drift = keysetDrift(row, [
      'id',
      'path',
      'required_schema_id',
      'min_schema_version',
      'kind',
    ]);
    if (drift.missing.length > 0 || drift.unexpected.length > 0) {
      const targetId = cleanText((row as any)?.id ?? '', 80) || '<unknown>';
      violations.push(
        `${targetId}:missing=${drift.missing.join(',') || 'none'};unexpected=${drift.unexpected.join(',') || 'none'}`,
      );
    }
  }
  return {
    id: 'schema_versioning_target_row_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkSchemaMigrationPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const targets = Array.isArray(policy.targets) ? policy.targets : [];
  const missing: string[] = [];
  for (const row of targets) {
    const targetPath = cleanText(row?.path ?? '', 400);
    if (!targetPath) continue;
    if (!fs.existsSync(path.resolve(root, targetPath))) missing.push(targetPath);
  }
  const migrations = policy?.migrations ?? {};
  const hasMigrationGuard =
    typeof migrations === 'object' &&
    cleanText(migrations.target_default_version ?? '', 40).length > 0 &&
    typeof migrations.allow_add_missing_fields_only === 'boolean';
  const ok = missing.length === 0 && hasMigrationGuard;
  return {
    id: 'config_migration_gate',
    ok,
    detail: ok
      ? `targets=${targets.length};migration_guard=ok`
      : `missing_targets=${missing.join(',') || 'none'};migration_guard=${hasMigrationGuard}`,
  };
}

function checkSchemaMigrationOutputsPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const version = cleanText(policy?.version ?? '', 40);
  const enabled = policy?.enabled === true;
  const latestPath = cleanText(policy?.outputs?.latest_path ?? '', 260);
  const historyPath = cleanText(policy?.outputs?.history_path ?? '', 260);
  const outputsShapeOk =
    latestPath.startsWith('local/state/contracts/schema_versioning_gate/') &&
    latestPath.endsWith('.json') &&
    historyPath.startsWith('local/state/contracts/schema_versioning_gate/') &&
    historyPath.endsWith('.jsonl') &&
    latestPath !== historyPath;
  const ok = version === '1.0' && enabled && outputsShapeOk;
  return {
    id: 'schema_migration_outputs_contract',
    ok,
    detail: ok
      ? `version=${version};latest=${latestPath};history=${historyPath}`
      : `version=${version || 'missing'};enabled=${String(enabled)};latest=${latestPath || 'missing'};history=${historyPath || 'missing'}`,
  };
}

function checkDependencyPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const blocked = Array.isArray(policy.blocked_packages) ? policy.blocked_packages : [];
  const slaDays = Number(policy.security_patch_sla_days ?? 0);
  const hasSla = Number.isFinite(slaDays) && slaDays > 0 && slaDays <= 30;
  const hasBlocked = blocked.length > 0;
  return {
    id: 'dependency_update_policy',
    ok: hasSla && hasBlocked,
    detail: hasSla && hasBlocked ? `sla_days=${slaDays};blocked=${blocked.length}` : `invalid:${rel}`,
  };
}

function checkDependencyPolicySchemaAndEcosystems(root: string): GateCheck {
  const policyRel = 'client/runtime/config/dependency_update_policy.json';
  const dependabotRel = '.github/dependabot.yml';
  const policy = readJson(path.resolve(root, policyRel), {});
  const dependabotRaw = fs.existsSync(path.resolve(root, dependabotRel))
    ? fs.readFileSync(path.resolve(root, dependabotRel), 'utf8')
    : '';
  const schemaId = cleanText(policy?.schema_id ?? '', 80);
  const schemaVersion = cleanText(policy?.schema_version ?? '', 40);
  const requiredEcosystems = Array.isArray(policy?.dependabot_required_ecosystems)
    ? policy.dependabot_required_ecosystems.map((value: unknown) => cleanText(value, 80)).filter(Boolean)
    : [];
  const ecosystemMatches = Array.from(
    dependabotRaw.matchAll(/package-ecosystem:\s*["']?([a-zA-Z0-9_-]+)["']?/g),
  ).map((match) => cleanText(match[1] ?? '', 80)).filter(Boolean);
  const ecosystemSet = new Set(ecosystemMatches);
  const missingInDependabot = requiredEcosystems.filter((ecosystem) => !ecosystemSet.has(ecosystem));
  const ok =
    schemaId === 'dependency_update_policy' &&
    schemaVersion === '1.0' &&
    requiredEcosystems.length > 0 &&
    missingInDependabot.length === 0;
  return {
    id: 'dependency_policy_schema_and_ecosystem_contract',
    ok,
    detail: ok
      ? `schema=${schemaId}@${schemaVersion};ecosystems=${requiredEcosystems.join(',')}`
      : `schema=${schemaId || 'missing'}@${schemaVersion || 'missing'};missing_in_dependabot=${missingInDependabot.join(',') || 'none'}`,
  };
}

function checkDependencyPolicyBlockedPackages(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const blockedPackages = Array.isArray(policy?.blocked_packages) ? policy.blocked_packages : [];
  const rows = blockedPackages.map((row: any) => ({
    ecosystem: cleanText(row?.ecosystem ?? '', 80),
    name: cleanText(row?.name ?? '', 120),
    reason: cleanText(row?.reason ?? '', 240),
  }));
  const duplicates = rows
    .map((row) => `${row.ecosystem}:${row.name}`)
    .filter((value, index, arr) => arr.indexOf(value) !== index);
  const malformed = rows.filter(
    (row) => !row.ecosystem || !row.name || !row.reason || !isCanonicalToken(row.ecosystem),
  );
  const ok = rows.length > 0 && duplicates.length === 0 && malformed.length === 0;
  return {
    id: 'dependency_policy_blocked_packages_contract',
    ok,
    detail: ok
      ? `blocked_packages=${rows.length}`
      : `duplicates=${Array.from(new Set(duplicates)).join(',') || 'none'};malformed=${malformed.map((row) => `${row.ecosystem}:${row.name}`).join(',') || 'none'}`,
  };
}

function checkDependencyVulnerabilityBudget(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const maxCritical = Number(policy?.max_critical_vulnerabilities ?? NaN);
  const maxHigh = Number(policy?.max_high_vulnerabilities ?? NaN);
  const ok = maxCritical === 0 && maxHigh === 0;
  return {
    id: 'dependency_vulnerability_budget_contract',
    ok,
    detail: ok ? 'max_critical=0;max_high=0' : `max_critical=${String(policy?.max_critical_vulnerabilities ?? 'missing')};max_high=${String(policy?.max_high_vulnerabilities ?? 'missing')}`,
  };
}

function checkDependencyEcosystemCanonicalSet(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const ecosystems = Array.isArray(policy?.dependabot_required_ecosystems)
    ? policy.dependabot_required_ecosystems.map((value: unknown) => cleanText(value, 80))
    : [];
  const canonical = ['npm', 'cargo', 'github-actions'];
  const ok = ecosystems.length === canonical.length && ecosystems.every((row, idx) => row === canonical[idx]);
  return {
    id: 'dependency_ecosystem_canonical_set_contract',
    ok,
    detail: ok ? ecosystems.join(',') : `ecosystems=${ecosystems.join(',') || 'missing'}`,
  };
}

function checkDependencyBlocklistBaseline(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const blocked = Array.isArray(policy?.blocked_packages) ? policy.blocked_packages : [];
  const signatures = new Set(
    blocked.map((row: any) => `${cleanText(row?.ecosystem ?? '', 80)}:${cleanText(row?.name ?? '', 120)}`),
  );
  const required = ['npm:event-stream', 'cargo:openssl-sys'];
  const missing = required.filter((sig) => !signatures.has(sig));
  const missingReasons = blocked.filter((row: any) => {
    const reason = cleanText(row?.reason ?? '', 240);
    return !reason;
  });
  const ok = missing.length === 0 && missingReasons.length === 0;
  return {
    id: 'dependency_blocklist_baseline_contract',
    ok,
    detail: ok
      ? `blocked=${blocked.length}`
      : `missing=${missing.join(',') || 'none'};missing_reason_rows=${missingReasons.length}`,
  };
}

function checkDependabotScheduleWeeklyMonday(root: string): GateCheck {
  const rel = '.github/dependabot.yml';
  const raw = fs.readFileSync(path.resolve(root, rel), 'utf8');
  const rows = Array.from(
    raw.matchAll(
      /-\s*package-ecosystem:\s*"([^"]+)"[\s\S]*?schedule:\s*[\r\n]+\s*interval:\s*"([^"]+)"[\s\S]*?\s*day:\s*"([^"]+)"[\s\S]*?open-pull-requests-limit:\s*(\d+)/g,
    ),
  ).map((match) => ({
    ecosystem: cleanText(match[1] ?? '', 80),
    interval: cleanText(match[2] ?? '', 80),
    day: cleanText(match[3] ?? '', 80),
    limit: Number(match[4] ?? NaN),
  }));
  const requiredEcosystems = new Set(['github-actions', 'cargo', 'npm']);
  const missingEcosystems = Array.from(requiredEcosystems).filter(
    (ecosystem) => !rows.some((row) => row.ecosystem === ecosystem),
  );
  const invalidRows = rows.filter(
    (row) => row.interval !== 'weekly' || row.day !== 'monday' || row.limit !== 10,
  );
  const ok = rows.length >= 3 && missingEcosystems.length === 0 && invalidRows.length === 0;
  return {
    id: 'dependabot_schedule_weekly_monday_contract',
    ok,
    detail: ok
      ? `ecosystems=${rows.map((row) => row.ecosystem).join(',')}`
      : `missing=${missingEcosystems.join(',') || 'none'};invalid_rows=${invalidRows.map((row) => row.ecosystem).join(',') || 'none'}`,
  };
}

function checkDependabotSchedule(root: string): GateCheck {
  const rel = '.github/dependabot.yml';
  try {
    const raw = fs.readFileSync(path.resolve(root, rel), 'utf8');
    const ecosystems = new Set<string>();
    const re = /package-ecosystem:\s*["']?([a-zA-Z0-9_-]+)["']?/g;
    let match: RegExpExecArray | null = null;
    while ((match = re.exec(raw)) != null) {
      const ecosystem = cleanText(match[1] ?? '', 60);
      if (ecosystem) ecosystems.add(ecosystem);
    }
    const ok = ecosystems.has('npm') && ecosystems.has('cargo') && ecosystems.has('github-actions');
    return {
      id: 'dependabot_schedule',
      ok,
      detail: ok ? `ecosystems=${Array.from(ecosystems).sort().join(',')}` : `missing required ecosystems in ${rel}`,
    };
  } catch {
    return { id: 'dependabot_schedule', ok: false, detail: `invalid:${rel}` };
  }
}

function checkReleaseWorkflowChannelResolutionContract(root: string): GateCheck {
  const rel = '.github/workflows/release.yml';
  const source = fs.readFileSync(path.resolve(root, rel), 'utf8');
  const requiredNeedles = [
    'Resolve release channel',
    'client/runtime/config/release_channel_policy.json',
    'if(!["alpha","beta","stable"].includes(ch)) ch="alpha";',
    'echo "release_channel=${CHANNEL_INPUT}" >> "$GITHUB_OUTPUT"',
  ];
  const missing = requiredNeedles.filter((needle) => !source.includes(needle));
  return {
    id: 'release_workflow_channel_resolution_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function checkReleaseWorkflowReleasePolicyEnforcementContract(root: string): GateCheck {
  const rel = '.github/workflows/release.yml';
  const source = fs.readFileSync(path.resolve(root, rel), 'utf8');
  const requiredNeedles = [
    'Release Runtime Contract Gate',
    'npm run -s ops:release-contract:gate',
    'Release policy + compatibility + migration gate',
    'release_policy_gate.ts',
    '--strict=1',
    'Enforce mandatory release proof-pack artifacts',
    'release_proof_pack_contract_failed',
    'runtime_trusted_core_report_current.json',
    'layer2_lane_parity_guard_current.json',
    'layer2_receipt_replay_current.json',
  ];
  const missing = requiredNeedles.filter((needle) => !source.includes(needle));
  return {
    id: 'release_workflow_release_policy_enforcement_contract',
    ok: missing.length === 0,
    detail: missing.length === 0 ? 'ok' : `missing=${missing.join(',')}`,
  };
}

function checkReleaseWorkflowHeredocIndentContract(root: string): GateCheck {
  const source = readReleaseWorkflow(root);
  const heredocOpeners = countRegex(source, /<<'NODE'/g);
  const indentedClosers = countRegex(source, /^ {2,}NODE$/gm);
  const nakedClosers = countRegex(source, /^NODE$/gm);
  const ok = heredocOpeners > 0 && heredocOpeners === indentedClosers && nakedClosers === 0;
  return {
    id: 'release_workflow_heredoc_indent_contract',
    ok,
    detail: ok
      ? `heredocs=${heredocOpeners}`
      : `openers=${heredocOpeners};indented_closers=${indentedClosers};naked_closers=${nakedClosers}`,
  };
}

function checkReleaseWorkflowTriggerContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_trigger_contract', [
    'on:',
    'push:',
    'branches:',
    '- main',
    'workflow_dispatch:',
    'release_channel:',
    'default: alpha',
    'options:',
    '- alpha',
    '- beta',
    '- stable',
  ]);
}

function checkReleaseWorkflowPermissionsContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_permissions_contract', [
    'permissions:',
    'contents: write',
    'packages: write',
    'attestations: write',
    'id-token: write',
  ]);
}

function checkReleaseWorkflowWindowsPrebuiltJobContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_windows_prebuilt_job_contract', [
    'windows-prebuilt:',
    'runs-on: windows-latest',
    'Build Windows prebuilt binaries',
    'infring-ops-x86_64-pc-windows-msvc.exe',
    'infringd-x86_64-pc-windows-msvc.exe',
    'conduit_daemon-x86_64-pc-windows-msvc.exe',
    'infring-pure-workspace-x86_64-pc-windows-msvc.exe',
  ]);
}

function checkReleaseWorkflowReleaseNeedsWindowsContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_release_needs_windows_contract', [
    'release:',
    'needs:',
    '- windows-prebuilt',
  ]);
}

function checkReleaseWorkflowToolchainContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_toolchain_contract', [
    'Setup Node',
    'node-version: "22"',
    'Setup Rust',
    'targets: x86_64-unknown-linux-musl',
    'Install dependencies',
    'run: npm ci',
  ]);
}

function checkReleaseWorkflowWindowsArtifactRoundTripContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_windows_artifact_roundtrip_contract', [
    'Upload Windows prebuilt artifact bundle',
    'name: release-windows-prebuilt',
    'Download Windows prebuilt artifacts',
    'name: release-windows-prebuilt',
    'if-no-files-found: error',
  ]);
}

function checkReleaseWorkflowWindowsBaselineEnforcementContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_windows_baseline_enforcement_contract', [
    'Enforce Windows prebuilt artifact baseline',
    '"infring-ops-x86_64-pc-windows-msvc.exe"',
    '"infringd-x86_64-pc-windows-msvc.exe"',
    '"infringd-tiny-max-x86_64-pc-windows-msvc.exe"',
    '"conduit_daemon-x86_64-pc-windows-msvc.exe"',
    '"infring-pure-workspace-x86_64-pc-windows-msvc.exe"',
    '"infring-pure-workspace-tiny-max-x86_64-pc-windows-msvc.exe"',
  ]);
}

function checkReleaseWorkflowPolicyAndContractGatesContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_policy_and_contract_gates_contract', [
    'Release Runtime Contract Gate',
    'npm run -s ops:release-contract:gate',
    'Release policy + compatibility + migration gate',
    'release_policy_gate.ts',
    '--strict=1',
  ]);
}

function checkReleaseWorkflowScorecardContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_scorecard_contract', [
    'Generate release scorecard',
    '--policy=core/local/artifacts/release_policy_gate_current.json',
    '--closure=core/local/artifacts/production_readiness_closure_gate_current.json',
    '--support-bundle=core/local/artifacts/support_bundle_latest.json',
    '--require-release-artifacts=1',
  ]);
}

function checkReleaseWorkflowProofPackUploadContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_proof_pack_upload_contract', [
    'Upload release proof-pack evidence',
    'name: release-proof-pack-${{ steps.semver.outputs.tag }}',
    'core/local/artifacts/release_proof_pack_current.json',
    'validation/release_gates/proof_packs/${{ steps.semver.outputs.tag }}/**',
  ]);
}

function checkReleaseWorkflowProofPackMandatoryArtifactGateContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_proof_pack_mandatory_artifact_gate_contract', [
    'Enforce mandatory release proof-pack artifacts',
    'required_missing',
    'category_threshold_failure_count',
    'layer2_lane_parity_guard_current.json',
    'layer2_receipt_replay_current.json',
    'runtime_trusted_core_report_current.json',
    'release_proof_pack_contract_failed',
  ]);
}

function checkReleaseWorkflowRuntimeEvidenceChainContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_runtime_evidence_chain_contract', [
    'Release topology, compatibility, and recovery evidence',
    'npm run -s ops:runtime-proof:verify',
    'npm run -s ops:workspace-tooling:release-proof',
    'npm run -s ops:queue-backpressure:policy:gate',
    'npm run -s ops:boundedness:release-gate',
    'npm run -s ops:layer2:parity:guard',
    'npm run -s ops:layer2:receipt:replay',
    'npm run -s ops:gateway-status:manifest',
    'npm run -s ops:trusted-core:report',
    'npm run -s ops:release:proof-pack -- --version=${{ steps.semver.outputs.tag }}',
  ]);
}

function checkReleaseWorkflowCanaryRollbackContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_canary_rollback_contract', [
    'Canary gate with rollback enforcement',
    'release-gate-canary-rollback-enforcer gate --strict=1',
    'Auto rollback stale tag on canary failure',
    'git push origin ":refs/tags/${TAG}"',
  ]);
}

function checkReleaseWorkflowSigningAndSbomContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_signing_and_sbom_contract', [
    'Install Syft',
    'Generate SPDX SBOM artifacts',
    'Sign release artifacts',
    'release_public.pem',
    'openssl dgst -sha256 -sign /tmp/release_private.pem',
  ]);
}

function checkReleaseWorkflowChecksumAndAssetsContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_checksum_and_assets_contract', [
    'Generate release SHA256SUMS',
    'client/runtime/local/state/release/artifacts/SHA256SUMS',
    'release_proof_pack_current.json',
    'runtime_proof_verify_current.json',
    'runtime_multi_day_soak_evidence_current.json',
    'release_verdict_current.json',
  ]);
}

function checkReleaseWorkflowTagUniquenessContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_tag_uniqueness_contract', [
    'Enforce release tag uniqueness progression',
    'release_tag_uniqueness_gate_current.json',
    'release_tag_points_to_same_commit_as_previous_tag',
    'release_tag_uniqueness_gate_failed',
  ]);
}

function checkReleaseWorkflowReleasePublishContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_release_publish_contract', [
    'Publish GA release',
    'softprops/action-gh-release@v2',
    'generate_release_notes: true',
    'fail_on_unmatched_files: true',
    'client/runtime/local/state/release/artifacts/SHA256SUMS',
    'core/local/artifacts/release_verdict_current.json',
  ]);
}

function checkReleaseWorkflowClosureRefreshContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_closure_refresh_contract', [
    'Refresh closure evidence after final scorecard',
    'npm run -s ops:production-closure:gate',
    'Generate release verdict',
    'npm run -s ops:release:verdict',
    'Refresh support bundle incident truth package',
    'npm run -s ops:support-bundle:export',
  ]);
}

function checkReleaseWorkflowSemverResolutionContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_semver_resolution_contract', [
    'Resolve semantic release plan (conventional commits)',
    'release_semver_contract.ts run --strict=1 --write=1',
    'echo "release_ready=${RELEASE_READY}" >> "$GITHUB_OUTPUT"',
    'echo "tag=${TAG}" >> "$GITHUB_OUTPUT"',
    'echo "release_channel=${RELEASE_CHANNEL}" >> "$GITHUB_OUTPUT"',
  ]);
}

function checkReleaseWorkflowLicensingManifestContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_licensing_manifest_contract', [
    'Generate release licensing manifest',
    'LICENSE_MATRIX.json',
    'release_licensing_manifest.json',
    'release_bundle_spdx',
    'source_matrix_sha256',
  ]);
}

function checkReleaseWorkflowSizeGateContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_size_gate_contract', [
    'Enforce rich-host static infringd size gate (35 MB)',
    '--max-mb=35',
    'infringd_static_size_mb',
    'Append static binary size to release notes',
  ]);
}

function checkInstallerChecksumVerification(root: string): GateCheck {
  const installPath = path.resolve(root, 'install.sh');
  const script = fs.readFileSync(installPath, 'utf8');
  const strictDefault = /INSTALL_ALLOW_UNVERIFIED_ASSETS="\$\{INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS:-\$\{INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS:-0\}\}"/.test(
    script
  );
  const hasVerifyFn = /verify_downloaded_asset\(\)/.test(script);
  const usedOnDownload = /verify_downloaded_asset "\$version_tag" "\$asset_name" "\$asset_out"/.test(script);
  const ok = strictDefault && hasVerifyFn && usedOnDownload;
  return {
    id: 'installer_checksum_verification',
    ok,
    detail: ok
      ? 'checksum verification enabled by default'
      : `strict_default=${strictDefault};verify_fn=${hasVerifyFn};download_hook=${usedOnDownload}`,
  };
}

function checkProductionTransportPolicy(root: string): GateCheck {
  const sdkPath = path.resolve(root, 'packages/infring-sdk/src/transports.ts');
  const sdkCliDevOnlyPath = path.resolve(root, 'packages/infring-sdk/src/transports/cli_dev_only.ts');
  const bridgePath = path.resolve(root, 'adapters/runtime/ops_lane_bridge.ts');
  const runnerPath = path.resolve(root, 'adapters/runtime/run_infring_ops.ts');
  const legacyHelperPath = path.resolve(root, 'adapters/runtime/dev_only/legacy_process_runner.ts');
  const processFallbackHelperPath = path.resolve(root, 'adapters/runtime/dev_only/ops_lane_process_fallback.ts');
  const sdkSource = fs.readFileSync(sdkPath, 'utf8');
  const sdkCliDevOnlySource = fs.existsSync(sdkCliDevOnlyPath)
    ? fs.readFileSync(sdkCliDevOnlyPath, 'utf8')
    : '';
  const bridgeSource = fs.readFileSync(bridgePath, 'utf8');
  const runnerSource = fs.readFileSync(runnerPath, 'utf8');
  const legacyHelperSource = fs.existsSync(legacyHelperPath) ? fs.readFileSync(legacyHelperPath, 'utf8') : '';
  const processFallbackHelperSource = fs.existsSync(processFallbackHelperPath)
    ? fs.readFileSync(processFallbackHelperPath, 'utf8')
    : '';

  const sdkLock =
    sdkSource.includes('resident_ipc_authoritative') &&
    sdkSource.includes('createResidentIpcTransport') &&
    !sdkSource.includes('spawn(') &&
    !sdkSource.includes('spawnSync(');
  const sdkCliLock =
    sdkCliDevOnlySource.includes('process_transport_forbidden_in_production') &&
    sdkCliDevOnlySource.includes('isProductionReleaseChannel') &&
    sdkCliDevOnlySource.includes('INFRING_RELEASE_CHANNEL');
  const bridgeLock =
    bridgeSource.includes('process_fallback_forbidden_in_production') &&
    bridgeSource.includes('processFallbackPolicy') &&
    bridgeSource.includes('isProductionReleaseChannel') &&
    !bridgeSource.includes('spawnSync(') &&
    bridgeSource.includes("./dev_only/ops_lane_process_fallback.ts");
  const legacyLock =
    runnerSource.includes('legacyProcessRunnerForced') &&
    runnerSource.includes('isProductionReleaseChannel') &&
    runnerSource.includes('INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER') &&
    runnerSource.includes('legacy_process_runner_dev_only') &&
    runnerSource.includes('INFRING_DEV_ENABLE_LEGACY_PROCESS_RUNNER') &&
    !runnerSource.includes('spawnSync(') &&
    runnerSource.includes("./dev_only/legacy_process_runner.ts");
  const helperLock =
    legacyHelperSource.includes('legacy_process_runner_dev_only') &&
    legacyHelperSource.includes('spawnSync(') &&
    processFallbackHelperSource.includes('process_fallback_dev_only') &&
    processFallbackHelperSource.includes('spawnSync(');

  const ok = sdkLock && sdkCliLock && bridgeLock && legacyLock && helperLock;
  return {
    id: 'production_transport_policy',
    ok,
    detail: ok
      ? 'production release channel enforces resident IPC topology'
      : `sdk_lock=${sdkLock};sdk_cli_lock=${sdkCliLock};bridge_lock=${bridgeLock};legacy_lock=${legacyLock};helper_lock=${helperLock}`,
  };
}

function checkAssimilationV1SupportContract(root: string): GateCheck {
  const rel = 'client/runtime/config/assimilation_v1_support_contract.json';
  const policy = readJson(path.resolve(root, rel), {});
  const canonicalSlice = cleanText(policy?.canonical_slice?.name ?? '', 120);
  const supportLevel = cleanText(policy?.production_contract?.support_level ?? '', 120);
  const components = Array.isArray(policy?.canonical_slice?.supported_components)
    ? policy.canonical_slice.supported_components
    : [];
  const guardScript = cleanText(policy?.enforcement?.guard_script ?? '', 240);
  const ok =
    cleanText(policy?.status ?? '', 80) === 'frozen_v1_vertical_slice' &&
    canonicalSlice.length > 0 &&
    supportLevel === 'experimental_opt_in' &&
    policy?.production_contract?.release_supported === false &&
    components.length === 3 &&
    guardScript === 'tests/tooling/scripts/ci/assimilation_v1_support_guard.ts';
  return {
    id: 'assimilation_v1_support_contract',
    ok,
    detail: ok
      ? `slice=${canonicalSlice};support_level=${supportLevel};components=${components.length}`
      : `invalid:${rel}`,
  };
}

function checkProductionClosurePolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/production_readiness_closure_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const canonical = cleanText(policy?.production_surface_contract?.canonical_surface ?? '', 40).toLowerCase();
  const transportMode = cleanText(policy?.release_candidate_topology?.transport_mode ?? '', 120);
  const forbidFallback = policy?.release_candidate_topology?.forbid_process_transport_in_production === true;
  const ok = canonical === 'rich' && transportMode === 'resident_ipc_authoritative' && forbidFallback;
  return {
    id: 'production_closure_policy',
    ok,
    detail: ok
      ? `canonical_surface=${canonical};transport_mode=${transportMode}`
      : `invalid:${rel}`,
  };
}

function checkDependencyPolicyTopLevelKeysetContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const drift = keysetDrift(policy, [
    'schema_id',
    'schema_version',
    'security_patch_sla_days',
    'max_critical_vulnerabilities',
    'max_high_vulnerabilities',
    'dependabot_required_ecosystems',
    'blocked_packages',
  ]);
  const violations: string[] = [];
  if (drift.missing.length > 0) violations.push(`missing=${drift.missing.join(',')}`);
  if (drift.unexpected.length > 0) violations.push(`unexpected=${drift.unexpected.join(',')}`);
  return {
    id: 'dependency_policy_top_level_keyset_contract',
    ok: violations.length === 0,
    detail: violations.length === 0 ? 'ok' : violations.join('; '),
  };
}

function checkDependencyBlockedPackageTokenContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const blocked = Array.isArray(policy?.blocked_packages) ? policy.blocked_packages : [];
  const invalid = blocked.filter((row: any) => {
    const ecosystem = cleanText(row?.ecosystem ?? '', 80);
    const name = cleanText(row?.name ?? '', 120);
    return !isCanonicalToken(ecosystem) || !isCanonicalNameToken(name);
  });
  return {
    id: 'dependency_blocked_package_token_contract',
    ok: invalid.length === 0,
    detail: invalid.length === 0 ? 'ok' : invalid.map((row: any) => `${cleanText(row?.ecosystem ?? 'unknown', 80)}:${cleanText(row?.name ?? 'unknown', 120)}`).join(','),
  };
}

function checkDependencyBlockedPackageReasonQualityContract(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const blocked = Array.isArray(policy?.blocked_packages) ? policy.blocked_packages : [];
  const weak = blocked.filter((row: any) => cleanText(row?.reason ?? '', 280).length < 12);
  return {
    id: 'dependency_blocked_package_reason_quality_contract',
    ok: weak.length === 0,
    detail: weak.length === 0 ? 'ok' : weak.map((row: any) => cleanText(row?.name ?? 'unknown', 120)).join(','),
  };
}

function checkVerifyReleaseProfileNoDuplicateGateIdsContract(root: string): GateCheck {
  const rel = 'tests/tooling/config/verify_profiles.json';
  const profiles = readJson(path.resolve(root, rel), {});
  const gateIds = Array.isArray(profiles?.profiles?.release?.gate_ids)
    ? profiles.profiles.release.gate_ids.map((value: unknown) => cleanText(value, 160))
    : [];
  const duplicates = gateIds.filter((row, idx, arr) => row && arr.indexOf(row) !== idx);
  return {
    id: 'verify_release_profile_no_duplicate_gate_ids_contract',
    ok: duplicates.length === 0,
    detail: duplicates.length === 0 ? 'ok' : Array.from(new Set(duplicates)).join(','),
  };
}

function checkVerifyRuntimeProofProfileNoDuplicateGateIdsContract(root: string): GateCheck {
  const rel = 'tests/tooling/config/verify_profiles.json';
  const profiles = readJson(path.resolve(root, rel), {});
  const gateIds = Array.isArray(profiles?.profiles?.['runtime-proof']?.gate_ids)
    ? profiles.profiles['runtime-proof'].gate_ids.map((value: unknown) => cleanText(value, 160))
    : [];
  const duplicates = gateIds.filter((row, idx, arr) => row && arr.indexOf(row) !== idx);
  return {
    id: 'verify_runtime_proof_profile_no_duplicate_gate_ids_contract',
    ok: duplicates.length === 0,
    detail: duplicates.length === 0 ? 'ok' : Array.from(new Set(duplicates)).join(','),
  };
}

function checkReleaseWorkflowDispatchChannelChoiceContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_dispatch_channel_choice_contract', [
    'workflow_dispatch:',
    'release_channel:',
    'type: choice',
    'default: alpha',
    '- alpha',
    '- beta',
    '- stable',
  ]);
}

function checkReleaseWorkflowProofPackUploadTagPathContract(root: string): GateCheck {
  return checkWorkflowNeedleSet(root, 'release_workflow_proof_pack_upload_tag_path_contract', [
    'Upload release proof-pack evidence',
    'name: release-proof-pack-${{ steps.semver.outputs.tag }}',
    'validation/release_gates/proof_packs/${{ steps.semver.outputs.tag }}/**',
    'if-no-files-found: error',
  ]);
}

function buildReport(root: string) {
  const checks: GateCheck[] = [
    checkFileExists(root, 'client/runtime/config/release_channel_policy.json', 'release_channel_policy_file'),
    checkReleaseChannelPolicyObjectContract(root),
    checkReleaseChannelPolicyTopLevelKeysetContract(root),
    checkReleaseChannelPromotionRuleRowKeysetContract(root),
    checkReleaseChannelPolicyWhitespaceContract(root),
    checkReleaseChannelPolicySchema(root),
    checkReleaseChannelDefaultChannel(root),
    checkReleaseChannelPolicy(root),
    checkReleaseChannelPromotionRules(root),
    checkReleaseChannelCanonicalOrder(root),
    checkReleaseChannelDefaultAlpha(root),
    checkReleaseChannelPromotionCanonicalMatrix(root),
    checkReleaseChannelPolicyTokenHygiene(root),
    checkReleaseChannelPolicyChannelsNoDuplicateContract(root),
    checkReleaseChannelPolicyChannelsLowercaseContract(root),
    checkReleaseChannelPolicyChannelsTrimmedContract(root),
    checkReleaseChannelDefaultLowercaseContract(root),
    checkReleaseChannelPromotionRuleTokenPresenceContract(root),
    checkReleaseChannelPromotionRuleLowercaseContract(root),
    checkReleaseChannelPromotionRuleTrimmedContract(root),
    checkReleaseChannelPromotionRuleNoStableSourceContract(root),
    checkReleaseChannelPromotionRuleNoAlphaTargetContract(root),
    checkReleaseChannelPromotionRuleSourceCoverageContract(root),
    checkReleaseChannelPromotionRuleTargetCoverageContract(root),
    checkFileExists(root, 'client/runtime/config/release_compatibility_policy.json', 'release_compatibility_policy_file'),
    checkCompatibilityPolicyBooleanFlagTypeContract(root),
    checkCompatibilityPolicyRequiredDeprecationDaysIntegerContract(root),
    checkCompatibilityPolicyRequiredDeprecationDaysUpperBoundContract(root),
    checkCompatibilityPolicyRegistryPathExistsContract(root),
    checkCompatibilityPolicyTopLevelKeysetContract(root),
    checkCompatibilityPolicyRegistryPathShapeContract(root),
    checkCompatibilityPolicySchema(root),
    checkCompatibilityPolicyRegistryPath(root),
    checkCompatibilityPolicyNoticeFlag(root),
    checkDeprecationPolicy(root),
    checkCompatibilityRequiredDeprecationFloor(root),
    checkCompatibilityRegistrySchema(root),
    checkApiCliRegistryTopLevelKeysetContract(root),
    checkApiCliRegistryContractVersionSemverContract(root),
    checkApiCliRegistryContractTokenShapeContract(root),
    checkApiCliRegistryDeprecatedGuidePathContract(root),
    checkCompatibilityRegistryNameUniqueness(root),
    checkCompatibilityRegistryDeprecatedPayload(root),
    checkCompatibilityRegistryDeprecationWindowFloor(root),
    checkSchemaVersioningPolicyTopLevelKeysetContract(root),
    checkSchemaVersioningOutputsKeysetContract(root),
    checkSchemaVersioningTargetPathShapeContract(root),
    checkSchemaVersioningTargetRowKeysetContract(root),
    checkSchemaVersioningTargetsCanonicalSet(root),
    checkSchemaVersioningTargetsUniqueIds(root),
    checkSchemaVersioningTargetsSchemaAlignment(root),
    checkSchemaVersioningTargetsMinVersion(root),
    checkSchemaVersioningOutputsCanonicalPaths(root),
    checkSchemaMigrationPolicy(root),
    checkSchemaMigrationOutputsPolicy(root),
    checkFileExists(root, 'client/runtime/config/dependency_update_policy.json', 'dependency_update_policy_file'),
    checkDependencyPolicyTopLevelKeysetContract(root),
    checkDependencyPolicySecurityPatchSlaIntegerContract(root),
    checkDependencyPolicyVulnerabilityBudgetsIntegerContract(root),
    checkDependencyPolicyRequiredEcosystemUniquenessContract(root),
    checkDependencyPolicyRequiredEcosystemCanonicalOrderContract(root),
    checkDependencyBlockedPackageTokenContract(root),
    checkDependencyBlockedPackageReasonQualityContract(root),
    checkDependencyPolicy(root),
    checkDependencyPolicySchemaAndEcosystems(root),
    checkDependencyPolicyBlockedPackages(root),
    checkDependencyVulnerabilityBudget(root),
    checkDependencyEcosystemCanonicalSet(root),
    checkDependencyBlocklistBaseline(root),
    checkDependabotSchedule(root),
    checkDependabotScheduleWeeklyMonday(root),
    checkVerifyReleaseProfileNoDuplicateGateIdsContract(root),
    checkVerifyRuntimeProofProfileNoDuplicateGateIdsContract(root),
    checkReleaseWorkflowDispatchChannelChoiceContract(root),
    checkReleaseWorkflowProofPackUploadTagPathContract(root),
    checkReleaseWorkflowChannelResolutionContract(root),
    checkReleaseWorkflowReleasePolicyEnforcementContract(root),
    checkReleaseWorkflowHeredocIndentContract(root),
    checkReleaseWorkflowTriggerContract(root),
    checkReleaseWorkflowPermissionsContract(root),
    checkReleaseWorkflowWindowsPrebuiltJobContract(root),
    checkReleaseWorkflowReleaseNeedsWindowsContract(root),
    checkReleaseWorkflowToolchainContract(root),
    checkReleaseWorkflowWindowsArtifactRoundTripContract(root),
    checkReleaseWorkflowWindowsBaselineEnforcementContract(root),
    checkReleaseWorkflowPolicyAndContractGatesContract(root),
    checkReleaseWorkflowScorecardContract(root),
    checkReleaseWorkflowProofPackUploadContract(root),
    checkReleaseWorkflowProofPackMandatoryArtifactGateContract(root),
    checkReleaseWorkflowRuntimeEvidenceChainContract(root),
    checkReleaseWorkflowCanaryRollbackContract(root),
    checkReleaseWorkflowSigningAndSbomContract(root),
    checkReleaseWorkflowChecksumAndAssetsContract(root),
    checkReleaseWorkflowTagUniquenessContract(root),
    checkReleaseWorkflowReleasePublishContract(root),
    checkReleaseWorkflowClosureRefreshContract(root),
    checkReleaseWorkflowSemverResolutionContract(root),
    checkReleaseWorkflowLicensingManifestContract(root),
    checkReleaseWorkflowSizeGateContract(root),
    checkInstallerChecksumVerification(root),
    checkProductionTransportPolicy(root),
    checkProductionClosurePolicy(root),
    checkAssimilationV1SupportContract(root),
  ];
  const ok = checks.every((row) => row.ok);
  return {
    ok,
    type: 'release_policy_gate',
    checks,
    failed: checks.filter((row) => !row.ok).map((row) => row.id),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = path.resolve(__dirname, '../../../..');
  const args = parseArgs(argv);
  const report = {
    ...buildReport(root),
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    strict: args.strict,
    inputs: {
      out: args.outPath,
    },
  };
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  parseArgs,
  run,
};
