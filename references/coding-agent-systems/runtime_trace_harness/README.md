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

Grok Build is available as an opt-in live probe through the local Grok binary.
The default binary path is `/Users/jay/.grok/bin/grok`, overrideable with
`INFRING_GROK_BIN`. The harness passes the requested model through `--model`
and refuses mismatched `INFRING_GROK_MODEL` overrides so comparison runs do not
silently use an uncontrolled model. If Grok is not authenticated, the attempt is
reported as `grok_not_authenticated` rather than counted as a coding result.

```bash
python3 references/coding-agent-systems/runtime_trace_harness/level3_level4_live_baseline.py \
  --systems grok \
  --levels 3 \
  --model kimi-k2.6:cloud
```

Optional sandbox knob:

```bash
INFRING_GROK_SANDBOX=...
```

Control-model limitation: Grok exposes `--cli-chat-proxy-base-url` and
`--xai-api-base-url` on the `grok agent` subcommand, but not on the top-level
`--single` path used by this deterministic fixture harness. The current Grok
probe therefore supports model control only for model IDs accepted by Grok's
single-turn runtime. An `agent stdio` or `agent headless` bridge is needed
before Ollama/local control models can be routed through Grok.

Preferred control-model path: add a custom `[model.<alias>]` entry to
`~/.grok/config.toml` and run the harness with that alias. For example,
`infring-kimi-control` can point to Ollama's OpenAI-compatible endpoint at
`http://127.0.0.1:11434/v1` while sending `kimi-k2.6:cloud` as the upstream
model id. A Level 3 smoke run passed through this path with
`model_controlled=true`.

For behavior-only probes, set `INFRING_GROK_USE_DEFAULT_MODEL=1` to omit
`--model` and let Grok use its configured default model. Those runs are marked
`model_controlled=false`.

The harness intentionally does not pass Claude-style `--tools` names to Grok.
The first smoke probe showed those names do not match Grok Build's tool filter,
so the probe relies on `--always-approve` and `--permission-mode
bypassPermissions` while capturing emitted tool events for comparison.

The Grok probe writes streaming/stdout/stderr artifacts under each seeded
fixture at:

```text
.infring/system_outputs/grok/
```

If Grok emits a session id in streaming JSON, the harness also attempts a local
`grok trace --local --json` export into the same output directory.

## Output

The output conforms to:

```text
references/coding-agent-systems/runtime_trace_schema.json
```

The live output conforms to:

```text
references/coding-agent-systems/runtime_live_trace_schema.json
```
