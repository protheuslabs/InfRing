#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::discovery.
//
// Runtime engine discovery is a Gateway-adapter concern. Defaults are probes,
// not truth: user/config overrides win first, PATH/default paths are lower
// authority, and missing installable engines become bounded projections for UI.

'use strict';

const fs = require('fs');
const path = require('path');

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function envValue(env, key) {
  if (!key) return '';
  return cleanString((env || process.env)[key], 2000);
}

function getPathValue(object, dotted) {
  if (!object || !dotted) return undefined;
  const parts = String(dotted).split('.').filter(Boolean);
  let current = object;
  for (const part of parts) {
    if (!current || typeof current !== 'object' || !(part in current)) return undefined;
    current = current[part];
  }
  return current;
}

function expandLocationTemplate(raw, env = process.env) {
  let value = cleanString(raw, 2000);
  if (!value) return '';
  if (value.startsWith('~/')) value = path.join(env.HOME || env.USERPROFILE || '', value.slice(2));
  value = value.replace(/%USERNAME%/g, env.USERNAME || env.USER || '');
  value = value.replace(/%USERPROFILE%/g, env.USERPROFILE || '');
  value = value.replace(/%LOCALAPPDATA%/g, env.LOCALAPPDATA || '');
  value = value.replace(/\$HOME/g, env.HOME || env.USERPROFILE || '');
  return value;
}

function pathExists(candidate, env = process.env) {
  const expanded = expandLocationTemplate(candidate, env);
  if (!expanded) return false;
  try {
    return fs.existsSync(expanded);
  } catch {
    return false;
  }
}

function pathDirs(env = process.env) {
  const raw = env.PATH || env.Path || env.path || '';
  return String(raw).split(path.delimiter).filter(Boolean);
}

function commandCandidates(command, env = process.env) {
  const cmd = cleanString(command, 500);
  if (!cmd) return [];
  const extensions = process.platform === 'win32'
    ? ['', '.cmd', '.exe', '.bat', '.ps1']
    : [''];
  if (cmd.includes('/') || cmd.includes('\\')) return [expandLocationTemplate(cmd, env)];
  const out = [];
  for (const dir of pathDirs(env)) {
    for (const ext of extensions) out.push(path.join(dir, cmd.endsWith(ext) ? cmd : `${cmd}${ext}`));
  }
  return out;
}

function findCommandOnPath(commands, env = process.env) {
  for (const command of asArray(commands)) {
    for (const candidate of commandCandidates(command, env)) {
      if (pathExists(candidate, env)) return { command: cleanString(command, 500), path: candidate };
    }
  }
  return null;
}

function firstExistingPath(paths, env = process.env) {
  for (const candidate of asArray(paths)) {
    const expanded = expandLocationTemplate(candidate, env);
    if (expanded && pathExists(expanded, env)) return expanded;
  }
  return '';
}

function result(engine, status, source, fields = {}) {
  return {
    engine_id: cleanString(engine && engine.engine_id, 120),
    status,
    discovery_source: source,
    custom_location_allowed: Boolean(engine && engine.discovery && engine.discovery.custom_location_allowed),
    authority_order: asArray(engine && engine.discovery && engine.discovery.authority_order),
    ...fields,
  };
}

function resolveEngineDiscovery(engine, options = {}) {
  const discovery = (engine && engine.discovery) || {};
  const env = options.env || process.env;
  const config = options.config || {};
  const explicitCommand = cleanString(options.command, 1000);
  const explicitUrl = cleanString(options.url, 1000);

  if (explicitCommand) return result(engine, 'available', 'user_override', { command: explicitCommand });
  if (explicitUrl) return result(engine, 'available', 'user_override', { url: explicitUrl });

  const configCommand = cleanString(getPathValue(config, discovery.override_config_key), 1000);
  const configUrl = cleanString(getPathValue(config, discovery.override_url_config_key || discovery.override_config_key), 1000);
  if (configCommand && !/^https?:\/\//i.test(configCommand)) return result(engine, 'available', 'config_value', { command: configCommand });
  if (configUrl && /^https?:\/\//i.test(configUrl)) return result(engine, 'available', 'config_value', { url: configUrl });

  for (const envName of asArray(discovery.env_vars)) {
    const value = envValue(env, envName);
    if (!value) continue;
    if (/^https?:\/\//i.test(value)) return result(engine, 'available', 'environment_variable', { env_var: envName, url: value });
    return result(engine, 'available', 'environment_variable', { env_var: envName, command: value });
  }

  const pathCommand = findCommandOnPath(discovery.path_commands, env);
  if (pathCommand) return result(engine, 'available', 'path_discovery', { command: pathCommand.command, resolved_path: pathCommand.path });

  const defaultPath = firstExistingPath(discovery.default_paths, env);
  if (defaultPath) return result(engine, 'available', 'default_location_probe', { command: defaultPath, resolved_path: defaultPath });

  const defaultUrls = asArray(discovery.default_urls).map((url) => cleanString(url, 1000)).filter(Boolean);
  if (defaultUrls.length) {
    return result(engine, 'configurable', 'default_url_probe', {
      url: defaultUrls[0],
      default_urls: defaultUrls,
      reason: 'default URL is a probe candidate; health adapter must verify reachability before reporting available',
    });
  }

  const install = (engine && engine.install) || {};
  return result(engine, install.download_available ? 'not_downloaded' : 'not_configured', 'missing_installable', {
    download_available: Boolean(install.download_available),
    download_action_ref: cleanString(install.download_action_ref, 500) || null,
    browser_fallback_url: cleanString(install.browser_fallback_url, 1000) || null,
  });
}

module.exports = {
  cleanString,
  commandCandidates,
  expandLocationTemplate,
  findCommandOnPath,
  getPathValue,
  pathExists,
  resolveEngineDiscovery,
};
