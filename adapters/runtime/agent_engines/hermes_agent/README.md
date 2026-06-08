# Hermes Agent Runtime Adapter

Status: safe CLI bridge.

Hermes Agent connects to InfRing only through the canonical Gateway Agent Runtime Socket. Hermes CLI/dashboard schemas stay private to this adapter and must be normalized into runtime-neutral Gateway events before Shell, Kernel, or Observability can consume them.

Private schema contract:

- `validation/conformance/contracts/agent_runtime_private_schemas/hermes_agent.json`

Reference clone:

- `references/coding-agent-systems/hermes-agent`

Default probes:

- CLI executable: `hermes`
- Upstream/user default CLI paths only, such as `~/.local/bin/hermes`, `~/.hermes/bin/hermes`, or `%LOCALAPPDATA%\hermes\bin\hermes.exe`
- Dashboard/API URL: `http://127.0.0.1:9119`

Local override:

- InfRing-managed/reference installs must be selected through explicit user override, config, or environment variable.
- Example: `INFRING_HERMES_AGENT_COMMAND=$HOME/.infring/external_runtimes/hermes_agent/venv/bin/hermes`
- See `core/local/agent_runtime/hermes_agent.local.env.example`.

Current live bridge:

- InfRing can use `hermes --toolsets safe -z <bounded InfRing context prompt>` as a prompt-text compatibility bridge.
- The bridge intentionally forbids Hermes `terminal`, `file`, `debugging`, `all`, and full Hermes preset toolsets.
- This limitation is deliberate: Hermes oneshot mode auto-bypasses native approvals, so mutating work must wait for a mediated approval bridge that can pause on InfRing Gateway decisions.
- Structured JSON/native session transport remains the target.

Boundary rules:

- Do not import Kernel internals from this adapter.
- Do not expose Hermes private event schemas as public socket schemas.
- Do not let Shell call this adapter directly.
- Gateway must health-check the configured Hermes endpoint or CLI safe bridge before projecting it as available.
- Durable effects remain proposal/receipt based through Gateway/Kernel policy.
