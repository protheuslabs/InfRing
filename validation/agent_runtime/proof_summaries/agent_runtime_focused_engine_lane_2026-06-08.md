# Agent Runtime focused engine lane proof

Generated: 2026-06-08T22:14:30.000Z

Status: pass

## Scope

- Mode: InfRing-mediated Agent Runtime
- Focus engines: InfRing Native, Codex, Claude Code
- Catalog-only engines: 7
- Raw artifacts: local only

## Proof accounting

This is substrate/platform proof. It shows InfRing can govern selected runtime engines through Gateway with bounded projections, approval pause/resume, artifact effects, and proof-ledger separation.

It is not native InfRing intelligence proof. Native coding promotion remains on the separate native useful-work scoreboard.

## Result summary

- Completed: 3
- Failed: 0
- Red gaps: 0
- Yellow gaps: 2

## Current golden-pair usability headline

Codex and Claude Code both pass the practical usability loop: approval pause, bounded approval projection, decision receipt, transcript persistence, context reload, artifact effect, activity trace, and visible failure/error projection.

Both engines are now daily-driver candidates in the Agent Runtime scorecard.

Transport accounting is intentionally conservative:

- Codex: app-server schema generation identified a candidate `thread/start` + `turn/start` surface, but live submission remains disabled by default.
- Claude Code: `stream-json` input/output has a candidate mapping and passed live InfRing-mediated work through the adapter path, but the dedicated upstream live acceptance probe remains disabled by default.

Do not count candidate native transport surfaces as accepted upstream live typed-transport proof until the disposable live acceptance probes submit and pass.

## Focused rows

| Engine | Status | Approval | Artifact effect | Context | Parity |
| --- | --- | --- | --- | --- | --- |
| InfRing Native | completed | pass | pass | pass | pass |
| Codex | completed | pass | pass | pass | pass |
| Claude Code | completed | pass | pass | pass | pass |

## Source artifact refs

- `core/local/artifacts/agent_runtime_engine_focus_guard_current.json`
- `core/local/artifacts/agent_runtime_framework_coordination_guard_current.json`
- `core/local/artifacts/agent_runtime_real_work_replay_guard_current.json`
- `core/local/artifacts/agent_runtime_engine_scorecard_current.json`
- `core/local/artifacts/agent_runtime_native_transport_probe_current.json`
- `core/local/artifacts/agent_runtime_codex_app_server_mapping_probe_current.json`
- `core/local/artifacts/agent_runtime_codex_app_server_live_acceptance_probe_current.json`
- `core/local/artifacts/agent_runtime_claude_stream_json_mapping_probe_current.json`
- `core/local/artifacts/agent_runtime_claude_stream_json_live_acceptance_probe_current.json`
- `core/local/artifacts/agent_runtime_live_work_eval_claude_native_probe.json`
- `core/local/artifacts/agent_runtime_golden_pair_promotion_guard_current.json`
- `core/local/artifacts/proof_ledger_separation_guard_current.json`

## Limits

- This summary covers the focused promotion lane, not every registered engine.
- Catalog-only engines remain future support surfaces, not readiness claims.
- Raw local artifacts are intentionally not copied into source control.
