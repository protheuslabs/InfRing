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

Codex still depends on bounded prompt/stdin context compatibility because true native structured context input is blocked on upstream CLI support.

## Accounting rule

This proof shows InfRing can mediate external engines through Gateway approval/resume and artifact verification. It does not prove native InfRing coding intelligence or native workflow capability.

Raw local artifacts are referenced in the JSON summary but are not copied into source control.

