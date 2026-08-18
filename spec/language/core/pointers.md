# Core Pointers and Provenance

Status: **provisional normative; storage identity and raw-pointer formation defined**

This document owns the currently defined Core semantics for dynamic storage-instance identity as observed by pointer formation, structural pointer targets, raw-pointer values, provenance formation, and preservation under ordinary value transport.

Storage extent, stored-value lifetime, initialization state, replacement, and destruction are owned by [Core value and storage semantics](value-storage.md). Shared/exclusive authority used while forming a pointer from a place is owned by [Core borrowing](borrowing.md).

## Semantic decomposition

Runen distinguishes all of the following:

- a static MIR local declaration;
- a static structural `Place` rooted in that declaration;
- one dynamic storage instance created for one execution of a local storage extent;
- a structural storage region within that instance;
- the stored-value lifetime currently occupying that region, if any;
- a loan and its current alias authority;
- a raw-pointer value;
- the pointer's provenance;
- a numeric or physical address.

None of those concepts is implicitly identified with another unless this specification states so.

In particular:

- `LocalId` is not dynamic storage identity;
- `LoanId` is not pointer provenance;
- stored-value lifetime is not storage identity;
- pointer provenance is not defined as a numeric address;
- a raw pointer is not a source-language reference and does not inherit a loan interval.

## Dynamic storage-instance identity

Each dynamic local storage extent has one **storage-instance identity**.

For the current single-body Core reference machine, every local creates one such instance when execution of the body begins. Distinct simultaneously existing local storage extents have distinct identities.

The identity remains stable for the complete storage extent. Therefore the following operations do not create a new storage instance while the containing storage extent continues:

- first initialization;
- move from the storage;
- explicit destruction;
- ordinary replacement or reinitialization;
- interior replacement or reinitialization.

Those operations may begin or end stored-value lifetimes, but they do not replace the underlying storage instance.

When the storage extent ends, that dynamic storage instance ends. This revision does not define reuse of a former identity by a later storage instance.

A reference implementation may represent storage-instance identity using a deterministic integer or another convenient token for verification. That representation is not Runen-observable and does not define a physical address, allocation number, ABI property, or source-visible identity.

Future dynamic storage owners such as call activations or allocations must create storage-instance identities according to their owning semantics. They must not reinterpret a static `LocalId` as globally unique dynamic storage identity.

## Structural storage region

A **structural storage region** consists of:

1. one dynamic root storage-instance identity; and
2. zero or more structural projections selecting a sub-place within that root.

The current projection vocabulary contains only struct fields.

The structural projection path is semantic structure. It is not a byte offset, field address, ABI layout, physical range, or promise that the selected storage cannot relocate.

Two different stored-value lifetimes occupying the same structural region remain lifetimes in the same storage region when the root storage extent has not ended.

## Raw-pointer type

The current proving kernel has a capability-neutral raw-pointer type parameterized by one pointee type.

The pointee relation is semantic indirection, not structural containment. Consequently a raw-pointer pointee edge does not make a finite structural type recursively infinite. A struct may therefore contain a raw pointer whose pointee type eventually refers back to that struct, while direct structural recursion remains invalid.

This revision does not define source spelling, shared-versus-mutable raw-pointer qualifiers, variance, nullability, layout, size, alignment, or ABI representation.

Raw-pointer types are copyable in the current Core value model.

## Raw-pointer value

A non-null raw-pointer value formed by the currently defined operation carries two semantic components:

- a **target structural storage region**; and
- **provenance** rooted in the dynamic storage instance from which the pointer was formed.

For this initial foundation, provenance is the identity of that target's root storage instance.

The target and provenance are represented separately even though the target also names its root instance. This is deliberate: later pointer derivation may refine where within storage a pointer targets without making target position and provenance the same semantic concept.

A raw-pointer value does not contain a `LoanId`, does not extend a borrow interval, and does not by itself carry shared or exclusive loan authority.

The proving MIR's constant-value vocabulary cannot fabricate a non-null raw pointer. Non-null provenance is introduced only by the defined dynamic formation operation below. A reference implementation may therefore use a richer private runtime-value representation than the MIR constant representation.

## Pointer formation

The proving MIR forms a non-null raw pointer with:

```text
AddressOf(src: PlaceAccess)
```

`AddressOf` is an operand producing an owned raw-pointer value whose pointee type must exactly equal the type selected by `src`.

Formation proceeds conceptually as follows:

1. authorize `src` using the borrowing model's **shared** access requirement;
2. resolve `src` to one concrete structural place;
3. identify the dynamic storage instance corresponding to that place's local storage extent;
4. form the target structural storage region from that storage instance plus the place's structural projection path;
5. create a raw-pointer value whose target is that region and whose provenance is rooted in that storage instance.

Pointer formation itself does not read, copy, move, mutate, initialize, destroy, or otherwise access the pointee value.

Therefore `AddressOf` does **not** require the selected place to be Live. Never-initialized and Dead storage may be targeted while its storage extent still exists.

This distinction is intentional: storage existence and stored-value liveness are independent semantic facts.

## Borrowing interaction during formation

`AddressOf` requires shared alias authority only for the act of selecting existing storage.

Consequences under the current borrowing model:

- direct formation may coexist with overlapping shared loans;
- an overlapping exclusive loan blocks direct formation;
- formation through an active shared loan is permitted when that loan currently retains shared authority over the selected region;
- formation through an active exclusive loan is permitted because exclusive authority includes shared authority;
- delegated child loans constrain formation through a parent according to the ordinary shared-authority delegation rules.

The resulting raw pointer does not retain the source loan's authority and does not keep that loan active.

Ending the source loan therefore does not alter or erase a raw-pointer value already formed from that storage.

This does **not** imply that later memory access through the raw pointer is legal. Pointer access and its safety preconditions are not defined by this revision.

## Stored-value replacement and provenance

Pointer provenance is rooted in the dynamic storage instance, not in the particular stored-value lifetime occupying that storage.

Consequently, while the storage extent continues:

- moving the current value out does not by itself alter an already formed pointer's provenance;
- destroying the current value does not by itself alter that provenance;
- ordinary assignment may end an old stored-value lifetime and begin a new one without changing the root storage identity;
- interior assignment may do the same while shared loans remain active;
- forming a pointer before and after such a transition derives from the same root storage-instance identity and the same structural region when the same place is selected.

This revision intentionally makes no claim that dereferencing such a pointer while the target is uninitialized, Dead, replaced, or otherwise invalid is legal. Those access/validity rules belong to later pointer and unsafe-memory semantics.

## Raw-pointer value transport

A formed raw pointer is an ordinary owned value for the value operations currently represented by Core.

- `Copy` of a raw pointer produces an equal raw-pointer value and preserves target and provenance.
- `Move` transfers the raw-pointer value and preserves target and provenance while ending the source pointer's stored-value lifetime in the ordinary way.
- storing or replacing a raw-pointer value preserves its target and provenance as part of that value.
- destruction of a raw-pointer value has no pointer-specific semantic effect in this revision.

Pointer-value transport does not reactivate, recreate, or extend any loan from which the pointer may originally have been formed.

## Determinism and verification

For a fixed validated body and execution, storage-instance creation, structural target formation, and raw-pointer provenance formation are deterministic.

Reference-oracle instrumentation may expose storage-instance identities and formed pointer components to conformance tests. Such instrumentation is verification-only and is not Runen-observable program behavior.

No program may branch on, print, compare, serialize, or otherwise observe the oracle's numeric storage-instance representation under the semantics defined by this revision.

## Not yet defined

This revision deliberately does **not** define:

- dereferencing a raw pointer;
- loads or stores through raw pointers;
- whether raw-pointer memory access is an unsafe operation;
- pointer arithmetic, offsets, one-past rules, or byte-wise addressing;
- numeric or physical addresses;
- pointer equality, ordering, hashing, or identity observations;
- pointer-to-integer or integer-to-pointer conversion;
- exposed, wildcard, or address-only provenance;
- null-pointer formation or null semantics;
- allocation, deallocation, heap objects, or allocation APIs;
- relocation or address stability;
- pinning;
- alignment or bounds requirements for pointer access;
- value validity or invalid bit patterns;
- dangling-pointer access rules or the complete undefined-behavior taxonomy;
- source `unsafe` syntax or unsafe blocks;
- first-class references, reference lifetimes, or source borrow inference;
- atomics, concurrency, data races, memory ordering, or synchronization;
- source syntax, public library APIs, ABI, or FFI representation.

Those concerns are intentionally deferred to their owning later semantics rather than being inferred from pointer formation alone.
