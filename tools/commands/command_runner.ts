#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = process.cwd();
const REGISTRY_PATH = 'tools/commands/command_registry.json';
type Entry = {
  id: string;
  group: string;
  command: string;
  lifecycle?: string;
  owner?: string;
  domain?: string;
  work_gate?: string;
  description?: string;
  metadata_curated?: boolean;
  operator_surface?: boolean;
  operator_surface_rank?: number;
  operator_surface_reason?: string;
};
type Registry = {
  entries: Entry[];
  script_count: number;
  group_counts: Record<string, number>;
  compression_policy?: {
    canonical_runner?: string;
    compatibility_aliases_allowed?: boolean;
    new_package_scripts_require_policy_update?: boolean;
    goal?: string;
  };
};
function registry(): Registry {
  return JSON.parse(fs.readFileSync(path.join(ROOT, REGISTRY_PATH), 'utf8'));
}
function packageScripts(): Record<string, string> {
  const pkg = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8'));
  return pkg.scripts && typeof pkg.scripts === 'object' ? pkg.scripts : {};
}
function syntheticPackageEntry(id: string, command: string): Entry {
  return {
    id,
    group: id.split(':')[0] || 'unknown',
    command,
    lifecycle: 'package_script_compatibility',
    owner: 'package_json_compatibility_surface',
    domain: 'compatibility',
    work_gate: 'manual_compatibility',
    description: 'Compatibility package script backing entry. Prefer curated operator-surface commands when available.',
    operator_surface: false,
  };
}
function expandedEntries(data: Registry, includeCompat: boolean): Entry[] {
  if (!includeCompat) return data.entries;
  const scripts = packageScripts();
  const seen = new Set(data.entries.map((entry) => entry.id));
  const synthetic = Object.entries(scripts)
    .filter(([id]) => !seen.has(id))
    .map(([id, command]) => syntheticPackageEntry(id, String(command)));
  return [...data.entries, ...synthetic];
}
function quote(arg: string): string {
  if (/^[A-Za-z0-9_./:=@%+-]+$/.test(arg)) return arg;
  return "'" + arg.replace(/'/g, "'\\''") + "'";
}
function flag(args: string[], name: string): string {
  const prefix = `${name}=`;
  return args.find((arg) => arg.startsWith(prefix))?.slice(prefix.length) || '';
}
function includesText(entry: Entry, needle: string): boolean {
  if (!needle) return true;
  const haystack = [entry.id, entry.group, entry.domain, entry.work_gate, entry.lifecycle, entry.owner, entry.description, entry.command]
    .filter(Boolean)
    .join('\n')
    .toLowerCase();
  return haystack.includes(needle.toLowerCase());
}
function usage(): void {
  console.error('Preferred entrypoint: npm run -s cmd -- <command-id> [args...]');
  console.error('Compatibility backing: package.json scripts remain addressable through this registry runner.');
  console.error('Usage: npm run -s cmd -- <command-id> [args...]');
  console.error('       npm run -s cmd -- list [--group=<group>] [--domain=<domain>] [--work-gate=<gate>] [--lifecycle=<state>] [--search=<text>] [--include-compat=1] [--operator-surface=0]');
  console.error('       npm run -s cmd -- info <command-id>');
  console.error('       npm run -s cmd -- groups');
}
function publicEntry(entry: Entry): Partial<Entry> {
  return {
    id: entry.id,
    group: entry.group,
    lifecycle: entry.lifecycle,
    owner: entry.owner,
    domain: entry.domain,
    work_gate: entry.work_gate,
    description: entry.description,
    operator_surface: entry.operator_surface,
    operator_surface_rank: entry.operator_surface_rank,
    operator_surface_reason: entry.operator_surface_reason,
  };
}
function entrypoint(data: Registry): { preferred: string; backing: string; policy_goal: string } {
  return {
    preferred: 'npm run -s cmd --',
    backing: 'package.json:scripts',
    policy_goal: data.compression_policy?.goal || 'structured command runner is the default operator surface',
  };
}
function suggestedIds(entries: Entry[], needle: string): string[] {
  const haystack = String(needle || '').toLowerCase();
  const tokens = haystack.split(/[:_-]+/).filter(Boolean);
  return entries
    .filter((entry) => entry.operator_surface === true)
    .map((entry) => {
      const id = entry.id.toLowerCase();
      let score = 0;
      if (id === haystack) score += 100;
      if (id.includes(haystack) || haystack.includes(id)) score += 50;
      for (const token of tokens) if (id.includes(token)) score += 10;
      if (entry.metadata_curated) score += 2;
      return { id: entry.id, score };
    })
    .filter((row) => row.score > 0)
    .sort((a, b) => b.score - a.score || a.id.localeCompare(b.id))
    .slice(0, 5)
    .map((row) => row.id);
}
function main(): void {
  const data = registry();
  const args = process.argv.slice(2);
  const first = args[0] || 'help';
  if (first === 'help' || first === '--help' || first === '-h') { usage(); return; }
  if (first === 'list') {
    const filters = {
      group: flag(args, '--group'),
      domain: flag(args, '--domain'),
      workGate: flag(args, '--work-gate'),
      lifecycle: flag(args, '--lifecycle'),
      search: flag(args, '--search'),
      includeCompat: flag(args, '--include-compat') === '1',
      operatorSurface: flag(args, '--operator-surface') !== '0',
    };
    const sourceEntries = expandedEntries(data, filters.includeCompat);
    const rows = sourceEntries.filter((entry) => {
      if (!filters.includeCompat && filters.operatorSurface && entry.operator_surface !== true) return false;
      if (!filters.includeCompat && filters.operatorSurface && entry.lifecycle === 'compatibility_alias' && !entry.operator_surface) return false;
      if (!filters.includeCompat && !filters.operatorSurface && entry.lifecycle === 'compatibility_alias') return false;
      if (filters.group && entry.group !== filters.group) return false;
      if (filters.domain && entry.domain !== filters.domain) return false;
      if (filters.workGate && entry.work_gate !== filters.workGate) return false;
      if (filters.lifecycle && entry.lifecycle !== filters.lifecycle) return false;
      return includesText(entry, filters.search);
    }).sort((a, b) => (a.operator_surface_rank || 999999) - (b.operator_surface_rank || 999999) || a.id.localeCompare(b.id));
    console.log(JSON.stringify({ ok: true, type: 'command_registry_list', entrypoint: entrypoint(data), count: rows.length, filters, entries: rows.map(publicEntry) }, null, 2));
    return;
  }
  if (first === 'info') {
    const id = args[1] || '';
    const entry = data.entries.find((row) => row.id === id) || (() => {
      const command = packageScripts()[id];
      return command ? syntheticPackageEntry(id, command) : undefined;
    })();
    if (!entry) {
      console.error(`Unknown command id: ${id}`);
      process.exit(2);
    }
    console.log(JSON.stringify({ ok: true, type: 'command_registry_info', entrypoint: entrypoint(data), entry }, null, 2));
    return;
  }
  if (first === 'groups') {
    const sourceEntries = expandedEntries(data, true);
    const domains = sourceEntries.reduce((acc: Record<string, number>, entry) => {
      const key = entry.domain || 'unclassified';
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    }, {});
    const workGates = sourceEntries.reduce((acc: Record<string, number>, entry) => {
      const key = entry.work_gate || 'unclassified';
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    }, {});
    console.log(JSON.stringify({ ok: true, type: 'command_registry_groups', entrypoint: entrypoint(data), script_count: data.script_count, registry_entries: data.entries.length, expanded_entry_count: sourceEntries.length, group_counts: data.group_counts, domains, work_gates: workGates }, null, 2));
    return;
  }
  const entry = data.entries.find((row) => row.id === first);
  if (!entry) {
    const scripts = packageScripts();
    const compatCommand = scripts[first];
    if (compatCommand) {
      const result = spawnSync('npm', ['run', '-s', first, '--', ...args.slice(1)], { cwd: ROOT, stdio: 'inherit', env: process.env });
      if (result.error) {
        console.error(result.error.message);
        process.exit(1);
      }
      process.exit(typeof result.status === 'number' ? result.status : 1);
    } else {
      console.error(`Unknown command id: ${first}`);
      const suggestions = suggestedIds(data.entries, first);
      if (suggestions.length) console.error(`Suggested operator-surface commands: ${suggestions.join(', ')}`);
      usage();
      process.exit(2);
    }
  }
  const forwarded = args.slice(1);
  const command = forwarded.length ? `${entry.command} ${forwarded.map(quote).join(' ')}` : entry.command;
  const result = spawnSync(command, { cwd: ROOT, shell: true, stdio: 'inherit', env: process.env });
  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }
  process.exit(typeof result.status === 'number' ? result.status : 1);
}
main();
