# Aider Level 3 Speed Isolation

Date: 2026-05-22

## Question

Why does Aider complete the Level 3 existing-project edit much faster than Infring when both use `kimi-k2.6:cloud`?

## Direct comparison

Cross-framework Level 3 run with `kimi-k2.6:cloud`:

| System | Result | Wall time | Time to first mutation | Notes |
|---|---:|---:|---:|---|
| Aider | pass | 9.3s | 8.3s | Source and test files were preselected into chat. |
| ForgeCode | pass | 32.8s | 15.8s | Uses explicit inspect/edit/validate loop. |
| Infring | pass | 220.5s | 169.6s | Correct output, but native loop hit a long first-mutation tail. |
| mini-SWE-agent | functional success, harness fail | 17.9s | 10.4s | Mutated source/test and validation/probe passed, but did not report clean completion. |
| SWE-agent | fail | 5.3s | none | Local setup failed before task execution. |

## Aider A/B probe

The isolation probe compares Aider with the same prompt, model, edit format, validation command, and semantic probe.

Command:

```bash
python3 references/coding-agent-systems/runtime_trace_harness/aider_level3_speed_probe.py --model kimi-k2.6:cloud
```

Observed run:

| Aider variant | Result | Wall time | Outcome |
|---|---:|---:|---|
| `preselected_files` with `--file math_tools.py --file test_math_tools.py` | pass | 14.8s | Mutated both files and validation/probe passed. |
| `no_preselected_files` | fail | 12.6s | Exited quickly but made no mutation; asked for files to be added to chat. |

## Isolated factor

Aider's Level 3 advantage is not primarily better repository discovery in this task.

The key variable is:

```text
explicit file-scope injection -> one diff response -> deterministic patch apply -> auto-test
```

Without explicit file preselection, Aider refuses to edit existing files because it cannot safely create SEARCH/REPLACE blocks for files that are not in chat.

With explicit file preselection, Aider sends the relevant source/test content in one model request, receives SEARCH/REPLACE blocks, applies them deterministically, then auto-runs the validation command.

## What Infring should assimilate

Build a primitive lane:

```text
small_scoped_edit_artifact as a profile inside bounded_patch_artifact_lane
```

Proposed shape:

```text
runtime selects small source/test context
-> one provider call asks for patch artifact only
-> deterministic patch apply
-> validation command
-> semantic/completion probe when present
-> receipt-backed synthetic final
```

The lane should not replace the general native tool loop. It should be selected only when:

- project context is small or source/test targets are confidently selected,
- the task is a bounded existing-project edit,
- the target files are text files with deterministic patchable content,
- validation/probe commands are known or derivable,
- no broad architecture planning is needed.

## Integrated mapping

The portable Aider mechanic is represented as:

```text
small_scoped_edit_artifact
```

This is not an Aider clone. It maps into Infring's unified model as a Tier 2a
primitive profile:

```text
selected file context -> compact edit artifact -> safe_file_patch receipts
```

The parent bounded patch lane may activate this profile when file count and
context bytes are within budget. If not, the broader bounded patch artifact path
or the open native tool loop remains available.

Default runtime decision:

Keep `small_scoped_edit_artifact` dormant for now. The primitive remains part of
the model, but live same-model attempts showed high tail latency and timeout
cascades. The stable default is the general bounded patch artifact lane.

Quick-edit performance rule:

The general bounded patch artifact lane should not silently drift into the full
native loop after invalid artifact output or artifact-call timeout. It gets one
compact artifact retry, then a structured artifact failure unless the workflow
explicitly allows open-loop escalation.

Current optimization boundary:

- Keep model routing omitted for now so Infring, Aider, ForgeCode, and other
  comparisons remain same-model comparisons.
- Emit phase timing from the Infring lane before deciding whether the next speed
  gap is prompt/model latency, runtime startup, workflow loading, patch
  application, validation, or final synthesis.
- If the SEARCH/REPLACE profile times out or emits an invalid artifact, retry
  the general bounded patch artifact prompt with the same selected context before
  falling back to the full native tool loop.

## Infring gap

Infring currently asks the model to participate in the tool loop:

```text
bootstrap/read context
-> model emits native tool calls
-> runtime executes tools
-> model may rediscover/re-read/revalidate
-> runtime synthesizes final
```

That makes Infring more general, but for Level 3 it adds avoidable latency and first-turn stall risk.

## Primitive conclusion

The next high-ROI coding primitive is not more prompt tuning inside the open native loop.

It is a separate artifact lane that makes the fast path boring:

```text
selected files in prompt
patch artifact out
deterministic apply
validation receipt
synthetic final
```
