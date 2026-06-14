# Agent Runtime proof summaries

This directory stores bounded, source-controlled summaries of Agent Runtime proof runs.

Raw live artifacts remain under `core/local/artifacts/**` and are not copied here. These summaries only record compact operator evidence: run identity, scope, pass/fail signals, known gaps, and proof-accounting classification.

Validation guard:

```bash
npm run -s ops:agent-runtime:proof-summary:guard
```

The guard is intentionally strict. It rejects oversized summaries, copied raw traces, prompt/output bodies, failed approval-resume rows, and summaries that blur external-engine substrate proof into native InfRing intelligence proof.

## Proof accounting

Agent Runtime mediated success is substrate/platform proof. It proves InfRing can host, constrain, normalize, score, and resume external engines through Gateway.

It is not native InfRing intelligence proof. Native InfRing workflow and coding capability remains a separate scoreboard.

## Secondary runtime promotion plans

Secondary runtime promotion plans are operator guidance, not promotion evidence.

They may say which secondary engine is closest to live probing, which dependency or setup step is missing, and which evidence should be gathered next. They must not claim that a secondary engine is daily-driver-ready, golden-pair-equivalent, or native InfRing intelligence proof.

Current compact plan artifact:

```text
core/local/artifacts/agent_runtime_secondary_promotion_plan_guard_current.json
```

Validation command:

```bash
npm run -s ops:agent-runtime:secondary-promotion-plan:guard
```

Promotion remains separate: Codex and Claude Code are the golden external pair until the full baseline proves otherwise.
