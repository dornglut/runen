# Runen Standard Environment

Status: **provisional boundary; APIs not yet standardized**

The Runen Standard Environment defines standard facilities available to implementations that claim the corresponding environment/profile.

It is separate from the Runen Language Specification.

A library namespace, module, runtime service, or common implementation technique is **not** a language primitive merely because it is shipped by a standard implementation.

## 1. Purpose

The Standard Environment is the home for facilities that should be portable and familiar but whose semantics do not need to be fundamental language syntax.

Candidate families include:

- `core` — fundamental library abstractions built on Core language semantics;
- `alloc` — allocation/container facilities for environments that provide allocation;
- `exec` — standard task/resource/parallel facilities built on Exec semantics;
- `model` — standard Model authoring/runtime facilities;
- `arch` — architecture-specific low-level facilities;
- `platform` — host/platform integration;
- `time` — clocks/timers and clock-domain adapters;
- `network` — networking/protocol facilities;
- `spatial` — spatial data structures/operations;
- `field` — field/sampling facilities;
- `render` — rendering abstractions;
- `ui` — user-interface abstractions;
- `asset` — asset/resource loading and management.

This list is architectural placement guidance, not a promise that every named family ships in an initial standard environment.

## 2. Profile dependency

An implementation claiming only **Runen Core** does not need the Hosted Standard Environment.

A profile may require a particular standard-environment subset. That requirement must be stated by the profile rather than inferred from a library's popularity.

## 3. Freestanding boundary

Freestanding Core MUST remain implementable without:

- heap allocation;
- filesystem;
- networking;
- threads;
- async runtime;
- GPU runtime;
- Model runtime;
- rendering/UI/asset systems.

Standard facilities that need those services belong behind explicit environment/profile boundaries.

## 4. Semantic ownership

The Standard Environment may expose ergonomic APIs for concepts whose semantics are language-level, but it does not redefine them.

Examples:

- a standard `Buffer` API must obey the normative Exec Buffer semantics;
- a standard query builder must obey Model query semantics;
- a standard `Result` type may represent recoverable domain errors without redefining `Fault`;
- a standard allocator does not redefine Core pointer provenance;
- a standard network API does not turn remote state into shared memory.

## 5. Standardization discipline

A facility SHOULD be standardized only when at least one of the following is true:

- interoperability requires common semantics;
- common portable source materially benefits from a shared API;
- the language semantics require a standard operation to be usable across implementations;
- multiple proving workloads show stable common structure.

A facility SHOULD remain a library/ecosystem concern when premature standardization would freeze one implementation architecture or domain-specific mechanism.

## 6. Currently deferred API work

No complete Standard Environment API is frozen by this document.

In particular, the following remain future work:

- allocation/container API;
- task executor/host integration;
- Buffer construction/mapping API;
- atomics and collectives API;
- clock conversion API;
- Model authoring API details;
- networking/protocol types;
- rendering/UI/spatial/field packages;
- component/package/versioning model.

Their absence does not make the corresponding language semantic architecture undefined where `language.md` already defines it.