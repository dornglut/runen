# Core Functions and Direct Calls

Status: **provisional normative; incomplete**

This document owns the currently represented Core semantics for finite function programs, function identity, owned-value parameter slots, direct calls, dynamic function activations, result transfer, recursion, divergence, and defined-fault propagation through direct calls.

It consumes local storage, initialization state, owned move/copy, destruction domains, and function-termination cleanup from [Core value and storage semantics](value-storage.md); access authority and explicit-loan termination from [Core borrowing](borrowing.md); safe-reference values, reference-backed authority, carrier lifecycle, and reference access from [Core references](references.md); raw-pointer value and provenance semantics from [Core pointers and provenance](pointers.md); and defined-fault classification from [Core faults](faults.md). It does not redefine those owners.

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

A designated parameter local is otherwise an ordinary Core local. Its assignment-mutability property, structural type, storage extent, stored-value lifetime, borrow interactions, reference-carrier contents, and termination cleanup follow the existing Core owners.

The parameter sequence may be empty.

The result specification is exactly one of:

- no result value; or
- one result value of exactly one represented Core type.

No-result structure does not introduce Unit, Void, or another Core value type.

Parameter slots remain ordinary owned-value slots even when their types contain safe references. This revision introduces no separate borrowed-parameter slot or call pass-mode category.

## Parameter-transfer-safe types

The represented Core direct-call relation permits a bounded class of safe-reference-containing values to cross **into parameters**, but it continues to prohibit cross-activation raw-pointer transfer and does not yet transfer references whose referent storage itself contains raw-pointer or safe-reference values.

Define a Core type as **reference-parameter-referent-safe** exactly as follows:

- an ordinary non-pointer, non-reference scalar leaf is reference-parameter-referent-safe;
- a raw-pointer type is not reference-parameter-referent-safe;
- a safe-reference type is not reference-parameter-referent-safe; and
- a structural aggregate is reference-parameter-referent-safe exactly when every recursively structurally contained field type is reference-parameter-referent-safe.

A represented Core type is **parameter-transfer-safe** exactly as follows:

- an ordinary non-pointer, non-reference scalar leaf is parameter-transfer-safe;
- a raw-pointer type is not parameter-transfer-safe;
- a safe-reference type is parameter-transfer-safe exactly when its exact referent type is reference-parameter-referent-safe; and
- a structural aggregate is parameter-transfer-safe exactly when every recursively structurally contained field type is parameter-transfer-safe.

The parameter value may therefore itself be a safe reference or an aggregate containing multiple safe-reference leaves. The restriction applies to the storage reached **through** each transferred reference: that referent structural value contains neither a raw-pointer leaf nor another safe-reference leaf in this first slice.

This is a call-transfer restriction only. It does not prohibit general Core safe-reference types from referring to reference-containing or raw-pointer-containing types inside one activation when another accepted operation permits such a reference.

The restriction is semantically required for independent function validation. A callee receiving a reference to raw-pointer-containing storage could otherwise obtain a raw-pointer value from suspended ancestor storage, silently creating the cross-activation raw-pointer relation that this function owner does not define. A callee receiving a reference to safe-reference-containing storage could move, replace, copy, or otherwise change reference carriers/authority identities in suspended ancestor storage, including creating a callee-local reference escape path, without a callable authority/effect contract capable of describing the caller's resulting state.

Every represented function parameter type MUST be parameter-transfer-safe.

Raw-pointer locals, raw-pointer operations, and safe references with richer referent types remain permitted inside one activation under their existing owners. The parameter-transfer restriction therefore does not weaken or redefine those intra-activation semantics.

## Result-transfer-safe types

The first safe-reference call slice remains deliberately narrower for results.

A represented Core type is **result-transfer-safe** exactly when its structural value shape contains neither a raw-pointer leaf nor a safe-reference leaf:

- an ordinary non-pointer, non-reference scalar leaf is result-transfer-safe;
- a raw-pointer type is not result-transfer-safe;
- a safe-reference type is not result-transfer-safe; and
- a structural aggregate is result-transfer-safe exactly when every recursively contained field type is result-transfer-safe.

Every represented function result type MUST be result-transfer-safe.

Safe-reference results are excluded because a bare result type does not identify which caller-origin authority/storage a result may derive from, which structural subregion is returned, or whether a candidate result improperly targets callee-local storage. This Core relation validates function bodies independently and currently has no callable borrow-origin/result contract. Reference-result semantics therefore require a later accepted contract rather than inference from implementation dataflow or runtime execution.

## Dynamic function activations

Every dynamic direct call creates one fresh **Core function activation** for the target function.

Each activation has independent dynamic local-storage state and independent body-local explicit-loan declaration state for the target body. Distinct simultaneously active or recursively nested calls therefore have distinct dynamic storage instances and distinct activations of equal body-local `LoanId` declarations even when they execute the same static function.

Safe-reference-backed authority is different. A reference carrier in one activation may name an authority whose target storage belongs to a still-live suspended ancestor activation, as defined by [Core references](references.md). Such authority is not re-declared as a callee-local `LoanId` merely because the carrier crosses the call boundary.

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

The result destination is a non-replacing initialization destination, not an assignment or replacement destination. At the call point:

- the destination MUST be wholly vacant under `value-storage.md`; and
- direct access to that destination MUST have the same exclusive access authority required by ordinary `Init`.

The containing local need not be mutable merely because the call may initialize the destination.

The destination admission facts are established for the call point before argument operand state transitions are applied. Argument evaluation cannot make an initially Live destination admissible to that same call merely by moving or destroying its prior value.

A successful result return initializes the admitted vacant destination without replacement destruction and begins new stored-value lifetimes there before control continues at the normal target. Those lifetimes may be the first or later lifetimes in the same continuing destination storage extent.

A faulting or diverging callee does not initialize the destination and does not follow the normal target.

Because result types are result-transfer-safe, the admitted destination cannot receive a raw-pointer-containing or safe-reference-containing result in this revision.

## Argument evaluation and parameter transfer

Argument count MUST equal the target parameter count exactly.

Each argument operand MUST produce one owned Core value whose Core type is exactly equal to the corresponding parameter-slot type. This revision introduces no implicit conversion, widening, narrowing, coercion, subtyping, or defaulting relation.

Arguments are evaluated strictly left to right in argument/parameter-slot order.

Each successfully evaluated argument value is held as one owned transient call value until all represented argument operands have evaluated successfully. Producing a transient call value does not itself require an addressable Core local storage place.

Existing operand semantics determine each argument's state effects. In particular, `Move` consumes its source stored value and `Copy` preserves its source stored value. Safe-reference carrier effects of Move and Shared-reference Copy are owned by [Core references](references.md).

After every argument operand has evaluated successfully and before callee activation creation, every safe-reference carrier recursively contained in the held argument values MUST satisfy both of these call-entry requirements in the final caller state:

1. its authority currently retains the complete alias/replacement capability promised by its exact reference permission over its complete target region; and
2. its complete target region is fully Live.

The requirements are evaluated after all left-to-right argument effects. A later argument may therefore change descendant authority state or target liveness relevant to an earlier held reference argument; final call admission observes that resulting state rather than the state at the instant the earlier operand was produced.

Under the reference delegation relation:

- a Shared carrier satisfies the authority requirement while Shared child authorities are active because the parent retains shared authority over overlapping storage;
- an Exclusive or ExclusiveReplace carrier fails the authority requirement while any active child authority delegates any part of its target, because the parent no longer retains complete exclusive authority over the complete referent;
- a child Exclusive or ExclusiveReplace reborrow whose own authority has no active child may satisfy the authority requirement even while its parent remains active in the caller; and
- a reference-containing aggregate satisfies call-entry admission only when every recursively contained reference carrier satisfies both requirements.

The fully-Live requirement is independent of authority. An Exclusive or ExclusiveReplace reference that validly continues to denote vacant storage after an earlier Move or Drop cannot cross this call boundary until its complete target has been legally restored to fully Live state.

These entry invariants ensure that an independently validated callee may rely on the exact Shared, Exclusive, or ExclusiveReplace capability stated by each reference parameter and on a fully-Live initial referent state without depending on hidden caller-side path or descendant state. They do not create a new reference authority, end an existing child, reinitialize storage, or add a borrowed-call pass mode.

After call-entry admission succeeds:

1. create one fresh activation of the target function;
2. transfer the transient argument values into the designated parameter locals in parameter-slot order; and
3. mark each transferred parameter local initialized with the transferred value before target-body entry.

Parameter transfer does not duplicate a transient argument value. When the value contains safe-reference carriers, those existing carriers are transferred unchanged into the callee parameter storage; the call boundary does not create a new reference authority, reborrow, target, or carrier merely because transfer occurred.

A caller that requires a temporary borrowed-call interval may explicitly produce a child safe-reference reborrow before the call and move that child value as the ordinary argument. The retained parent carrier remains in the caller while its authority is delegated over the child's target according to [Core references](references.md). The child itself is transferable when it retains the complete capability promised by its own reference type and its complete target is fully Live. No borrowed-call pass mode is implied or required.

The represented operand set in this revision does not itself add a new defined-fault or divergence relation for operand evaluation. Undefined behavior selected by an existing unsafe operand has no defined post-state to continue into a call. A future operand owner that adds another abnormal evaluation outcome must define its interaction with held transient call values rather than inferring that behavior from this direct-call relation.

## Reference-parameter referent state

Each safe-reference carrier transferred into a parameter denotes one **external referent domain** for that activation: the carrier's existing target storage region in a still-live activation plus the exact referent type and authority already carried by the value.

At callee body entry, every such external referent domain is fully Live by call admission. The callee validates operations on that domain using the ordinary reference permissions, structural path-state rules, and stored-value lifecycle. This is an abstract semantic storage domain for function validation; it is not a new parameter slot, `LoanId`, allocation, physical stack location, or hidden runtime copy of the caller's storage.

The combined alias-authority law guarantees that an Exclusive or ExclusiveReplace external referent target does not overlap another independently active authority at call entry. Shared external referent targets may overlap one another, but Shared operations cannot make a target vacant; legal `InteriorAssign` may change the semantic value while leaving the complete target fully Live.

Because each external referent type is reference-parameter-referent-safe, callee mutation of the target cannot create, destroy, replace, or transport a nested safe-reference carrier or raw-pointer provenance fact inside suspended caller storage. The caller-visible post-call path-state contract therefore needs to preserve structural liveness, not an unexpressed reference/provenance identity graph.

Every normal `Return` from a function with one or more safe-reference parameter carriers MUST leave the complete target region of every such external referent domain fully Live after any return-result operand effects and before activation cleanup begins.

This is the bounded normal-return referent-state postcondition of the first transfer slice. It is not a source-visible effect annotation or user-selectable callable dimension. A function may temporarily Move or Drop from an Exclusive or ExclusiveReplace external target only when every normally returning path legally restores the complete target to fully Live state before Return. Paths that yield defined Fault or diverge have no normal continuation and therefore do not satisfy this postcondition merely to terminate abnormally or remain suspended.

At a caller's normal continuation, each transferred external referent region is therefore known to be fully Live again. Its actual semantic value is exactly the value left by callee execution; path-state validation may conservatively forget non-reference/non-pointer value identity inside the target while retaining fully-Live structure. Storage disjoint from every transferred referent target preserves the caller state it had after argument evaluation.

The postcondition adds no synthetic write at return. It constrains the state produced by ordinary callee operations.

## Caller suspension

After successful argument evaluation and callee activation creation, the caller activation is suspended at the call.

Suspension performs no caller execution, cleanup, storage-extent ending, or explicit-loan termination. The caller's body-local explicit-loan activation state remains live and continues to constrain overlapping access under [Core borrowing](borrowing.md).

Caller storage is **not** generally frozen during suspension once safe-reference parameters exist. A callee that holds a valid safe-reference carrier targeting still-live caller storage may read, move, drop, interior-replace, or ordinarily replace the selected caller region exactly when that reference permission and the independently owned operation requirements authorize it. Those state changes are changes to the continuing caller storage instances even though the caller itself remains suspended.

Likewise, reference-backed authority spanning the call may change through callee reborrows and carrier destruction. Such changes are governed by [Core references](references.md), not by a hidden call-side alias rule.

Because every safe-reference parameter referent is reference-parameter-referent-safe, the callee receives no safe-reference access path through which a raw-pointer or nested safe-reference value can be extracted from suspended caller storage. This relation therefore creates neither a transferred raw-pointer path nor an uncontracted nested-reference escape path to suspended caller storage.

A semantic safe-reference target in caller storage does not imply a physical stack address or physical stack-frame representation.

## Normal return

A represented return contains either no result operand or exactly one result operand.

A function with no result type MUST return with no result operand.

A function with one result type MUST return with exactly one owned result operand whose Core type is exactly equal to that result type.

Because every result type is result-transfer-safe, the preserved result value contains no safe-reference or raw-pointer leaf.

For a result-bearing return:

1. evaluate the result operand completely under its existing operand semantics;
2. require every external referent domain introduced by safe-reference parameters to be fully Live after those result-operand effects;
3. preserve the successfully produced owned result value outside the activation-local cleanup set;
4. perform termination explicit-loan handling, reference-carrier cleanup consequences, and local cleanup for the current activation under the existing Core borrowing, reference, and value/storage rules;
5. require every local storage extent that ends during that cleanup to satisfy the safe-reference storage-extent validity relation from `references.md`;
6. terminate the current activation normally; and
7. transfer the preserved owned result value to the suspended caller without duplication.

For a no-result return, require the same fully-Live external-referent postcondition before performing the same activation termination relation, but produce no Core value.

When the caller expects a result, successful result transfer initializes the previously admitted vacant result destination without replacement destruction and normal execution resumes at the call's normal continuation block.

When the caller expects no result, normal execution resumes at the normal continuation block without producing or discarding a hidden value.

The external target values observed by the resumed caller are the actual values left by callee execution; the normal-return postcondition guarantees their complete structural liveness, not equality with the pre-call values.

A moved return source is already Dead before termination cleanup and therefore is not destroyed again by that cleanup.

A temporary reference reborrow moved into the callee cannot escape through the result in this revision. If its final carrier remains in a callee local, ordinary cleanup removes that carrier; when no descendant remains, the child authority ends and delegated parent authority is restored before the caller resumes.

The outer consumer of a represented Core function execution receives the same optional result structure: no-result normal completion yields no value, while result-bearing normal completion yields the preserved owned result value. This fact defines Core execution structure and does not establish source entry-point semantics.

## Defined-fault propagation through calls

The represented direct-call subset has no catch boundary.

When a called activation terminates with defined fault `F`:

1. that activation performs its ordinary defined-fault termination handling and cleanup under the existing Core owners, including reference-carrier removal and reference-authority termination consequences;
2. every ending local storage extent satisfies the safe-reference storage-extent validity relation before that extent ends;
3. the suspended caller's direct-call evaluation yields the same defined fault `F` instead of following its normal continuation;
4. the caller performs its own defined-fault termination handling and cleanup; and
5. the same `F` continues outward through each suspended direct caller.

Propagation therefore preserves the selected defined-fault outcome while cleaning each terminated activation exactly once.

No fully-Live external-referent normal-return postcondition is required on the fault path because the call has no normal continuation and the same fault immediately propagates through the suspended caller.

The caller result destination is not initialized on the fault path.

Reference-backed authorities that span multiple still-live activations remain governed by their carriers and descendants during this outward cleanup. Removal of a callee's final temporary-reborrow carrier may restore parent authority before the next caller itself terminates; no hidden catch or reference-return boundary is introduced.

This semantic propagation does not require physical stack unwinding and does not define a panic payload, exception object, backtrace, catch construct, recovery boundary, or physical unwinding mechanism.

Undefined behavior remains distinct from defined fault. Detection of undefined behavior does not trigger this defined-fault cleanup relation.

## Divergence

If a called activation diverges, its caller remains suspended at that call.

Divergence does not follow the normal continuation, does not initialize a result destination, and does not trigger return or fault cleanup merely because execution continues indefinitely.

Caller storage extents, body-local explicit loans, live reference carriers, external referent domains, and reference-backed authorities therefore continue while the caller remains suspended. A diverging callee may continue to access an ancestor target through a valid reference according to the ordinary reference relation.

There is no implicit fully-Live postcondition or execution-step limit on a path that never returns normally.

## Termination cleanup boundary

This document selects which activation terminates and when result or fault transfer occurs. It does not redefine the destruction domain, local cleanup order, or reference-carrier destruction consequence.

For every normally returning or defined-faulting activation, local cleanup remains the reverse local declaration order owned by `value-storage.md`, using the then-current initialization state and skipping Never-initialized or Dead values as already specified there. Safe-reference leaves reached by that ordinary destruction order lose their carriers under `references.md`.

A parameter local participates in that same local declaration order. A compiler refining another language-level cleanup relation is responsible for choosing Core local declaration order that preserves the required higher-level ordering; Core parameter-slot order does not silently replace the existing local cleanup owner.

Before each local storage extent ends after its cleanup, the reference owner additionally requires that no surviving safe-reference authority in the active call stack still targets that storage instance. This is a validity condition on the represented program, not a second cleanup order.

The external referent domains targeted through safe-reference parameters belong to still-live ancestor storage extents rather than this activation's local cleanup set. Normal Return checks their fully-Live postcondition before local cleanup; defined Fault propagation does not restore them merely for cleanup.

## Validation requirements

A represented Core program is language-valid under this relation only when all of the following hold in addition to existing body validity:

- every represented function body and callable type references the program-wide type domain;
- every designated parameter local exists in that function body and parameter-local designations are unique;
- every parameter type is parameter-transfer-safe;
- every result type is result-transfer-safe;
- every direct-call target function exists;
- every direct call has exactly the target parameter count;
- each argument operand type exactly matches its corresponding target parameter-local type;
- argument operand state transitions, including safe-reference carrier/authority transitions, are valid in left-to-right order;
- after all argument operands complete, every recursively contained safe-reference carrier has complete advertised authority and a fully-Live complete target before callee activation creation;
- function-body validation treats each transferred safe-reference target as a fully-Live external referent domain with its exact reference permission;
- every normal Return proves every such external referent domain fully Live after result-operand effects and before cleanup;
- a caller normal continuation treats transferred referent regions as fully Live with their actual callee-produced non-reference/non-pointer values, while validation may conservatively forget value identity;
- result-destination presence exactly matches target result/no-result structure;
- a result destination has exactly the target result type, is wholly vacant at the call point, and has ordinary direct exclusive initialization authority;
- every normal call continuation block exists in the caller body;
- every return's result presence and type exactly match its enclosing function result structure;
- result operand state effects are valid before termination; and
- every ending activation/local storage extent satisfies the safe-reference storage-extent validity relation.

Validation MUST NOT reject a program merely because its call graph contains a cycle or because a call might diverge or yield a defined fault.

Reference parameter transfer does not require recursive expansion of the callee body into each caller. The call-entry authority/liveness invariants establish the callee's bounded external-domain entry state, and the mandatory fully-Live normal-return postcondition establishes the caller's bounded continuation state. Reference-containing results, nested-reference referent transfer, and richer borrowed effects remain forbidden precisely because this revision has no callable origin/effect contract for them.

Body-local validation diagnostics must identify enough function-local context to distinguish equal body-local identifiers belonging to different functions. That diagnostic identity is implementation evidence and does not make numeric function or block handles Core-observable.

## Deliberate boundaries

This revision does not define:

- source syntax, source name resolution, source callable identity, source references/lifetimes, or source-to-Core lowering;
- cross-activation raw-pointer transfer or pointer escape;
- safe-reference-containing results or callable borrow-origin/result contracts;
- parameter transfer through a safe reference whose referent structural value contains another safe-reference or raw-pointer leaf;
- callable borrowed-effect summaries beyond the fixed fully-Live entry/normal-return referent-state contract above;
- a borrowed/reference parameter pass mode distinct from ordinary owned-value transfer;
- indirect calls, function values, closures, methods, virtual dispatch, or overload resolution;
- variadics, default arguments, generics, traits, or effect-polymorphic calls;
- async functions, tasks, Exec calls, cancellation, or scheduling;
- catch targets, panic completion, exception objects, or recoverable-result conventions;
- ABI, calling convention, FFI, linkage, symbol export, physical stack layout, tail-call guarantees, or target recursion capability;
- physical reference representation, address stability, relocation, or pinning;
- source entry-point semantics;
- numeric operations, literal construction, host floating-point representation, or physical scalar layout;
- optimizer transformation legality; or
- a universal inter-stratum refinement proof.

Those concerns require their own accepted semantic owner or consumer before this relation is extended.
