# Core Pointers and Provenance

Status: **provisional normative; incomplete**

This document defines the currently represented Core semantics for raw-pointer types and values, symbolic pointer targets, provenance formation from existing storage, preservation under ordinary pointer-value transport, and the first non-consuming raw-pointer target read.

Dynamic storage-instance identity, storage extent, structural storage regions, stored-value lifetime, initialization state, replacement, and destruction are defined by [Core value and storage semantics](value-storage.md). Shared/exclusive authority used while selecting storage for pointer formation and active-loan compatibility for raw target reads are defined by [Core borrowing](borrowing.md). Unsafe-operation classification and undefined behavior from violated unsafe preconditions are defined by [Core unsafe semantics](unsafe.md).

## Required distinctions

The following are distinct semantic concepts:

- a static MIR local declaration;
- a static structural `Place` rooted in that declaration;
- one dynamic storage instance created for one execution of a local storage extent;
- a structural storage region within that instance;
- the stored-value lifetime currently occupying that region, if any;
- a loan and its current alias authority;
- a raw-pointer value;
- pointer provenance;
- a numeric or physical address.

In particular, `LocalId` is not dynamic storage identity, `LoanId` is not pointer provenance, stored-value lifetime is not storage identity, provenance is not defined as a numeric address, and a raw pointer is not a source-language reference or continuation of a loan interval.

## Storage identity used by pointer formation

Pointer formation relies on the dynamic storage-instance and structural storage-region semantics defined by [Core value and storage semantics](value-storage.md):

- every dynamic local storage extent has one storage-instance identity;
- distinct simultaneously existing local storage extents have distinct identities;
- a storage-instance identity remains stable while its storage extent continues even when stored-value lifetimes begin or end;
- a structural storage region is that dynamic root identity plus a structural projection path;
- neither the identity token nor the projection path is a physical address, byte offset, ABI layout, or relocation guarantee.

## Raw-pointer type

The current proving kernel has a capability-neutral raw-pointer type parameterized by one pointee type.

The pointee relation is semantic indirection, not structural containment. Consequently a raw-pointer pointee edge does not make a finite structural type recursively infinite. A struct may therefore contain a raw pointer whose pointee type eventually refers back to that struct, while direct structural recursion remains invalid.

This revision does not define source spelling, shared-versus-mutable raw-pointer qualifiers, variance, nullability, layout, size, alignment, or ABI representation.

Raw-pointer types are copyable in the current Core value model.

## Raw-pointer value and provenance

A non-null raw-pointer value formed by the currently defined operation has a **target structural storage region** selected when the pointer is formed.

Formation also establishes pointer provenance **rooted in the dynamic storage instance containing that target region**. This is the only provenance fact defined by this revision. It does not define the complete future structure of provenance, access permissions carried by provenance, derivation history, invalidation rules, exposed provenance, or address reconstruction.

The current reference oracle therefore needs only one stored source of truth for this foundation: the symbolic target region. Its `target.instance` is sufficient verification evidence for the currently defined provenance root. A later provenance model may add independent semantic state when future operations require distinctions that cannot be derived from the target region.

The target is symbolic structural metadata selected at formation. It does not mean that the pointer dynamically follows relocated storage, nor does it itself define a physical address or unrestricted memory access. Relocation and address stability remain undefined.

A raw-pointer value does not contain a `LoanId`, does not extend a borrow interval, and does not by itself carry shared or exclusive loan authority.

The proving MIR's constant-value vocabulary cannot fabricate a non-null raw pointer. Non-null pointer formation depends on dynamic storage identity and therefore occurs only through the defined execution operation below. A reference implementation may use a richer private runtime-value representation than the MIR constant representation.

## Pointer formation

The proving MIR forms a non-null raw pointer with:

```text
AddressOf(src: PlaceAccess)
```

`AddressOf` is an operand producing an owned raw-pointer value whose pointee type must exactly equal the type selected by `src`.

Formation proceeds conceptually as follows:

1. authorize `src` using the borrowing model's **shared** access requirement;
2. resolve `src` to one concrete structural place;
3. obtain the dynamic storage instance corresponding to that place's local storage extent;
4. select the target structural storage region from that storage instance plus the place's structural projection path;
5. form a raw-pointer value targeting that region, with provenance rooted in the target's dynamic storage instance.

Pointer formation itself does not read, copy, move, mutate, initialize, destroy, or otherwise access the pointee value.

Therefore `AddressOf` does **not** require the selected place to be Live. Never-initialized and Dead storage may be targeted while its storage extent still exists.

Storage existence and stored-value liveness are independent semantic facts.

## Borrowing interaction during formation

`AddressOf` requires shared alias authority for the act of selecting existing storage. The detailed direct-access and reborrow-delegation rules are defined by [Core borrowing](borrowing.md).

The resulting raw pointer does not retain the source loan's authority and does not keep that loan active. Ending the source loan therefore does not alter or erase a raw-pointer value already formed from that storage.

Formation authority alone does not establish the unsafe preconditions for a later raw-pointer target read.

## Non-consuming raw-pointer read

The current proving MIR has one raw-pointer memory-access operation:

```text
RawRead { pointer: PlaceAccess }
```

`RawRead` is a non-consuming read whose result is discarded by the current proving MIR. It does not create a dereferenced `Place`, transfer ownership from the pointee, mutate the pointee, or create a new pointer value.

The `pointer` access selects the stored raw-pointer **value** used for the operation. Ordinary language validation therefore requires that:

- the selected place has a raw-pointer type;
- the pointer value itself is fully Live;
- reading that pointer value through its `PlaceAccess` has shared alias authority under [Core borrowing](borrowing.md).

Those are language-validation rules about accessing the pointer value. They are distinct from the unsafe obligations governing the pointer's pointee target.

For a language-valid `RawRead`, execution resolves the raw-pointer value's existing symbolic target region to the corresponding concrete structural place in the continuing dynamic storage instance. The operation has these unsafe target-access preconditions:

- the complete target place MUST be fully Live at the read;
- the target read MUST satisfy the active-loan shared-access compatibility rule defined by [Core borrowing](borrowing.md).

`RawRead` is classified as unsafe, and violation of either target-access precondition is undefined behavior, by [Core unsafe semantics](unsafe.md).

When its unsafe preconditions hold, `RawRead` reads the current stored value non-consumingly. It does not change initialization state, end or begin any stored-value lifetime, change storage-instance identity, alter pointer metadata, or change loan state.

The operation is not restricted to copyable pointee types because it does not produce an owned duplicate or move a value out of the target. The current proving MIR simply discards the semantic read result, as it does for ordinary `Read`.

## Stored-value replacement and raw-pointer targets

Pointer provenance formed here is rooted in the dynamic storage instance rather than in the particular stored-value lifetime occupying that storage.

Consequently, while the storage extent continues:

- moving the current value out does not by itself change the pointer's currently defined provenance root;
- destroying the current value does not by itself change that root;
- ordinary assignment may end an old stored-value lifetime and begin a new one without changing the root storage identity;
- interior assignment may do the same while shared loans remain active;
- forming a pointer before and after such a transition selects the same structural storage region when the same place is selected;
- a previously formed pointer may `RawRead` a later replacement value when the target is fully Live and the current borrowing precondition permits the read;
- the same pointer cannot legally `RawRead` the target while that target is Never-initialized, Dead, or only partially initialized as a whole.

Thus a pointer targets continuing storage rather than one frozen stored-value lifetime, while each actual access is still constrained by the target's state at that semantic step.

## Raw-pointer value transport

A formed raw pointer is an ordinary owned value for the value operations currently represented by Core.

- `Copy` produces another owned raw-pointer value preserving the defined pointer metadata, including target and provenance root; this does not define a language-level pointer equality operation.
- `Move` transfers the raw-pointer value preserving that metadata while ending the source pointer's stored-value lifetime in the ordinary way.
- storing or replacing a raw-pointer value preserves its defined pointer metadata as part of that value.
- destruction of a raw-pointer value has no pointer-specific semantic effect in this revision.

Pointer-value transport does not reactivate, recreate, or extend any loan from which the pointer may originally have been formed.

## Determinism and verification

For a fixed validated body and execution that has not entered undefined behavior, symbolic target selection, the currently defined provenance root, target resolution, and successful `RawRead` behavior are deterministic from the storage identities, storage state, and active-loan state supplied by the owning Core semantics.

Reference-oracle instrumentation may expose storage-instance identities, formed pointer targets, successful raw reads, and detected unsafe-precondition violations to conformance tests. Such instrumentation is verification-only and is not Runen-observable program behavior.

No program may branch on, print, compare, serialize, or otherwise observe the oracle's numeric storage-instance representation or UB diagnostic taxonomy under the semantics defined by this revision.

## Not yet defined

This revision deliberately does **not** define:

- raw-pointer stores or mutation through raw pointers;
- a general dereference-place abstraction;
- ownership-moving raw loads such as `ptr::read`-style semantics;
- pointer arithmetic, offsets, one-past rules, or byte-wise addressing;
- numeric or physical addresses;
- pointer equality, ordering, hashing, or identity observations;
- pointer-to-integer or integer-to-pointer conversion;
- the complete structure, permissions, derivation, invalidation, or exposure rules of provenance beyond the storage root established here;
- null-pointer formation or null semantics;
- allocation, deallocation, heap objects, or allocation APIs;
- relocation or address stability;
- pinning;
- alignment or bounds rules beyond the current structural target model;
- value validity or invalid bit patterns beyond requiring the existing semantic target value to be fully Live for `RawRead`;
- dangling access outside the currently represented local-storage extents or the complete undefined-behavior taxonomy;
- source `unsafe` syntax or unsafe blocks;
- first-class references, reference lifetimes, or source borrow inference;
- atomics, concurrency, data races, memory ordering, or synchronization;
- source syntax, public library APIs, ABI, or FFI representation.

Those concerns remain open until additional operations require further semantic distinctions.
