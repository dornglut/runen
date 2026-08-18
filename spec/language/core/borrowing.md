# Core Borrowing

Status: **provisional normative; root borrowing, reborrowing, interior-mutability interaction, and raw-pointer formation authorization defined**

This document defines structural place overlap, shared/exclusive borrowing, reborrow delegation, borrow intervals, and alias authority through active loans.

Storage extent, storage-instance identity, stored-value lifetime, initialization state, assignment mutability, interior mutability, destruction, and cleanup are defined by [Core value and storage semantics](value-storage.md). Raw-pointer values and provenance are defined by [Core pointers and provenance](pointers.md).

## Terms

### Place overlap

Two places overlap when they denote storage where one place contains the other in the current structural place model.

For the field-only places defined by this revision:

- places rooted in different locals are disjoint;
- identical places overlap;
- a local or aggregate place overlaps every structural descendant sub-place;
- two places with the same local root overlap exactly when either field-projection sequence is a prefix of the other;
- sibling field places that diverge at a field projection are disjoint.

Place overlap is symmetric. It depends only on semantic place structure and does not depend on physical addresses, field offsets, ABI layout, relocation, or pointer provenance.

### Loan

A loan is a semantic alias-access permission over one concrete root place.

A loan has a kind: **shared** or **exclusive**.

Shared versus exclusive describes competing-access authority. It does not itself declare the containing local mutable for ordinary assignment, and it does not itself make storage interior-mutable.

The three concerns are independent:

- **assignment mutability** controls ordinary `Assign`;
- **alias authority** is shared or exclusive;
- **interior mutability** is explicit type/storage capability.

A loan is not a numeric address, pointer bit pattern, source-language reference value, allocation identity, dynamic storage-instance identity, or provenance token.

### Alias authority

**Shared authority** permits operations whose borrowing requirement is shared. In this revision those include `Read`, copyable `Copy`, shared reborrow creation, raw-pointer `AddressOf` formation, and `InteriorAssign` when the target independently lies within an interior-mutable region.

**Exclusive authority** includes shared authority and additionally permits operations whose borrowing requirement is exclusive. In this revision those include `Move`, ordinary `Assign`, `Drop`, and exclusive reborrow creation, subject to each operation's independent value/storage preconditions.

`Init` is also an exclusive-authority direct-storage operation.

Interior mutation is not a third alias kind. A shared write to explicitly interior-mutable storage uses shared alias authority plus a separate interior-mutability capability check.

### Borrow interval

A borrow interval is the interval of execution during which one loan activation is active.

Borrow interval, storage extent, dynamic storage-instance identity, and stored-value lifetime are distinct concepts.

Ending a borrow interval does not by itself end the target storage extent, its storage-instance identity, or the currently active stored-value lifetime.

Conversely, an exclusive loan may remain active while an exclusive operation through that loan ends one stored-value lifetime and later begins another in the same storage extent when ordinary assignment permits replacement.

For interior-mutable storage, a shared loan may likewise remain active while legal `InteriorAssign` ends old stored-value lifetimes and begins replacement lifetimes in the same storage extent. A shared loan is therefore not a promise that one stored-value lifetime remains unchanged for the whole borrow interval.

The current proving MIR gives a loan declaration a stable body-local `LoanId`. That declaration may be activated again after an earlier interval ends. Each activation is a distinct borrow interval and receives fresh dynamic root, kind, and parent state.

### Place access

A Core operation may reach storage either directly through a place or through an active loan plus zero or more structural field projections relative to that loan's concrete root.

Loan-relative projection does not create a new loan. It selects a sub-place governed by the existing loan.

`PlaceAccess` is proving-MIR access authority. It is not a stored reference value, raw-pointer value, address, dynamic storage-instance identity, or provenance identity.

### Loan forest

Active loans form a rooted forest.

- a root loan has no parent;
- a reborrowed child records the active parent loan from which its authority was delegated;
- a child root is the concrete place selected through its parent access when the child interval begins;
- one loan declaration may have at most one active interval at a time;
- an active loan cannot become its own ancestor because a new child requires an inactive destination `LoanId` and an already-active parent;
- every explicit loan end is leaf-to-root: a parent interval cannot end while a direct or indirect descendant remains active.

Parentage belongs to the current activation, not permanently to the reusable loan declaration.

## Borrow creation

The proving MIR has one borrow operation whose source is a `PlaceAccess`:

```text
Borrow { loan, kind, src: PlaceAccess }
```

A direct source begins a root interval. A loan-relative source begins a child interval.

For every borrow creation:

- the destination loan declaration MUST be inactive;
- the selected source type MUST equal the destination loan declaration type;
- the selected concrete root MUST be fully Live when the interval begins.

Borrow creation does not consume or mutate the stored value.

### Root borrow creation

For `PlaceAccess::Direct(p)`:

- a shared root borrow may begin when no active exclusive loan overlaps `p`;
- multiple overlapping shared root loans are permitted;
- an exclusive root borrow may begin only when no active loan of either kind overlaps `p`.

Exclusive borrowing does not by itself require the containing local to be mutable. Assignment mutability remains a separate rule applied when ordinary `Assign` is attempted.

Interior-mutability capability is also irrelevant to whether a root shared or exclusive loan may begin. It affects only the dedicated interior-replacement operation after alias authority has been established.

### Reborrow creation

For `PlaceAccess::Loan(parent, projections)`, the selected concrete child root is the parent loan's concrete root followed by the relative structural projections.

A shared child may be created from:

- an active shared parent that currently retains shared authority over the selected child root;
- an active exclusive parent that currently retains shared authority over the selected child root.

An exclusive child may be created only from an active exclusive parent that currently retains exclusive authority over the selected child root.

Therefore:

- overlapping shared children may coexist;
- an overlapping exclusive child blocks creation of any further overlapping child through the same parent;
- an overlapping shared child blocks creation of an exclusive sibling through the same parent;
- disjoint children do not constrain one another;
- an exclusive child may never be derived from a shared parent.

Reborrow creation uses only typed place structure, active-loan state, and the place-overlap relation. It does not inspect physical addresses or pointer provenance.

## Delegated parent authority

A child interval delegates only the portion of its direct parent's authority that overlaps the child's concrete root.

For an access through parent loan `P` to concrete place `p`, only active direct children of `P` are needed to determine the authority still retained by `P` at `p`.

This is sufficient because a direct child continues to own its delegated root until that direct child ends. Deeper descendants refine the direct child's authority but do not return any authority to the grandparent while the direct child remains active.

### Overlapping exclusive child

If an active exclusive direct child of `P` overlaps `p`, access through `P` to `p` is suspended completely.

`Read`, `Copy`, `AddressOf`, `Move`, ordinary `Assign`, `InteriorAssign`, `Drop`, and overlapping reborrow through `P` are invalid until that child interval ends.

### Overlapping shared child

If one or more active shared direct children of `P` overlap `p`, `P` retains shared authority over that overlapping storage:

- `Read` is permitted;
- `Copy` is permitted when the selected type is copyable;
- `AddressOf` is permitted;
- shared reborrow is permitted when its ordinary preconditions hold;
- `InteriorAssign` is permitted when the selected concrete target independently lies within an interior-mutable region;
- `Move`, ordinary `Assign`, `Drop`, and exclusive reborrow are invalid.

For an exclusive parent this is a temporary local downgrade from exclusive to shared authority. For a shared parent, an overlapping shared child does not further reduce the parent's already-shared authority.

The availability of `InteriorAssign` does not mean the shared child grants mutation permission. The child only leaves shared alias authority intact; the distinct interior-mutability capability is checked by the value/storage rules.

### Disjoint child

A direct child whose concrete root is disjoint from `p` does not constrain access through `P` to `p`.

An exclusive parent over an aggregate may therefore delegate one field to an exclusive child while retaining exclusive authority over a disjoint sibling field. It may perform `InteriorAssign` or `AddressOf` on a disjoint sibling when their independent preconditions hold.

## Direct access while loans are active

Direct access remains constrained by every active concrete loan, regardless of parent/child structure.

For a direct access target `p`:

- `Read(p)` is permitted when no active exclusive loan overlaps `p`;
- `Copy(p)` is permitted when no active exclusive loan overlaps `p` and the existing copyability rule permits the copy;
- `AddressOf(p)` is permitted when no active exclusive loan overlaps `p`; it does not additionally require `p` to be Live;
- `InteriorAssign(p, ...)` is permitted when no active exclusive loan overlaps `p` and `p` independently lies within an interior-mutable region;
- `Init(p, ...)`, `Move(p)`, ordinary `Assign(p, ...)`, and `Drop(p)` are permitted only when no active loan of either kind overlaps `p`, in addition to their existing value/storage preconditions.

The direct-access rules apply structurally. Loan delegation never weakens these constraints.

## Access through a shared loan

Subject to any authority delegated to its own active children, an active shared loan provides shared alias authority to its concrete root or structural sub-places.

It permits:

- `Read`;
- `Copy`, when the selected type is copyable;
- `AddressOf`;
- shared reborrow creation;
- `InteriorAssign`, only when the resolved concrete target lies within an interior-mutable region under the value/storage rules.

A shared loan does not permit `Move`, ordinary `Assign`, `Drop`, or exclusive reborrow in this revision.

Interior assignment therefore does not upgrade a shared loan to exclusive authority. It is one operation whose alias requirement is shared and whose independent storage capability is explicit interior mutability.

## Access through an exclusive loan

Subject to any authority delegated to its own active children, an active exclusive loan provides exclusive alias authority to its concrete root or structural sub-places.

It permits access using:

- `Read`;
- `Copy`, when the selected type is copyable;
- `AddressOf` because exclusive authority includes shared authority;
- shared or exclusive reborrow according to the ordinary child rules;
- `Move`;
- ordinary `Assign`, when the existing containing-local assignment-mutability rule permits assignment;
- `InteriorAssign`, when the selected concrete target independently lies within an interior-mutable region;
- `Drop`.

The ordinary initialization-state, type, assignment-mutability, interior-mutability, and destruction-domain rules still apply to the selected concrete place. Exclusive access does not weaken those independent rules.

In particular, an exclusive loan over an immutable local may authorize reading, copying, raw-pointer formation, moving, dropping, and exclusive reborrowing according to their ordinary rules, but it does not make ordinary assignment to that local legal. Likewise, exclusive authority alone does not make an unmarked target eligible for `InteriorAssign`.

An exclusive loan controls access to storage rather than to one immutable stored-value identity. Therefore:

1. `Move` or `Drop` through an exclusive root or child loan may end the selected stored-value lifetime and leave that storage Dead;
2. the exclusive borrow interval may remain active because the underlying storage extent and storage-instance identity still exist;
3. when the containing local is mutable, a later legal ordinary `Assign` through the same active exclusive loan may begin a new stored-value lifetime in that storage;
4. independently, when the target lies within an interior-mutable region, legal `InteriorAssign` may begin a replacement stored-value lifetime without relying on the containing local's assignment-mutability flag.

## Interior mutation under shared aliases

Interior mutability is the only defined shared-write exception in the current Core proving model, and it is explicit rather than implicit.

For `InteriorAssign(dst, src)`:

1. the borrowing model first requires shared authority for `dst` and resolves it to one concrete place;
2. the value/storage model independently requires that concrete place to lie within an interior-mutable region;
3. source evaluation and replacement follow the value/storage model's source-first lifecycle.

Multiple overlapping shared root loans may therefore remain active over the same marked storage while sequential `InteriorAssign` operations occur in the current single-threaded deterministic Core machine. Those loans continue to govern the same structural storage region; they do not identify one immutable stored-value lifetime.

This rule does not define concurrency safety. Data races, synchronization, atomics, memory ordering, and multi-agent execution belong to Exec concurrency semantics and are not inferred from the single-threaded Core rule.

This rule also does not define a source-language reference representation. A future source reference system may lower to or refine these semantic loans, but it cannot infer physical address identity, pointer provenance, or a value-stability guarantee from this proving-MIR model.

## Raw-pointer formation authorization

`AddressOf(src)` requires shared authority for `src` and resolves it to one concrete structural place. Unlike `Read` and `Copy`, formation does not require the selected stored value to be Live because it selects existing storage without reading the pointee value.

The resulting raw pointer does not contain the `LoanId`, borrow interval, shared/exclusive loan kind, or delegation state used to authorize formation. Forming a pointer does not end, shorten, or extend the source loan, and ending or later reactivating that loan does not alter the already formed pointer.

These rules authorize pointer formation only. They do not authorize dereference, load, store, arithmetic, or any other memory access through the resulting raw pointer.

## Borrow end and termination

`EndBorrow(loan)` requires the named loan to be active and to have no active child.

Ending a child interval restores the direct parent's original authority over the delegated region, subject to any other still-active direct children of that parent.

After an explicit borrow end, using that loan for place access is invalid until a new interval for that loan declaration begins.

Defined `Return` and defined `Fault` end the complete active loan forest before function termination cleanup begins. This semantic termination is not required to fabricate explicit `BorrowEnd` verification instrumentation for each automatically ended interval.

Cleanup then follows the destruction-domain rules without borrow permissions extending beyond the function body.

Borrow end itself is not defined as Runen-observable program behavior by this revision.

## Determinism

For the current deterministic Core MIR, the complete active-loan forest state participates in semantic path-state repetition.

The repeated-state key therefore includes each active loan's kind, concrete root, and current parent relation in addition to storage state and control-flow position.

Interior mutability adds no hidden loan or borrow-guard state. A legal `InteriorAssign` changes only the ordinary storage state already represented by the value/storage model.

Raw-pointer formation likewise adds no hidden borrow state. The formed pointer is ordinary runtime value state after formation.

Borrow creation, reborrow delegation, access permission, address-formation alias authorization, interior-assignment alias authorization, and explicit borrow end are determined solely from typed places, active loans, structural parentage, and structural overlap. They do not depend on host references, backend alias analysis, physical scheduling, physical addresses, pointer provenance, or container iteration order.

## Separate semantic owners

This revision does not define lifetime parameters or source borrow inference, first-class reference values, borrowing transient operand values, source syntax or library APIs for interior mutability, runtime borrow guards, raw-pointer memory access or arithmetic, numeric-address conversion, complete provenance semantics, relocation or pinning, heap allocation/deallocation, value validity, undefined behavior, source `unsafe`, custom destructor bodies, or concurrency/memory-ordering semantics.

Raw-pointer formation and provenance are defined by [Core pointers and provenance](pointers.md); this document defines only the alias authority required to select storage for that formation.

Illustrative source spellings such as `&T`, `&mut T`, or an interior-cell type do not freeze grammar. Future source references and library abstractions may lower to or refine the semantic loan and interior-mutability models, but source representation is not defined here.
