# Core Effects and Faults

Status: **provisional normative**

An effect is semantically observable interaction outside ordinary pure value derivation, for example I/O, volatile access, time or random observation, external mutation, event emission, or communication.

Purity does not imply termination, absence of defined faults, safe speculation, or numeric reproducibility.

Defined faults are distinct from undefined behavior and from ordinary recoverable result values.

The complete panic/fault/unwind/catch model and exact source-level effect system are unspecified in this revision.
