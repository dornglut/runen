# Core Value and Storage Semantics

Status: **provisional normative**

This document owns the currently defined Core semantics for values, local storage places, initialization state, ownership transfer, assignment, and destruction.

## Terms

### Type

A semantic classification for values and places. This revision defines scalar types and closed structural aggregate types for the operations below.

### Local

A typed storage root belonging to one function body. A local is immutable or mutable for assignment purposes.

### Place

A storage location denoted by a local plus zero or more structural field projections.

A place denotes storage; it is not itself a value.

### Sub-place

A place reached by projecting a field from an aggregate place.

### Value

An initialized semantic datum whose structure is compatible with the type required by its use. The currently defined value representation does not carry independent nominal type identity.

### Live

A scalar storage leaf currently contains an initialized value.

### Never-initialized

A scalar storage leaf has not yet been initialized in the current object lifetime.

### Dead

A scalar storage leaf was initialized previously and was subsequently consumed by move or destruction.

Aggregate initialization state is derived recursively from its leaves; it is not a separate boolean flag.

## Structural initialization

A scalar place is fully initialized exactly when its leaf is Live.

An aggregate place is fully initialized exactly when all recursively contained scalar leaves are Live.

An aggregate may be partially initialized when only a strict subset of its leaves are Live.

A move from one field affects only that field. A partially initialized aggregate cannot be read, moved, or copied as a whole until every required leaf is Live again.

## First initialization

`Init(dst, value)` writes a value into a place only if every leaf in `dst` is Never-initialized.

`Init` MUST NOT reinitialize storage that was previously initialized and later became Dead.

The value MUST structurally match the type of `dst`.

First initialization does not require the containing local to be mutable.

## Read

`Read(src)` requires `src` to be fully initialized.

`Read` does not transfer ownership and does not change initialization state.

Reading a partially initialized or Dead place is invalid in safe Core.

## Move

`Move(src)` requires `src` to be fully initialized.

It produces the complete value previously stored at `src` and changes every leaf in `src` from Live to Dead.

A later read, copy, or second move of that place is invalid unless the affected storage is legally reinitialized.

Moving a sub-place affects only that sub-place. Disjoint initialized sibling places remain Live.

Move itself performs no destruction of the transferred value.

## Copy

`Copy(src)` requires that `src` is fully initialized and its type is copyable.

It produces an equal owned value while leaving `src` Live.

For the structural types defined by this revision, an aggregate is copyable exactly when all of its fields are copyable.

The general language mechanism that determines copyability is not defined by this revision.

## Assignment

`Assign(dst, value)` requires the local containing `dst` to be mutable.

Unlike `Init`, `Assign` is path-state tolerant: `dst` may be wholly Never-initialized, partially initialized, fully Live, or contain Dead subobjects.

Assignment evaluates conceptually as:

1. evaluate the source operand;
2. destroy every currently Live subobject of `dst` in deterministic destruction order;
3. write the new value into `dst`;
4. mark all written leaves Live.

Assignment may therefore perform a mutable first write, replace a Live value, replace partial storage, or reinitialize storage after move or destruction.

Never-initialized and Dead subobjects have nothing to destroy before the write.

The source value MUST structurally match the type of `dst`.

## Destruction

Destruction consumes only currently Live values.

For a scalar, destruction changes Live storage to Dead. Never-initialized and Dead storage has nothing to destroy during automatic cleanup.

For a struct, Live fields are destroyed in reverse declaration order.

`Drop(place)` requires at least one Live leaf in `place`. It destroys every currently Live subobject of that place and leaves those subobjects Dead. Never-initialized subobjects remain Never-initialized.

A moved or already-destroyed subobject MUST NOT be destroyed a second time.

## Function termination cleanup

On both defined `Return` and defined `Fault`, function locals are cleaned in reverse local declaration order.

Each local is cleaned according to the destruction rules above, so partial initialization is respected and Dead or moved leaves are skipped.

`Fault` is a defined terminal state, not undefined behavior.

## Determinism

For a fixed typed body using only the semantics defined here, state transitions and destruction order are deterministic.

The semantics defined here do not depend on physical addresses, host destruction behavior, container iteration order, physical scheduling, or backend behavior.

There is no implicit execution-step budget. Cyclic control flow may diverge.

## Open semantic surface

This revision does not define heap or raw allocation, deallocation, borrowing, interior mutability, raw pointers, provenance, pinning, atomics, custom destructors, panic catching, asynchronous cancellation, ABI/layout guarantees, or source grammar.