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

1. one finite ordered sequence of owned-value parameter slots;
2. either no result value or exactly one result type; and
3. exactly one **safe-reference result contract** selected from the bounded alternatives below.

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

A **safe-reference result contract** is one semantic callable fact selected from exactly these alternatives:

- **None** — no special safe-reference result contract;
- **SharedIdentity(i)** — one identity-preserving scalar Shared-reference result whose designated origin is parameter slot `i`; or
- **SharedDirectChild(i)** — one scalar Shared-reference result whose authority is a direct complete-referent Shared child of the exclusive safe-reference authority transferred through parameter slot `i`.

These alternatives are mutually exclusive. A represented function has exactly one of them; it cannot simultaneously advertise two result-origin facts. The variant names above identify semantic alternatives and do not require a particular implementation enum, field layout, serialization, or source spelling.

A designated result-origin slot selects one parameter slot by semantic slot identity. It is not a body-local `LocalId`, a source binding identity, a lifetime name, a physical frame location, a runtime counter, or a second parameter value.

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

The ordinary result-transfer class remains reference-free and raw-pointer-free.

A represented Core type is **result-transfer-safe** exactly when its structural value shape contains neither a raw-pointer leaf nor a safe-reference leaf:

- an ordinary non-pointer, non-reference scalar leaf is result-transfer-safe;
- a raw-pointer type is not result-transfer-safe;
- a safe-reference type is not result-transfer-safe; and
- a structural aggregate is result-transfer-safe exactly when every recursively contained field type is result-transfer-safe.

A bare safe-reference type therefore does not become result-transfer-safe merely because this revision adds bounded contract-bearing Shared-reference result forms. Reference-containing aggregate types and raw-pointer-containing types also remain outside the ordinary result-transfer-safe class.

## Safe-reference result contracts

A represented function result is admissible exactly when one of the following holds:

1. the function has no result type and its safe-reference result contract is `None`;
2. the function has one result-transfer-safe result type and its safe-reference result contract is `None`;
3. the function has one scalar Shared safe-reference result type and contract `SharedIdentity(i)` satisfying the identity-preserving contract below; or
4. the function has one scalar Shared safe-reference result type and contract `SharedDirectChild(i)` satisfying the complete direct-child contract below.

A reference leaf nested inside an aggregate parameter cannot be selected indirectly as the result origin in either special result form.

### Identity-preserving Shared result

For `SharedIdentity(i)`, every one of these declaration requirements MUST hold:

- the result type is exactly one scalar safe-reference type whose permission is `Shared`;
- the designated origin slot `i` exists in the function's ordered parameter sequence;
- the designated origin parameter type is itself exactly one scalar Shared safe-reference type; and
- the designated origin parameter type is exactly equal to the function result type.

Because exact safe-reference type identity already fixes exact referent type identity and permission, the equality requirement permits neither referent mismatch nor permission strengthening/weakening.

At activation entry, after the designated argument value has been transferred into the designated parameter local, that parameter's incoming safe-reference value establishes one **activation identity result origin**: the exact semantic target region and reference-authority identity carried by that incoming value.

The activation identity result origin is a validation fact about an already existing transferred reference value. It creates no carrier, authority, target, storage, `LoanId`, allocation, physical address, or hidden runtime object.

Independent function validation treats parameter slots according to their advertised callable identities. A body cannot satisfy an origin contract naming slot `i` merely by returning slot `j` because some particular dynamic call might pass aliasing Shared arguments. The contract must hold for every admitted call, including calls where the two parameter values name distinct authorities/targets.

This contract is identity-preserving. A normal result may be carried through ordinary Shared `Copy`, `Move`, initialization, local storage, or a nested contract-bearing direct call, but the final returned carrier MUST name the exact activation identity-result-origin authority and target. A reference reborrow creates a fresh child authority and therefore does not satisfy this contract. A fresh root reference, including one targeting callee-local storage, likewise does not satisfy it.

### Complete direct-child Shared result

For `SharedDirectChild(i)`, every one of these declaration requirements MUST hold:

- the result type is exactly one scalar safe-reference type with permission `Shared` and exact referent type `T`;
- the designated origin slot `i` exists in the function's ordered parameter sequence;
- the designated origin parameter type is exactly one scalar safe-reference type with permission `Exclusive` or `ExclusiveReplace` and exact referent type `T`; and
- the designated origin parameter type remains parameter-transfer-safe under the ordinary parameter rule.

At activation entry, after the designated argument has been transferred into the designated parameter local, the incoming value establishes one **activation derived-result parent origin**: its exact semantic target region and exact reference-authority identity.

The derived-result parent origin records an existing transferred authority. It creates no carrier, child authority, reborrow, target, storage, allocation, `LoanId`, address, lifetime object, or hidden runtime object.

A normal result satisfies this contract only when the produced Shared carrier names an active authority whose target is exactly the activation derived-result parent target and whose direct parent authority identity is exactly the activation derived-result parent authority. The child must retain the complete Shared capability promised by the result type.

The relation is intentionally **direct child + complete same referent**. A structural subregion child, a grandchild or other arbitrary descendant, a fresh root, an authority descended from another parameter, or an independently rooted authority cannot satisfy this contract merely because its final type or dynamic target aliases a designated argument.

`Exclusive` and `ExclusiveReplace` are both admitted parent permissions because both carry exclusive alias authority and both permit Shared reborrow under [Core references](references.md). Replacement capability does not change Shared-child formation. A Shared parent is not admitted by this first derived form: complete-target Shared-to-Shared fresh authority identity adds no currently represented capability or target-selection power beyond `SharedIdentity`, while later structural subregion consumers may require a separate extension.

The returned Shared child is a capability downgrade through that handle, not release of the parent exclusive authority interval. Result transfer preserves the child and its existing ancestry under `references.md`; it does not detach, re-root, reparent, widen, narrow, restore, or otherwise rewrite the authority branch.

Neither special contract defines a result-origin projection, alternative-origin set, general descendant summary, authority-detachment relation, or general borrowed-effect summary.

## Dynamic function activations

Every dynamic direct call creates one fresh **Core function activation** for the target function.

Each activation has independent dynamic local-storage state and independent body-local explicit-loan declaration state for the target body. Distinct simultaneously active or recursively nested calls therefore have distinct dynamic storage instances and distinct activations of equal body-local `LoanId` declarations even when they execute the same static function.

Safe-reference-backed authority is different. A reference carrier in one activation may name an authority whose target storage belongs to a still-live suspended ancestor activation, as defined by [Core references](references.md). Such authority is not re-declared as a callee-local `LoanId` merely because the carrier crosses the call boundary.

When an activation is created, one fresh dynamic local storage instance is created for every local declaration in the target body. All of those local storage instances initially contain no initialized stored value.

Parameter transfer then initializes the designated parameter locals as defined below. The function body begins only after all parameter transfers have completed. When the target has `SharedIdentity(i)` or `SharedDirectChild(i)`, the designated transferred parameter value also establishes the corresponding activation origin fact defined above before body execution begins.

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

An admitted result destination may receive either an ordinary result-transfer-safe value or one scalar Shared-reference value authorized by the target's selected safe-reference result contract. Raw-pointer-containing results, reference-containing aggregate results, and uncontracted safe-reference results remain invalid.

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
2. transfer the transient argument values into the designated parameter locals in parameter-slot order;
3. mark each transferred parameter local initialized with the transferred value before target-body entry; and
4. when the target has `SharedIdentity(i)` or `SharedDirectChild(i)`, record the exact target/authority identity carried by the selected transferred parameter as the corresponding activation origin fact.

Parameter transfer does not duplicate a transient argument value. When the value contains safe-reference carriers, those existing carriers are transferred unchanged into the callee parameter storage; the call boundary does not create a new reference authority, reborrow, target, or carrier merely because transfer occurred.

A caller that requires a temporary borrowed-call interval may explicitly produce a child safe-reference reborrow before the call and move that child value as the ordinary argument. The retained parent carrier remains in the caller while its authority is delegated over the child's target according to [Core references](references.md). The child itself is transferable when it retains the complete capability promised by its own reference type and its complete target is fully Live. No borrowed-call pass mode is implied or required.

If such a caller-created Shared child is supplied to the parameter designated by `SharedIdentity(i)` and the callee returns that exact authority, the child does not end merely because the callee returns: the preserved result carrier keeps that same child authority active after callee cleanup. Its parent remains delegated until the returned carrier later ends under ordinary reference lifecycle rules.

The represented operand set in this revision does not itself add a new defined-fault or divergence relation for operand evaluation. Undefined behavior selected by an existing unsafe operand has no defined post-state to continue into a call. A future operand owner that adds another abnormal evaluation outcome must define its interaction with held transient call values rather than inferring that behavior from this direct-call relation.

## Reference-parameter referent state

Each safe-reference carrier transferred into a parameter denotes one **external referent domain** for that activation: the carrier's existing target storage region in a still-live activation plus the exact referent type and authority already carried by the value.

At callee body entry, every such external referent domain is fully Live by call admission. The callee validates operations on that domain using the ordinary reference permissions, structural path-state rules, and stored-value lifecycle. This is an abstract semantic storage domain for function validation; it is not a new parameter slot, `LoanId`, allocation, physical stack location, or hidden runtime copy of the caller's storage.

The combined alias-authority law guarantees that an Exclusive or ExclusiveReplace external referent target does not overlap another independently active authority at call entry. Shared external referent targets may overlap one another, but Shared operations cannot make a target vacant; legal `InteriorAssign` may change the semantic value while leaving the complete target fully Live.

Because each external referent type is reference-parameter-referent-safe, callee mutation of the target cannot create, destroy, replace, or transport a nested safe-reference carrier or raw-pointer provenance fact inside suspended caller storage. The caller-visible post-call path-state contract therefore needs to preserve structural liveness, not an unexpressed nested reference/provenance graph. The bounded scalar Shared-reference result contracts separately preserve either one already transferred top-level Shared authority identity or one direct complete-referent Shared child of one transferred exclusive authority; neither permits extracting a nested reference from the referent.

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

For an ordinary result-transfer-safe result, the preserved result value remains free of safe-reference and raw-pointer leaves.

For a contract-bearing scalar Shared-reference result, the result value contains exactly one Shared-reference carrier of the declared type, and that carrier is additionally constrained by the selected safe-reference result contract.

For any result-bearing return:

1. evaluate the result operand completely under its existing operand semantics;
2. when the function has `SharedIdentity(i)`, require the produced result carrier to name the activation's exact identity-result-origin authority and target, and require that authority to remain active with complete Shared capability;
3. when the function has `SharedDirectChild(i)`, require the produced result carrier to name an active Shared authority whose exact target equals the activation derived-result parent target, whose direct parent authority identity equals the activation derived-result parent authority identity, and whose capability is complete Shared;
4. require every external referent domain introduced by safe-reference parameters to be fully Live after those result-operand effects;
5. preserve the successfully produced owned result value, including any permitted result carrier, outside the activation-local cleanup set;
6. perform termination explicit-loan handling, reference-carrier cleanup consequences, and local cleanup for the current activation under the existing Core borrowing, reference, and value/storage rules;
7. require every local storage extent that ends during that cleanup to satisfy the safe-reference storage-extent validity relation from `references.md`;
8. terminate the current activation normally; and
9. transfer the preserved owned result value to the suspended caller without duplication.

Under `SharedIdentity`, a Shared result produced by ordinary Shared `Copy` may be a different carrier from the carrier originally transferred into the designated parameter while still naming the same authority and target. A result produced by `Move` may transport an existing carrier through one or more callee locals. Neither operation creates a new authority. A reborrow remains invalid under this identity contract because reborrow creates a fresh child authority.

Under `SharedDirectChild`, one direct complete-referent Shared reborrow from the designated `Exclusive` or `ExclusiveReplace` origin is the admitted derivation. The result may be transported through ordinary Shared value operations or an exact identity-preserving nested call after that child exists, provided the final returned authority still has the required direct-parent identity and exact target. A second derivation from that child creates a grandchild and does not satisfy the advertised direct-child contract for the original parent.

A fresh root reference, including one targeting callee-local storage, satisfies neither special result contract.

For a no-result return, require the same fully-Live external-referent postcondition before performing the same activation termination relation, but produce no Core value.

When the caller expects a result, successful result transfer initializes the previously admitted vacant result destination without replacement destruction and normal execution resumes at the call's normal continuation block.

When the caller expects no result, normal execution resumes at the normal continuation block without producing or discarding a hidden value.

The external target values observed by the resumed caller are the actual values left by callee execution; the normal-return postcondition guarantees their complete structural liveness, not equality with the pre-call values.

A moved return source is already Dead before termination cleanup and therefore is not destroyed again by that cleanup.

A safe-reference value may escape through a normal result only when it satisfies the function's selected safe-reference result contract. An unpreserved temporary reborrow follows ordinary callee cleanup: when its remaining callee-owned carriers and descendants end, its authority ends and delegated parent capability is restored according to `references.md`. A preserved direct-child result instead keeps its parent authority delegated after Return for as long as that child branch remains active.

The outer consumer of a represented Core function execution receives the same optional result structure: no-result normal completion yields no value, an ordinary result-bearing normal completion yields the preserved owned result value, and a contract-bearing Shared-reference result preserves the exact authority/target/ancestry relation guaranteed by the selected callable contract. This fact defines Core execution structure and does not establish source entry-point semantics.

## Contract-bearing Shared-reference result at the caller

### Identity-preserving result

For a target with `SharedIdentity(i)`, let the successfully admitted held argument for slot `i` contain Shared carrier `C` naming target `R` and authority `A`.

The callable contract guarantees that every normal result from that activation contains one Shared carrier naming exactly `R` and `A`. Dynamic result transfer preserves the carrier produced by the callee's return operand; the call boundary does not create a fresh authority, target, reborrow, or additional carrier merely because the result crosses activations.

Independent caller path-state validation MUST preserve that guarantee without expanding the callee body. For the normal continuation it accounts for one surviving result carrier naming `R` and `A` before the transferred/held argument carriers are given their ordinary end-of-call cleanup consequences. It then:

1. applies the existing fully-Live normal-return summary to every transferred external referent domain;
2. removes the transient call carriers that do not survive as caller values under the ordinary carrier lifecycle;
3. initializes the previously admitted result destination with the preserved result carrier, without replacement destruction or authority creation; and
4. continues at the target's normal continuation block.

This summary describes the net guaranteed carrier state; it does not assert that the call boundary performed a Shared `Copy`. The actual result carrier was produced by ordinary callee execution and preserved across cleanup.

If the caller retained another Shared carrier for authority `A` before the call, that carrier and the returned carrier coexist normally. If `C` was the only carrier before transfer, the returned carrier keeps `A` active after callee cleanup. If `A` is itself a child authority created by the caller, preserving the returned carrier keeps that child active and its parent remains delegated until the result carrier later ends.

A nested or recursive identity-contract call composes by the same rule. If a callee's selected argument names authority `A`, its normal result is known from callable structure alone to name the same `A`; an enclosing `SharedIdentity` function may therefore forward that result when `A` is its own advertised activation identity result origin. Call-graph expansion or fixed-point inference is unnecessary.

### Complete direct-child result

For a target with `SharedDirectChild(i)`, let the successfully admitted held argument for slot `i` contain one `Exclusive` or `ExclusiveReplace` carrier naming target `R` and authority `A`.

The callable contract guarantees that every normal result from that activation contains one Shared carrier naming a callee-created authority `C` such that:

- `C` targets exactly `R`;
- `C` has permission `Shared`; and
- the direct parent of `C` is exactly `A`.

The child authority is fresh by the ordinary reborrow relation that created it. Dynamic result transfer preserves the produced child carrier; the call boundary does not create another authority, target, reborrow, or carrier and does not detach or re-root `C`.

Independent caller path-state validation MUST preserve this finite guaranteed relation without expanding the callee body. For the normal continuation it accounts for the surviving result carrier `C` and direct parent edge `A -> C` before applying the ordinary end-of-call carrier consequences, then:

1. applies the existing fully-Live normal-return summary to every transferred external referent domain;
2. removes the transferred/held argument carriers that do not survive as caller values under the ordinary carrier lifecycle;
3. keeps `A` active carrierlessly when removal of its transferred parameter carrier would otherwise leave it without a carrier, because `C` remains its active child;
4. preserves every pre-existing ancestor of `A` through the unchanged authority chain;
5. initializes the previously admitted result destination with the preserved `C` carrier, without replacement destruction, authority creation, or parent-carrier synthesis; and
6. continues at the target's normal continuation block.

Because `A` is an active exclusive authority while `C` survives, direct/root alias checks after caller continuation continue to observe that exclusive ancestor under the common conflict law from `borrowing.md`. The returned Shared child therefore weakens capability through the returned handle but does not release the original exclusive authority interval. When `C` and any descendants later end, carrierless ancestors that then have no active child end transitively under the ordinary reference lifecycle.

If `A` was itself a child authority with retained ancestor `P`, the returned branch is `P -> A -> C`; result transfer does not skip `A` or attach `C` directly to `P`.

Nested composition remains finite. An already-derived Shared child `C` may pass through `SharedIdentity` unchanged. A nested `SharedDirectChild` call receiving exact exclusive origin `A` may produce `C`, and an enclosing `SharedDirectChild` function advertising the same exact origin `A` may forward that `C` unchanged because its direct parent remains `A`. Deriving again from `C` produces a grandchild and does not satisfy an outer contract advertising `A` in this slice.

Direct recursion and mutual recursion consume these same callable summaries. The direct-child relation is checked against the concrete transferred origin of each activation; no call-graph expansion, ancestry fixed point, or body-derived public contract is required.

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

The caller result destination is not initialized on the fault path. A safe-reference result contract creates or preserves no result carrier on that path.

Reference-backed authorities that span multiple still-live activations remain governed by their carriers and descendants during this outward cleanup. Removal of a callee's final temporary-reborrow carrier may restore parent authority before the next caller itself terminates; no hidden catch or synthetic reference-result boundary is introduced.

This semantic propagation does not require physical stack unwinding and does not define a panic payload, exception object, backtrace, catch construct, recovery boundary, or physical unwinding mechanism.

Undefined behavior remains distinct from defined fault. Detection of undefined behavior does not trigger this defined-fault cleanup relation.

## Divergence

If a called activation diverges, its caller remains suspended at that call.

Divergence does not follow the normal continuation, does not initialize a result destination, does not produce a safe-reference result carrier, and does not trigger return or fault cleanup merely because execution continues indefinitely.

Caller storage extents, body-local explicit loans, live reference carriers, external referent domains, and reference-backed authorities therefore continue while the caller remains suspended. A diverging callee may continue to access an ancestor target through a valid reference according to the ordinary reference relation.

There is no implicit fully-Live postcondition or execution-step limit on a path that never returns normally.

## Termination cleanup boundary

This document selects which activation terminates and when result or fault transfer occurs. It does not redefine the destruction domain, local cleanup order, or reference-carrier destruction consequence.

For every normally returning or defined-faulting activation, local cleanup remains the reverse local declaration order owned by `value-storage.md`, using the then-current initialization state and skipping Never-initialized or Dead values as already specified there. Safe-reference leaves reached by that ordinary destruction order lose their carriers under `references.md`.

A parameter local participates in that same local declaration order. A compiler refining another language-level cleanup relation is responsible for choosing Core local declaration order that preserves the required higher-level ordering; Core parameter-slot order does not silently replace the existing local cleanup owner.

Before each local storage extent ends after its cleanup, the reference owner additionally requires that no surviving safe-reference authority in the active call stack still targets that storage instance. This is a validity condition on the represented program, not a second cleanup order.

A preserved contract-bearing Shared-reference result carrier is outside the callee activation-local cleanup set. Under `SharedIdentity`, its callable origin guarantees a still-live external target directly. Under `SharedDirectChild`, its exact target equals the designated incoming external target and its parent is the designated incoming external authority. In either case the preserved result therefore does not depend on callee-local target storage. It may keep its authority and, for the direct-child variant, a now-carrierless parent authority active after other callee-local carriers have been destroyed without violating callee storage-extent ending.

The external referent domains targeted through safe-reference parameters belong to still-live ancestor storage extents rather than this activation's local cleanup set. Normal Return checks their fully-Live postcondition before local cleanup; defined Fault propagation does not restore them merely for cleanup.

## Validation requirements

A represented Core program is language-valid under this relation only when all of the following hold in addition to existing body validity:

- every represented function body and callable type references the program-wide type domain;
- every designated parameter local exists in that function body and parameter-local designations are unique;
- every parameter type is parameter-transfer-safe;
- every function result is either absent with contract `None`, one ordinary result-transfer-safe type with contract `None`, or one scalar Shared-reference type with exactly one valid special safe-reference result contract;
- every `SharedIdentity(i)` slot exists, designates one scalar Shared-reference parameter, and has exact type equality with the result;
- every `SharedDirectChild(i)` slot exists, designates one scalar `Exclusive` or `ExclusiveReplace` reference parameter, and has exact referent-type equality with the scalar Shared result;
- every direct-call target function exists;
- every direct call has exactly the target parameter count;
- each argument operand type exactly matches its corresponding target parameter-local type;
- argument operand state transitions, including safe-reference carrier/authority transitions, are valid in left-to-right order;
- after all argument operands complete, every recursively contained safe-reference carrier has complete advertised authority and a fully-Live complete target before callee activation creation;
- function-body validation treats each transferred safe-reference target as a fully-Live external referent domain with its exact reference permission;
- function-body validation remembers the exact target/authority carried by the designated parameter when a special safe-reference result contract exists;
- every normal Return proves every external referent domain fully Live after result-operand effects and before cleanup;
- every `SharedIdentity` Return additionally proves exact designated activation-origin authority/target identity and complete Shared capability before cleanup;
- every `SharedDirectChild` Return additionally proves exact designated target equality, direct-parent authority identity, and complete Shared capability before cleanup;
- a caller normal continuation treats transferred referent regions as fully Live with their actual callee-produced non-reference/non-pointer values, while validation may conservatively forget value identity;
- a caller normal continuation for `SharedIdentity` preserves one result carrier for the exact authority/target of the designated admitted argument before transient call-carrier cleanup and initializes the admitted destination with that carrier;
- a caller normal continuation for `SharedDirectChild` preserves one callee-created Shared result authority with exact target equal to the designated argument target and direct parent equal to the designated argument authority, preserving that ancestry through transient call-carrier cleanup before initializing the admitted destination;
- result-destination presence exactly matches target result/no-result structure;
- a result destination has exactly the target result type, is wholly vacant at the call point, and has ordinary direct exclusive initialization authority;
- every normal call continuation block exists in the caller body;
- every return's result presence and type exactly match its enclosing function result structure;
- result operand state effects are valid before termination;
- no defined-fault or diverging path receives a synthetic result carrier or normal-continuation state; and
- every ending activation/local storage extent satisfies the safe-reference storage-extent validity relation.

Validation MUST NOT reject a program merely because its call graph contains a cycle or because a call might diverge or yield a defined fault.

Reference parameter transfer does not require recursive expansion of the callee body into each caller. The call-entry authority/liveness invariants establish the callee's bounded external-domain entry state, the mandatory fully-Live normal-return postcondition establishes the caller's bounded referent continuation state, and the selected special result contract establishes either one identity-preserving Shared returned authority or one direct complete-referent Shared child of one designated exclusive origin. Reference-containing aggregate results, structural-subregion results, arbitrary-descendant result contracts, Exclusive/ExclusiveReplace results, nested-reference referent transfer, raw-pointer results, and richer borrowed effects remain forbidden because this revision does not define their required callable origin/effect contracts.

Body-local validation diagnostics must identify enough function-local context to distinguish equal body-local identifiers belonging to different functions. That diagnostic identity is implementation evidence and does not make numeric function or block handles Core-observable.

## Deliberate boundaries

This revision does not define:

- source syntax, source name resolution, source callable identity, source references/lifetimes, or source-to-Core lowering;
- cross-activation raw-pointer transfer or pointer escape;
- safe-reference-containing aggregate results;
- Shared-origin complete-referent fresh-child result derivation;
- Shared-reference result origins selecting structural subregions or projections;
- arbitrary descendant or grandchild result-origin relations;
- another origin parameter, multiple alternative result origins, or generic callable borrow/effect summaries;
- authority detachment, re-rooting, reparenting, or result-origin projection rewriting;
- Exclusive or ExclusiveReplace reference results;
- parameter transfer through a safe reference whose referent structural value contains another safe-reference or raw-pointer leaf;
- callable borrowed-effect summaries beyond the fixed fully-Live entry/normal-return referent-state contract and the two bounded scalar Shared result contracts above;
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

The admitted direct-child result deliberately preserves the complete authority ancestry selected by ordinary reference reborrow. A carrierless exclusive parent may therefore remain active and continue to constrain direct/root access while the returned Shared child survives. This is a capability downgrade through the returned handle, not an implicit release or shortening of the exclusive authority interval. When the final descendant branch ends, childless carrierless ancestors terminate transitively under the existing reference lifecycle.

Broader structural-subregion or arbitrary-descendant results require a later consumer to define the exact finite origin/effect relation needed by independent caller validation. This revision does not infer such a relation and introduces no authority detachment or re-rooting mechanism.

Those concerns require their own accepted semantic owner or consumer before this relation is extended.
