# Coding Runtime Trace Harness

This harness extracts comparable runtime-loop evidence from downloaded coding-agent reference repos.

It is a sensor harness, not a production runtime.

## What it captures

- step loops
- action/tool execution
- observation capture
- trajectory or state persistence
- edit contracts
- validation/repair hooks
- runtime command receipts
- stuck/budget/limit handling
- finalization/submit boundaries

## Why it exists

Infring coding should be rebuilt from observed mechanics of successful systems, not from eval-specific prompt patches.

The first mode is static source/doc sensing because it is provider-free, repeatable, and safe against the downloaded repos. Later modes can add live task runs for systems that are configured locally.

## Usage

```bash
python3 references/coding-agent-systems/runtime_trace_harness/trace_coding_runtime.py \
  --root references/coding-agent-systems \
  --out references/coding-agent-systems/runtime_trace_observations.json
```

Provider-free live probes:

```bash
python3 references/coding-agent-systems/runtime_trace_harness/live_trace_coding_runtime.py \
  --root references/coding-agent-systems \
  --out references/coding-agent-systems/runtime_live_trace_observations.json
```

Level 3/4 live coding baselines:

```bash
python3 references/coding-agent-systems/runtime_trace_harness/level3_level4_live_baseline.py \
  --systems infring,aider,forgecode \
  --levels 3 \
  --model kimi-k2.6:cloud
```

Claude Code is available as an opt-in live probe through the local Claude Code
CLI. It is not included in the default system list because it can use real
account usage. The probe is model-controlled: for Ollama-style control models it
prefers `ollama launch claude --model <model> --yes -- ...`, so Claude Code runs
through Ollama's Anthropic-compatible bridge instead of Anthropic's default API.
It does not silently route to `sonnet` or another stronger model. If
`INFRING_CLAUDE_CODE_MODEL` is set, it must equal the requested harness model or
the run is reported as `claude_code_control_model_mismatch`.

```bash
python3 references/coding-agent-systems/runtime_trace_harness/level3_level4_live_baseline.py \
  --systems claude-code \
  --levels 3 \
  --model kimi-k2.6:cloud
```

If the installed Claude Code CLI cannot run the requested control model, the
attempt should fail as a controlled-model compatibility result rather than being
counted as an uncontrolled Claude model comparison.

Bridge mode can be selected with:

```bash
INFRING_CLAUDE_CODE_BRIDGE=ollama-launch        # default
INFRING_CLAUDE_CODE_BRIDGE=anthropic-base-url   # sets ANTHROPIC_* env for Ollama
INFRING_CLAUDE_CODE_BRIDGE=direct               # bare claude --model, only for non-Ollama controls
```

The Claude Code probe writes stream/debug artifacts under each seeded fixture at:

```text
.infring/system_outputs/claude-code/
```

Codex is also available as an opt-in live probe through the local `codex` CLI.
The harness passes `--model` through from the requested model by default. Models
that look like local/Ollama model names, such as `kimi-k2.6:cloud` or `qwen*`,
are routed through Codex's own OSS runtime with `--oss --local-provider=ollama`.
Set `INFRING_CODEX_LOCAL_PROVIDER` to override that provider, or
`INFRING_CODEX_MODEL` to pin a Codex-specific model constant while keeping the
report's requested model visible.

```bash
INFRING_CODEX_MODEL=gpt-5.5 \
python3 references/coding-agent-systems/runtime_trace_harness/level3_level4_live_baseline.py \
  --systems codex \
  --levels 3 \
  --model gpt-5.5
```

```bash
python3 references/coding-agent-systems/runtime_trace_harness/level3_level4_live_baseline.py \
  --systems codex \
  --levels 3 \
  --model kimi-k2.6:cloud
```

The Codex probe writes JSONL/final-message artifacts under each seeded fixture at:

```text
.infring/system_outputs/codex/
```

## Output

The output conforms to:

```text
references/coding-agent-systems/runtime_trace_schema.json
```

The live output conforms to:

```text
references/coding-agent-systems/runtime_live_trace_schema.json
```
