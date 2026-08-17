# Time and Ordering Domains

Status: **provisional normative**

Runen does not define one implicit universal clock.

## Clock domain

A clock domain defines when temporal values or events are active or sampled. Examples may include simulation ticks, frames, audio samples, monotonic time, or another temporal coordinate.

Moving information between clock domains requires an explicit semantic operation such as sampling, holding, buffering, synchronization, interpolation, or resampling.

## State revision

A state revision identifies state-domain version or progress according to that domain's contract. It is not automatically wall time or a clock tick.

A committed state-domain state has an opaque semantic revision identity. Ordering or persistence properties are defined by the state-domain contract; no universal integer representation is required.

## Causal frontier

A causal frontier describes causal knowledge or ordering where a profile defines it. It is not automatically a state revision or clock domain.

## Freshness

Freshness identifies which source observation or revision a materialized or maintained result represents and how stale it may legally be.

Freshness is distinct from result correctness and from propagation progress.