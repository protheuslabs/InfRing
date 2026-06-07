#!/usr/bin/env node
/* eslint-disable no-console */
import {
  ArchivedTodoItem,
  TODO_ARCHIVE_REGISTRY_PATH,
  TODO_REGISTRY_PATH,
  TodoItem,
  TodoRegistry,
  assertTodoItemExists,
  auditTodoRegistry,
  findTodoItemIndex,
  loadTodoArchiveRegistry,
  loadTodoRegistry,
  newCompletedAt,
  newUpdatedAt,
  normalizeSection,
  saveTodoArchiveRegistry,
  saveTodoRegistry,
  sortArchivedItems,
  sortTodoItems,
  syncTodoViews,
  todayForOutput,
} from './todo_registry_lib.ts';

function flag(argv: string[], name: string): string | undefined {
  const prefix = `--${name}=`;
  const direct = argv.find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const index = argv.indexOf(`--${name}`);
  if (index >= 0 && index + 1 < argv.length) return argv[index + 1];
  return undefined;
}

function boolFlag(argv: string[], name: string): boolean {
  const value = flag(argv, name);
  return value === '1' || value === 'true' || argv.includes(`--${name}`);
}

function requireFlag(argv: string[], name: string): string {
  const value = flag(argv, name);
  if (!value) throw new Error(`missing required flag --${name}`);
  return value.trim();
}

function setUpdated(registry: TodoRegistry): void {
  registry.updated_at = newUpdatedAt();
}

function normalizeWorkGate(value: string | undefined): TodoItem['work_gate'] | undefined {
  if (!value) return undefined;
  const normalized = value.trim().toLowerCase();
  if (normalized === 'real_work' || normalized === 'reliability' || normalized === 'simplification') {
    return normalized;
  }
  throw new Error(`invalid work gate: ${value}`);
}

function normalizeRealWorkScore(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const score = Number(value);
  if (Number.isInteger(score) && score >= 1 && score <= 5) return score;
  throw new Error(`invalid real work score: ${value}`);
}

function output(payload: unknown): void {
  console.log(JSON.stringify(payload, null, 2));
}

function statusPayload() {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  return {
    ok: true,
    type: 'todo_status',
    today: todayForOutput(),
    registry_path: TODO_REGISTRY_PATH,
    archive_registry_path: TODO_ARCHIVE_REGISTRY_PATH,
    active_items: registry.items.length,
    archived_items: archive.items.length,
    red: registry.items.filter((item) => item.section === 'red').length,
    yellow: registry.items.filter((item) => item.section === 'yellow').length,
    white: registry.items.filter((item) => item.section === 'white').length,
  };
}

function listPayload() {
  const registry = loadTodoRegistry();
  return {
    ok: true,
    type: 'todo_list',
    items: sortTodoItems(registry.items),
  };
}

function addItem(argv: string[]) {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  const item: TodoItem = {
    id: requireFlag(argv, 'id'),
    title: requireFlag(argv, 'title'),
    section: normalizeSection(requireFlag(argv, 'section')),
    owner: requireFlag(argv, 'owner'),
    deadline: requireFlag(argv, 'deadline'),
    source_family: requireFlag(argv, 'source-family'),
    summary: requireFlag(argv, 'summary'),
    work_gate: normalizeWorkGate(flag(argv, 'work-gate')),
    real_work_score: normalizeRealWorkScore(flag(argv, 'real-work-score')),
  };
  if (registry.items.some((candidate) => candidate.id === item.id)) {
    throw new Error(`duplicate TODO item: ${item.id}`);
  }
  registry.items.push(item);
  setUpdated(registry);
  saveTodoRegistry(registry);
  syncTodoViews(registry, archive);
  return { ok: true, type: 'todo_add', item };
}

function updateItem(argv: string[]) {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  const id = requireFlag(argv, 'id');
  const item = assertTodoItemExists(registry.items, id);
  const title = flag(argv, 'title');
  const owner = flag(argv, 'owner');
  const deadline = flag(argv, 'deadline');
  const sourceFamily = flag(argv, 'source-family');
  const summary = flag(argv, 'summary');
  const workGate = flag(argv, 'work-gate');
  const realWorkScore = flag(argv, 'real-work-score');
  if (title) item.title = title.trim();
  if (owner) item.owner = owner.trim();
  if (deadline) item.deadline = deadline.trim();
  if (sourceFamily) item.source_family = sourceFamily.trim();
  if (summary) item.summary = summary.trim();
  if (workGate !== undefined) item.work_gate = normalizeWorkGate(workGate);
  if (realWorkScore !== undefined) item.real_work_score = normalizeRealWorkScore(realWorkScore);
  setUpdated(registry);
  saveTodoRegistry(registry);
  syncTodoViews(registry, archive);
  return { ok: true, type: 'todo_update', item };
}

function moveItem(argv: string[]) {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  const id = requireFlag(argv, 'id');
  const target = normalizeSection(requireFlag(argv, 'section'));
  const item = assertTodoItemExists(registry.items, id);
  item.section = target;
  setUpdated(registry);
  saveTodoRegistry(registry);
  syncTodoViews(registry, archive);
  return { ok: true, type: 'todo_move', item };
}

function completeItem(argv: string[]) {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  const id = requireFlag(argv, 'id');
  const note = flag(argv, 'note')?.trim();
  const index = findTodoItemIndex(registry.items, id);
  if (index < 0) throw new Error(`missing TODO item: ${id}`);
  const [item] = registry.items.splice(index, 1);
  const archived: ArchivedTodoItem = {
    ...item,
    completed_at: newCompletedAt(),
    completion_note: note,
  };
  archive.items.push(archived);
  setUpdated(registry);
  archive.updated_at = newUpdatedAt();
  saveTodoRegistry(registry);
  saveTodoArchiveRegistry(archive);
  syncTodoViews(registry, archive);
  return { ok: true, type: 'todo_complete', item: archived };
}

function deleteItem(argv: string[]) {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  const id = requireFlag(argv, 'id');
  const index = findTodoItemIndex(registry.items, id);
  if (index < 0) throw new Error(`missing TODO item: ${id}`);
  const [item] = registry.items.splice(index, 1);
  setUpdated(registry);
  saveTodoRegistry(registry);
  syncTodoViews(registry, archive);
  return { ok: true, type: 'todo_delete', item };
}

function renderViews() {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  syncTodoViews(registry, archive);
  return {
    ok: true,
    type: 'todo_render',
    active_items: registry.items.length,
    archived_items: archive.items.length,
  };
}

function auditItems(argv: string[]) {
  const registry = loadTodoRegistry();
  const archive = loadTodoArchiveRegistry();
  const apply = boolFlag(argv, 'apply');
  const { changed, promotions } = auditTodoRegistry(registry);
  if (apply && changed) {
    saveTodoRegistry(registry);
    syncTodoViews(registry, archive);
  }
  return { ok: true, type: 'todo_audit', apply, changed, promotions };
}

function archiveList() {
  const archive = loadTodoArchiveRegistry();
  return { ok: true, type: 'todo_archive_list', items: sortArchivedItems(archive.items) };
}

function helpPayload() {
  return {
    ok: true,
    type: 'todo_help',
    commands: [
      'status',
      'list',
      'archive-list',
      'render',
      'audit [--apply=1]',
      'add --id --title --section --owner --deadline --source-family --summary [--work-gate] [--real-work-score]',
      'update --id [--title] [--owner] [--deadline] [--source-family] [--summary] [--work-gate] [--real-work-score]',
      'move --id --section',
      'complete --id [--note]',
      'delete --id',
    ],
  };
}

function main() {
  const argv = process.argv.slice(2);
  const command = argv[0] ?? 'help';
  let payload: unknown;
  switch (command) {
    case 'status':
      payload = statusPayload();
      break;
    case 'list':
      payload = listPayload();
      break;
    case 'archive-list':
      payload = archiveList();
      break;
    case 'render':
      payload = renderViews();
      break;
    case 'audit':
      payload = auditItems(argv);
      break;
    case 'add':
      payload = addItem(argv);
      break;
    case 'update':
      payload = updateItem(argv);
      break;
    case 'move':
      payload = moveItem(argv);
      break;
    case 'complete':
      payload = completeItem(argv);
      break;
    case 'delete':
      payload = deleteItem(argv);
      break;
    default:
      payload = helpPayload();
      break;
  }
  output(payload);
}

main();
