# Model State Domains and Observation

Status: **provisional normative**

## State domain

A **state domain** controls a coherent set of logical state, invariants, revisions, admission, and commits.

The term **authority** is reserved for security/capability meaning and is not the primary Model ownership term.

A state domain may additionally define observation/isolation, durability, failure, replication, or maintenance contracts.

No state domain is implicitly process-global.

## ObservationSet

A Model evaluation spanning state domains is evaluated relative to an explicit immutable `ObservationSet` identifying the admitted observations for that evaluation or reaction wave.

The `ObservationSet` is immutable for that wave.

It does not imply one globally synchronized distributed snapshot unless a stronger profile explicitly guarantees one.

Compatibility rules for composing observations from multiple state domains are unspecified in this revision.

## Revisions and causality

A state revision identifies version/progress in a state domain's semantics.

A causal frontier describes causal knowledge or order where a profile requires it.

Neither concept is a clock domain by definition.