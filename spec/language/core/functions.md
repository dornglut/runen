# Core Functions and Direct Calls

Status: **provisional normative; incomplete**

This document owns the currently represented Core semantics for finite function programs, function identity, owned-value parameter slots, direct calls, dynamic function activations, result transfer, recursion, divergence, and defined-fault propagation through direct calls.

It consumes local storage, initialization state, owned move/copy, destruction domains, and function-termination cleanup from [Core value and storage semantics](value-storage.md); access authority and loan termination from [Core borrowing](borrowing.md); raw-pointer value and provenance semantics from [Core pointers and provenance](pointers.md); and defined-fault classification from [Core faults](faults.md). It does not redefine those owners.

This relation is independent of source syntax, source name resolution, ABI, calling convention, physical stack layout, backend realization, or a particular compiler representation.

## Core program and function identity

A represented Core program contains:

- one finite Core type-identity domain; and
- one finite sequence of represented Core function entities.

Each represented function entity has one identity within that program. Function identity is semantic within the represented Core program but does not require a stable numeric encoding, serialized identifier, symbol name, physical address, or source declaration identity.

Each represented function contains exactly one body under the Core body relation and exactly one callable structure consisting of:

1. one finite ordered sequence of owned-value parameter slots; and
2. either no result value or exactly one result type.

The program-wide type domain is shared by every function body and callable structure in that program. A type identity used by two different functions therefore denotes the same represented Core type.

This revision defines no overload set, indirect call, function value, closure, method receiver, variadic parameter list, default argument, generic callable, effect signature, ABI signature, or linkage identity.

## Parameter slots and parameter locals

Every represented parameter slot designates exactly one local declaration in the function body. Parameter-local designations are ordered in parameter-slot order and MUST be unique within that function.

The type of parameter slot `i` is exactly the declared Core type of the local designated by slot `i`. The semantic callable structure does not contain a second independent parameter-type sequence. An implementation MAY cache the derived parameter-type sequence only when it remains provably equal to the designated locals' declared types.

A designated parameter local is otherwise an ordinary Core local. Its assignment-mutability property, structural type, storage extent, stored-value lifetime, borrow interactions, and termination cleanup follow the existing Core owners.

The parameter sequence may be empty.

The result specification is exactly one of:

- no result value; or
- one result value of exactly one represented Core type.

No-result structure does not introduce Unit, Void, or another Core value type.

## Call-transfer-safe types

The first represented Core direct-call relation transfers owned values between activations but does not transfer raw-pointer-containing values between activations.

A represented Core type is **call-transfer-safe** exactly when its structural value shape contains no raw-pointer leaf:

- a non-pointer scalar leaf is call-transfer-safe;
- a raw-pointer leaf is not call-transfer-safe; and
- a structural aggregate is call-transfer-safe exactly when every recursively contained field type is call-transfer-safe.

Every represented function parameter type and represented function result type MUST be call-transfer-safe.

Raw-pointer locals and raw-pointer operations remain permitted inside one activation under their existing owners. This restriction therefore does not weaken or redefine intra-activation pointer semantics; it only bounds inter-activation value transfer.

This revision does not define reference parameters, borrowed parameters, cross-activation loans, pointer escape, pointer-to-caller storage transfer, pointer-to-callee storage return, or another pass mode.

## Dynamic function activations

Every dynamic direct call creates one fresh **Core function activation** for the target function.

Each activation has independent dynamic local-storage state and independent active-loan state for the target body. Distinct simultaneously active or recursively nested calls therefore have distinct dynamic storage instances even when they execute the same static function and local declarations.

When an activation is created, one fresh dynamic local storage instance is created for every local declaration in the target body. All of those local storage instances initially contain no initialized stored value.

Parameter transfer then initializes the designated parameter locals as defined below. The function body begins only after all parameter transfers have completed.

Activation identity is not Core-observable program data and does not imply a physical stack frame, stack address, calling convention, target stack guarantee, thread identity, or task identity.

## Direct-call control transfer

A represented direct call identifies exactly one target function entity and exactly one normal continuation block in the caller body.

The call contains:

- the target function identity;
- one ordered argument operand for each target parameter slot;
- either no result destination or exactly one direct result destination place; and
- the caller's normal continuation block.

The target function MUST exist in the same represented Core program.

Call-graph cycles are valid. Direct recursion and mutual recursion therefore remain valid represented Core programs. A recursive execution may diverge.

## Result destination admission

A no-result target requires no result destination.

A result-bearing target requires exactly one direct destination place in the caller whose Core type is exactly equal to the target result type.

The result destination is a first-initialization destination, not an assignment or replacement destination. At the call point:

- every scalar leaf in the destination MUST still be Never-initialized under `value-storage.md`; and
- direct access to that destination MUST have the same exclusive access authority required by ordinary `Init`.

The containing local need not be mutable merely because the call may initialize the destination.

The destination admission facts are established for the call point before argument operand state transitions are applied. Argument evaluation never itself initializes the call result destination under this relation.

A successful result return initializes the result destination and begins the corresponding stored-value lifetimes before control continues at the normal target.

A faulting or diverging callee does not initialize the destination and does not follow the normal target.

## Argument evaluation and parameter transfer

Argument count MUST equal the target parameter count exactly.

Each argument operand MUST produce one owned Core value whose Core type is exactly equal to the corresponding parameter-slot type. This revision introduces no implicit conversion, widening, narrowing, coercion, subtyping, or defaulting relation.

Arguments are evaluated strictly left to right in argument/parameter-slot order.

Each successfully evaluated argument value is held as one owned transient call value until all represented argument operands have evaluated successfully. Producing a transient call value does not itself require an addressable Core local storage place.

Existing operand semantics determine each argument's state effects. In particular, `Move` consumes its source stored value and `Copy` preserves its source stored value.

After every argument operand has evaluated successfully:

1. create one fresh activation of the target function;
2. transfer the transient argument values into the designated parameter locals in parameter-slot order; and
3. mark each transferred parameter local initialized with the transferred value before target-body entry.

Parameter transfer does not duplicate a transient argument value.

The represented operand set in this revision does not itself add a new defined-fault or divergence relation for operand evaluation. Undefined behavior selected by an existing unsafe operand has no defined post-state to continue into a call. A future operand owner that adds another abnormal evaluation outcome must define its interaction with held transient call values rather than inferring that behavior from this direct-call relation.

## Caller suspension

After successful argument evaluation and callee activation creation, the caller activation is suspended at the call.

The caller's local storage state and active-loan state remain unchanged while the callee executes except for state effects already caused by argument operand evaluation.

Because represented call-transfer types contain no raw-pointer leaf, the callee receives no transferred raw-pointer path to suspended caller storage through this relation.

Caller suspension does not perform cleanup and does not end caller storage extents or loans.

## Normal return

A represented return contains either no result operand or exactly one result operand.

A function with no result type MUST return with no result operand.

A function with one result type MUST return with exactly one owned result operand whose Core type is exactly equal to that result type.

For a result-bearing return:

1. evaluate the result operand completely under its existing operand semantics;
2. preserve the successfully produced owned result value outside the activation-local cleanup set;
3. perform termination loan handling and local cleanup for the current activation under the existing Core borrowing and value/storage rules;
4. terminate the current activation normally; and
5. transfer the preserved owned result value to the suspended caller without duplication.

For a no-result return, perform the same activation termination relation but produce no Core value.

When the caller expects a result, successful result transfer initializes the admitted result destination and normal execution resumes at the call's normal continuation block.

When the caller expects no result, normal execution resumes at the normal continuation block without producing or discarding a hidden value.

A moved return source is already Dead before termination cleanup and therefore is not destroyed again by that cleanup.

The outer consumer of a represented Core function execution receives the same optional result structure: no-result normal completion yields no value, while result-bearing normal completion yields the preserved owned result value. This fact defines Core execution structure and does not establish source entry-point semantics.

## Defined-fault propagation through calls

The represented direct-call subset has no catch boundary.

When a called activation terminates with defined fault `F`:

1. that activation performs its ordinary defined-fault termination handling and cleanup under the existing Core owners;
2. the suspended caller's direct-call evaluation yields the same defined fault `F` instead of following its normal continuation;
3. the caller performs its own defined-fault termination handling and cleanup; and
4. the same `F` continues outward through each suspended direct caller.

Propagation therefore preserves the selected defined-fault outcome while cleaning each terminated activation exactly once.

The caller result destination is not initialized on the fault path.

This semantic propagation does not require physical stack unwinding and does not define a panic payload, exception object, backtrace, catch construct, recovery boundary, or physical unwinding mechanism.

Undefined behavior remains distinct from defined fault. Detection of undefined behavior does not trigger this defined-fault cleanup relation.

## Divergence

If a called activation diverges, its caller remains suspended at that call.

Divergence does not follow the normal continuation, does not initialize a result destination, and does not trigger return or fault cleanup merely because execution continues indefinitely.

There is no implicit execution-step limit in this relation.

## Termination cleanup boundary

This document selects which activation terminates and when result or fault transfer occurs. It does not redefine the destruction domain or local cleanup order.

For every normally returning or defined-faulting activation, local cleanup remains the reverse local declaration order owned by `value-storage.md`, using the then-current initialization state and skipping Never-initialized or Dead values as already specified there.

A parameter local participates in that same local declaration order. A compiler refining another language-level cleanup relation is responsible for choosing Core local declaration order that preserves the required higher-level ordering; Core parameter-slot order does not silently replace the existing local cleanup owner.

## Validation requirements

A represented Core program is language-valid under this relation only when all of the following hold in addition to existing body validity:

- every represented function body and callable type references the program-wide type domain;
- every designated parameter local exists in that function body and parameter-local designations are unique;
- every parameter and result type is call-transfer-safe;
- every direct-call target function exists;
- every direct call has exactly the target parameter count;
- each argument operand type exactly matches its corresponding target parameter-local type;
- argument operand state transitions are valid in left-to-right order;
- result-destination presence exactly matches target result/no-result structure;
- a result destination has exactly the target result type, is wholly Never-initialized at the call point, and has ordinary direct exclusive initialization authority;
- every normal call continuation block exists in the caller body;
- every return's result presence and type exactly match its enclosing function result structure; and
- result operand state effects are valid before termination.

Validation MUST NOT reject a program merely because its call graph contains a cycle or because a call might diverge or yield a defined fault.

Body-local validation diagnostics must identify enough function-local context to distinguish equal body-local identifiers belonging to different functions. That diagnostic identity is implementation evidence and does not make numeric function or block handles Core-observable.

## Deliberate boundaries

This revision does not define:

- source syntax, source name resolution, source callable identity, or source-to-Core lowering;
- cross-activation raw-pointer transfer, reference parameters, borrowed parameters, or pass modes other than owned value;
- indirect calls, function values, closures, methods, virtual dispatch, or overload resolution;
- variadics, default arguments, generics, traits, or effect-polymorphic calls;
- async functions, tasks, Exec calls, cancellation, or scheduling;
- catch targets, panic completion, exception objects, or recoverable-result conventions;
- ABI, calling convention, FFI, linkage, symbol export, physical stack layout, tail-call guarantees, or target recursion capability;
- source entry-point semantics;
- numeric operations, literal construction, host floating-point representation, or physical scalar layout;
- optimizer transformation legality; or
- a universal inter-stratum refinement proof.

Those concerns require their own accepted semantic owner or consumer before this relation is extended.
