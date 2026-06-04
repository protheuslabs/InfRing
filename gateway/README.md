# Gateway Domain

Gateway is InfRing's external boundary membrane: the system skin.

Gateway owns:

- Shell, CLI, SDK, app, plugin, and external-agent socket ingress
- request classification and admission at the boundary
- payload budgets and large-payload normalization
- permission-gate entrypoints
- trace propagation at external boundaries
- bounded egress projections and lazy detail refs

Gateway does not own:

- Kernel truth
- Orchestration planning truth
- Shell rendering or display-local state
- provider/framework-specific protocol translation

Provider/framework translation belongs under `adapters/**` and must sit behind Gateway sockets. Legacy Gateway hosts still under `adapters/runtime/**` are compatibility debt only and must be retired through `GATEWAY-PHYSICAL-REROOT`.
