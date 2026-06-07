import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

export type TodoSection = 'red' | 'yellow' | 'white';

export type TodoItem = {
  id: string;
  title: string;
  section: TodoSection;
  owner: string;
  deadline: string;
  source_family: string;
  summary: string;
  work_gate?: 'real_work' | 'reliability' | 'simplification';
  real_work_score?: number;
};

export type ArchivedTodoItem = TodoItem & {
  completed_at: string;
  completion_note?: string;
};

export type TodoRegistry = {
  version: number;
  updated_at: string;
  policy: {
    red_due_in_days_or_less: number;
    yellow_due_in_days_or_less: number;
    non_escalating_deadlines: string[];
    dependency_deadline_prefixes: string[];
  };
  items: TodoItem[];
};

export type TodoArchiveRegistry = {
  version: number;
  updated_at: string;
  historical_snapshot: {
    preserved_archive_path: string;
    total_rows: number;
    queued: number;
    in_progress: number;
    blocked: number;
    blocked_external_prepared: number;
    done: number;
    existing_coverage_validated: number;
  };
  items: ArchivedTodoItem[];
};

export const TODO_REGISTRY_PATH = 'docs/workspace/todo/todo_registry.json';
export const TODO_ARCHIVE_REGISTRY_PATH = 'docs/workspace/todo/todo_archive_registry.json';
export const TODO_MARKDOWN_PATH = 'docs/workspace/todo/TODO.md';
export const TODO_ARCHIVE_MARKDOWN_PATH = 'docs/workspace/todo/TODO_ARCHIVE.md';
export const TODO_ARCHIVE_LEGACY_PATH = 'docs/workspace/todo/TODO_ARCHIVE_LEGACY.md';

function rootPath(rel: string): string {
  return path.resolve(process.cwd(), rel);
}

function readJson<T>(rel: string): T {
  return JSON.parse(readFileSync(rootPath(rel), 'utf8')) as T;
}

function writeJson(rel: string, value: unknown): void {
  writeFileSync(rootPath(rel), `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function writeText(rel: string, value: string): void {
  writeFileSync(rootPath(rel), value, 'utf8');
}

function isExactDate(value: string): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(value);
}

function daysUntil(deadline: string, now = new Date()): number | null {
  if (!isExactDate(deadline)) return null;
  const [year, month, day] = deadline.split('-').map((part) => Number(part));
  const target = Date.UTC(year, month - 1, day);
  const current = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  return Math.floor((target - current) / 86400000);
}

function compareDeadline(a: string, b: string): number {
  const rank = (value: string): [number, string] => {
    if (isExactDate(value)) return [0, value];
    if (value === 'external') return [2, value];
    if (value === 'none') return [3, value];
    return [1, value];
  };
  const [aRank, aValue] = rank(a);
  const [bRank, bValue] = rank(b);
  return aRank - bRank || aValue.localeCompare(bValue);
}

function renderHeader(title: string, updatedAt: string): string[] {
  return [title, '', `Updated: ${updatedAt}`, ''];
}

function renderTodoSection(title: string, items: TodoItem[]): string[] {
  const lines = [`## ${title}`];
  if (!items.length) {
    lines.push('- none');
    lines.push('');
    return lines;
  }
  for (const item of items) {
    lines.push(`- \`${item.id}\` — ${item.title}`);
    lines.push(`  owner: \`${item.owner}\``);
    lines.push(`  deadline: \`${item.deadline}\``);
    lines.push(`  source_family: \`${item.source_family}\``);
    if (item.work_gate) lines.push(`  work_gate: \`${item.work_gate}\``);
    if (typeof item.real_work_score === 'number') lines.push(`  real_work_score: \`${item.real_work_score}\``);
    lines.push(`  summary: ${item.summary}`);
  }
  lines.push('');
  return lines;
}

export function loadTodoRegistry(): TodoRegistry {
  return readJson<TodoRegistry>(TODO_REGISTRY_PATH);
}

export function loadTodoArchiveRegistry(): TodoArchiveRegistry {
  return readJson<TodoArchiveRegistry>(TODO_ARCHIVE_REGISTRY_PATH);
}

export function saveTodoRegistry(registry: TodoRegistry): void {
  writeJson(TODO_REGISTRY_PATH, registry);
}

export function saveTodoArchiveRegistry(registry: TodoArchiveRegistry): void {
  writeJson(TODO_ARCHIVE_REGISTRY_PATH, registry);
}

export function sortTodoItems(items: TodoItem[]): TodoItem[] {
  return [...items].sort(
    (a, b) => compareDeadline(a.deadline, b.deadline) || a.id.localeCompare(b.id),
  );
}

export function sortArchivedItems(items: ArchivedTodoItem[]): ArchivedTodoItem[] {
  return [...items].sort(
    (a, b) => b.completed_at.localeCompare(a.completed_at) || a.id.localeCompare(b.id),
  );
}

export function findTodoItemIndex(items: TodoItem[], id: string): number {
  return items.findIndex((item) => item.id === id);
}

export function assertTodoItemExists(items: TodoItem[], id: string): TodoItem {
  const item = items.find((candidate) => candidate.id === id);
  if (!item) throw new Error(`missing TODO item: ${id}`);
  return item;
}

export function normalizeSection(value: string): TodoSection {
  const normalized = value.trim().toLowerCase();
  if (normalized === 'red' || normalized === 'yellow' || normalized === 'white') return normalized;
  throw new Error(`invalid section: ${value}`);
}

export function auditTodoRegistry(
  registry: TodoRegistry,
  now = new Date(),
): { changed: boolean; promotions: Array<{ id: string; from: TodoSection; to: TodoSection; reason: string }> } {
  const promotions: Array<{ id: string; from: TodoSection; to: TodoSection; reason: string }> = [];
  for (const item of registry.items) {
    const days = daysUntil(item.deadline, now);
    if (days === null) continue;
    let target: TodoSection = 'white';
    if (days <= registry.policy.red_due_in_days_or_less) target = 'red';
    else if (days <= registry.policy.yellow_due_in_days_or_less) target = 'yellow';
    if (target !== item.section) {
      promotions.push({
        id: item.id,
        from: item.section,
        to: target,
        reason: `deadline ${item.deadline} is ${days} day(s) away`,
      });
      item.section = target;
    }
  }
  if (promotions.length) registry.updated_at = now.toISOString();
  return { changed: promotions.length > 0, promotions };
}

export function renderTodoMarkdown(registry: TodoRegistry): string {
  const red = sortTodoItems(registry.items.filter((item) => item.section === 'red'));
  const yellow = sortTodoItems(registry.items.filter((item) => item.section === 'yellow'));
  const white = sortTodoItems(registry.items.filter((item) => item.section === 'white'));
  const lines = renderHeader('# TODO', registry.updated_at);
  lines.push('## How To Use This File');
  lines.push('- This is the live operating board, not the historical ledger.');
  lines.push(`- Canonical structured data lives in [todo_registry.json](/Users/jay/.openclaw/workspace/${TODO_REGISTRY_PATH}).`);
  lines.push(`- Completed items must be moved to [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_MARKDOWN_PATH}) instead of being left here.`);
  lines.push(`- Archive history is rendered from [todo_archive_registry.json](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_REGISTRY_PATH}) and the preserved legacy appendix at [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_LEGACY_PATH}).`);
  lines.push('- Run manual commands through `npm run -s ops:todo:board -- <command>` so JSON and Markdown stay in sync.');
  lines.push('- Every active item must declare `owner` and `deadline`.');
  lines.push('- Active items should declare `work_gate` as `real_work`, `reliability`, or `simplification` when possible; `real_work` is the practical TODO gate for the usability law.');
  lines.push('- Allowed deadline values: exact date like `2026-05-07`, `none`, `external`, or dependency-shaped values like `after_red_section`.');
  lines.push(`- Deadline promotion policy: items due in <= ${registry.policy.red_due_in_days_or_less} days belong in Red; items due in <= ${registry.policy.yellow_due_in_days_or_less} days belong in Yellow; everything later stays in White unless manually escalated.`);
  lines.push('');
  lines.push('## Rollup');
  lines.push(`- active_items: ${registry.items.length}`);
  lines.push(`- red: ${red.length}`);
  lines.push(`- yellow: ${yellow.length}`);
  lines.push(`- white: ${white.length}`);
  lines.push('');
  lines.push(...renderTodoSection('Red Section (Do Immediately)', red));
  lines.push(...renderTodoSection('Yellow Section (Do Soon)', yellow));
  lines.push(...renderTodoSection('White Section (Do At Leisure)', white));
  lines.push('## Archive Rule');
  lines.push(`- When an item in this file is completed, remove it from this live board and append it to [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_MARKDOWN_PATH}) through the scripted flow.`);
  lines.push('- Do not let completed rows accumulate here again.');
  lines.push('- Treat Markdown as a rendered operator surface, not the canonical mutation target.');
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function renderTodoArchiveMarkdown(registry: TodoArchiveRegistry): string {
  const items = sortArchivedItems(registry.items);
  const lines = renderHeader('# TODO Archive', registry.updated_at);
  lines.push('## How To Use This File');
  lines.push('- This is the historical ledger for completed work, not the live queue.');
  lines.push(`- Canonical structured data lives in [todo_archive_registry.json](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_REGISTRY_PATH}).`);
  lines.push(`- The pre-JSON historical snapshot remains preserved at [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_LEGACY_PATH}).`);
  lines.push('');
  lines.push('## Historical Snapshot');
  lines.push(`- total_rows: ${registry.historical_snapshot.total_rows}`);
  lines.push(`- queued: ${registry.historical_snapshot.queued}`);
  lines.push(`- in_progress: ${registry.historical_snapshot.in_progress}`);
  lines.push(`- blocked: ${registry.historical_snapshot.blocked}`);
  lines.push(`- blocked_external_prepared: ${registry.historical_snapshot.blocked_external_prepared}`);
  lines.push(`- done: ${registry.historical_snapshot.done}`);
  lines.push(`- existing_coverage_validated: ${registry.historical_snapshot.existing_coverage_validated}`);
  lines.push('');
  lines.push('## Scripted Completion Archive');
  if (!items.length) {
    lines.push('- none yet');
  } else {
    for (const item of items) {
      lines.push(`- \`${item.id}\` — ${item.title}`);
      lines.push(`  completed_at: \`${item.completed_at}\``);
      lines.push(`  previous_section: \`${item.section}\``);
      lines.push(`  owner: \`${item.owner}\``);
      lines.push(`  deadline: \`${item.deadline}\``);
      lines.push(`  source_family: \`${item.source_family}\``);
      lines.push(`  summary: ${item.summary}`);
      if (item.completion_note) lines.push(`  completion_note: ${item.completion_note}`);
    }
  }
  lines.push('');
  lines.push('## Legacy Appendix');
  lines.push(`- Preserved historical markdown: [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/${TODO_ARCHIVE_LEGACY_PATH})`);
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function syncTodoViews(registry: TodoRegistry, archive: TodoArchiveRegistry): void {
  writeText(TODO_MARKDOWN_PATH, renderTodoMarkdown(registry));
  writeText(TODO_ARCHIVE_MARKDOWN_PATH, renderTodoArchiveMarkdown(archive));
}

export function newUpdatedAt(): string {
  return new Date().toISOString();
}

export function newCompletedAt(): string {
  return new Date().toISOString();
}

export function todayForOutput(): string {
  return new Date().toISOString().slice(0, 10);
}
