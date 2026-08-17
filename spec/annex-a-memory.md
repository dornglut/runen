# Semantic Annex A — Values, Places, Initialization, Move, Copy, and Destruction

Status: **A0 executable subset / provisional**

This annex is intentionally narrower than the complete Annex A planned for ownership, borrowing, provenance, pinning, and unsafe behavior. It defines only the semantics implemented by the first executable reference machine.

## A0.1 Purpose

A0 exists to answer one question before syntax or code generation begins:

> Can Runen's basic value/place/initialization/destruction rules be small, deterministic, executable, and independent of Rust implementation accidents?

The reference machine is a conformance oracle. It is not a production runtime design.

## A0.2 Terms

### Type

A semantic classification for values and places. A0 contains scalar types and closed structural types.

### Local

A typed storage root belonging to one function body. A local is either immutable or mutable for assignment purposes.

### Place

A storage location denoted by a local plus zero or more structural field projections.

A place denotes storage; it is not itself a value.

### Sub-place

A place reached by projecting a field from an aggregate place.

### Value

An initialized semantic datum whose structure is compatible with the declared A0 type required by its use. A0 does not assign independent nominal type identity to the runtime-independent `Value` representation itself.

### Live

A scalar storage leaf currently contains an initialized value.

### Never-initialized

A scalar storage leaf has not yet been initialized in the current object lifetime.

### Dead

A scalar storage leaf was initialized previously and was subsequently consumed by move or destruction.

Aggregate initialization state is derived recursively from its leaves; it is not a separate boolean flag.

## A0.3 Structural initialization

A scalar place is fully initialized exactly when its leaf is `Live`.

An aggregate place is fully initialized exactly when all of its recursively contained scalar leaves are `Live`.

An aggregate may be partially initialized when only a strict subset of its leaves are `Live`.

This representation is required so that a move from one field does not invalidate disjoint fields. A partially initialized aggregate is not readable, movable, or copyable as a whole until every required leaf is live again.

## A0.4 First initialization

`Init(dst, value)` writes a value into a place only if every leaf in `dst` is `Never-initialized`.

`Init` MUST NOT be used to reinitialize storage that was previously initialized and later became `Dead`. Reinitialization/replacement is expressed by `Assign`.

The value MUST structurally match the type of `dst`.

A first initialization does not require the containing local to be mutable.

## A0.5 Read

`Read(src)` requires `src` to be fully initialized.

`Read` does not transfer ownership and does not change initialization state. In A0 it exists as an explicit semantic observation used to exercise the abstract machine; later MIR may refine ordinary reads into more specific operations.

Reading a partially initialized or dead place is a semantic error in safe Core.

## A0.6 Move

`Move(src)` requires `src` to be fully initialized.

It produces the complete value previously stored at `src` and changes every leaf in `src` from `Live` to `Dead`.

A later read, copy, or second move of that place is rejected unless the affected storage is legally reinitialized.

Moving a sub-place affects only that sub-place. Disjoint initialized sibling places remain live.

Move itself performs no destruction of the transferred value.

## A0.7 Copy

`Copy(src)` requires:

1. `src` is fully initialized; and
2. the type of `src` is structurally copyable.

It produces an equal owned value while leaving `src` live.

In A0, copyability is compiler-known:

- `bool` and `i64` are copyable;
- `Tracked` is not copyable;
- a struct is copyable exactly when all fields are copyable.

This marker is deliberately provisional. General trait semantics are not part of A0.

## A0.8 Assignment

`Assign(dst, value)` requires:

1. the local containing `dst` is mutable; and
2. `dst` is not wholly `Never-initialized`.

A first write to wholly never-initialized storage MUST use `Init`, even when the containing local is mutable. `Assign` is reserved for replacement or reinitialization of storage that already has initialization history.

Evaluation is conceptually:

1. evaluate the source operand;
2. destroy every currently live subobject of `dst` in deterministic destruction order;
3. write the new value into `dst`;
4. mark all written leaves live.

`Assign` may therefore reinitialize storage that became dead after a move. `Init` may not be substituted for that reinitialization.

The source value MUST match the type of `dst`.

## A0.9 Destruction

Destruction consumes only currently live values.

For a scalar:

- a live value is destroyed and becomes `Dead`;
- a never-initialized or already-dead value has nothing to destroy during automatic cleanup.

For a struct, live fields are destroyed in **reverse declaration order**.

An explicit `Drop(place)` requires at least one live leaf in `place`; it destroys all currently live subobjects of that place and leaves those subobjects dead. Never-initialized subobjects remain never-initialized.

A moved or already-dropped subobject MUST NOT be destroyed a second time.

A0 uses `Tracked(id)` solely to make destruction order/count observable in conformance tests.

## A0.10 Function termination cleanup

On both defined `Return` and defined `Fault`, the reference semantics clean up function locals in **reverse local declaration order**.

Each local is cleaned according to A0.9, so partial initialization is respected and dead/moved leaves are skipped.

`Fault` is a defined terminal state. It is not undefined behavior.

## A0.11 Determinism and continued execution

For a fixed typed A0 body, all A0 state transitions, destruction order, and terminal cleanup are deterministic.

No rule depends on allocator address, host-language drop behavior, hash iteration, thread scheduling, or backend behavior.

A0 imposes no implicit execution-step budget. A cyclic control-flow body may continue indefinitely. Tooling MAY externally bound or stop a reference execution for testing or resource control, but such a bound is not a Runen semantic error and MUST NOT be represented as one.

## A0.12 Reference-machine independence

The Rust implementation of the reference machine MUST NOT define Runen semantics by accident.

In particular:

- Rust ownership is not used as the semantic definition of Runen ownership;
- Rust `Drop` is not used as the semantic definition of Runen destruction;
- native addresses are not used as Runen place identity;
- host container iteration order is not semantically observable.

Every reference-machine transition MUST correspond to a rule in this annex or a later normative revision.

## A0.13 Required conformance cases

The A0 gate requires at least:

1. move invalidates the source;
2. copy preserves the source;
3. a partial move leaves disjoint fields live while making the containing aggregate unreadable as a whole;
4. independently initialized fields can form a fully initialized aggregate;
5. non-copy values cannot be copied;
6. `Init` cannot reinitialize storage that became dead after move;
7. `Assign` can reinitialize mutable storage that became dead after move;
8. `Assign` cannot perform the first initialization of wholly never-initialized storage;
9. assignment to immutable storage is rejected;
10. assignment drops a live replacement target before writing the new value;
11. explicit drop of a partially initialized aggregate destroys only its live subobjects;
12. an explicit drop is not repeated during scope cleanup;
13. struct fields are destroyed in reverse declaration order;
14. fault cleanup destroys each live local value exactly once in reverse declaration order.

## A0.14 Explicitly deferred

A0 does not define:

- borrows or references;
- reborrowing;
- interior mutability;
- raw allocation/storage;
- raw pointers or provenance;
- pinning/address stability;
- atomics or data races;
- custom destructors;
- panic payloads/catching;
- async cancellation;
- ABI/layout guarantees;
- source-language grammar.

Those are downstream Annex-A/Annex-B proving slices and MUST NOT be inferred from this A0 machine.
