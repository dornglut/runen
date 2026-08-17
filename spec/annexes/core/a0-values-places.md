# Core Annex A0 — Values, Places, Initialization, Move, Copy, Assignment, and Destruction

Status: **normative for the A0 semantic subset**

This annex specifies the accepted A0 subset of Core value and storage semantics. It does not define borrowing, raw pointers, atomics, ABI, source grammar, or other later Core facilities.

## Terms

### Type

A semantic classification for values and places. A0 contains scalar types and closed structural aggregate types.

### Local

A typed storage root belonging to one function body. A local is immutable or mutable for assignment purposes.

### Place

A storage location denoted by a local plus zero or more structural field projections.

A place denotes storage; it is not itself a value.

### Sub-place

A place reached by projecting a field from an aggregate place.

### Value

An initialized semantic datum whose structure is compatible with the A0 type required by its use. A0 does not assign independent nominal type identity to the runtime-independent value representation itself.

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

Reading a partially initialized or Dead place is invalid in safe A0 Core.

## Move

`Move(src)` requires `src` to be fully initialized.

It produces the complete value previously stored at `src` and changes every leaf in `src` from Live to Dead.

A later read, copy, or second move of that place is invalid unless the affected storage is legally reinitialized.

Moving a sub-place affects only that sub-place. Disjoint initialized sibling places remain Live.

Move itself performs no destruction of the transferred value.

## Copy

`Copy(src)` requires that `src` is fully initialized and its type is copyable.

It produces an equal owned value while leaving `src` Live.

A0 copyability is structural: an aggregate is copyable exactly when all of its fields are copyable.

The complete trait mechanism for expressing copyability is outside A0.

## Assignment

`Assign(dst, value)` requires the local containing `dst` to be mutable.

Unlike `Init`, `Assign` is path-state tolerant: `dst` may be wholly Never-initialized, partially initialized, fully Live, or contain Dead subobjects.

Assignment evaluates conceptually as:

1. evaluate the source operand;
2. destroy every currently Live subobject of `dst` in deterministic destruction order;
3. write the new value into `dst`;
4. mark all written leaves Live.

Assignment may therefore perform a mutable first write, replace a Live value, replace partial storage, or reinitialize storage after move/destruction.

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

For a fixed typed A0 body, A0 state transitions and destruction order are deterministic.

A0 semantics are independent of allocator addresses, host-language destruction behavior, container iteration order, thread scheduling, and backend behavior.

A0 imposes no implicit execution-step budget. Cyclic control flow may diverge.

## Exclusions

A0 does not specify:

- borrows, references, or reborrowing;
- interior mutability;
- raw allocation or raw pointers;
- provenance;
- pinning or address stability;
- atomics or data races;
- custom destructors;
- panic catching or payloads;
- async cancellation;
- ABI or layout guarantees;
- source-language grammar.
