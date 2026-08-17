# Core Borrowing

Status: **provisional normative; root borrowing defined, reborrowing incomplete**

This document owns the currently defined Core semantics for structural place overlap, root shared/exclusive borrowing, borrow intervals, and access permission through loans.

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

A loan is a semantic access permission over one root place.

A loan has a kind: **shared** or **exclusive**.

Shared versus exclusive describes competing-access and consuming-access authority. It is not itself a declaration that the containing local is mutable for assignment.

A loan is not defined by this revision as a numeric address, pointer bit pattern, source-language reference value, allocation identity, or provenance token.

### Borrow interval

A borrow interval is the interval of execution during which one loan activation is active.

Borrow interval, storage extent, and stored-value lifetime are distinct concepts.

Ending a borrow interval does not by itself end the target storage extent or the currently active stored-value lifetime.

Conversely, an exclusive loan may remain active while an operation through that loan ends one stored-value lifetime and later begins another in the same storage extent when the ordinary assignment rule permits that replacement.

The current proving MIR gives a loan declaration a stable body-local identity. That identity may be activated again after an earlier borrow interval has ended; distinct activations are distinct borrow intervals even when they reuse the same declaration.

### Place access

A Core operation may reach storage either directly through a place or through an active loan plus zero or more structural field projections relative to that loan's root place.

Loan-relative projection does not create a new loan. It selects a sub-place governed by the existing loan.

## Root borrow creation

A root borrow starts from a direct place.

The target place MUST be fully Live when the borrow begins.

A root shared borrow may begin when no active exclusive loan overlaps the target place. Multiple overlapping shared loans are permitted.

A root exclusive borrow may begin only when no active loan of either kind overlaps the target place.

Exclusive borrowing does not by itself require the containing local to be mutable. The existing assignment-mutability rule remains a separate requirement applied when `Assign` is attempted, including when assignment is authorized through an exclusive loan.

A loan identity MUST NOT be started while that loan is already active.

The current proving MIR may explicitly end a borrow interval with `EndBorrow`. Reusing that inactive loan identity later is permitted when its declared type matches the new target place.

Defined `Return` and defined `Fault` end any remaining borrow intervals before function termination cleanup begins.

## Direct access while loans are active

For a direct access target `p`:

- `Read(p)` is permitted when no active exclusive loan overlaps `p`;
- `Copy(p)` is permitted when no active exclusive loan overlaps `p` and the existing copyability rule permits the copy;
- `Init(p, ...)`, `Move(p)`, `Assign(p, ...)`, and `Drop(p)` are permitted only when no active loan of either kind overlaps `p`, in addition to their existing value/storage preconditions.

The direct-access rules apply structurally. A loan over an aggregate therefore constrains direct access to overlapping fields, while a loan over one field does not constrain a disjoint sibling field.

## Access through a shared loan

An active shared loan permits non-consuming access to its root place or structural sub-places:

- `Read`;
- `Copy`, when the selected type is copyable.

A shared loan does not permit `Move`, `Assign`, or `Drop` in this revision.

Shared-loan mutation exceptions are not implicit. Interior mutability is a separate semantic owner and is not defined by this revision.

## Access through an exclusive loan

An active exclusive loan permits access to its root place or structural sub-places using:

- `Read`;
- `Copy`, when the selected type is copyable;
- `Move`;
- `Assign`, when the existing assignment rule permits assignment to the containing local;
- `Drop`.

The ordinary initialization-state, type, assignment-mutability, and destruction-domain rules still apply to the concrete selected place. Exclusive access does not weaken those independent rules.

In particular, an exclusive loan over an immutable local may authorize reading, copying, moving, or dropping according to their ordinary rules, but it does not make assignment to that local legal.

An exclusive loan controls access to storage rather than to one immutable stored-value identity. Therefore:

1. `Move` or `Drop` through an exclusive loan may end the selected stored-value lifetime and leave that storage Dead;
2. the exclusive borrow interval may remain active because the underlying storage extent still exists;
3. when the containing local is mutable, a later legal `Assign` through the same active exclusive loan may begin a new stored-value lifetime in that storage.

This distinction is intentional and follows the separation between storage extent, stored-value lifetime, borrow interval, and assignment mutability.

## Borrow end and termination

`EndBorrow` requires the named loan to be active and ends that borrow interval.

After an explicit borrow end, using that loan for place access is invalid until a new borrow interval for that loan identity begins.

At defined `Return` or defined `Fault`, all remaining root borrow intervals end before termination cleanup. Cleanup then operates according to the destruction-domain rules without borrow permissions extending beyond the function body.

Borrow end itself is not defined as Runen-observable program behavior by this revision.

## Determinism

For the current deterministic Core MIR, active-loan state is part of the semantic path state used to validate repeated control-flow states.

Borrow creation, access permission, and explicit borrow end are determined solely from typed places, active loans, and structural overlap. They do not depend on host references, backend alias analysis, physical scheduling, addresses, or container iteration order.

## Separate semantic owners

This revision does not define:

- reborrowing or parent/child loan delegation;
- lifetime parameters, source-level lifetime syntax, lifetime elision, or source borrow inference;
- first-class reference values or their representation/equality;
- borrowing transient operand values that do not already inhabit a place;
- interior mutability;
- raw pointers, pointer arithmetic, provenance, integer-address conversion, or exposed addresses;
- relocation or pinning/address stability;
- heap allocation/deallocation;
- value validity, invalid bit patterns, or undefined-behavior closure;
- custom destructor bodies;
- atomics or Exec memory/concurrency semantics.

Illustrative source spellings such as `&T` or `&mut T` do not freeze grammar. Future source references may lower to or refine the semantic loan model, but source representation is not defined here.
