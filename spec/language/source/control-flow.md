# Source Control Flow

Status: **provisional normative; incomplete**

This document owns the represented source semantics for statement-level conditional and bounded-loop control flow: condition admission and selection, validation of represented outcomes, explicit child lexical-scope composition, conditional omitted-else behavior, conditional local normal-continuation composition, bounded-`while` backedge admission, bounded unlabeled `break`/`continue` target and target-state admission, definite structural ownership for binding roots and replacement-capable external referent roots, exact raw-pointer-origin provenance at represented normal successors, and the source-to-Core control-flow refinement boundary.

It consumes represented source type identity and the intrinsic `Bool` type from [Source type foundation](types.md); producer-backed field-value result typing and validation from [Source field-value access](field-access.md); owned-value producers, producer evaluation, lexical-block execution, local normal-continuation presence, return execution, explicit-fault execution, loop-transfer cleanup, lexical cleanup, defined-fault propagation, bounded-loop body execution sequencing, and divergence from [Source function execution](function-execution.md); binding identity, lexical scope, lookup, assignment mutability, binding structural lifecycle, and raw-pointer local integration from [Source function-local bindings](local-bindings.md); structural ownership state from [Source structural ownership](structural-ownership.md); replacement-capable external referent structural roots plus sequential safe-reference authority/carrier state from [Source safe references](references.md); exact raw-pointer origin provenance and lexical target validity from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md); represented Core Bool branching and CFG path-state validity from [Core control flow](../core/control-flow.md); and vacant non-replacing initialization/result-destination admission from [Core value and storage semantics](../core/value-storage.md) and [Core functions](../core/functions.md). Concrete `if`/`else`/`while`/`break`/`continue` spelling and the represented `ConditionalValue` grammar are owned by [Source concrete syntax](concrete-syntax.md).

This document does not redefine owned-value producer semantics, field-receiver semantics, return execution, explicit-fault execution, structural path/state mathematics, safe-reference authority/carrier/reborrow/lifetime semantics, raw-pointer origin production/transport/unsafe semantics, binding scope/mutability rules, lexical cleanup order, fault cleanup, loop-transfer cleanup ordering, Core path state, Core value/storage semantics, or concrete grammar.

## Represented conditional statement

One represented source conditional statement consists of:

- exactly one represented conditional-value producer;
- exactly one explicit **then arm** block; and
- zero or one explicit **else arm** block.

The concrete form is owned by `concrete-syntax.md`.

A represented conditional is a statement. It produces no source value, introduces no Unit/Void value, and is not an owned-value producer.

This revision defines no conditional expression, direct `else if` form, pattern condition, guard, match, catch, label, or unrestricted nonterminal-within-block return. The separately represented bounded `while` and bounded unlabeled loop-transfer relations are defined below; no other loop or transfer form is implied by conditional semantics.

## Conditional-value admission

The represented condition is one concrete `ConditionalValue` from `concrete-syntax.md`.

The condition MUST produce exactly one owned source value whose source type is exactly the intrinsic `Bool` type under `types.md`.

No truthiness, implicit conversion, coercion, integer-to-Bool relation, structural conversion, or second Bool-like type is introduced.

Concrete syntax deliberately excludes standalone safe-reference formation/reborrow/dereference and raw-address/raw-move forms from `ConditionalValue`; that grammar restriction is owned by `concrete-syntax.md`. A bounded producer-backed `FieldValueUse` whose receiver is a record construction remains a distinct admitted field-value producer because the mandatory selector is part of that complete field-value spelling. This semantic owner does not widen those grammar restrictions from parser lookahead or type information.

An admitted producer-backed `FieldValueUse` condition MUST have final selected field type exactly `Bool` under `field-access.md`. That requirement applies to the complete field-value result; the internal direct-call or record-construction receiver retains its independently selected exact receiver type.

A syntactically represented conditional value whose resolved/produced type is not exactly `Bool` is source-invalid.

The same exact admission relation is consumed by both represented `IfStatement` and `WhileStatement`. `while` does not add a second condition grammar or type rule.

## Definite enclosing source state

For control-flow purposes, one **definite enclosing source state** contains these independently owned components:

1. every active enclosing parameter/local binding identity with its declared source type, assignment-mutability classification, and exact structural ownership consumed-path set;
2. every active replacement-capable parameter external referent structural root from `references.md`, with its exact structural ownership consumed-path set; and
3. for every active raw-pointer local, its exact `PointerOrigin(binding)` provenance from `raw-pointers-unsafe.md`.

Safe-reference authority parent/child/carrier state is validated sequentially by `references.md` and remains subject to lexical cleanup. Bounded Shared direct-child call results remain ordinary sequential authority/carrier state under that owner, including carrierless-parent ancestry while returned descendants remain active. This state is **not** converted into a generic branch-state lattice, authority-graph union, join, or fixed point by this document. The represented exclusions on reference fields, mutable/rebindable reference locals, and replacement-capable results, together with lexical carrier cleanup and the exact bounded result relation from `references.md`, require no additional control-flow authority-state dimension.

Every structural-state equality below applies independently to all continuing structural owned-value roots: ordinary binding roots and replacement-capable external referent roots. Raw-pointer provenance equality remains a separate exact requirement.

## Condition validation state

Source validation of one represented conditional begins in the definite enclosing source state that exists immediately before the condition producer.

Validate the condition through its existing producer owner with exact required source type `Bool`.

A producer-backed `FieldValueUse` condition consumes the field-value owner's existing validation transaction with exact required final type `Bool`; this conditional relation adds no separate receiver-validation or ownership-commit rule.

During source validation, apply each semantic ownership, reference-authority, external-referent, or raw-pointer consequence selected by that condition producer exactly once before conditional outcome state splitting.

The resulting definite enclosing state is the **post-condition state**. The explicit then arm and, when present, the explicit else arm are each source-validated from semantically identical copies of that same post-condition state. When else is omitted, the false normal outcome is the unchanged post-condition state as defined below.

The successful condition result is one owned Bool transient held by the conditional operation for branch selection. That transient is not a function-local binding, external referent root, or member of the post-condition state. Any source-state consequences that condition production applied to pre-existing roots, reference authority, or raw-pointer provenance are already reflected in the post-condition state.

For a producer-backed field-value condition, its producer-specific receiver transient lifecycle has already completed under `function-execution.md` before the successful Bool result is transferred into this distinct condition transient.

## Validation does not prune by Bool value

Both represented conditional outcomes MUST be considered for source validity independently of the semantic Bool value that one concrete runtime execution may observe.

In particular:

- a condition spelled as the literal `true` does not exempt the false outcome—explicit else arm or omitted-else outcome—from source validity; and
- a condition spelled as the literal `false` does not exempt the then arm from source validation.

An explicit arm that contains a represented terminal return, explicit `fault;`, or source-valid loop transfer is still validated in full even when a constant condition would prevent that arm from executing in one concrete run.

This is a source-validity rule. It does not assert that both outcomes execute in one concrete activation and does not create an unknown or three-valued Bool.

This revision therefore does not require source constant propagation, constant folding, symbolic execution, unreachable-arm weakening, or a value lattice merely to validate a conditional statement.

## Condition execution and runtime selection

When execution reaches one represented conditional statement:

1. evaluate the condition producer exactly once under its existing source execution semantics with required type `Bool`;
2. preserve every ownership, safe-reference, external-referent, raw-pointer, transient, fault, and divergence consequence of that producer evaluation;
3. on successful production, hold exactly one owned Bool condition transient;
4. consume that transient for conditional selection;
5. when its Bool value is `true`, execute only the then arm;
6. when its Bool value is `false` and an explicit else arm exists, execute only that else arm; and
7. when its Bool value is `false` and no explicit else arm exists, take the omitted-else normal false outcome defined below.

A producer-backed field-value condition reaches step 3 only after its complete producer-specific field-receiver lifecycle has finished under `function-execution.md`; this conditional relation does not define a second receiver lifecycle.

Conditional selection itself performs no additional binding read, move, duplicate, assignment, reference formation/reborrow, referent replacement, pointer retargeting, structural consumption, cleanup, call, fault selection, return, loop transfer, or hidden source state transition beyond ending ownership of the successful Bool condition transient used for selection.

The represented condition is therefore evaluated once even though source validation considers both outcomes.

## Condition fault

If condition evaluation yields one defined fault `F` before successful Bool production:

- no explicit conditional arm begins;
- no conditional normal successor is selected;
- ownership and other source-state transitions already completed during condition evaluation remain effective; and
- the active activation follows the existing producer/receiving cleanup and defined-fault propagation relations from `function-execution.md` with the same fault `F`.

The conditional statement does not add a second fault cleanup boundary.

A condition producer's ability to yield a defined fault is a dynamic execution possibility. It does not by itself change the conditionally selected arms' static local normal-continuation classification.

## Condition divergence

If condition evaluation diverges before successful Bool production:

- no explicit conditional arm begins;
- no conditional normal successor is selected; and
- no lexical-scope, activation, conditional, external-referent restoration, or loop-transfer cleanup occurs merely because execution remains suspended.

Producer-owned transients and completed ownership/source-state transitions persist exactly as required by their existing owners.

Divergence is likewise a dynamic execution possibility and does not create a static no-local-normal outcome for an otherwise normally continuable represented construct.

## Explicit arm lexical scopes

Every explicit conditional arm is exactly one represented nested block and therefore exactly one child lexical scope under `local-bindings.md` and `function-execution.md`.

An explicit then arm and explicit else arm are sibling lexical scopes of the same enclosing scope.

Consequently:

- bindings introduced in one arm do not enter the other arm;
- bindings introduced in one arm do not survive the normal end of that arm;
- sibling arms MAY independently introduce the same lexical identifier key because their scopes do not overlap;
- neither arm may introduce a key that illegally shadows an active enclosing function-local binding;
- ordinary locals, safe-reference locals/reborrows, record-pattern bindings, nested blocks, assignments, reference replacement, raw-pointer operations inside applicable unsafe blocks, calls, explicit fault statements, represented bounded `while`, source-valid bounded `break;`/`continue;`, and the arm's optional terminal return retain their existing semantics; and
- nested represented conditionals may occur because `IfStatement` is itself a represented body statement inside an arm block.

This document does not create an abstract source-visible branch-scope identity beyond the ordinary child lexical scopes already owned by `local-bindings.md`.

## Explicit arm normal completion

When an explicit arm has a local normal continuation and the selected execution reaches that normal completion:

1. finish its contained body-statement sequence under `function-execution.md`;
2. normally exit that child lexical scope;
3. clean bindings declared directly in that arm using the existing lexical-scope cleanup relation, including ending any safe-reference carriers stored there; and
4. only after that cleanup does the arm produce its **normal enclosing outcome** for conditional-successor purposes.

Arm-local bindings have ended before normal-successor comparison and therefore are not members of the enclosing binding environment compared when two normal outcomes meet. External referent roots established by incoming replacement-capable parameters are activation-level state and remain continuing roots unless the activation terminates.

Normal cleanup of one arm does not itself clean, reset, restore, or retarget enclosing roots merely to make their states match another arm.

## Explicit arm return and explicit fault

When source validation determines that an explicit arm has no local normal continuation, that arm contributes no normal enclosing outcome to the conditional. Return and explicit fault are activation-terminating ways to establish such an outcome; bounded loop transfers are handled separately below.

At runtime, when the selected arm reaches a represented return:

- the return performs the replacement-capable normal-completion restoration check owned by `references.md`/`function-execution.md` after result effects and before activation cleanup;
- a source-valid return then terminates the current source function activation under `function-execution.md`;
- the arm does not first perform independent normal child-scope cleanup;
- every then-active lexical scope participates exactly once in return-induced activation cleanup; and
- no conditional normal join or normal successor is taken on that execution.

A returning path may consume a complete root or an arbitrary source-valid structural subvalue of an enclosing binding and may change an external referent during return-value production. It need not equal a normal sibling outcome, but every incoming replacement-capable external referent must satisfy the separately required fully-available normal-return postcondition before cleanup.

At runtime, when the selected arm reaches represented `fault;`:

- the statement selects the distinguished source defined-fault reason `ExplicitFault` under `function-execution.md`;
- the arm does not first perform independent normal child-scope cleanup;
- every then-active lexical scope participates exactly once in the existing activation fault cleanup;
- completed ownership/provenance/reference transitions before the statement remain effective;
- no replacement-capable external-referent restoration is synthesized for the fault; and
- no conditional normal join or normal successor is taken on that execution.

A path that explicitly faults may therefore reach the fault statement with enclosing binding/external-referent roots in any source-valid then-current structural state and with any source-valid raw-pointer origins. That state does not have to equal a normal sibling outcome because the explicitly faulting path contributes no normal successor state.

These rules do not introduce path-dependent ownership/origin at a normal successor because activation-terminating paths contribute no normal successor state.

## Explicit arm loop transfer

A source-valid `break;` or `continue;` reached inside an explicit conditional arm likewise gives that execution no **local** normal arm outcome, but it does not terminate the function activation merely for that reason.

The transfer statement independently selects the nearest enclosing represented `while` and MUST satisfy the applicable exact target-state rule below, including every continuing binding root, replacement-capable external referent root, and raw-pointer origin, before the containing conditional can be source-valid. Its exited-scope cleanup is owned by `function-execution.md` and includes the active arm scope plus every intervening active child scope through the target loop body scope.

The transfer path contributes no normal enclosing state to the conditional's ordinary local successor composition. Consequently:

- one transfer arm and one locally normal arm leave the normal arm as the conditional's sole normal outcome;
- both explicit arms transferring leave the conditional with no local normal continuation, including when one arm breaks and the other continues;
- an omitted else remains one normal false outcome, so a transfer-only then arm with omitted else leaves that false outcome as the sole normal successor; and
- a transfer arm's enclosing state is not compared with a normal sibling merely to form a conditional join, because the transfer has already proved exact validity at its loop target.

The conditional does not merge break and continue destinations, create an abrupt-completion lattice, or reconstruct their target facts from lower CFG edges. Their distinct retained statements and nearest-loop lexical context remain sufficient for faithful lowering.

## Producer-originating arm fault and divergence

If execution of an accepted producer or other operation inside the selected arm yields a defined fault before the represented successful statement structure reaches its local normal continuation or loop transfer, that concrete execution produces no normal enclosing outcome.

The active child scope participates exactly once in the existing activation fault cleanup from `function-execution.md`. It does not first perform independent normal arm cleanup and then fault cleanup. No replacement-capable external-referent restoration is synthesized on that fault path.

Such a producer-originating fault possibility does **not** by itself remove the arm's static local normal continuation. Static completion follows the represented statement/control structure under `function-execution.md`; a producer that may fault dynamically still contributes its ordinary successful continuation when one is represented.

If execution inside the selected arm diverges, that concrete execution produces no runtime normal outcome and performs no normal arm, successor, loop-transfer cleanup, or external-referent restoration merely because execution remains suspended.

Divergence likewise does not alter static local normal-continuation presence under this revision. The represented explicit `fault;`, `break;`, and `continue;` statements are different: successful execution of each has no local fallthrough by definition.

## Omitted else

When no explicit else arm is present, runtime `false` selects the **omitted-else normal false outcome**.

That outcome:

- executes no body statement;
- introduces no lexical binding;
- creates no source-visible synthetic lexical scope;
- performs no cleanup or restoration; and
- carries every enclosing binding root, replacement-capable external referent root, and raw-pointer origin exactly as they exist in the post-condition state.

For normal-successor validation, the omitted-else false outcome is therefore the unchanged post-condition enclosing state and always contributes one normal outcome.

An explicit `else {}` is an actual child lexical scope, even though its empty body yields the same enclosing structural ownership/external-referent/raw-origin outcome after its ordinary empty cleanup.

## Enclosing state at a conditional successor

Let `E` be the post-condition definite enclosing source state.

Its binding component contains exactly the active parameter/local binding identities that remain in the enclosing lexical environment after successful condition production. Condition evaluation cannot introduce one new function-local binding because represented conditional values are owned-value producers rather than declarations.

Its external-referent component contains exactly the replacement-capable parameter structural roots established for the current activation by `references.md`; ordinary child scopes do not create or remove those roots.

For each binding root and external referent root, `E` retains exact structural ownership state. For every binding of exact type `RawPtr(T)`, `E` additionally retains the exact pointer-origin provenance established by `raw-pointers-unsafe.md`.

Each explicit arm is source-validated from its own copy of `E`. An omitted else uses unchanged `E` as its normal false outcome.

After normal completion and local cleanup of an explicit arm that has a local normal continuation, its normal outcome contains only enclosing binding identities belonging to `E`, the same external referent roots, and applicable raw origins. An arm with no local normal continuation contributes no enclosing state for normal-successor composition; its return, explicit fault, or loop transfer follows its own target relation.

Binding identity, declared type, assignment-mutability classification, and external referent root identity/type are unchanged by conditional branching itself. Where two normal outcomes meet, the conditional compares each continuing structural root's ownership state and every continuing raw-pointer binding's exact pointer-origin provenance.

## Conditional local normal-continuation composition

After both represented outcomes have been source-validated, compose only their **local normal** outcomes:

- **two normal outcomes:** the conditional has a normal successor only when exact structural-ownership-state equality succeeds for every continuing binding root and replacement-capable external referent root, and exact raw-pointer-origin equality succeeds for every applicable binding in `E`; the equal common state is the one definite normal successor;
- **exactly one normal outcome:** that sole outcome is the conditional's one definite normal successor without any structural/origin equality comparison against the no-local-normal outcome;
- **zero normal outcomes:** the conditional has no local normal continuation and no normal join; and
- **omitted else:** the false outcome is always normal, so a conditional without explicit else always has at least one normal outcome.

This is only composition of the local-fallthrough relation from `function-execution.md`. It introduces no source state set, completion lattice, maybe-owned state, maybe-origin state, authority-graph join, implicit cleanup/restore edge, or runtime completion tag.

A sole normal outcome is not a union, intersection, widening, or normalization of branch states. It is exactly the enclosing state produced by that one normally completing outcome after ordinary arm-local cleanup.

A zero-local-normal conditional does not by itself imply function-activation termination. It may be activation-terminating when all paths return/fault, or it may occur inside a represented loop where all paths perform source-valid loop transfers. The enclosing control owner determines those destinations.

## Exact structural-ownership state equality for two normal outcomes

For one represented continuing structural root, two normal outcomes have equal structural ownership state exactly when their prefix-free consumed-path sets under `structural-ownership.md` are equal.

When **two** normal outcomes meet, a represented conditional has a valid normal successor only when every continuing binding root and every replacement-capable external referent root in `E` has equal structural ownership state on those two outcomes.

When all continuing structural states are equal:

- each common state is the one definite structural ownership state of that root at the normal continuation; and
- subsequent source validation proceeds through the existing single-state binding/external-referent relations in `local-bindings.md`, `references.md`, and `structural-ownership.md`.

When two normal outcomes exist and any continuing structural root has unequal normal outcome states, the conditional statement is source-invalid at its normal join boundary. No normal post-conditional ownership state is established.

The equality requirement is semantic equality of source structural state. It is not equality of runtime values, record values, Core local/external-referent state, parser nodes, HIR data structures, compiler hashes, or physical storage.

No equality comparison is performed between a normal outcome and an outcome that has no local normal continuation. A return, explicit fault, or loop transfer instead obeys its independently established destination/cleanup/restoration relation.

## Exact raw-pointer-origin equality for two normal outcomes

For one enclosing binding of exact type `RawPtr(T)`, two normal outcomes have equal pointer-origin state exactly when the stored raw-pointer value on each outcome carries `PointerOrigin(x)` for the same source binding identity `x` under `raw-pointers-unsafe.md`.

When **two** normal outcomes meet, a represented conditional has a valid normal successor only when every raw-pointer binding identity in `E` has equal pointer origin on those outcomes in addition to satisfying every structural ownership equality.

When the origins are equal, that exact origin is the one definite pointer-origin provenance at the normal continuation. When any continuing raw-pointer binding has different origins, the conditional is source-invalid at its normal join boundary even if both pointer values have the same raw-pointer type and both target bindings would individually satisfy lexical target validity.

No pointer-origin set, alternative-origin union, maybe-origin value, runtime origin tag, lower Core pointer-target comparison, or physical-address equality substitutes for exact source binding-origin equality.

No origin comparison is performed against an outcome that has no local normal continuation; return, fault, or loop transfer instead carries its actual then-current origin state into its independently established destination/cleanup relation.

## Definite-state consequences

The exact-state rules for two normal outcomes have these required consequences.

### Equal complete consumption

If both normal outcomes consume the complete root of the same continuing structural root and do not replace/restore it, both consumed-path sets contain the complete root path and that root may join as unavailable.

If two normal outcomes exist and one consumes the complete root while the other leaves it fully available, the states differ and the conditional is source-invalid. This applies equally to a local binding root and a replacement-capable external referent root.

A no-local-normal outcome is not compared against a normal sibling. Return/fault cleanup or a source-valid loop-transfer target relation consumes that path's actual then-current state as applicable; a normal return additionally obeys the replacement-capable external-referent restoration law.

### Equal partial consumption

If both normal outcomes consume exactly the same represented nested structural paths of one continuing root, the equal resulting consumed-path set may join.

If two normal outcomes leave different consumed sibling or nested paths, the states differ and the conditional is source-invalid.

No structural similarity, equal remaining-frontier shape, or equal number of consumed paths substitutes for exact consumed-path-set equality.

A no-local-normal outcome may independently reach its return/fault/loop-transfer destination after different valid structural operations; the applicable destination relation handles that path rather than normalizing it toward a normal sibling.

### Duplicable uses

A represented non-consuming duplicate use leaves the consumed-path set unchanged under `structural-ownership.md` and therefore does not by itself prevent an equal-state join when two normal outcomes meet.

Duplicating a Shared reference or raw-pointer value likewise preserves its exact authority/origin identity under its owner. A replacement-capable reference value is non-duplicable.

### Assignment and replacement

Whole-binding assignment retains its existing source-first replacement semantics. Safe-reference complete-referent replacement retains the separate source-first semantics from `references.md`.

A successful whole-binding assignment may begin from a fully available, partially available, or unavailable binding state and ends by establishing fresh complete structural ownership for the replacement value. A successful `*r = Value` may likewise restore the selected replacement-capable external referent or local-root structural domain to complete ownership when its post-RHS reference-authority requirement succeeds.

Consequently, two normal arms with different earlier ownership histories may still produce equal normal structural states when explicit accepted replacement operations re-establish the same complete-state classification before normal arm completion. The conditional inserts no implicit repair.

For a mutable raw-pointer local, accepted ordinary assignment additionally installs the incoming exact pointer origin. Two normal arms may therefore retarget that pointer during their execution but can join only if the final normal origins are exactly equal. Assignment to two different target bindings does not join merely because both targets have the same source type.

The conditional does not special-case replacement and does not require runtime values installed by different arms to be equal.

### Branch-dependent runtime values

Two normal outcomes may produce different runtime values in the same mutable enclosing binding or replacement-capable external referent while still having equal structural ownership state.

For a raw-pointer binding they may likewise produce different implementation-level pointer encodings only when the source-defined pointer origin remains the same; source pointer equality/representation is not defined. Different source pointer origins do not join.

Structural ownership/provenance definiteness is not runtime value equality, constant propagation, or SSA value merging.

### Omitted else

With no explicit else arm, the false normal outcome is the unchanged post-condition state.

When the then arm also completes normally, its normal enclosing structural ownership state MUST equal that unchanged post-condition state for every continuing binding/external-referent root, and every enclosing raw-pointer binding's origin MUST equal its unchanged post-condition origin. Therefore a normally completing then arm may not leave one continuing root consumed/partially consumed or one raw-pointer local retargeted unless accepted operations inside the then arm explicitly restore the applicable post-condition state before normal completion.

When the then arm has no local normal continuation, the omitted-else false outcome is the sole normal successor and no structural/origin equality is required against that then arm. The then path independently obeys return, fault, or loop-transfer validity as applicable.

### Zero-field and zero-leaf values

Zero-field and recursively zero-leaf source values participate in consumed-path-state equality exactly like other structural owned values when two normal outcomes meet.

Their source ownership state MUST NOT be inferred from whether a Core representation has a scalar destruction leaf or emits a physical destruction operation.

## Post-successor source operations

After a conditional with one valid normal successor, every represented continuing binding root and replacement-capable external referent root has exactly one committed structural ownership state: either the equal state of two normal outcomes or the exact state of the sole normal outcome. Every continuing raw-pointer binding likewise has exactly one committed pointer origin by the same two-outcome/sole-outcome rule.

Subsequent whole-binding use, field-value use, record-pattern use, assignment, safe-reference operation, raw-pointer operation, nested conditional, bounded `while`, lexical cleanup, return cleanup, loop transfer, or fault cleanup consumes those ordinary definite states through their existing source owners.

No post-successor operation performs a second conditional-state analysis merely because a value was previously used inside a represented conditional.

A conditional with zero local normal outcomes admits no subsequent statement in the same containing sequence under `function-execution.md`; any source-valid non-local destinations are already represented by its selected return/fault/loop-transfer paths.

## No implicit join normalization

When two normal outcomes meet, this conditional relation does not derive a common state by:

- union or intersection of consumed paths or pointer origins;
- prefix minimization;
- meet, join, widening, or another lattice operation;
- automatically consuming still-owned values on one branch edge;
- automatically restoring a replacement-capable external referent on one branch edge;
- automatically retargeting a raw-pointer local on one branch edge;
- inventing a maybe-owned or maybe-origin state;
- joining safe-reference authority graphs; or
- consulting lower Core liveness, external-referent state, reference-authority state, or pointer-target metadata.

In particular, source-invalid unequal two-normal-outcome states are not made valid by silently cleaning additional owned subvalues, restoring a referent, ending/reconstructing reference authority, or retargeting a raw pointer on an outcome that retained a different state.

When exactly one normal outcome exists, using that outcome directly is not normalization and performs no branch-edge cleanup/restore/retargeting.

This avoids introducing path-specific source destruction, restoration, authority, or pointer-retarget timing as an implicit consequence of merely reaching a branch successor.

## No conditional ownership, provenance, authority, or completion flags

This revision introduces no source drop flag, runtime moved-state flag, hidden path tag, conditional cleanup bit, runtime pointer-origin tag, runtime reference-authority tag, runtime completion tag, or source-visible ownership/provenance/authority test.

Every continuing structural root that reaches a represented normal continuation has one definite structural ownership state fixed statically by the rules above, and every raw-pointer binding has one exact pointer origin. Loop-transfer destinations likewise require one exact statically established target state rather than a runtime ownership/provenance classification.

Safe-reference parent/child authority remains the sequential relation owned by `references.md`. This control-flow owner does not synthesize branch authority state by merging graphs.

Future source control-flow forms or later extensions may introduce additional accepted relations only through their own normative changes. Their existence MUST NOT retroactively alter the semantics of two-normal-outcome conditionals already accepted by the exact-state relation.

## Constant conditions

A concrete source execution with condition value `true` executes only the then arm. A concrete source execution with condition value `false` executes only the false outcome.

Source validation nevertheless validates both represented outcomes and computes each arm's local normal-continuation presence independently of that concrete Bool value. When both outcomes are normal, the exact structural/origin equality requirements apply to both even for a constant condition. When only one outcome is normal, it is the sole static normal successor even if a particular constant Bool execution selects a no-local-normal return/fault/transfer outcome instead.

This difference between runtime selection and conservative source validation does not introduce nondeterministic source execution.

## Nested conditionals

A represented conditional may occur inside any explicit arm block because it is a body statement under `concrete-syntax.md`.

The inner conditional independently establishes zero or one definite local normal successor by the composition rules above. If it has one, the containing arm may continue from exactly that binding/external-referent/raw-origin state. If it has none, no later statement or terminal return in that same arm sequence is source-valid; any represented return/fault/loop-transfer destination remains independently controlling.

Consequently, nested conditionals compose recursively through local normal-continuation presence without a general source CFG or state-set relation.

This revision defines no direct `else if` grammar. Equivalent nested selection may be written only through the represented explicit block nesting admitted by `concrete-syntax.md`.

## Represented bounded while

One represented source `while` statement consists of exactly one represented `ConditionalValue` and exactly one explicit body `BlockStatement` under `concrete-syntax.md`.

The `while` is statement-only. It produces no source value, introduces no Unit/Void value, and is not an owned-value producer. It has no represented `else`, label, result value, iteration binding, pattern condition, iterator protocol, or unconditional-loop form. Its body may contain the bounded unlabeled `break;` and `continue;` statements defined below.

The condition consumes the exact same admission/type/producer relation defined above: its result type MUST be exactly intrinsic `Bool`, standalone record construction and direct safe-reference/raw-pointer value forms remain excluded by grammar, and all existing producer validation and transactional ownership/reference/provenance rules apply unchanged.

## While validation environments H and C

Let `H` be the complete definite enclosing source state immediately before validation of the loop condition. It includes every active enclosing binding root, every replacement-capable external referent root, and every raw-pointer origin as defined above.

Validate the loop condition through its existing producer owner from a copy of `H`, requiring exact intrinsic `Bool`. A failed condition validation makes the `while` source-invalid and commits no speculative condition ownership/reference/provenance change to the surrounding environment.

On successful condition validation, apply the condition producer's selected source-state consequences exactly once to that validation copy. Call the resulting definite enclosing state `C`.

`C` is the state after one successful condition production and before runtime Bool selection. The successful Bool condition transient is not a function-local binding or external referent root and is not part of `C`.

The represented false outcome is always one static normal outcome carrying exactly `C`. It introduces no body scope, performs no body cleanup or referent restoration, and establishes the loop's definite post-loop state when source validation succeeds.

The represented true outcome validates the explicit body from a semantically identical copy of `C` as one ordinary child lexical block under `local-bindings.md` and `function-execution.md`.

Condition evaluation introduces no new function-local binding or replacement-capable external referent identity. Consequently every continuing root in `H` remains represented in `C` and remains present after ordinary normal completion/cleanup of the body or after an admitted loop transfer. Body-local bindings end before ordinary backedge comparison or as part of transfer cleanup and are not target-state dimensions.

## While normal-backedge admission

If the body has a local normal continuation, the `while` admits that normal outcome as a backedge **only** when every continuing binding root and replacement-capable external referent root from `H` has exactly the same structural ownership state after ordinary body-scope normal cleanup as it had in `H`, and every enclosing raw-pointer binding has exactly the same pointer origin as it had in `H`.

For structural ownership, equality is exactly equality of the prefix-free consumed-path set under `structural-ownership.md`. For raw-pointer origin, equality is exactly the `PointerOrigin(x)` identity relation from `raw-pointers-unsafe.md`. Binding/root identities, declared source types, and assignment-mutability classifications remain the same enclosing facts and are not reconstructed or merged.

The comparison is deliberately against `H`, not `C`. The condition will execute again after a normal backedge; therefore the body must explicitly restore whatever enclosing structural ownership/external-referent/raw-pointer state the next condition evaluation requires to begin from the same accepted loop-head state. A condition may itself transform `H` to a different `C` each iteration through its existing producer consequences.

If any continuing root's normal post-body structural ownership state or any raw-pointer origin differs from its applicable state in `H`, the `while` is source-invalid with no admitted ordinary backedge and no committed post-loop state.

This is exact source-state equality, not equality of runtime values. A mutable enclosing ordinary binding or replacement-capable referent may therefore contain a different runtime value after explicit replacement while still satisfying the backedge when its structural ownership state equals `H`. A mutable raw-pointer local may be temporarily retargeted inside the body but must have its exact `H` origin restored before a normal backedge.

A successful represented whole-binding assignment or safe-reference complete-referent replacement may explicitly restore complete ownership before the backedge when otherwise source-valid. Raw-pointer assignment may restore a required pointer origin. The loop adds no special restoration operation. An immutable ordinary binding receives no implicit restoration and cannot be assigned merely to satisfy the loop invariant.

A body-local binding never participates in this equality because ordinary normal body-scope cleanup ends it before comparison. Re-entering the same static body on a later dynamic iteration does not create a new source binding identity; it creates a new dynamic value owned by that same static binding identity while the child scope is active. The lexical pointer-target-validity rule from `raw-pointers-unsafe.md` prevents a longer-lived enclosing raw-pointer local from retaining an origin naming such a shorter-lived body-local instance.

## Nearest enclosing loop target

Every source-valid `break;` or `continue;` is lexically contained in at least one represented `while` body.

The target is exactly the nearest enclosing represented `while` whose body lexical scope contains the transfer point. Ordinary nested blocks, unsafe blocks, and conditional arm scopes do not become transfer targets. While execution is inside a nested represented loop, that inner loop is the target of an unlabeled transfer from its body and shadows any outer loop for this purpose.

Target selection is a static source lexical/control fact. It is independent of parser node identity, HIR/Core block numbering, dynamic iteration count, runtime stack layout, or physical branch structure.

An occurrence with no enclosing represented `while` is source-invalid. This revision defines no label namespace, labeled transfer, dynamic target selection, or transfer to a non-nearest enclosing loop.

## Loop-transfer exited scopes and cleanup boundary

For one admitted transfer to target loop `L`, the exited lexical scopes are every then-active source lexical scope from the scope containing the transfer point outward through and including `L`'s body lexical scope, stopping before the lexical scope containing the `while` statement itself.

Thus a transfer directly in the loop body exits exactly that body scope; a transfer in nested blocks, unsafe blocks, or a conditional arm exits those active descendant scopes innermost-first and then the target body scope; and an inner-loop transfer does not exit the outer loop body.

`function-execution.md` owns the actual cleanup ordering and remaining-frontier cleanup of those exited scopes, including safe-reference carriers stored in exited locals. The transfer-state comparisons below concern only the continuing roots/origins represented by the selected loop's `H`/`C`; body/descendant locals are not target-state dimensions and receive no separate normalization rule.

Transfer cleanup MUST NOT consume, reset, restore, retarget, or otherwise change an enclosing `H`/`C` root or raw-pointer origin merely to force target-state equality. Any required restoration must have been established by an ordinary accepted source operation before the transfer.

## Continue admission

For nearest target loop `L`, let `H_L` be that loop's already-defined loop-head definite state.

After the source-valid operations preceding the transfer, and independently of cleanup of body/descendant locals, `continue;` is admitted only when every continuing binding root and replacement-capable external referent root belonging to `H_L` has exactly the same structural ownership state as in `H_L` and every raw-pointer binding belonging to `H_L` has exactly the same pointer origin as in `H_L`.

The equality relations are exactly the same prefix-free consumed-path-state and exact pointer-origin equalities used for an ordinary normal backedge. Binding/root identity, declared type, and assignment-mutability classification remain the same enclosing facts.

If any enclosing structural state or raw-pointer origin differs, the `continue;` is source-invalid. The language does not repair the state through implicit cleanup, replacement, retargeting, reset, union, intersection, widening, authority-graph manipulation, or a runtime ownership/provenance flag.

A mutable enclosing ordinary binding or replacement-capable referent may contain a different runtime value while satisfying the target relation because target equality is structural ownership equality, not runtime value equality. A mutable raw-pointer local may likewise be retargeted temporarily only when its exact required origin is restored before the transfer. Explicit accepted replacement/assignment may restore required state; there is no implicit restoration.

## Break admission

For nearest target loop `L`, let `C_L` be that loop's already-defined successful post-condition state and definite post-loop state.

After the source-valid operations preceding the transfer, and independently of cleanup of body/descendant locals, `break;` is admitted only when every continuing binding root and replacement-capable external referent root belonging to `C_L` has exactly the same structural ownership state as in `C_L` and every raw-pointer binding belonging to `C_L` has exactly the same pointer origin as in `C_L`.

If any enclosing structural state or raw-pointer origin differs, the `break;` is source-invalid. No implicit edge cleanup/reset/restore/retargeting/normalization is performed to force equality.

The target is deliberately `C_L`, not `H_L`. The existing `while` relation already establishes `C_L` as the one definite post-loop source state after the loop's normal false exit. Requiring every explicit break path to establish exactly that same state preserves one definite post-loop state without adding a join, union, intersection, maybe-owned/origin state, authority graph merge, or runtime flag.

Runtime values in mutable bindings/referents need not equal values from a condition-false execution when the required source state equals `C_L`. `break;` does not synthesize or claim that the condition evaluated false.

## Loop-transfer execution

On successful dynamic execution of an admitted `continue;`:

1. perform the exited-scope cleanup owned by `function-execution.md` exactly once;
2. transfer to the selected loop's condition point; and
3. let the ordinary `while` relation perform the next condition evaluation exactly once.

`continue;` itself does not evaluate the condition, produce/consume a Bool, restore a referent, or create a source-visible iteration identity.

On successful dynamic execution of an admitted `break;`:

1. perform the exited-scope cleanup owned by `function-execution.md` exactly once; and
2. transfer directly to the selected loop's existing post-loop continuation with source state `C_L`.

`break;` does not re-evaluate the condition and does not synthesize a false condition result. Its execution may therefore have different effects from a later condition-false exit even though both reach the same definite structural ownership/pointer-origin state.

Both statements have no local fallthrough in their immediate source sequence. They are not return/fault termination of the current activation; the selected enclosing loop consumes their non-local destination.

## While body with no local normal continuation

If the represented body has no local normal continuation under `function-execution.md`, it contributes no ordinary normal backedge state and requires no body-wide equality comparison against `H`.

Return and explicit-fault termination use the actual then-current body/enclosing state through their existing activation cleanup relations. A normal return still requires every incoming replacement-capable external referent fully available before activation cleanup. Fault has no such restoration obligation. An admitted `break;` or `continue;` path instead must already have satisfied its own exact `C` or `H` target-state relation and performs transfer cleanup rather than ordinary body normal cleanup.

A producer-originating defined fault or divergence inside an otherwise locally normally continuable body remains a dynamic possibility and does not remove that body's represented local normal continuation. The ordinary backedge equality requirement still applies to the successful normal body path.

Regardless of whether the body has a local normal continuation, the represented false condition outcome remains a static normal successor with state exactly `C`.

## While validation does not constant-prune

A represented `while` always retains its static false normal outcome, including when the condition is the literal `true`.

Source validation therefore does not infer that `while true` eliminates following code or the root normal result obligation. The body is validated even for literal `false`, and the false outcome remains represented even for literal `true`.

This conservative static relation does not assert runtime nondeterminism: at runtime a successfully produced Bool still selects exactly its semantic value.

No source constant propagation, symbolic execution, unreachable-loop-exit weakening, or value lattice is required for this rule.

## While dynamic execution

For a source-valid represented `while`, runtime execution follows `function-execution.md` and the condition producer's existing owner:

1. evaluate the condition from the current loop-head dynamic state;
2. preserve every ownership/reference/provenance transition, transient consequence, defined-fault possibility, and divergence consequence of that evaluation;
3. on successful Bool production, consume the condition transient for selection;
4. if false, take the post-loop normal continuation without activating the body scope;
5. if true, activate and execute the ordinary child body block;
6. if the body completes normally, perform its ordinary normal child-scope cleanup and then repeat from condition evaluation;
7. if the body executes an admitted `continue;`, perform its transfer cleanup and repeat from condition evaluation without a second normal body cleanup;
8. if the body executes an admitted `break;`, perform its transfer cleanup and continue after the selected loop without re-evaluating the condition or performing normal body cleanup;
9. if the body returns or explicitly faults, follow the existing activation termination relation without a separate normal body cleanup/backedge; and
10. if condition/body execution faults through another accepted producer or diverges, preserve the existing fault/divergence semantics without inventing a normal backedge, referent restoration, or exit.

Each successful normal or continue backedge therefore reaches one fresh condition evaluation. A direct-call condition performs a fresh dynamic call on each such visit to the condition point; no loop-invariant hoisting or memoization authority is implied.

## While post-loop state

For every source-valid represented `while`, the definite normal post-loop state is exactly `C`, the state after successful condition validation and before false selection.

The backedge-restored `H` state is **not** the post-loop state. A condition producer's ownership/reference/provenance effects remain effective on the false execution that exits the loop.

An admitted explicit `break;` also reaches the post-loop continuation only after proving exact structural ownership and raw-pointer-origin equality with `C`. The break path need not have executed a false condition and may carry different runtime values/effects while preserving that source state.

No ordinary body state is merged into `C`: a normal body state must already equal `H` to be admitted as a backedge, while a no-local-normal body contributes no ordinary backedge state. A break path independently equals `C`; a continue path independently equals `H`. Subsequent source operations consume `C` directly through their existing single-state owners.

This distinction is required for condition producers that consume or otherwise transform enclosing source state before producing Bool. It is not observable for state dimensions a condition leaves unchanged, though condition execution/effects remain semantically distinct from explicit break.

## No loop state lattice or fixed point

The bounded `while` and loop-transfer relations require no source ownership/origin state set, may-be-owned/origin state, join/meet, widening, fixed-point iteration, generic source CFG, SSA construction, runtime moved/drop/origin/iteration/completion flag, implicit backedge/transfer cleanup/restoration/retargeting of enclosing values, authority-graph fixed point, or hidden lifetime generation.

Validation checks one exact condition transition `H -> C`, one body validation from `C`, one exact source-state equality proof from each ordinary normal cleaned-body outcome back to `H`, and one exact `H`/`C` proof at each represented continue/break transfer respectively. Repeated runtime execution is justified by those invariants; source validation does not enumerate iterations.

Safe-reference parent/child authority is still validated sequentially under `references.md`; no control-flow authority graph is merged. Lower Core CFG path states, reference authority, external-referent state, and pointer-target metadata remain proving/implementation facts and do not become source loop ownership/origin authority.

## Nested conditional/while composition

A represented `IfStatement` or `WhileStatement` may occur inside any represented child block where `BodyStatement` is admitted.

An inner conditional first establishes its definite local normal successor when one exists; that binding/external-referent/raw-origin state becomes the ordinary state for later statements in the containing loop body before the outer backedge comparison. A no-local-normal inner conditional may instead consist of source-valid return/fault/loop-transfer paths and admits no later sibling in that immediate sequence.

An inner bounded `while` exposes its definite `C` post-loop state to the containing sequence whether it reaches that state through its false condition or an admitted inner break. An inner continue never reaches the outer body continuation; it returns only to the inner condition point. Neither inner transfer targets the outer loop while the inner loop remains the nearest enclosing represented loop.

After an inner loop completes to its definite `C`, later outer-body operations may independently establish the outer `H` or `C` state required by a subsequent outer continue/break.

These compositions require no general source CFG, completion lattice, ownership/origin state-set merge, authority graph merge, or pointer-target lattice beyond the exact relations already defined here.

## Result-bearing and normal-completion boundary

The represented control-flow statements consume the result-bearing and replacement-capable normal-completion requirements from `function-execution.md` and `references.md`.

A conditional whose two explicit arms both terminate the current function activation by represented return and/or explicit fault has no local normal successor and may therefore discharge the remaining result-path obligation without a redundant root terminal return after it. Every normal return must independently satisfy the replacement-capable external-referent restoration law; explicit fault has no such obligation.

A conditional with no local normal continuation because its paths instead perform loop transfers does **not** by itself terminate the function activation and does not independently discharge a result or restoration obligation. Such transfers are source-valid only inside an enclosing represented loop, which consumes their destinations; that `while` still has its represented false normal outcome.

If exactly one conditional arm has a local normal continuation, that sole continuation remains subject to the ordinary result-bearing and eventual normal-completion requirements.

A conditional without explicit else always has the omitted-else normal false outcome and therefore cannot by itself eliminate every local normal path.

A represented bounded `while`, including `while true` and including bodies containing break/continue, always has its represented false normal outcome and therefore cannot by itself eliminate every normal root path or satisfy a missing-result/restoration obligation. A result-bearing path continuing after the loop still requires a source-valid result return before normal root completion; a no-result path reaching normal root fallthrough must satisfy the replacement-capable external-referent restoration law before activation cleanup.

## Conditional source/Core refinement

A faithful source-to-Core lowering MAY refine one source-valid represented conditional to the accepted Bool-valued `Branch` relation in [Core control flow](../core/control-flow.md).

After source validation has fixed the condition type, each arm's local normal-continuation presence, any required two-normal-outcome source structural ownership/external-referent/raw-pointer-origin equality, sequential safe-reference authority validity, and any nested loop-transfer target/cleanup facts, a lowering may:

1. lower the existing condition producer exactly once under that producer's accepted lowering relation;
2. materialize its Bool result in a compiler-owned Core temporary when useful;
3. consume that temporary once as the Core `Branch` condition operand;
4. lower the then arm to one or more Core blocks;
5. lower explicit else-arm blocks when present, or use the following normal continuation directly as the false target when no else body needs lower execution;
6. for each normally completing explicit arm, refine that arm's retained source normal cleanup before its normal successor edge;
7. for each no-local-normal explicit path, preserve its existing non-local refinement: `Return` for a source-valid return, `Fault(F_explicit)` for explicit `fault;`, retained transfer cleanup plus `Goto` to the selected loop header for `continue;`, or retained transfer cleanup plus `Goto` to the selected loop post-loop block for `break;`, without first emitting ordinary normal arm cleanup;
8. when two normal outcomes exist, transfer both to one lower normal join before subsequent source statements;
9. when exactly one normal outcome exists, continue subsequent source lowering from that sole normal path, with or without a dedicated lower join block; and
10. when zero local normal outcomes exist, emit no lower normal join merely to create an unreachable local continuation.

The exact shape or number of Core blocks is not source-observable. A loop transfer's selected Core block is a lower refinement of its source lexical target, not source-semantic block identity.

A compiler temporary used for condition selection is not a source binding. Producer-internal temporaries remain governed by their producer's accepted lowering relation.

Source external-referent structural equality, safe-reference authority validity, and raw-pointer origin equality are validation/refinement facts. A lowerer MUST retain enough information to refine accepted reference and pointer operations, but the conditional join does not require a new Core source-state merge operation, authority graph join, runtime origin object, or physical pointer equality check.

A result-producing return whose producer is a direct call may first create the existing call continuation block and then terminate that continuation with Core `Return`; this does not create a second source continuation.

The stable abstract Core defined-fault reason used for source `ExplicitFault` is selected by `function-execution.md` through the accepted [Core defined faults](../core/faults.md) relation. This conditional owner neither chooses an implementation string/code nor defines a second fault-lowering rule.

## Bounded-while source/Core refinement

A faithful lowering MAY refine one source-valid represented bounded `while` using only the already accepted Core CFG, value, call, safe-reference, pointer, and vacant initialization semantics.

After source validation has fixed exact Bool condition typing, retained condition/body HIR, body local normal-continuation presence/cleanup, any required H-state ordinary backedge structural/external-referent/raw-origin equality, sequential safe-reference authority validity, and all retained loop-transfer target/cleanup facts, a lowering may:

1. terminate the current predecessor with `Goto` to one fresh condition-header block;
2. lower the retained condition producer from that header through its existing lowering relation;
3. after successful condition production, terminate the resulting condition block with one Core Bool `Branch` whose true target is a fresh body-entry block and false target is a fresh post-loop block;
4. lower the ordinary child body from the body entry using its existing block lowering relation;
5. when the body has a local normal continuation, emit its retained ordinary normal cleanup before terminating that normal path with `Goto` back to the condition header;
6. for each admitted `continue;`, emit its retained transfer cleanup and `Goto` the selected nearest loop's condition header without a synthetic normal cleanup/backedge;
7. for each admitted `break;`, emit its retained transfer cleanup and `Goto` the selected nearest loop's post-loop block without a synthetic false condition or normal cleanup;
8. preserve existing `Return`/`Fault`/other terminating lower paths without synthetic normal backedges; and
9. continue lowering following source only from the post-loop block.

A direct-call condition may create its existing call continuation block before the Core `Branch`; the call's result destination and condition temporary remain compiler-owned storage. The loop relation requires no second call or branch operation.

The accepted Core vacant non-replacing `Init` relation and result-bearing direct-call destination relation permit one fixed compiler/source local that is Dead or otherwise wholly vacant on a later cycle to begin a new stored-value lifetime without changing its dynamic storage-instance identity. Lowering may therefore reuse statically allocated body locals and condition/result temporaries across normal or continue cycle visits when Core validation proves the required vacancy/authority. This is a lower proving fact, not source assignment mutability, source ownership/reference restoration, or source pointer-origin inference.

No new Core operation, Core state lattice, source/Core loop flag, runtime moved/origin/authority flag, dynamic slot identity, or hidden lifetime-generation mechanism is required. Existing Core `Goto` is sufficient for both loop transfers after source cleanup/target validity has been retained. Existing Core safe-reference and raw-pointer values preserve their already defined authority/target/provenance through accepted operations; source exact-state validity is established before lowering.

The exact Core block identities/count remain implementation facts. The semantic requirements are the predecessor-before-header edge, per-visit condition execution, correct Branch targets, ordinary normal body cleanup before ordinary backedge, transfer cleanup before transfer `Goto`, nearest-loop target preservation, exact source structural/external-referent/raw-origin target state, sequential safe-reference authority validity, absence of a synthetic ordinary backedge for no-local-normal body paths, and continuation of following source from the post-loop block.

## Lower path states do not define source control flow

Core CFG validation may preserve multiple distinct implementation states at a lower join or cycle even when source enclosing ownership/provenance is definite.

For a conditional, a source local declared only in the then-arm child scope may be represented by one Core local that is Dead after normal then-arm cleanup but Never-initialized on a false execution that never entered that arm. Those distinct lower states remain valid implementation facts under Core control flow.

For a bounded `while`, one static body local or compiler temporary may be Never-initialized before the first body/condition visit and Dead on a later cycle after its prior value was moved/dropped/cleaned. Accepted Core vacant initialization admits the later new lifetime when the selected destination is wholly vacant; source validity still comes only from the source loop and transfer relations above.

These lower differences do not make ended body/arm locals visible after their source scopes and do not create path-dependent source ownership, safe-reference authority, external-referent state, or pointer-origin provenance for continuing source state.

Lowering MUST NOT:

- use Core path-state union/intersection to reconstruct source structural ownership of binding or external-referent roots;
- use Core reference-authority/path state to reconstruct or merge source safe-reference authority graphs;
- use Core raw-pointer target/provenance metadata to reconstruct or merge source pointer origins;
- accept an unequal two-normal-outcome conditional join merely because every lower continuation operation happens to validate under multiple Core states;
- accept a conditional whose raw-pointer origins differ merely because lower pointer values share a Core type or implementation representation;
- accept a bounded-`while` ordinary/continue backedge whose source continuing structural/raw-origin state differs from `H` merely because Core cyclic path states validate;
- accept a break whose source continuing structural/raw-origin state differs from `C` merely because its lower `Goto` validates;
- redirect a source loop transfer to a non-nearest loop based on lower block convenience;
- invent a source local normal successor from lower reachability after source validation determined none;
- infer source normal/transfer cleanup or external-referent restoration from lower scalar liveness; or
- turn Core worklist behavior into source semantic authority.

Source local normal-continuation presence, conditional join validity, bounded-loop ordinary/explicit transfer validity, transfer target selection, and definite source successor structural/external-referent/raw-origin states are established before lowering by this document, `function-execution.md`, `references.md`, `local-bindings.md`, `structural-ownership.md`, and `raw-pointers-unsafe.md`.

## Determinism

For one fixed source-valid represented program and one fixed activation state, conditional and bounded-`while` runtime selection and bounded loop-transfer target selection are deterministic.

The condition producer has its existing deterministic or otherwise accepted source behavior. Once it yields one of the two semantic Bool values, exactly one runtime outcome is selected. A selected conditional arm or `while` body may normal-complete, return, explicitly fault, execute the uniquely selected nearest-loop break/continue transfer, yield another defined fault, or diverge according to its existing owners. A false `while` condition selects only the post-loop continuation; a normal true body or continue returns only to the selected loop's next condition evaluation; break reaches only the selected loop's post-loop continuation.

Validation of both represented conditional outcomes, the represented `while` false/body relations, every exact external-referent structural equality, every exact pointer-origin equality, every sequential safe-reference authority requirement, and every explicit transfer target-state relation is a static validity obligation, not runtime nondeterminism.

## Further boundaries

This revision does not define general expressions, grouping expressions, comparisons, logical operators, arithmetic operators, truthiness, coercions, standalone record-construction conditions, direct safe-reference/raw-pointer condition forms, conditional values/expressions, direct `else if`, unrestricted nonterminal-within-block return or arbitrary unreachable tails, additional loop forms (`loop`, `for`, do/while), loop `else`, loop values, match, refutable patterns, fault payloads, panic/throw syntax, catch/recovery, labels or a label namespace, labeled transfer, break/continue values, transfer to a non-nearest loop, source state/completion lattices, pointer-origin sets/unions, generic safe-authority graph joins, general loop fixed-point inference, path-dependent ownership/origin after a two-normal-outcome join or bounded-while target, automatic join/backedge/transfer normalization, drop flags, custom destructors, must-consume policy, additional reference/borrow forms beyond `references.md`, raw-pointer operation semantics, named/non-lexical lifetime inference, optimizer transformations, ABI/linkage, backend branches, Exec, Model, or stable serialized HIR/Core control-flow identity.

Those concerns require their own accepted owners or later extensions and MUST NOT be inferred from the represented conditional, bounded-`while`, or bounded unlabeled loop-transfer relations here.