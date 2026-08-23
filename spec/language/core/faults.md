# Core Faults

Status: **provisional normative; incomplete**

This document owns the represented Core semantics for defined-fault reason identity and explicit defined-fault termination.

It consumes termination loan handling from [Core borrowing](borrowing.md), termination cleanup and storage-extent ending from [Core value and storage semantics](value-storage.md), and same-fault propagation through suspended direct callers from [Core functions and direct calls](functions.md). [Core control flow](control-flow.md) consumes this document's explicit fault terminator as one represented basic-block termination category with no intra-activation successor.

Defined faults are distinct from undefined behavior and from ordinary recoverable result values.

## Defined-fault reason identity

A represented Core **defined-fault reason** is an abstract semantic identity used to distinguish and preserve one selected defined-fault outcome.

For two represented reasons `F1` and `F2`, semantic identity is sufficient to determine whether they are the same reason for the preservation requirements below. This revision defines no Core operation by which a program can inspect, compare, construct, transform, branch on, match, catch, or recover from a fault reason.

A fault reason is not an ordinary Core value and has no Core value type under this revision. It is not transferred through a local, place, operand, parameter slot, result slot, raw pointer, or loan.

The semantic identity does not prescribe:

- a string or message;
- a numeric code;
- a source location or fault-site identity;
- an exception object;
- a user-visible payload structure;
- a stable serialized encoding;
- allocation identity;
- ABI representation; or
- physical unwind metadata.

An implementation MAY use a string, number, enum tag, interned object, or another representation to distinguish represented fault reasons, provided the implementation preserves semantic reason identity where this specification requires it. Such representation is not Core-observable data merely because verification tooling can display it.

## Explicit fault terminator

The represented explicit defined-fault terminator has the abstract semantic shape:

```text
Fault(F)
```

where `F` is exactly one represented defined-fault reason.

`Fault(F)` is a terminator. It produces no ordinary value and has no normal intra-activation successor.

When execution reaches `Fault(F)` after every preceding statement in that basic block has completed with a defined continuation:

1. evaluate no operand;
2. perform no additional ordinary read, move, copy, write, initialization, assignment, borrow, pointer operation, or value production;
3. select exactly the embedded defined-fault reason `F`;
4. perform the current activation's existing defined-fault termination loan handling under `borrowing.md`;
5. perform the current activation's existing defined-fault local cleanup and storage-extent ending under `value-storage.md`;
6. terminate the current activation with defined fault `F`; and
7. apply the surrounding direct-call or outer-execution relation below without changing `F`.

Steps 4 and 5 consume the existing termination rules; this document does not create a second loan-ending, destruction-domain, cleanup-order, or storage-lifetime relation.

A place that was moved, destroyed, never initialized, or only partially initialized before `Fault(F)` therefore participates in termination cleanup exactly as required by its then-current state under `value-storage.md`. The fault terminator neither revives nor normalizes storage before cleanup.

## Direct-call propagation

When the faulting activation has a suspended direct caller, [Core functions and direct calls](functions.md) owns propagation through that call boundary.

The caller receives **the same defined-fault reason `F`** instead of following the call's normal continuation. The caller's result destination is not initialized on that fault path. The caller then performs its own applicable defined-fault termination handling and propagates the same `F` outward.

Repeated propagation through any finite prefix of suspended represented direct callers preserves exactly the initiating reason `F`. Each terminated activation performs its applicable termination handling exactly once.

This preservation requirement is semantic fault-reason identity, not equality of implementation strings, numeric codes, physical exception objects, or backtraces.

## Outermost represented execution

When `Fault(F)` or its direct-call propagation reaches the outermost represented Core activation with no suspended represented direct caller:

- that activation performs its applicable defined-fault termination handling exactly once;
- the represented Core execution terminates with defined-fault outcome `F`;
- no normal result value is produced; and
- no normal continuation is selected.

The embedding environment may record or report the defined-fault outcome only in ways that preserve the semantic distinction required by its applicable contract. This document defines no source-visible reporting format.

## Function-result independence

`Fault(F)` is valid as an explicit abnormal terminator in a represented function whether that function has:

- no result value; or
- exactly one result type.

The fault terminator requires no return operand and synthesizes no result value. A result-bearing function path that reaches `Fault(F)` terminates abnormally and therefore does not violate the normal-return requirement merely because no result value is produced on that path.

Normal `Return` result presence, result typing, result preservation, and caller result transfer remain owned by `functions.md` and are unchanged.

## Static validation and CFG boundary

A represented `Fault(F)` terminator:

- contains no block target;
- contains no operand;
- imposes no ordinary value-type or place-state precondition beyond the structural well-formedness of the enclosing represented Core body and the presence of one represented fault reason `F`; and
- contributes no intra-activation successor edge to CFG-reachable path-state validation.

Consequently, local/loan/path state after a faulting terminator is not propagated to another block in the same activation merely to continue validation. Disconnected blocks remain subject to the static/structural validation requirements owned by `control-flow.md`, but they do not become CFG-reachable through `Fault(F)`.

A fault terminator is not a branch to a hidden cleanup block, catch block, caller block, or synthetic return block in Core semantics. An implementation may realize equivalent control flow internally only when the externally defined Core relation above is preserved.

## Undefined-behavior boundary

Defined fault is not undefined behavior.

When an accepted unsafe operation selects undefined behavior, that outcome does not become `Fault(F)`, does not acquire a defined-fault reason, and does not invoke this document's defined-fault termination/propagation relation unless another future accepted owner explicitly defines a distinct conversion boundary.

Likewise, `Fault(F)` is a defined termination outcome and is not undefined merely because no catch boundary exists.

## Recoverable-value boundary

An ordinary Core or source value that represents a recoverable domain/application failure remains an ordinary value. It does not become a defined fault merely because an implementation or application interprets the value as failure.

This document introduces no implicit conversion between ordinary result values and defined-fault outcomes.

## Control-flow, cleanup, and implementation boundaries

`control-flow.md` owns basic-block execution, `Goto`, `Branch`, CFG reachability, and cyclic execution. It consumes only the fact that `Fault(F)` is one terminator with no intra-activation successor.

`borrowing.md` owns termination loan handling. `value-storage.md` owns destruction domains, cleanup order, stored-value lifetimes, and storage-extent ending. `functions.md` owns caller suspension, direct-call fault propagation, and the fact that a faulting call does not follow its normal continuation or initialize its result destination.

This document does not redefine those relations.

This revision defines no:

- source-language `fault`, `panic`, `throw`, or other syntax;
- fault payload/message type or user-visible fault object;
- catch/recovery boundary;
- pattern matching or comparison over fault reasons;
- exception hierarchy;
- backtrace semantics;
- effect signature or checked-exception system;
- physical stack unwinding;
- abort/process termination policy;
- task cancellation relation;
- ABI, linkage, calling convention, or binary fault representation;
- optimizer transformation legality; or
- backend exception/unwind instruction selection.

Those concerns require their own accepted owners or later consumers.