# Clock Domains

Status: **provisional normative**

Runen does not define one implicit universal clock.

A **clock domain** defines when temporal values or events are active or sampled. Examples may include simulation ticks, frames, audio samples, monotonic time, or another temporal coordinate.

Moving information between clock domains requires an explicit semantic operation such as sampling, holding, buffering, synchronization, interpolation, or resampling.

A clock domain is not a state revision, causal frontier, or freshness measure. Those concepts have separate semantic owners.