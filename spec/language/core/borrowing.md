# Core Borrowing

Status: **provisional normative; root borrowing and reborrowing defined**

This document owns the currently defined Core semantics for structural place overlap, shared/exclusive borrowing, reborrow delegation, borrow intervals, and access permission through active loans.

The storage existence, stored-value lifetime, initialization-state, ownership-transfer, assignment, destruction-domain, and cleanup rules that borrowing constrains are owned by [Core value and storage semantics](value-storage.md).

## Terms

### Place overlap

Two places overlap when they denote storage where one place contains the other in the current structural place model.

For the field-only places defined by this revision:

- places rooted in different locals are disjoint;
- identical places overlap;
- a local or aggregate place overlaps every structural descendant sub-place;
- two places with the same local root overlap exactly when either field-projection sequence is a prefix of the other;
- sibling field places that diverge at a field projection are disjoint.

Place overlap is symmetric. It depends only on semantic place structure and does not depend on physical addresses, field offsets, ABI layout, allocation identity, relocation, or pointer provenance.

### Loan

A loan is a semantic access permission over one concrete root place.

A loan has a kind: **shared** or **exclusive**.

Shared versus exclusive describes competing-access and consuming-access authority. It is not itself a declaration that the containing local is mutable for assignment.

A loan is not defined by this revision as a numeric address, pointer bit pattern, source-language reference value, allocation identity, or provenance token.

### Borrow interval

A borrow interval is the interval of execution during which one loan activation is active.

Borrow interval, storage extent, and stored-value lifetime are distinct concepts.

Ending a borrow interval does not by itself end the target storage extent or the currently active stored-value lifetime.

Conversely, an exclusive loan may remain active while an operation through that loan ends one stored-value lifetime and later begins another in the same storage extent when the ordinary assignment rule permits that replacement.

The current proving MIR gives a loan declaration a stable body-local `LoanId`. That declaration may be activated again after an earlier interval ends. Each activation is a distinct borrow interval and receives fresh dynamic root, kind, and parent state.

### Place access

A Core operation may reach storage either directly through a place or through an active loan plus zero or more structural field projections relative to that loan's concrete root.

Loan-relative projection does not create a new loan. It selects a sub-place governed by the existing loan.

`PlaceAccess` is semantic access authority in the proving MIR. It is not a stored reference value, pointer representation, address, or provenance identity.

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

The proving MIR has one borrow operation whose source is a `PlaceAccess`.

Conceptually:

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

Exclusive borrowing does not by itself require the containing local to be mutable. Assignment mutability remains a separate rule applied when `Assign` is attempted.

### Reborrow creation

For `PlaceAccess::Loan(parent, projections)`, the selected concrete child root is the parent loan's concrete root followed by the relative structural projections.

A shared child may be created from:

- an active shared parent that currently retains read authority over the selected child root;
- an active exclusive parent that currently retains read authority over the selected child root.

An exclusive child may be created only from an active exclusive parent that currently retains consuming authority over the selected child root.

Therefore:

- overlapping shared children may coexist;
- an overlapping exclusive child blocks creation of any further overlapping child through the same parent;
- an overlapping shared child blocks creation of an exclusive sibling through the same parent;
- disjoint children do not constrain one another;
- an exclusive child may never be derived from a shared parent.

Reborrow creation uses only typed place structure, active-loan state, and the place-overlap relation. It does not inspect physical addresses or provenance.

## Delegated parent authority

A child interval delegates only the portion of its direct parent's authority that overlaps the child's concrete root.

For an access through parent loan `P` to concrete place `p`, only active direct children of `P` are needed to determine the authority still retained by `P` at `p`.

This is sufficient because a direct child continues to own its delegated root until that direct child ends. Deeper descendants refine the direct child's authority but do not return any authority to the grandparent while the direct child remains active.

### Overlapping exclusive child

If an active exclusive direct child of `P` overlaps `p`, access through `P` to `p` is suspended completely.

`Read`, `Copy`, `Move`, `Assign`, `Drop`, and overlapping reborrow through `P` are invalid until that child interval ends.

### Overlapping shared child

If one or more active shared direct children of `P` overlap `p`, `P` retains only shared/non-consuming authority over that overlapping storage:

- `Read` is permitted;
- `Copy` is permitted when the selected type is copyable;
- `Move`, `Assign`, `Drop`, and exclusive reborrow are invalid.

For an exclusive parent this is a temporary local downgrade from exclusive to shared authority. For a shared parent, an overlapping shared child does not further reduce the parent's already-shared authority.

### Disjoint child

A direct child whose concrete root is disjoint from `p` does not constrain access through `P` to `p`.

An exclusive parent over an aggregate may therefore delegate one field to an exclusive child while retaining exclusive authority over a disjoint sibling field.

## Direct access while loans are active

Direct access remains constrained by every active concrete loan, regardless of parent/child structure.

For a direct access target `p`:

- `Read(p)` is permitted when no active exclusive loan overlaps `p`;
- `Copy(p)` is permitted when no active exclusive loan overlaps `p` and the existing copyability rule permits the copy;
- `Init(p, ...)`, `Move(p)`, `Assign(p, ...)`, and `Drop(p)` are permitted only when no active loan of either kind overlaps `p`, in addition to their existing value/storage preconditions.

The direct-access rules apply structurally. Loan delegation never weakens these constraints.

## Access through a shared loan

Subject to any authority delegated to its own active children, an active shared loan permits non-consuming access to its concrete root or structural sub-places:

- `Read`;
- `Copy`, when the selected type is copyable.

A shared loan does not permit `Move`, `Assign`, or `Drop` in this revision.

Shared-loan mutation exceptions are not implicit. Interior mutability is a separate semantic owner and is not defined by this revision.

## Access through an exclusive loan

Subject to any authority delegated to its own active children, an active exclusive loan permits access to its concrete root or structural sub-places using:

- `Read`;
- `Copy`, when the selected type is copyable;
- `Move`;
- `Assign`, when the existing assignment rule permits assignment to the containing local;
- `Drop`.

The ordinary initialization-state, type, assignment-mutability, and destruction-domain rules still apply to the selected concrete place. Exclusive access does not weaken those independent rules.

In particular, an exclusive loan over an immutable local may authorize reading, copying, moving, dropping, and exclusive reborrowing according to their ordinary rules, but it does not make assignment to that local legal.

An exclusive loan controls access to storage rather than to one immutable stored-value identity. Therefore:

1. `Move` or `Drop` through an exclusive root or child loan may end the selected stored-value lifetime and leave that storage Dead;
2. the exclusive borrow interval may remain active because the underlying storage extent still exists;
3. when the containing local is mutable, a later legal `Assign` through the same active exclusive loan may begin a new stored-value lifetime in that storage.

This distinction follows the separation between storage extent, stored-value lifetime, borrow interval, access exclusivity, and assignment mutability.

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

Borrow creation, reborrow delegation, access permission, and explicit borrow end are determined solely from typed places, active loans, structural parentage, and structural overlap. They do not depend on host references, backend alias analysis, physical scheduling, addresses, allocation identity, or container iteration order.

## Separate semantic owners

This revision does not define:

- lifetime parameters, source-level lifetime syntax, lifetime elision, or source borrow inference;
- first-class reference values, stored references, or their representation/equality;
- borrowing transient operand values that do not already inhabit a place;
- interior mutability or shared-write exceptions;
- raw pointers, pointer arithmetic, provenance, integer-address conversion, or exposed addresses;
- relocation or pinning/address stability;
- heap allocation/deallocation;
- value validity, invalid bit patterns, or undefined-behavior closure;
- custom destructor bodies;
- atomics or Exec memory/concurrency semantics.

Illustrative source spellings such as `&T` or `&mut T` do not freeze grammar. Future source references may lower to or refine the semantic loan model, but source representation is not defined here.