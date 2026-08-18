# Core Pointers and Provenance

Status: **provisional normative; incomplete**

This document defines the currently represented Core semantics for raw-pointer types and values, symbolic pointer targets, provenance formation from existing storage, preservation under pointer-value transport, one non-consuming raw-pointer target read, one ownership-moving raw-pointer operand, and one source-first raw-pointer target replacement.

Dynamic storage-instance identity, storage extent, structural storage regions, stored-value lifetime, initialization state, ownership transfer, replacement, and destruction are defined by [Core value and storage semantics](value-storage.md). Shared/exclusive authority used while selecting storage for pointer formation, accessing stored pointer values, and checking raw target access compatibility is defined by [Core borrowing](borrowing.md). Unsafe-operation classification and undefined behavior from violated unsafe preconditions are defined by [Core unsafe semantics](unsafe.md).

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

The current reference oracle therefore needs only one stored source of truth for this foundation: the symbolic target region. Its `target.instance` is sufficient verification evidence for the currently defined provenance root. Exact static structural `Place` bookkeeping derived from that target is verification-only when used to model defined path-state effects; it is not a second provenance identity or Runen-observable value.

The target is symbolic structural metadata selected at formation. It does not mean that the pointer dynamically follows relocated storage, nor does it itself define a physical address or unrestricted memory access. Relocation and address stability remain undefined.

A raw-pointer value does not contain a `LoanId`, does not extend a borrow interval, and does not by itself carry shared or exclusive loan authority.

The proving MIR's constant-value vocabulary cannot fabricate a non-null raw pointer. Non-null pointer formation depends on dynamic storage identity and therefore occurs only through the defined execution operation below. Once formed, a pointer may be transported by the ordinary value operations and by a defined `RawMove` of pointer-valued target storage. A reference implementation may use a richer private runtime-value representation than the MIR constant representation.

## Pointer formation

The proving MIR forms a non-null raw pointer with:

```text
AddressOf(src: PlaceAccess)
```

`AddressOf` is an operand producing an owned raw-pointer value whose pointee type MUST exactly equal the type selected by `src`.

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

Formation authority alone does not establish the unsafe preconditions for a later raw-pointer target read, ownership move, or replacement.

## Non-consuming raw-pointer read

The current proving MIR has one raw-pointer target-read operation:

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

## Ownership-moving raw-pointer operand

The current proving MIR has one ownership-producing raw-pointer operand:

```text
RawMove(pointer: PlaceAccess)
```

`RawMove` is an unsafe ownership transfer through an already stored raw-pointer value. It yields one owned value of the pointer's pointee type to the enclosing Core operation. It does not create a dereferenced `Place`, define a byte-wise load, or create a second pointer/provenance representation.

The `pointer` access selects and obtains the stored raw-pointer value using the same language-validation requirements as the pointer-value access of `RawRead`: the selected place MUST have raw-pointer type, the pointer value MUST be fully Live, and the `PlaceAccess` MUST have shared alias authority under [Core borrowing](borrowing.md).

A language-valid `RawMove` proceeds conceptually as follows:

1. obtain the raw-pointer value through `pointer` and snapshot that value's symbolic target structural storage region;
2. resolve the snapshotted target to the corresponding concrete structural place in the continuing dynamic storage instance;
3. require the complete target place to be fully Live;
4. require the target ownership transfer to satisfy the active-loan **exclusive** compatibility rule defined by [Core borrowing](borrowing.md);
5. apply the stored-value lifetime and ownership-transfer state transition defined for `Move` by [Core value and storage semantics](value-storage.md) to the resolved target;
6. yield the transferred complete pointee value as the result of the operand.

Step 5 reuses the value/storage transition of `Move`; it does **not** import ordinary `Move`'s `PlaceAccess` authorization requirement. Raw target alias legality is the distinct compatibility precondition in step 4 because the raw pointer carries no loan authority.

The target snapshot in step 1 is semantically significant. Obtaining the pointer value does not itself consume or mutate that stored pointer value. If the pointer targets the storage containing that pointer value, target selection therefore occurs before the value/storage owner's `Move` transition is applied to that target.

The ownership-transfer state transition in step 5 is governed entirely by [Core value and storage semantics](value-storage.md). This document adds no independent rules for leaf initialization-state transitions, stored-value lifetime ending, destruction, storage extent, or storage-instance identity. The owned value produced by that transition is consumed by the enclosing operation according to that operation's existing semantics.

If the moved pointee value itself contains raw pointers, their already defined target/provenance metadata is part of the moved value and is preserved by the ordinary ownership-transfer lifecycle. `RawMove` does not derive new provenance merely because transport occurred through a raw pointer.

The target-liveness requirement and target exclusive-access compatibility requirement are unsafe preconditions. `RawMove` is classified as unsafe, and violation of either requirement is undefined behavior, by [Core unsafe semantics](unsafe.md). Such a violation is not malformed MIR, a language-validation error, a defined `Fault`, or a recoverable result.

Ordinary local assignment mutability and interior-mutability markers are not permissions for `RawMove`; the operation is an unsafe ownership transfer whose target alias requirement is defined separately by the borrowing model.

## Source-first raw-pointer replacement

The current proving MIR has one raw-pointer target-replacement operation:

```text
RawAssign { pointer: PlaceAccess, src: Operand }
```

`RawAssign` is an unsafe semantic replacement through an already stored raw-pointer value. It does not create a dereferenced `Place`, define byte-wise memory access, or create a second pointer/provenance representation.

The `pointer` operand selects and obtains a stored raw-pointer value using the same language-validation requirements as the pointer-value access of `RawRead` and `RawMove`: the selected place MUST have raw-pointer type, the pointer value MUST be fully Live, and the `PlaceAccess` MUST have shared alias authority under [Core borrowing](borrowing.md).

The type of `src` MUST exactly equal the pointee type of that raw-pointer value.

A language-valid `RawAssign` proceeds conceptually as follows:

1. obtain the raw-pointer value through `pointer` and snapshot that value's symbolic target structural storage region;
2. resolve the snapshotted target to the corresponding concrete structural place in the continuing dynamic storage instance;
3. evaluate `src` completely according to the ordinary Core operand semantics, including any unsafe preconditions of a `RawMove` source operand;
4. require the target write to satisfy the active-loan **exclusive** compatibility rule defined by [Core borrowing](borrowing.md);
5. apply the source-first replacement lifecycle defined by [Core value and storage semantics](value-storage.md) to the resolved target using the already evaluated source value.

The target snapshot in step 1 is semantically significant. Later source evaluation or target replacement does not retroactively change which target this `RawAssign` selected. Obtaining the pointer value for the snapshot does not itself consume or mutate that stored pointer value. Ordinary source evaluation and the target replacement can nevertheless affect storage that aliases the pointer operand place when their independently applicable semantics permit it.

If source evaluation itself enters undefined behavior, there is no defined source value with which to continue the outer replacement. In particular, a failing `RawMove` source does not cause the `RawAssign` target destruction or write to occur as defined behavior.

Unlike `RawRead` and `RawMove`, `RawAssign` has no target-liveness precondition. The operation is defined when the resolved target is Never-initialized, partially initialized, fully Live, or Dead. The replacement lifecycle destroys exactly the then-Live destruction domain after source evaluation and leaves the complete written target Live, as defined by the value/storage owner.

`RawAssign` is not ordinary `Assign` and is not `InteriorAssign`. The containing local's ordinary assignment-mutability flag and the interior-mutability marker are therefore not preconditions of `RawAssign`. They remain capabilities specific to the operations defined by the value/storage semantics. `RawAssign` instead relies on its unsafe classification and its raw target exclusive-access precondition.

Violation of the raw target exclusive-access precondition is undefined behavior under [Core unsafe semantics](unsafe.md). It is not a language-validation error or a defined `Fault`.

A successful `RawAssign` does not change the target storage extent or storage-instance identity and does not create, end, or transfer any loan. Its stored-value lifetime and destruction effects are exactly those of the reused replacement lifecycle.

## Stored-value transitions and raw-pointer targets

Pointer provenance formed here is rooted in the dynamic storage instance rather than in the particular stored-value lifetime occupying that storage.

Consequently, while the storage extent continues:

- moving the current value out, whether by ordinary `Move` or defined `RawMove`, does not by itself change the pointer's currently defined provenance root;
- destroying the current value does not by itself change that root;
- ordinary assignment may end an old stored-value lifetime and begin a new one without changing the root storage identity;
- interior assignment may do the same while shared loans remain active;
- raw-pointer replacement follows the same stored-value lifetime transition rules without changing the target storage identity;
- forming a pointer before and after such a transition selects the same structural storage region when the same place is selected;
- when the target is fully Live and the current borrowing precondition permits the read, `RawRead` through a previously formed pointer reads the later replacement value;
- the same pointer cannot legally `RawRead` or `RawMove` the target while that complete target is Never-initialized, Dead, or only partially initialized;
- when the raw target exclusive-access precondition permits ownership transfer, `RawMove` applies the value/storage owner's `Move` transition to the current complete target value;
- when the raw target exclusive-access precondition permits the write, `RawAssign` is defined to replace or initialize the continuing target region regardless of its prior initialization state.

Thus a pointer targets continuing storage rather than one frozen stored-value lifetime, while each actual access is still constrained by the preconditions of that access operation.

## Raw-pointer value transport

A formed raw pointer is an ordinary owned value for the value operations currently represented by Core.

- `Copy` produces another owned raw-pointer value preserving the defined pointer metadata, including target and provenance root; this does not define a language-level pointer equality operation.
- ordinary `Move` transfers the raw-pointer value preserving that metadata while ending the source pointer's stored-value lifetime in the ordinary way.
- defined `RawMove` of pointer-valued target storage preserves that raw-pointer metadata in the value produced by the value/storage owner's `Move` transition.
- storing or replacing a raw-pointer value preserves its defined pointer metadata as part of that value.
- destruction of a raw-pointer value has no pointer-specific semantic effect in this revision.

Pointer-value transport does not reactivate, recreate, or extend any loan from which the pointer may originally have been formed.

## Determinism and verification

For a fixed validated body and execution that has not entered undefined behavior, symbolic target selection, the currently defined provenance root, target resolution, successful `RawRead`, successful `RawMove`, and successful `RawAssign` behavior are deterministic from the storage identities, storage/value state, active-loan state, and operand semantics supplied by the owning Core semantics.

Reference-oracle instrumentation MAY expose storage-instance identities, formed pointer targets, successful raw reads/moves/replacements, and detected unsafe-precondition violations to conformance tests. Exact static pointer-target bookkeeping retained solely to propagate defined path-state is verification-only and is not Runen-observable program behavior.

A Runen program MUST NOT branch on, print, compare, serialize, or otherwise observe the oracle's numeric storage-instance representation, validator pointer-target bookkeeping, or UB diagnostic taxonomy under the semantics defined by this revision.

## Not yet defined

This revision deliberately does **not** define:

- raw-pointer mutation beyond the semantic source-first `RawAssign` operation defined above;
- non-dropping or byte-wise raw writes;
- a general dereference-place abstraction;
- a non-consuming owned raw-copy load distinct from the discarded-result `RawRead`;
- byte-wise or representation-level raw loads;
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
- value validity or invalid bit patterns beyond the existing semantic-value and initialization requirements used by the defined operations;
- dangling access outside the currently represented local-storage extents or the complete undefined-behavior taxonomy;
- source `unsafe` syntax or unsafe blocks;
- first-class references, reference lifetimes, or source borrow inference;
- atomics, concurrency, data races, memory ordering, or synchronization;
- source syntax, public library APIs, ABI, or FFI representation.

Those concerns remain open until additional operations require further semantic distinctions.
