# Agent Runtime live proof summary: approval resume

- Date: 2026-06-08
- Source commit observed: `8ca71d2ef`
- Mode: InfRing-mediated external runtime
- Task: `approval_pause_resume`
- Classification: substrate/platform proof
- Native InfRing proof: no

## Result

| Framework | Status | Approval pause | Decision accepted | Resume forwarded | Post-resume failure | Artifact verified |
|---|---|---:|---:|---:|---:|---:|
| Codex | completed | yes | yes | yes | no | yes |
| Claude Code | completed | yes | yes | yes | no | yes |
| OpenCode | completed | yes | yes | yes | no | yes |

## Summary

- Completed: 3
- Failed: 0
- Registry coverage: pass
- Red gaps: 0
- Yellow gaps: 1

## Known gap

Codex and Claude Code both pass the practical usability loop in deterministic replay: approval pause, bounded approval projection, decision receipt, transcript persistence, context reload, artifact effect, and activity trace.

Codex now has live typed app-server transport acceptance for `thread/start` + `turn/start`. Claude Code remains `practical_with_gaps`, not daily-driver clean, because `upstream_native_transport_probe` is still pending for Claude stream-json.

Codex and Claude Code now both have dry-run candidate mappings ready, with Codex promoted one step further:

- Codex: app-server schema generation plus accepted live `thread/start` + `turn/start` typed transport.
- Claude Code: `stream-json` input/output plus an `AgentRuntimeStructuredTurn`-derived candidate stream without prompt compatibility enabled.

Claude live acceptance remains unproven and disabled by default.

## Accounting rule

This proof shows InfRing can mediate external engines through Gateway approval/resume and artifact verification. It does not prove native InfRing coding intelligence or native workflow capability.

Raw local artifacts are referenced in the JSON summary but are not copied into source control.
