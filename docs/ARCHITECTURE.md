# Architecture

## Workspace components

`ghostctl` is the CLI boundary. Its Phase C doctor performs bounded host
inspection and emits human-readable or versioned JSON reports. Its Phase D
commands build and run the deterministic CUDA probe without trace capture.
Capture, experiment validation, and report generation remain later-phase work.

`ghost-core` owns side-effect-free, versioned data models. Phase B defines
environment, experiment, group, and run documents. Deserialization rejects any
schema version other than the current version.

`ghost-analyze` is the future trace-normalization and statistical-comparison
library. Analysis logic is deferred to Phase F.

## Data flow

The planned Milestone 0 flow is:

```text
configuration -> environment validation -> probe execution -> immutable runs
              -> schema validation -> normalization -> statistical report
```

No capture or analysis behavior is implemented as part of Phases A/B.

## Experiment lifecycle

An experiment identifies one independent variable and references one group per
value. Each group references preserved run manifests. A failed attempt remains
in its run directory and receives a failure status; a completed run is never
mutated. Artifact paths are relative to the manifest that records them.

## Analysis pipeline

Phase F will parse captured text, normalize process-specific noise, group
repeated patterns, compare within-group and between-group variance, and emit
JSON and Markdown reports. It must not assign hardware semantics without
evidence.

## Schema versioning

Every standalone JSON document contains `schema_version`. Version `1` is the
only accepted version. Rust deserialization and JSON Schema both reject unknown
versions instead of silently interpreting them. A future schema change must
add explicit migration or multi-version handling before incrementing the
version.
