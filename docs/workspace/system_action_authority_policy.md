# System Action Authority Policy

Updated: 2026-06-06

## Rule

System actions are OS/runtime authority, not Gateway authority.

Gateway may own external route admission and bounded response projection for system controls, but the actual effects must remain in Core / ops system-action authority paths with receipts.

## Ownership split

| Concern | Owner | Notes |
| --- | --- | --- |
| Route admission | Gateway | Validate method, attach trace, bound payload, fail closed, forward to authority. |
| Restart effect | Core / ops | Canonical route exists in `core/layer0/ops` through `/api/system/restart`. |
| Shutdown effect | Core / ops | Canonical route exists in `core/layer0/ops` through `/api/system/shutdown`. |
| Update effect | Core / ops | Canonical route exists in `core/layer0/ops` through `/api/system/update`. |
| Host process exit | Legacy dashboard host shim | Allowed only as temporary compatibility glue for dashboard-host shutdown. |
| Shell rendering | Shell | Render controls and results only. |

## Current authority map

- Core route owner: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/late_c.rs`
- Core effect owner: `core/layer0/ops/src/dashboard_release_update.rs`
- Legacy host shim: `adapters/runtime/infring_dashboard.ts`
- Gateway read/projection wrapper: `gateway/runtime/gateway_system_routes.ts`

## Gateway must not do these

- Spawn restart, shutdown, or update processes directly.
- Resolve runtime binaries for system actions.
- Call resident IPC bridges directly for system action effects.
- Call `process.exit()` for authority decisions.
- Treat dashboard-host fallback dispatch as canonical authority.

## Required migration

The legacy dashboard host may temporarily keep shutdown/restart fallback glue so the UI remains usable during the physical Gateway re-root. That fallback is debt. The target shape is:

```text
Shell / CLI / SDK
  -> Gateway system-control route
  -> Core/ops system-action authority
  -> deterministic receipt/result
  -> Gateway bounded projection
  -> Shell / CLI / SDK
```

Acceptance criteria:

- Gateway system control routes forward to Core/ops authority.
- Mutating system actions return receipts or receipt refs.
- Dashboard host no longer owns detached subprocess dispatch for restart/update/shutdown.
- Dashboard host shutdown cleanup remains host lifecycle only, not system authority.
