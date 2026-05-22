# Native Coding Useful-Work Eval v1

Purpose: make native coding capability measurable as useful software work, not just successful tool execution or passing baseline tests.

## Promotion rule

A coding workflow change may use this eval as promotion evidence only when the worker path is Infring-native: Infring agent creation, provider routing, native file/command tools, and native receipts. Codex subagent runs can validate prompts and judges, but they are not capability evidence for Infring-native coding.

Default promotion target: `19/20` passes.

## Pass criteria

Each attempt must satisfy all checks:

- `validation_passes_after_worker`: the seeded validation command passes after the worker run.
- `new_regression_tests_exercised`: the post-worker test count is greater than the seed baseline.
- `expected_symbols_present`: source or test files contain the required public identifiers for the requested slice.
- `semantic_probe_passes`: an independent probe imports and exercises the requested behavior.
- `source_or_test_mutated_after_seed`: source or tests changed after seed creation.
- `native_receipts_observed`: the worker output includes native tool receipt evidence.
- `mutation_receipt_observed`: the receipt evidence includes `file_write` or `file_patch`.
- `validation_receipt_observed_or_command_verified`: validation is receipt-backed when possible, with judge command verification as the fallback proof.
- `final_answer_reports_changed_files`: the final worker output includes changed-file reporting.
- `final_answer_reports_validation`: the final worker output reports validation or test status.
- `worker_runtime_within_level2_budget`: each Level 2 worker runtime must stay within the current hard budget.

## No-fake-success rules

- Existing tests passing before mutation is baseline status, not completion evidence.
- Process exit zero alone is not software success.
- A final answer claiming success without changed source/test files is a failure.
- A changed file without semantic probe success is a failure.
- A semantic-looking final answer without native receipts is a failure for native capability claims.

## Timing metrics

The judge reports:

- `timing.batch_elapsed_ms`
- `timing.completion_span_ms`
- `timing.average_attempt_elapsed_ms`
- `timing.average_time_to_first_mutation_ms`
- `attempts[].timing.time_to_first_mutation_ms`

These metrics let us distinguish capability improvements from slower, more brittle loops.

## Level 2 latency gate

Correctness alone is not a Level 2 pass. A worker that eventually succeeds after an excessive local edit loop is operationally unhealthy.

Current hard gate:

- `LEVEL2_MAX_WORKER_RUNTIME_MS = 180000`

Any Level 2 attempt above this worker-runtime budget fails even if validation and semantic probes pass.

## Harness commands

Seed:

```bash
cargo run --quiet --manifest-path orchestration/Cargo.toml --bin native_coding_useful_work_eval_execute seed --attempts=20
```

Run one native worker per generated prompt and save each worker stdout to:

```text
<batch_root>/agent_outputs/<attempt_id>.json
```

Judge:

```bash
cargo run --quiet --manifest-path orchestration/Cargo.toml --bin native_coding_useful_work_eval_execute judge --batch-root=<batch_root>
```

## Current case set

- `existing_behavior_patch`: patch an existing pricing module with a discount feature while preserving baseline behavior.
- `multi_requirement_vertical_slice`: add a multi-file routing audit slice with aggregate behavior.
- `validation_repair_ready_csv_loader`: add CSV import behavior with invalid-row reporting and repair-ready validation.
- `no_fake_success_existing_tests`: reject baseline-test-only success by requiring new feature-flag behavior and new tests.

## Ownership

This eval belongs to the coding workflow/tooling lane. It does not replace research grounding evals, Shell Socket conformance, or general tool-CD marketplace work.
