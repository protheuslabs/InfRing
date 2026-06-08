# Agent Runtime proof summaries

This directory stores bounded, source-controlled summaries of Agent Runtime proof runs.

Raw live artifacts remain under `core/local/artifacts/**` and are not copied here. These summaries only record compact operator evidence: run identity, scope, pass/fail signals, known gaps, and proof-accounting classification.

Validation guard:

```bash
node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/agent_runtime_proof_summary_guard.ts
```

The guard is intentionally strict. It rejects oversized summaries, copied raw traces, prompt/output bodies, failed approval-resume rows, and summaries that blur external-engine substrate proof into native InfRing intelligence proof.

## Proof accounting

Agent Runtime mediated success is substrate/platform proof. It proves InfRing can host, constrain, normalize, score, and resume external engines through Gateway.

It is not native InfRing intelligence proof. Native InfRing workflow and coding capability remains a separate scoreboard.
