# OpenClaw Agent Runtime Adapter

Status: planned adapter.

OpenClaw connects to InfRing only through the canonical Gateway Agent Runtime Socket. Any OpenClaw-specific API, socket, CLI, or event shape is private adapter input and must be normalized by Gateway before Shell, Kernel, or Observability can consume it.

Private schema contract:

- `validation/conformance/contracts/agent_runtime_private_schemas/openclaw.json`

Reference clone:

- `references/coding-agent-systems/openclaw`

InfRing-managed OpenClaw workspace:

- `~/.infring/external_runtimes/openclaw/workspace`

OpenClaw's upstream default remains `~/.openclaw/workspace` for ordinary new OpenClaw instances, but the InfRing-managed instance must not default there because this InfRing checkout is itself under `~/.openclaw`.

Boundary rules:

- Do not import Kernel internals from this adapter.
- Do not expose OpenClaw private event schemas as public socket schemas.
- Do not let Shell call this adapter directly.
- Gateway must health-check the configured OpenClaw endpoint before projecting it as available.
- Durable effects remain proposal/receipt based through Gateway/Kernel policy.
