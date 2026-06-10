# TODO Directory

This directory holds the internal work board for the repo.

Use [REAL_WORK_FIRST.md](/Users/jay/.openclaw/workspace/docs/workspace/REAL_WORK_FIRST.md) as the hard intake filter for new active work. Items should improve usability, reliability, or simplicity. In TODO metadata, `work_gate: real_work` is the practical work-board name for the usability law.

## Purpose

The files here are for operator and agent coordination. They track what we want to change, not what the system considers true.

Use this directory to:

- keep a live execution board
- keep a historical archive of completed work
- prepare for later script-based CRUD, audit, deadline, and archive operations

Do not use this directory as:

- Kernel authority
- Orchestration planning truth
- Validation evidence
- Observability truth
- a substitute for contracts, traces, receipts, issue candidates, or dossiers

## Files

- [todo_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_registry.json)
  Canonical live TODO data. Scripts mutate this file and render the Markdown views from it.
- [todo_archive_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_archive_registry.json)
  Canonical archive data for completed live-board items.
- [TODO.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO.md)
  Rendered live operator board. This should stay compact and action-oriented.
- [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE.md)
  Rendered historical ledger and completed-work archive.
- [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md)
  Preserved pre-JSON historical appendix.
- [AGENTIC_LOOP_ITEM_TEMPLATE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/AGENTIC_LOOP_ITEM_TEMPLATE.md)
  Template for non-trivial TODO/SRS items that should be executed through the agentic implementation loop.

## Current Field Conventions

Active backlog families in the live board should declare:

- `owner`
  The current human or agent expected to move the item next.
- `deadline`
  A normalized due marker such as an exact date, `none`, `external`, or a dependency-shaped value like `after_red_section`.
- `work_gate`
  Optional filter value: `real_work`, `reliability`, or `simplification`. `real_work` maps to the repo-wide `usability` law.
- `real_work_score`
  Optional priority score from `1` to `5`.

## Agentic Implementation Loop Intake

Non-trivial work items should use [AGENTIC_LOOP_ITEM_TEMPLATE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/AGENTIC_LOOP_ITEM_TEMPLATE.md) or carry the same minimum fields in registry metadata:

- `Objective`
- `Smallest slice`
- `Allowed lanes`
- `Forbidden lanes`
- `Evidence`
- `Promotion rule`

This connects TODO/SRS execution to [AGENTIC_IMPLEMENTATION_LOOP.md](/Users/jay/.openclaw/workspace/docs/workspace/AGENTIC_IMPLEMENTATION_LOOP.md). The goal is to make agentic work loop-based and evidence-producing instead of one-prompt implementation drift.

Allowed owner values can evolve, but the starting set is:

- `unassigned`
- `jay`
- `codex`
- `claude_code`
- `sentinel`

## Operating Rule

When a live item is completed:

1. remove it from the live board
2. preserve it in the archive
3. keep the archive as the historical reference, not the live board

## Scripted Workflow

- Canonical mutation target: `todo_registry.json` and `todo_archive_registry.json`
- Human-facing rendered views: `TODO.md` and `TODO_ARCHIVE.md`
- Manual command surface: `npm run -s ops:todo:board -- <command>`

Current supported commands:

- `status`
- `list`
- `archive-list`
- `render`
- `audit --apply=1`
- `add`
- `update`
- `move`
- `complete`
- `delete`

## Deadline Promotion Policy

- Exact-date deadlines due in `<= 7` days belong in `Red`.
- Exact-date deadlines due in `8-14` days belong in `Yellow`.
- Exact-date deadlines due in `> 14` days belong in `White`.
- `none`, `external`, and dependency-shaped values like `after_red_section` do not auto-promote by date.

## Future Direction

The next step from here is to make agents use the scripted surface consistently instead of hand-editing Markdown. Markdown now exists for readability, not as the canonical mutation target.
