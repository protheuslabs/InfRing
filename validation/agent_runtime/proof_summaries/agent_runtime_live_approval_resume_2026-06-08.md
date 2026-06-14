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
- Yellow gaps: 2

## Known gap

Codex and Claude Code both pass the practical usability loop: approval pause, bounded approval projection, decision receipt, transcript persistence, context reload, artifact effect, activity trace, and visible failure/error projection.

Both engines are daily-driver candidates in the Agent Runtime scorecard.

Transport accounting is intentionally conservative:

- Codex: app-server schema generation identified a candidate `thread/start` + `turn/start` surface, but live submission remains disabled by default.
- Claude Code: `stream-json` input/output has a candidate mapping and passed live InfRing-mediated work through the adapter path, but the dedicated upstream live acceptance probe remains disabled by default.

Do not count candidate native transport surfaces as accepted upstream live typed-transport proof until the disposable live acceptance probes submit and pass.

## Accounting rule

This proof shows InfRing can mediate external engines through Gateway approval/resume and artifact verification. It does not prove native InfRing coding intelligence or native workflow capability.

Raw local artifacts are referenced in the JSON summary but are not copied into source control.
