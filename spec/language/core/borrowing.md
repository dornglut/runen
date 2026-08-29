# Core Borrowing

Status: **provisional normative; incomplete**

This document defines structural place overlap, shared/exclusive alias authority, explicit shared/exclusive borrowing, explicit-loan reborrow delegation, explicit borrow intervals, and the common overlap/conflict law consumed by safe reference-backed authority.

Storage extent, storage-instance identity, stored-value lifetime, initialization state, assignment mutability, interior mutability, destruction, and cleanup are defined by [Core value and storage semantics](value-storage.md). First-class safe-reference values, reference permission classes, reference-backed authority identity/carrier lifetime, and reference reborrowing are defined by [Core references](references.md). Raw-pointer values, provenance, target selection, target reads, target ownership moves, and target replacement are defined by [Core pointers and provenance](pointers.md).

## Terms

### Place overlap

Two places overlap when they denote storage where one place contains the other in the current structural place model.

For the field-only places defined by this revision:

- places rooted in different locals are disjoint;
- identical places overlap;
- a local or aggregate place overlaps every structural descendant sub-place;
- two places with the same local root overlap exactly when either field-projection sequence is a prefix of the other;
- sibling field places that diverge at a field projection are disjoint.

For cross-activation safe references, the same structural relation applies to resolved `StorageRegion` targets: regions rooted in distinct dynamic storage instances are disjoint, and regions rooted in one dynamic storage instance overlap exactly when one structural projection sequence is a prefix of the other. Static `Place` and dynamic `StorageRegion` remain distinct identities; this paragraph supplies only the same structural overlap law after an accepted owner has resolved an access to a concrete region.

Place overlap is symmetric. It depends only on semantic storage structure and does not depend on physical addresses, field offsets, ABI layout, relocation, or pointer provenance.

### Loan

An explicit **loan** is a semantic alias-access permission over one concrete root place created by the proving-MIR `Borrow` operation.

A loan has a kind: **shared** or **exclusive**.

Shared versus exclusive describes competing-access authority. It does not itself declare the containing local mutable for ordinary assignment, and it does not itself make storage interior-mutable.

The three concerns are independent:

- **assignment mutability** controls ordinary direct/explicit-loan `Assign`;
- **alias authority** is shared or exclusive;
- **interior mutability** is explicit type/storage capability.

A loan is not a numeric address, pointer bit pattern, source-language reference value, allocation identity, dynamic storage-instance identity, or provenance token.

A safe reference-backed authority from [Core references](references.md) is likewise shared or exclusive alias authority, but it is not an explicit loan and has no `LoanId`. Its lifecycle is owned by the reference carrier/descendant relation rather than `EndBorrow`.

### Alias authority

**Shared authority** permits operations whose borrowing requirement is shared. For explicit loans in this revision those include `Read`, copyable `Copy`, shared explicit reborrow creation, raw-pointer `AddressOf` formation, access to the stored raw-pointer value used by `RawRead`, `RawMove`, or `RawAssign`, and `InteriorAssign` when the target independently lies within an interior-mutable region.

**Exclusive authority** includes shared authority and additionally permits operations whose borrowing requirement is exclusive. For explicit loans in this revision those include `Move`, ordinary `Assign`, `Drop`, and exclusive explicit reborrow creation, subject to each operation's independent value/storage preconditions.

Safe-reference access consumes the same shared/exclusive alias categories but has its exact operation permissions, including the independent ExclusiveReplace capability, defined by [Core references](references.md). In particular, an Exclusive safe reference without replacement capability does not gain ordinary `Assign` merely because exclusive alias authority is sufficient for the aliasing part of that operation.

`Init` and the direct destinations of represented `IntegerAdd`, `IntegerSub`, `IntegerMul`, `IntegerXor`, `IntegerOr`, `FloatAdd`, `FloatSub`, `FloatMul`, and `FloatDiv` are also exclusive-authority direct-storage operations.

None of `IntegerAdd`, `IntegerSub`, `IntegerMul`, `IntegerXor`, `IntegerOr`, `FloatAdd`, `FloatSub`, `FloatMul`, or `FloatDiv` has an explicit-loan-relative or safe-reference-relative destination form in this revision. Each operation's `dst: Place` is authorized only by the direct-access rule below. Its `left` and `right` operands separately retain whatever authority their existing operand forms require; classifying the destination as exclusive does not upgrade, replace, or duplicate operand access authority.

Interior mutation is not a third alias kind. A shared write to explicitly interior-mutable storage uses shared alias authority plus a separate interior-mutability capability check.

A raw pointer does not itself carry either alias authority kind. After a raw operation's pointer value has been obtained using ordinary shared authority, the raw target access defined by [Core pointers and provenance](pointers.md) has a separate compatibility requirement against the complete active alias-authority set:

- `RawRead` has **shared target-access compatibility**;
- `RawMove` has **exclusive target-access compatibility**;
- `RawAssign` has **exclusive target-access compatibility**.

These target requirements do not arise from a `LoanId` or reference-authority identity stored in the raw pointer; raw pointers contain no such authority.

### Borrow interval

An explicit borrow interval is the interval of execution during which one explicit loan activation is active.

Borrow interval, safe-reference authority interval, storage extent, dynamic storage-instance identity, and stored-value lifetime are distinct concepts.

Ending an explicit borrow interval does not by itself end the target storage extent, its storage-instance identity, or the currently active stored-value lifetime.

Conversely, an exclusive explicit loan may remain active while an exclusive operation through that loan ends one stored-value lifetime and later begins another in the same storage extent when ordinary assignment permits replacement.

For interior-mutable storage, a shared explicit loan may likewise remain active while legal `InteriorAssign` ends old stored-value lifetimes and begins replacement lifetimes in the same storage extent. A shared loan is therefore not a promise that one stored-value lifetime remains unchanged for the whole borrow interval.

The current proving MIR gives an explicit loan declaration a stable body-local `LoanId`. That declaration may be activated again after an earlier interval ends. Each activation is a distinct explicit borrow interval and receives fresh dynamic root, kind, and parent state.

Reference-backed authority intervals instead receive opaque dynamic authority identities under `references.md`; they are not activations of a reusable body-local `LoanId` declaration.

### Place access

A Core operation using the explicit place-access relation may reach storage either directly through a place or through an active explicit loan plus zero or more structural field projections relative to that loan's concrete root.

Explicit-loan-relative projection does not create a new loan. It selects a sub-place governed by the existing loan.

`PlaceAccess` is proving-MIR access authority. It is not a stored reference value, raw-pointer value, address, dynamic storage-instance identity, or provenance identity.

Safe-reference access is a distinct access category owned by [Core references](references.md). It resolves one live stored reference carrier to its dynamic `StorageRegion` target and does not convert that authority into `PlaceAccess::Loan`.

The pointer-value access used by `RawRead`, `RawMove`, and `RawAssign` is ordinary shared-authority `PlaceAccess` to the stored raw-pointer value. None of these operations turns the raw pointer's target into `PlaceAccess` or safe-reference access; target selection is defined by the pointer value and checked separately against the active alias-authority set.

### Explicit loan forest

Active explicit loans form a rooted forest.

- a root explicit loan has no parent;
- a reborrowed explicit child records the active explicit parent loan from which its authority was delegated;
- a child root is the concrete place selected through its parent access when the child interval begins;
- one explicit loan declaration may have at most one active interval at a time;
- an active explicit loan cannot become its own ancestor because a new child requires an inactive destination `LoanId` and an already-active parent;
- every explicit loan end is leaf-to-root: a parent interval cannot end while a direct or indirect explicit child remains active.

Parentage belongs to the current activation, not permanently to the reusable explicit loan declaration.

Safe-reference-backed authorities form their own dynamic parent/child relation under `references.md`. They are not members of this `LoanId` forest, and this revision creates no cross-kind parent edge between an explicit loan and a reference-backed authority.

## Combined alias-conflict domain

Explicit loans and safe-reference-backed authorities are distinct identity/lifecycle mechanisms but participate in one alias-conflict domain after their targets have been resolved to concrete structural storage regions.

For the common conflict law:

- an active Shared reference-backed authority is an active shared alias authority over its target;
- an active Exclusive or ExclusiveReplace reference-backed authority is an active exclusive alias authority over its target;
- an active shared explicit loan is an active shared alias authority over its concrete root; and
- an active exclusive explicit loan is an active exclusive alias authority over its concrete root.

Whenever this document tests whether a **direct** storage access, a new **root explicit borrow**, or a raw-pointer **target access** is compatible with currently active aliases, that test ranges over all overlapping active authorities in this combined domain, not only body-local `LoanId` entries.

The existing explicit-loan parent/child delegation rules remain local to the explicit loan forest. The safe-reference parent/child delegation rules remain local to the reference-authority relation. Sharing one conflict domain does not merge their identities, ending rules, or access representations.

## Borrow creation

The proving MIR has one explicit borrow operation whose source is a `PlaceAccess`:

```text
Borrow { loan, kind, src: PlaceAccess }
```

A direct source begins a root explicit interval. An explicit-loan-relative source begins an explicit child interval.

For every explicit borrow creation:

- the destination loan declaration MUST be inactive;
- the selected source type MUST equal the destination loan declaration type;
- the selected concrete root MUST be fully Live when the interval begins.

Borrow creation does not consume or mutate the stored value.

### Root borrow creation

For `PlaceAccess::Direct(p)`:

- a shared root borrow may begin when no active exclusive authority in the combined alias-conflict domain overlaps `p`;
- multiple overlapping shared root explicit loans and/or Shared reference-backed authorities are permitted;
- an exclusive root borrow may begin only when no active authority of either kind in the combined alias-conflict domain overlaps `p`.

Exclusive borrowing does not by itself require the containing local to be mutable. Assignment mutability remains a separate rule applied when ordinary `Assign` is attempted.

Interior-mutability capability is also irrelevant to whether a root shared or exclusive loan may begin. It affects only the dedicated interior-replacement operation after alias authority has been established.

### Explicit reborrow creation

For `PlaceAccess::Loan(parent, projections)`, the selected concrete child root is the parent explicit loan's concrete root followed by the relative structural projections.

A shared explicit child may be created from:

- an active shared parent that currently retains shared authority over the selected child root;
- an active exclusive parent that currently retains shared authority over the selected child root.

An exclusive explicit child may be created only from an active exclusive parent that currently retains exclusive authority over the selected child root.

Therefore:

- overlapping shared explicit children may coexist;
- an overlapping exclusive explicit child blocks creation of any further overlapping child through the same parent;
- an overlapping shared explicit child blocks creation of an exclusive sibling through the same parent;
- disjoint children do not constrain one another;
- an exclusive child may never be derived from a shared parent.

Reborrow creation uses only typed place structure, active explicit-loan state, and the place-overlap relation. It does not inspect physical addresses or pointer provenance.

Because any independently rooted incompatible reference-backed authority would already be excluded by the combined root-admission rules, this explicit child relation does not create a cross-kind parent or duplicate reference reborrow semantics.

## Delegated parent authority

An explicit child interval delegates only the portion of its direct explicit parent's authority that overlaps the child's concrete root.

For an access through explicit parent loan `P` to concrete place `p`, only active direct explicit children of `P` are needed to determine the authority still retained by `P` at `p`.

This is sufficient because a direct child continues to own its delegated root until that direct child ends. Deeper descendants refine the direct child's authority but do not return any authority to the grandparent while the direct child remains active.

### Overlapping exclusive child

If an active exclusive direct child of `P` overlaps `p`, access through `P` to `p` is suspended completely.

`Read`, `Copy`, `AddressOf`, pointer-value access for `RawRead`, `RawMove`, or `RawAssign`, `Move`, ordinary `Assign`, `InteriorAssign`, `Drop`, and overlapping explicit reborrow through `P` are invalid until that child interval ends.

### Overlapping shared child

If one or more active shared direct children of `P` overlap `p`, `P` retains shared authority over that overlapping storage:

- `Read` is permitted;
- `Copy` is permitted when the selected type is copyable;
- `AddressOf` is permitted;
- pointer-value access for `RawRead`, `RawMove`, and `RawAssign` is permitted subject to each operation's independently owned non-authority preconditions;
- shared explicit reborrow is permitted when its ordinary preconditions hold;
- `InteriorAssign` is permitted when the selected concrete target independently lies within an interior-mutable region;
- `Move`, ordinary `Assign`, `Drop`, and exclusive explicit reborrow are invalid.

For an exclusive parent this is a temporary local downgrade from exclusive to shared authority. For a shared parent, an overlapping shared child does not further reduce the parent's already-shared authority.

The availability of `InteriorAssign` does not mean the shared child grants mutation permission. The child only leaves shared alias authority intact; the distinct interior-mutability capability is checked by the value/storage rules.

Raw target-access compatibility after obtaining a raw pointer value is a separate check and does not inherit the pointer-value access path's explicit-loan or reference-authority identity.

### Disjoint child

A direct child whose concrete root is disjoint from `p` does not constrain access through `P` to `p`.

Delegating one field of an aggregate to an exclusive child leaves an exclusive parent with exclusive authority over a disjoint sibling field. On such a disjoint sibling, the parent retains the authority needed for `InteriorAssign`, `AddressOf`, or shared pointer-value access for `RawRead`/`RawMove`/`RawAssign`, subject to each operation's independent preconditions.

## Direct access while alias authorities are active

Direct access remains constrained by every active overlapping authority in the combined alias-conflict domain, regardless of explicit-loan parent/child structure or safe-reference parent/child structure.

For a direct access target `p`:

- `Read(p)` is permitted when no active exclusive authority overlaps `p`;
- `Copy(p)` is permitted when no active exclusive authority overlaps `p` and the existing copyability rule permits the copy;
- `AddressOf(p)` is permitted when no active exclusive authority overlaps `p`; it does not additionally require `p` to be Live;
- using `p` for the stored pointer-value access of `RawRead`, `RawMove`, or `RawAssign` has shared authority when no active exclusive authority overlaps `p`, subject to each operation's independently owned non-authority preconditions and separate raw-target unsafe preconditions;
- `InteriorAssign(p, ...)` is permitted when no active exclusive authority overlaps `p` and `p` independently lies within an interior-mutable region;
- `Init(p, ...)`, `IntegerAdd { dst: p, ... }`, `IntegerSub { dst: p, ... }`, `IntegerMul { dst: p, ... }`, `IntegerXor { dst: p, ... }`, `IntegerOr { dst: p, ... }`, `FloatAdd { dst: p, ... }`, `FloatSub { dst: p, ... }`, `FloatMul { dst: p, ... }`, `FloatDiv { dst: p, ... }`, `Move(p)`, ordinary `Assign(p, ...)`, and `Drop(p)` are permitted only when no active authority of either kind overlaps `p`, in addition to their existing value/storage preconditions.

The direct-access rules apply structurally. Delegation never weakens these constraints.

The `IntegerAdd`, `IntegerSub`, `IntegerMul`, `IntegerXor`, `IntegerOr`, `FloatAdd`, `FloatSub`, `FloatMul`, and `FloatDiv` rules above apply only to their direct destinations. They do not make an active explicit loan or safe reference a legal represented result destination, because this revision defines no loan-relative or reference-relative destination form for any of these operations. A left or right operand that uses an existing place/reference access is checked independently under that operand form's ordinary authority rules after destination admission.

## Access through a shared explicit loan

Subject to any authority delegated to its own active explicit children, an active shared explicit loan provides shared alias authority to its concrete root or structural sub-places.

It permits:

- `Read`;
- `Copy`, when the selected type is copyable;
- `AddressOf`;
- pointer-value access for `RawRead`, `RawMove`, and `RawAssign`, subject to each operation's independently owned non-authority preconditions;
- shared explicit reborrow creation;
- `InteriorAssign`, only when the resolved concrete target lies within an interior-mutable region under the value/storage rules.

A shared explicit loan does not permit `Move`, ordinary `Assign`, `Drop`, or exclusive explicit reborrow in this revision.

Interior assignment therefore does not upgrade a shared loan to exclusive authority. It is one operation whose alias requirement is shared and whose independent storage capability is explicit interior mutability.

A raw operation reached through a shared loan uses that loan only to obtain the pointer value. The raw target is then checked independently against all active authorities in the combined alias-conflict domain.

## Access through an exclusive explicit loan

Subject to any authority delegated to its own active explicit children, an active exclusive explicit loan provides exclusive alias authority to its concrete root or structural sub-places.

It permits access using:

- `Read`;
- `Copy`, when the selected type is copyable;
- `AddressOf` because exclusive authority includes shared authority;
- pointer-value access for `RawRead`, `RawMove`, and `RawAssign` because exclusive authority includes shared authority;
- shared or exclusive explicit reborrow according to the ordinary child rules;
- `Move`;
- ordinary `Assign`, when the existing containing-local assignment-mutability rule permits assignment;
- `InteriorAssign`, when the selected concrete target independently lies within an interior-mutable region;
- `Drop`.

The ordinary initialization-state, type, assignment-mutability, interior-mutability, and destruction-domain rules still apply to the selected concrete place. Exclusive access does not weaken those independent rules.

In particular, an exclusive explicit loan over an immutable local has the authority needed for reading, copying, raw-pointer formation, shared pointer-value access for `RawRead`/`RawMove`/`RawAssign`, moving, dropping, and exclusive reborrowing according to their ordinary rules; it does not make ordinary assignment to that local legal. Likewise, exclusive authority alone does not make an unmarked target eligible for `InteriorAssign`.

An exclusive explicit loan controls access to storage rather than to one immutable stored-value identity. Therefore:

1. `Move` or `Drop` through an exclusive root or child loan may end the selected stored-value lifetime and leave that storage Dead;
2. the exclusive borrow interval may remain active because the underlying storage extent and storage-instance identity still exist;
3. when the containing local is mutable, a later legal ordinary `Assign` through the same active exclusive loan may begin a new stored-value lifetime in that storage;
4. independently, when the target lies within an interior-mutable region, legal `InteriorAssign` may begin a replacement stored-value lifetime without relying on the containing local's assignment-mutability flag.

Safe-reference access consumes the same alias categories but its additional replacement capability distinction is owned by [Core references](references.md).

## Interior mutation under shared aliases

Interior mutability is the only defined **safe shared-authority write** exception in the current Core proving model, and it is explicit rather than implicit. Unsafe raw-pointer target ownership transfer and replacement each have a separate exclusive target-access compatibility requirement and do not weaken this rule.

For `InteriorAssign(dst, src)` through an explicit `PlaceAccess`:

1. the borrowing model first requires shared authority for `dst` and resolves it to one concrete place;
2. the value/storage model independently requires that concrete place to lie within an interior-mutable region;
3. source evaluation and replacement follow the value/storage model's source-first lifecycle.

Reference-relative `InteriorAssign` consumes the same shared alias requirement and independent interior-mutability requirement but resolves the target through [Core references](references.md).

Multiple overlapping shared explicit root loans and/or Shared reference-backed authorities may therefore remain active over the same marked storage while sequential legal `InteriorAssign` operations occur in the current single-threaded deterministic Core machine. Those authorities continue to govern the same structural storage region; they do not identify one immutable stored-value lifetime.

This rule does not define concurrency safety. Data races, synchronization, atomics, memory ordering, and multi-agent execution belong to Exec concurrency semantics and are not inferred from the single-threaded Core rule.

This rule also does not define a source-language reference representation. A future source reference system may lower to or refine the represented Core reference and borrowing semantics, but it cannot infer physical address identity, pointer provenance, or a value-stability guarantee from this proving-MIR model.

## Raw-pointer formation authorization

`AddressOf(src)` requires shared authority for its existing `PlaceAccess` source and resolves it to one concrete structural place. Unlike `Read` and `Copy`, formation does not require the selected stored value to be Live because it selects existing storage without reading the pointee value.

The resulting raw pointer does not contain the `LoanId`, explicit borrow interval, shared/exclusive loan kind, safe-reference authority identity, reference permission, or delegation state used to constrain formation. Forming a pointer does not end, shorten, or extend any explicit loan or safe-reference authority, and ending or later reactivating an explicit loan does not alter the already formed pointer.

The first safe-reference slice does not admit reference-relative access as an `AddressOf` source. That exclusion is owned by [Core references](references.md) and prevents safe-reference parameter transfer from implicitly adding cross-activation raw-pointer formation.

Formation authority does not grant later target access through the resulting pointer.

## Raw-pointer target-access compatibility

`RawRead`, `RawMove`, and `RawAssign` are defined by [Core pointers and provenance](pointers.md). This document owns their alias-authority relationships.

First, each operation obtains its stored raw-pointer value using the ordinary shared-authority `PlaceAccess` rules above. That pointer-value access is direct or explicit-loan-relative and is subject to the non-authority operation/value preconditions owned by the pointer and value/storage semantics. It does not grant or preserve authority over the pointee.

After the pointer value selects one concrete structural target region `p`, target compatibility is checked against **every** currently active overlapping authority in the combined alias-conflict domain rather than against one source loan, reference authority, or pointer-formation access.

### Raw target read

`RawRead` has a shared target-access compatibility requirement:

- the read is compatible when no active exclusive authority overlaps `p`;
- overlapping shared explicit loans or Shared reference-backed authorities do not by themselves block the read;
- any overlapping exclusive explicit loan, Exclusive reference authority, or ExclusiveReplace reference authority makes the raw target-read precondition fail;
- disjoint active authorities do not constrain the read.

### Raw target ownership move

`RawMove` has an exclusive target-access compatibility requirement because a defined ownership transfer changes the target's stored-value lifetime state:

- the move is compatible only when no active authority of either kind overlaps `p`;
- any overlapping shared authority makes the raw target-move precondition fail;
- any overlapping exclusive authority makes the raw target-move precondition fail;
- disjoint active authorities do not constrain the move.

`RawMove` does not acquire target authority from ordinary local assignment mutability, reference replacement capability, or from an interior-mutability marker. Those capabilities do not weaken ownership-transfer alias requirements.

### Raw target replacement

`RawAssign` has an exclusive target-access compatibility requirement:

- the replacement is compatible only when no active authority of either kind overlaps `p`;
- any overlapping shared authority makes the raw target-write precondition fail;
- any overlapping exclusive authority makes the raw target-write precondition fail;
- disjoint active authorities do not constrain the replacement.

`RawAssign` does not acquire target authority from ordinary local assignment mutability, safe-reference replacement capability, or an interior-mutability marker. Those are distinct capabilities owned by the operations to which they apply.

Ending the explicit loan that authorized `AddressOf` therefore neither grants nor revokes later raw access by itself; only the complete active alias-authority state at the raw target-access step matters.

Violation of any represented raw target compatibility requirement is classified by [Core unsafe semantics](unsafe.md), not as a new borrow/reference-validation diagnostic. Exact raw-pointer target bookkeeping used for path-state propagation does not turn these unsafe proof obligations into language-validation rules.

## Explicit borrow end and function termination

`EndBorrow(loan)` requires the named explicit loan to be active and to have no active explicit child.

Ending an explicit child interval restores the direct explicit parent's original authority over the delegated region, subject to any other still-active direct explicit children of that parent.

After an explicit borrow end, using that `LoanId` for `PlaceAccess` is invalid until a new interval for that loan declaration begins.

Defined `Return` and defined `Fault` end the complete active **explicit-loan** forest before function local cleanup begins. This semantic termination is not required to fabricate explicit `EndBorrow` verification instrumentation for each automatically ended interval.

Reference-backed authorities are not wholesale-ended at this boundary. Reference carriers are destroyed at the ordinary value/storage cleanup points, and their authorities end under the carrier/child relation from [Core references](references.md). Safe-reference storage-extent validity must hold before each target storage extent ends.

Cleanup then follows the destruction-domain rules without explicit-loan permissions extending beyond the function body.

Explicit borrow end itself is not defined as Runen-observable program behavior by this revision. Reference-authority ending is likewise non-observable except through the legal access state it establishes.

## Determinism

For the current deterministic Core MIR, the complete active explicit-loan forest state participates in semantic path-state repetition. With first-class safe references, the active reference-authority/carrier state supplied by [Core references](references.md) likewise participates whenever it can affect later alias admission or access.

The repeated-state key for explicit loans therefore includes each active loan's kind, concrete root, and current parent relation in addition to storage state and control-flow position. A proving implementation additionally retains the semantically relevant reference authority identity, target, permission/alias class, parent relation, and carrier facts required by the reference owner; exact numeric identities remain verification-only.

Interior mutability adds no hidden explicit loan or borrow-guard state. A legal `InteriorAssign` changes only the ordinary storage state already represented by the value/storage model.

Raw-pointer formation likewise adds no hidden borrow state. The formed pointer is ordinary runtime value state after formation.

Explicit borrow creation, explicit reborrow delegation, ordinary place-access permission, reference-root/reborrow conflict consumption, address-formation alias authorization, raw-operation pointer-value access authorization, interior-assignment alias authorization, `IntegerAdd`/`IntegerSub`/`IntegerMul`/`IntegerXor`/`IntegerOr`/`FloatAdd`/`FloatSub`/`FloatMul`/`FloatDiv` direct-destination authorization, raw target-read compatibility, raw target-move compatibility, raw target-write compatibility, and explicit borrow end are determined from typed structural storage identity, the combined active authority state, the applicable parentage relation, and structural overlap. They do not depend on host references, backend alias analysis, physical scheduling, physical addresses, pointer provenance permissions, or container iteration order. Raw target selection itself remains owned by the pointer semantics.

## Separate semantic owners

This revision does not define lifetime parameters or source borrow inference, source syntax or library APIs for references/interior mutability, runtime borrow guards, raw-pointer target selection or target lifecycle, raw-pointer operations beyond the currently defined compatibility relationships, pointer arithmetic, numeric-address conversion, complete provenance semantics, relocation or pinning, heap allocation/deallocation, value-validity rules, the complete undefined-behavior taxonomy, source `unsafe`, custom destructor bodies, or concurrency/memory-ordering semantics.

First-class safe-reference types/values, reference permission classes, reference-backed authority/carrier lifetime, reference formation/reborrow, reference access, and safe-reference validity are defined by [Core references](references.md). Raw-pointer formation, target selection, provenance, `RawRead`, `RawMove`, and `RawAssign` non-authority semantics are defined by [Core pointers and provenance](pointers.md); this document defines alias authority for existing pointer formation and stored pointer-value `PlaceAccess`, plus combined active-authority compatibility for the raw target read, ownership move, and replacement. Ordinary ownership-transfer, initialization/replacement lifecycles, and represented `IntegerAdd`/`IntegerSub`/`IntegerMul`/`IntegerXor`/`IntegerOr`/`FloatAdd`/`FloatSub`/`FloatMul`/`FloatDiv` result-storage semantics remain owned by [Core value and storage semantics](value-storage.md). Unsafe classification and UB are owned by [Core unsafe semantics](unsafe.md).

Illustrative source spellings such as `&T`, `&mut T`, or an interior-cell type do not freeze grammar. Future source references and library abstractions may lower to or refine the accepted Core reference/borrowing models, but source representation is not defined here.