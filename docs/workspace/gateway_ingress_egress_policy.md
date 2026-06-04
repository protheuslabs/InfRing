# Gateway Ingress/Egress Interface Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Shell-facing gateways, CLI-facing gateways, SDK-facing gateways, Core/Orchestration boundary adapters, and Gateway route contracts
Effective: April 2026

## Purpose

Gateways are boundary surfaces, not authority mirrors.

This policy separates Gateway traffic into explicit route classes so the Shell cannot become a full-state mirror, an authority relay, or a hidden runtime owner. The Gateway may accept bounded requests, emit bounded projections, report health/status, and fetch explicit details by ID. It must not collapse those responsibilities into one full-state endpoint.

The Gateway is the external ambiguity firewall. It is where ambiguous, untrusted, user-facing, Shell-facing, CLI-facing, SDK-facing, app-facing, package-facing, plugin-facing, and external-agent-facing traffic becomes typed, bounded, leased, auditable traffic before it can touch Core or Orchestration.

## Physical Domain Rule

Gateway is InfRing's skin. The canonical implementation domain for Gateway boundary behavior is `gateway/**`.

Adapters are not the skin. `adapters/**` may contain provider, framework, local process, HTTP, WebSocket, CLI, and protocol translators that sit behind Gateway sockets, but adapters must not own:

- Shell-facing sockets;
- CLI-facing or SDK-facing route authority;
- Gateway ingress classification;
- payload budget policy;
- permission gate policy;
- trace identity policy at the boundary;
- route admission;
- bounded egress projection policy.

Compatibility hosts that still live under `adapters/runtime/**` are transitional debt only. They must be declared in a validation contract with a retirement TODO and must route policy decisions to Gateway-owned modules. New Gateway policy under `adapters/**` is forbidden.

## Core Axiom

Ingress asks for work.

Egress reports bounded display output.

Health/status reports bounded readiness truth from the owner.

Detail fetch expands one referenced object.

Gateway enforces external ambiguity at the boundary.

No Gateway route may return the whole system state just because one consumer might need a subset.

Default route payloads must also satisfy the Interface Payload Budget policy in `docs/workspace/interface_payload_budget_policy.md`.

Conduit route posture must satisfy the Conduit/Scrambler Posture policy in `docs/workspace/conduit_scrambler_posture_policy.md`.

Cross-domain route inventory must satisfy `docs/workspace/cross_domain_nexus_route_inventory.md`.

Gateway itself is a Nexus boundary participant. Ingress and egress both enter and exit through Nexi, and each A-to-B edge is a Conduit; no Gateway route is exempt from Nexus checkpointing just because Gateway is the external ambiguity boundary.

## Gateway Enforcement Responsibilities

Gateway owns enforcement at the point where external ambiguity enters or leaves the system. It must enforce:

- authentication;
- authorization;
- payload limits;
- projection shaping;
- rate limits;
- capability checks;
- mutation approval checks;
- audit receipts;
- route policy;
- Shell isolation;
- lazy detail access;
- issue and eval submission controls.

These are boundary enforcement duties, not truth ownership duties. Gateway may verify, shape, deny, admit, rate-limit, and receipt traffic, but it must not become the canonical source of policy truth, permission truth, workflow truth, execution truth, memory truth, or runtime state.

When Gateway needs a truth-bearing answer, it must ask the owner through a Nexus checkpoint and Conduit. Gateway may cache bounded refs, receipts, and short-lived projection metadata only when the route contract and payload budget permit it.

## Gateway Non-Authority Rule

Gateway must not:

- infer user intent beyond route classification and typed request shaping;
- plan workflows;
- decide canonical permission truth;
- decide canonical policy truth;
- own durable runtime state;
- store full Shell/session/conversation mirrors;
- expose full Core, Orchestration, trace, tool, eval, memory, or policy objects by default;
- allow the Shell to use Gateway transport behavior as hidden readiness, retry, or admission authority.

## Required Route Classes

### Request Ingress

Request ingress routes carry operator, Shell, CLI, SDK, or adapter requests into the authoritative system.

Request ingress may carry:

- request id;
- actor/session id;
- command or intent;
- selected refs;
- typed input fields;
- capability/lease context;
- idempotency key;
- trace/correlation id.

Request ingress should return an acknowledgement, receipt, accepted/rejected status, and follow-up refs. It must not return full conversation history, full tool results, full traces, full workflow graphs, or raw runtime state.

Request ingress must apply authentication, authorization, payload limits, rate limits, capability checks, mutation approval checks, route policy, and audit receipts before forwarding any accepted request through the target Nexus checkpoint and Conduit.

### Event Output Egress

Event/output egress routes carry bounded system output projections back to the Shell or another presentation surface.

Event/output egress may carry:

- event id;
- event kind;
- display projection;
- status label;
- cursor/window refs;
- detail refs;
- receipt refs.

Event/output egress must not carry raw Core, Orchestration, planner, tool, trace, artifact, or eval envelopes by default.

Event/output egress must apply projection shaping, Shell isolation, payload budgets, and audit receipt linkage before any data enters Shell-facing state.

### Health And Status

Health/status routes expose bounded status projections from the owner of truth.

Health/status may carry:

- state enum;
- label;
- source;
- source sequence;
- age seconds;
- stale flag;
- degraded reason;
- next retry hint.

Health/status must not require the Shell to infer readiness, reconstruct runtime truth, or query full internal state to decide what is healthy.

### Detail Fetch

Detail-fetch routes expand one referenced object by ID.

Detail fetch may carry:

- stable detail id;
- requested view;
- capability scope;
- size/window bounds;
- receipt/audit context.

Detail fetch may return richer data than default projections, but it must be bounded, capability-scoped, audited, revocable, and Nexus-checkpointed.

Detail fetch is lazy by default. Raw traces, tool outputs, artifacts, workflow details, eval details, execution observations, and diagnostics must be fetched by stable detail ref, not bundled into default Gateway responses.

### Bounded Search/Query

Search/query routes return bounded result projections, not raw corpus mirrors.

Search/query may carry:

- query text or structured query;
- scope refs;
- cursor/window bounds;
- result limit;
- capability context.

Search/query must return hit ids, snippets, labels, counts, and detail refs. Deep payload search belongs behind the Gateway owner, not inside Shell memory.

Search/query must never require the Shell to retain or stringify full payloads in order to search.

## Prohibited Mixed Routes

Gateway routes must not combine unrelated responsibilities into a single full-state route.

Disallowed route shapes include:

- `full_state`
- `all_state`
- `mirror_state`
- `raw_runtime_state`
- `request_plus_full_result`
- `health_plus_state_dump`
- `search_plus_raw_payloads`
- `event_plus_trace_body`

If a route needs more than one class, split it into separate ingress, egress, status, detail, or query routes with shared correlation ids.

## Direction And Ownership Rule

Each route must declare:

- source domain;
- target domain;
- route class;
- owner of truth;
- payload class;
- capability or lease requirement;
- Nexus checkpoint requirement;
- audit receipt requirement;
- bounded response requirement.
- Conduit/Scrambler security posture.

Shell-facing route ownership is presentation-boundary ownership only. The route may collect input or display output, but truth and admission remain behind Core/Orchestration owners.

Gateway routes that carry sensitive detail fetches, authority-bearing request ingress, external agent/plugin ingress, recovery actions, policy/permission data, trace bodies, tool results, or execution observations must declare `strong_scrambler` Conduit posture rather than defaulting to standard transport.

## Gateway And Shell Relationship

The Shell may call Gateway routes. The Shell must not own Gateway policy.

The dashboard is a Shell. It must connect through Gateway sockets and receive bounded projections. It must not become the Gateway host, the runtime router, or an adapter dispatcher. Dashboard code may render, collect input, and display refs; it may not own Gateway ingress normalization, permission policy, runtime-engine selection authority, or payload-budget enforcement.

The Shell may keep refs, cursors, previews, and display-local state. It must not cache Gateway full responses as durable runtime state, infer missing health, or make hidden retry/admission decisions from Gateway transport behavior.

Default Shell-facing Gateway responses must declare byte, array, depth, string, cursor, detail-ref, audit, and Nexus budgets before they are accepted as default routes.

Issue-reporting and eval-submission routes are request ingress routes with additional controls. Taskbar issue submission may target external issue creation only through an approved Gateway route; chat-local report/eval submission may trigger internal eval only through a bounded Gateway route. Neither path may be implemented as Shell-owned policy, Shell-owned mutation, or raw context upload.

External surfaces have no first-party Shell exception. Shell, CLI, SDK, issue submission, eval submission, app, plugin, package, external-agent, and future mobile/Tauri traffic must enter through Gateway routes before reaching Nexus, Conduit, Kernel, or Orchestration owners.

## Enforcement

Enforcement command: `npm run -s ops:gateway:interface:guard`.

External surface coverage command: `npm run -s ops:gateway:external-surface:guard`.

Payload budget command: `npm run -s ops:interface:payload-budget:guard`.

Route inventory command: `npm run -s ops:nexus:route-inventory:guard`.

The guard validates `validation/conformance/contracts/gateway_ingress_egress_contract.json`, verifies this policy document contains the canonical route-class and external ambiguity firewall language, and fails closed if required route classes are missing, if required Gateway enforcement responsibilities are missing, if a route class allows full-state/default raw payloads, or if detail/search/status classes lack bounded/capability/audit/Nexus constraints.
