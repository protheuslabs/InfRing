# Layer Capability Enforcement Policy

This policy tells Kernel Sentinel how to watch layer drift.

## Layer0 hardware envelope

`Layer 0` is the microcontroller-to-edge minimal kernel band. Sentinel should treat web retrieval, browser automation, provider orchestration, semantic quality diagnostics, query rewriting, and freshness-intent policy as outside the `Layer 0` hardware envelope.

## Automation surfaces

- Changed-file blocker: `ops:layer-placement:check`
- Whole-repo report: `ops:layer-placement:report`
- Layer0 dependency proof: `ops:layer0:dependency-boundary:guard`
- Layer capability trend report: `ops:layer-capability:trend:report`
- Layer capability trend guard: `ops:layer-capability:trend:guard`
- Policy/config source: `client/runtime/config/layer_placement_policy.json`

## Sentinel expectations

- Sentinel must flag new Layer 0 web surfaces.
- Sentinel must flag new Layer 0 semantic policy.
- Sentinel must distinguish legacy layer debt from new regressions introduced in active changes.

## Legacy debt handling

Legacy violations should remain visible in the whole-repo report, but changed-file enforcement must remain strict so we do not normalize new drift while older debt is still being unwound.
