# Agent Runtime Task Harness

This harness exists to compare agentic frameworks in two modes:

1. Native mode: the framework is run through its own CLI/runtime.
2. InfRing socket mode: the same task is run through the InfRing Agent Runtime socket.

The purpose is not only "does it answer?" The purpose is to catalog what each framework can do natively, then prove whether InfRing can preserve those capabilities through a clean socket boundary.

## Proof accounting

External-engine success is substrate proof, not native InfRing intelligence proof.

- `substrate/platform proof`: InfRing safely hosts, constrains, scores, and normalizes an external runtime through Gateway.
- `native intelligence proof`: InfRing-native workflows, tools, memory, and receipts perform useful work without relying on an external framework as the primary actor.

The harness primarily produces substrate/platform proof. Native InfRing promotion targets must remain on a separate scoreboard.

## Ownership

This is a Validation-domain asset.

Validation owns the task matrix, capability catalog, scoring rules, and reports. Gateway owns the runtime socket. Adapters translate framework-private schemas. Shell only displays projections and must not own the harness logic.

## Files

- `framework_capability_catalog.json`: known framework capabilities, native probes, socket IDs, and known gaps.
- `agentic_task_matrix.json`: controlled tasks used to compare native and InfRing-mediated behavior.
- `agent_runtime_task_harness_contract.json`: result shape and scoring contract.
- `tests/tooling/scripts/ci/agent_runtime_task_harness.ts`: executable runner.

## Default behavior

The runner is dry-run by default. Dry-run mode checks coverage, command plans, socket plans, and catalog completeness without invoking external agents.

Live execution requires either:

```bash
INFRING_AGENT_RUNTIME_TASK_HARNESS_LIVE=1 node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/agent_runtime_task_harness.ts --mode=both --framework=codex_cli
```

or:

```bash
node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/agent_runtime_task_harness.ts --mode=both --framework=codex_cli --live=1
```

## Core comparison

For every framework/task pair, the harness records:

- native availability
- socket availability
- context continuity
- activity/decision-dialog visibility
- tool-call visibility
- approval behavior
- working-directory behavior
- artifact/read/write behavior
- model-selection behavior
- failure reporting

## Pass condition

A framework is not considered "working through InfRing" just because it returns text. The socket path should preserve the useful native capability surface:

- The agent receives bounded continuity context.
- Durable actions become proposals or receipts.
- Approval-required actions pause or produce a resumable request.
- Tool/status/dialog output becomes user-readable runtime activity.
- Failures become explicit chat-visible diagnostics.
- The same working directory and model intent are respected.

## Report output

Reports are written under:

```text
core/local/artifacts/agent-runtime-task-harness/
```

The JSON report is the source of truth. The Markdown report is a compact operator summary.
