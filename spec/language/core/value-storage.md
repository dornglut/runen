# Core Value and Storage Semantics

Status: **provisional normative**

This document owns the currently defined Core semantics for values, local storage places, storage extent, stored-value lifetime, initialization state, ownership transfer, assignment, destruction domains, and cleanup.

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

### Storage extent

The storage extent of a place is the interval of execution during which that storage exists and may potentially hold a value.

For the Core MIR defined by this revision, every local storage root exists from function-body entry through that local's termination cleanup. Structural sub-place storage exists within the storage extent of its containing local.

Storage extent is independent of initialization state. Never-initialized, Live, and Dead storage all continue to exist until their storage extent ends.

Ending, moving, destroying, or replacing a stored value does not by itself end the containing storage extent.

Storage extent does not imply a physical address, allocation identity, relocation rule, or pointer provenance.

### Stored-value lifetime

A stored-value lifetime is the interval during which one scalar storage leaf is Live with one stored semantic value.

A stored-value lifetime begins when a semantic write to that scalar leaf completes and changes it to Live.

A stored-value lifetime ends when the stored value is consumed by move, destroyed, or destroyed as part of replacement. A later write into the same storage begins a new stored-value lifetime without creating a new storage extent.

`Read` and `Copy` do not end the source stored-value lifetime.

The current revision defines stored-value lifetime at scalar storage leaves. Aggregate initialization and liveness are derived recursively from the states of those leaves; an aggregate does not acquire a separate hidden lifetime identity.

Transient values produced while evaluating constants, moves, or copies are semantic values. This revision does not give such transient operand results independently addressable storage or a separately specified storage extent.

### Live

A scalar storage leaf currently contains an initialized value and therefore has an active stored-value lifetime.

### Never-initialized

A scalar storage leaf has not yet begun any stored-value lifetime during its current storage extent.

### Dead

A scalar storage leaf previously had a stored-value lifetime that ended by move, destruction, or replacement, and it has not subsequently been written again.

Aggregate initialization state is derived recursively from its leaves; it is not a separate boolean flag.

### Destruction domain

The destruction domain of a place at a semantic step is the ordered sequence of currently Live scalar leaf places recursively contained by that place.

For a scalar place, the destruction domain is that place itself when Live and is empty when Never-initialized or Dead.

For an aggregate place, the destruction domain is formed by recursively concatenating field destruction domains in reverse field declaration order.

A destruction domain is determined from semantic storage state at the point where destruction is to occur. Never-initialized, moved, and already-destroyed leaves are not members of the domain.

The destruction domain specifies which stored-value lifetimes are ended by destruction and in what order. It does not define a custom destructor body.

## Structural initialization

A scalar place is fully initialized exactly when its leaf is Live.

An aggregate place is fully initialized exactly when all recursively contained scalar leaves are Live.

An aggregate may be partially initialized when only a strict subset of its leaves are Live.

A move from one field affects only that field. A partially initialized aggregate cannot be read, moved, or copied as a whole until every required leaf is Live again.

Partial initialization does not change storage extent. It changes only which scalar leaves currently have stored-value lifetimes.

## First initialization

`Init(dst, value)` writes a value into a place only if every leaf in `dst` is Never-initialized.

`Init` MUST NOT reinitialize storage whose previous stored-value lifetime ended and which therefore became Dead.

The value MUST structurally match the type of `dst`.

First initialization does not require the containing local to be mutable.

Each scalar leaf written by a successful `Init` begins its first stored-value lifetime in that storage extent.

## Read

`Read(src)` requires `src` to be fully initialized.

`Read` does not transfer ownership, change initialization state, or end any stored-value lifetime.

Reading a partially initialized or Dead place is invalid in safe Core.

## Move

`Move(src)` requires `src` to be fully initialized.

It produces the complete value previously stored at `src` and changes every leaf in `src` from Live to Dead.

Each affected source stored-value lifetime ends at the move. Move does not destroy the transferred value.

A later read, copy, or second move of that place is invalid unless the affected storage is legally reinitialized.

Moving a sub-place affects only that sub-place. Disjoint initialized sibling places remain Live and their stored-value lifetimes continue.

The semantic value produced by the move may subsequently be written into another place; such a write begins stored-value lifetimes at the destination rather than extending the ended source storage lifetimes.

## Copy

`Copy(src)` requires that `src` is fully initialized and its type is copyable.

It produces an equal owned value while leaving `src` Live. The source stored-value lifetimes therefore continue unchanged.

When the produced copy is written into destination storage, that write begins distinct stored-value lifetimes at the destination leaves.

For the structural types defined by this revision, an aggregate is copyable exactly when all of its fields are copyable.

The general language mechanism that determines copyability is not defined by this revision.

## Assignment

`Assign(dst, value)` requires the local containing `dst` to be mutable.

Unlike `Init`, `Assign` is path-state tolerant: `dst` may be wholly Never-initialized, partially initialized, fully Live, or contain Dead subobjects.

Assignment evaluates conceptually as:

1. evaluate the source operand completely;
2. determine the destruction domain of `dst` from the resulting storage state;
3. destroy exactly that domain in its defined order, ending those old stored-value lifetimes;
4. write the new value into `dst`;
5. mark all written leaves Live, beginning new stored-value lifetimes there.

The source-first rule is semantically significant. If source evaluation moves from storage related to `dst`, those moved leaves are already Dead when the destination destruction domain is determined and therefore MUST NOT be destroyed as part of replacement.

Assignment may therefore perform a mutable first write, replace a Live value, replace partial storage, or reinitialize storage after move or destruction.

Never-initialized and Dead subobjects have nothing to destroy before the write.

The source value MUST structurally match the type of `dst`.

Assignment changes stored-value lifetimes but does not by itself end the destination storage extent.

## Destruction

Destruction consumes only currently Live stored values.

Destroying a scalar Live place ends its stored-value lifetime and changes the leaf to Dead. Never-initialized and Dead storage has nothing to destroy during automatic cleanup.

Destroying an aggregate destroys exactly its destruction domain. The recursive definition of that domain gives reverse declaration order for struct fields while skipping leaves that are not Live.

`Drop(place)` requires a non-empty destruction domain. It destroys exactly that domain once. Destroyed leaves become Dead; Never-initialized leaves remain Never-initialized.

A moved or already-destroyed subobject MUST NOT be destroyed a second time.

The current revision has no custom destructor body. A later custom-destructor specification may refine actions that occur during destruction, but it must preserve the selected destruction domain and ordering unless the canonical owner of those rules explicitly changes them.

## Function termination cleanup

On both defined `Return` and defined `Fault`, function locals are cleaned in reverse local declaration order.

When a local is reached for cleanup, its then-current destruction domain is computed and destroyed. Partial initialization is therefore respected and Never-initialized, Dead, moved, or already-destroyed leaves are skipped.

A local's storage extent continues through its cleanup and ends after that cleanup completes. Structural sub-place storage ends with the containing local.

Defined `Fault` uses the same stored-value lifetime and destruction-domain rules as defined `Return`. `Fault` is a defined terminal state, not undefined behavior.

For a cyclic execution that diverges, no termination cleanup occurs merely because execution has run for a long time; there is no implicit step budget that ends storage extents.

## Determinism

For a fixed typed body using only the semantics defined here, state transitions, stored-value lifetime transitions, destruction domains, and destruction order are deterministic.

The semantics defined here do not depend on physical addresses, host destruction behavior, container iteration order, physical scheduling, or backend behavior.

There is no implicit execution-step budget. Cyclic control flow may diverge.

## Separate semantic owners

This document does not define heap or raw allocation, deallocation, borrowing or borrow duration, interior mutability, references, raw pointers, provenance, pinning, atomics, custom destructor bodies, panic catching, asynchronous cancellation, ABI/layout guarantees, or source grammar.

Where this revision defines lifetime facts that those concerns may later depend on, their canonical owners govern the additional policy. In particular, this document does not decide whether a future borrow or pointer remains valid across mutation, replacement, relocation, or any other operation.
