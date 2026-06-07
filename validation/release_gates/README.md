# Validation release gates

This subdomain owns release-blocking controlled checks and promotion gates.

Canonical release-gate definitions now live here instead of under test harness paths:

- `config/release_gates.yaml` contains profile thresholds for runtime proof, boundedness, recovery, gateway chaos, and quality telemetry.
- `contracts/release_proof_pack_manifest.json` contains the required proof-pack artifact contract and category completeness/freshness budgets.
- `policies/release_blocker_rubric.json` contains release-blocker classification, status, ownership, and budget policy.
- `proof_packs/` contains generated release proof-pack snapshots and historical proof-pack evidence owned by Validation release gates.
- Temporary compatibility mirrors should be declared only if release-gate migration debt is reintroduced; there are no active release-gate mirror registries right now.

Manual/live operator evidence may be included as optional proof-pack evidence when it depends on a running host process rather than deterministic CI setup. For example, `ops:agent-runtime:socket-live-gateway:guard` proves the current live Gateway process accepts `/ws/agent-runtime` and emits bounded `engine.list.result` plus `heartbeat` events, but it must remain manual/live-only. Release lanes that need deterministic live evidence should run `ops:agent-runtime:socket-disposable-gateway:guard`, which starts a disposable Gateway host on a temporary port, probes the socket, then tears the host down.

Harnesses may live under `tests/tooling/**`, but release-gate truth should be read from this subdomain.
