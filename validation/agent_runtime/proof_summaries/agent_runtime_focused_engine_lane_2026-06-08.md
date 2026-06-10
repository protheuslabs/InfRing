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

Codex and Claude Code both pass the practical usability loop in deterministic replay: approval pause, bounded approval projection, decision receipt, transcript persistence, context reload, artifact effect, and activity trace.

Codex now has live typed app-server transport acceptance for `thread/start` + `turn/start`. Claude Code remains `practical_with_gaps`, not daily-driver clean, because `upstream_native_transport_probe` is still pending for Claude stream-json.

Codex and Claude Code now both have dry-run candidate mappings ready, with Codex promoted one step further:

- Codex: app-server schema generation plus accepted live `thread/start` + `turn/start` typed transport.
- Claude Code: `stream-json` input/output plus an `AgentRuntimeStructuredTurn`-derived candidate stream without prompt compatibility enabled.

Claude live acceptance remains unproven and disabled by default.

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
- `core/local/artifacts/proof_ledger_separation_guard_current.json`

## Limits

- This summary covers the focused promotion lane, not every registered engine.
- Catalog-only engines remain future support surfaces, not readiness claims.
- Raw local artifacts are intentionally not copied into source control.
