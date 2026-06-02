# Agent runtime engines

This directory is the adapter-side home for swappable agent runtime engines.

Canonical contracts:

- `validation/conformance/contracts/agent_runtime_socket_contract.json`
- `validation/conformance/contracts/agent_runtime_engine_registry.json`

## Boundary rule

Agent runtimes plug into InfRing through the Gateway. They do not connect directly
to the Shell, Kernel, Validation, Observability, or Orchestration internals.

```text
Shell
  -> Gateway /ws/agent-runtime
    -> Agent Runtime Router
      -> selected engine adapter
        -> engine-specific transport
```

Current router seam:

- `agent_runtime_router.ts`
- `infring_native.ts`

The router loads the canonical registry, selects by `engine_id`, requires
`trace_id`, dispatches only to registered adapters, and rejects forbidden default
payload fields before events can become Gateway projections.

The native adapter exposes `orchestration/**` as `engine_id: infring_native`
through the same adapter methods as external engines. It does not move
orchestration code and does not fabricate live dispatch; Gateway wiring must
inject an orchestration client before `start_session` or `submit_turn` can run.

## Native engine

`orchestration/**` is the planned implementation path for:

```text
engine_id: infring_native
engine_kind: native_orchestration
```

The native engine is swappable at the runtime-engine layer. Kernel authority is
not swappable.

## External engines

External systems such as Codex CLI, Claude Code, OpenHands, OpenClaw, OpenFang,
or a custom socket engine may be added as adapters when they implement the same
Gateway-facing runtime contract:

- `health_check`
- `start_session`
- `submit_turn`
- `stream_events`
- `cancel_turn`
- `collect_artifacts`
- `emit_receipts`

Adapters may use custom internals. Their default Gateway stream must remain a
bounded projection with refs for heavy data.

## Authority invariant

Engines produce work intentions and outputs. Durable effects still require the
Kernel/Gateway authority path:

- capability checks
- workspace scope checks
- mutation approval
- trace propagation
- receipt emission
- artifact/detail refs

No engine adapter is allowed to bypass Kernel authority or write canonical memory
directly.
