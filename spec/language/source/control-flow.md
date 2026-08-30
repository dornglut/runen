# Source Control Flow

Status: **provisional normative; incomplete**

This document owns the represented source semantics for statement-level conditional and bounded-loop control flow: condition admission and selection, validation of represented outcomes, explicit child lexical-scope composition, conditional omitted-else behavior, conditional local normal-continuation composition, bounded-`while` backedge admission, bounded unlabeled `break`/`continue` target and target-state admission, definite structural ownership and raw-pointer-origin provenance at represented normal successors, and the source-to-Core control-flow refinement boundary.

It consumes represented source type identity and the intrinsic `Bool` type from [Source type foundation](types.md); producer-backed field-value result typing and validation from [Source field-value access](field-access.md); owned-value producers, producer evaluation, lexical-block execution, local normal-continuation presence, return execution, explicit-fault execution, loop-transfer cleanup, lexical cleanup, defined-fault propagation, bounded-loop body execution sequencing, and divergence from [Source function execution](function-execution.md); binding identity, lexical scope, lookup, assignment mutability, binding structural lifecycle, and raw-pointer local integration from [Source function-local bindings](local-bindings.md); structural ownership state from [Source structural ownership](structural-ownership.md); exact raw-pointer origin provenance and lexical target validity from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md); represented Core Bool branching and CFG path-state validity from [Core control flow](../core/control-flow.md); and vacant non-replacing initialization/result-destination admission from [Core value and storage semantics](../core/value-storage.md) and [Core functions](../core/functions.md). Concrete `if`/`else`/`while`/`break`/`continue` spelling and the represented `ConditionalValue` grammar are owned by [Source concrete syntax](concrete-syntax.md).

This document does not redefine owned-value producer semantics, field-receiver semantics, return execution, explicit-fault execution, structural path/state mathematics, raw-pointer origin production/transport/unsafe semantics, binding scope/mutability rules, lexical cleanup order, fault cleanup, loop-transfer cleanup ordering, Core path state, Core value/storage semantics, or concrete grammar.

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

Concrete syntax deliberately excludes a **standalone** record construction from `ConditionalValue`; that grammar restriction is owned by `concrete-syntax.md`. A bounded producer-backed `FieldValueUse` whose receiver is a record construction remains a distinct admitted field-value producer because the mandatory selector is part of that complete field-value spelling. The first raw-pointer slice likewise keeps raw-address and raw-move forms out of `ConditionalValue`; this semantic owner does not widen either grammar restriction from parser lookahead or type information.

An admitted producer-backed `FieldValueUse` condition MUST have final selected field type exactly `Bool` under `field-access.md`. That requirement applies to the complete field-value result; the internal direct-call or record-construction receiver retains its independently selected exact receiver type.

A syntactically represented conditional value whose resolved/produced type is not exactly `Bool` is source-invalid.

The same exact admission relation is consumed by both represented `IfStatement` and `WhileStatement`. `while` does not add a second condition grammar or type rule.

## Condition validation state

Source validation of one represented conditional begins in the enclosing function-local binding environment that exists immediately before the condition producer.

Validate the condition through its existing producer owner with exact required source type `Bool`.

A producer-backed `FieldValueUse` condition consumes the field-value owner's existing validation transaction with exact required final type `Bool`; this conditional relation adds no separate receiver-validation or ownership-commit rule.

During source validation, apply each semantic ownership consequence selected by that condition producer exactly once before conditional outcome state splitting.

The resulting enclosing binding environment is the **post-condition environment**. For every active raw-pointer local, that environment also carries the exact pointer-origin provenance already established by `raw-pointers-unsafe.md`. The represented first-slice `ConditionalValue` forms do not directly retarget raw-pointer locals, but the origin remains part of the definite enclosing state consumed below.

The explicit then arm and, when present, the explicit else arm are each source-validated from semantically identical copies of that same post-condition environment. When else is omitted, the false normal outcome is the unchanged post-condition environment as defined below.

The successful condition result is one owned Bool transient held by the conditional operation for branch selection. That transient is not a function-local binding and is not a member of the post-condition environment. Any ownership consequences that condition production applied to pre-existing bindings are already reflected in the post-condition environment.

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
2. preserve every ownership transition, transient consequence, fault possibility, and divergence consequence of that producer evaluation;
3. on successful production, hold exactly one owned Bool condition transient;
4. consume that transient for conditional selection;
5. when its Bool value is `true`, execute only the then arm;
6. when its Bool value is `false` and an explicit else arm exists, execute only that else arm; and
7. when its Bool value is `false` and no explicit else arm exists, take the omitted-else normal false outcome defined below.

A producer-backed field-value condition reaches step 3 only after its complete producer-specific field-receiver lifecycle has finished under `function-execution.md`; this conditional relation does not define a second receiver lifecycle.

Conditional selection itself performs no additional binding read, move, duplicate, assignment, pointer retargeting, structural consumption, cleanup, call, fault selection, return, loop transfer, or hidden source state transition beyond ending ownership of the successful Bool condition transient used for selection.

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
- no lexical-scope, activation, conditional, or loop-transfer cleanup occurs merely because execution remains suspended.

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
- ordinary locals, record-pattern bindings, nested blocks, assignments, raw-pointer operations inside applicable unsafe blocks, calls, explicit fault statements, represented bounded `while`, source-valid bounded `break;`/`continue;`, and the arm's optional terminal return retain their existing semantics; and
- nested represented conditionals may occur because `IfStatement` is itself a represented body statement inside an arm block.

This document does not create an abstract source-visible branch-scope identity beyond the ordinary child lexical scopes already owned by `local-bindings.md`.

## Explicit arm normal completion

When an explicit arm has a local normal continuation and the selected execution reaches that normal completion:

1. finish its contained body-statement sequence under `function-execution.md`;
2. normally exit that child lexical scope;
3. clean bindings declared directly in that arm using the existing lexical-scope cleanup relation; and
4. only after that cleanup does the arm produce its **normal enclosing outcome** for conditional-successor purposes.

Arm-local bindings have ended before normal-successor comparison and therefore are not members of the enclosing environment compared when two normal outcomes meet.

Normal cleanup of one arm does not itself clean, reset, or retarget enclosing bindings merely to make their states match another arm.

## Explicit arm return and explicit fault

When source validation determines that an explicit arm has no local normal continuation, that arm contributes no normal enclosing outcome to the conditional. Return and explicit fault are activation-terminating ways to establish such an outcome; bounded loop transfers are handled separately below.

At runtime, when the selected arm reaches a represented return:

- the return terminates the current source function activation under `function-execution.md`;
- the arm does not first perform independent normal child-scope cleanup;
- every then-active lexical scope participates exactly once in return-induced activation cleanup; and
- no conditional normal join or normal successor is taken on that execution.

A returning path may consume the complete root or an arbitrary source-valid structural subvalue of an enclosing binding while evaluating its return value and may contain source-valid pointer retargeting before return. That returning state does not have to equal a normal sibling outcome merely to make the conditional source-valid. Return cleanup uses the returning path's actual then-current structural ownership and pointer-origin state.

At runtime, when the selected arm reaches represented `fault;`:

- the statement selects the distinguished source defined-fault reason `ExplicitFault` under `function-execution.md`;
- the arm does not first perform independent normal child-scope cleanup;
- every then-active lexical scope participates exactly once in the existing activation fault cleanup;
- completed ownership/provenance transitions before the statement remain effective; and
- no conditional normal join or normal successor is taken on that execution.

A path that explicitly faults may therefore reach the fault statement with an enclosing binding fully available, partially consumed, or unavailable and with any source-valid then-current raw-pointer origins according to preceding operations. That state does not have to equal a normal sibling outcome because the explicitly faulting path contributes no normal successor state.

These rules do not introduce path-dependent ownership/origin at a normal successor because activation-terminating paths contribute no normal successor state.

## Explicit arm loop transfer

A source-valid `break;` or `continue;` reached inside an explicit conditional arm likewise gives that execution no **local** normal arm outcome, but it does not terminate the function activation merely for that reason.

The transfer statement independently selects the nearest enclosing represented `while` and MUST satisfy the applicable exact target-state rule below, including raw-pointer origins, before the containing conditional can be source-valid. Its exited-scope cleanup is owned by `function-execution.md` and includes the active arm scope plus every intervening active child scope through the target loop body scope.

The transfer path contributes no normal enclosing environment to the conditional's ordinary local successor composition. Consequently:

- one transfer arm and one locally normal arm leave the normal arm as the conditional's sole normal outcome;
- both explicit arms transferring leave the conditional with no local normal continuation, including when one arm breaks and the other continues;
- an omitted else remains one normal false outcome, so a transfer-only then arm with omitted else leaves that false outcome as the sole normal successor; and
- a transfer arm's enclosing structural/pointer-origin state is not compared with a normal sibling merely to form a conditional join, because the transfer has already proved exact validity at its loop target.

The conditional does not merge break and continue destinations, create an abrupt-completion lattice, or reconstruct their target facts from lower CFG edges. Their distinct retained statements and nearest-loop lexical context remain sufficient for faithful lowering.

## Producer-originating arm fault and divergence

If execution of an accepted producer or other operation inside the selected arm yields a defined fault before the represented successful statement structure reaches its local normal continuation or loop transfer, that concrete execution produces no normal enclosing outcome.

The active child scope participates exactly once in the existing activation fault cleanup from `function-execution.md`. It does not first perform independent normal arm cleanup and then fault cleanup.

Such a producer-originating fault possibility does **not** by itself remove the arm's static local normal continuation. Static completion follows the represented statement/control structure under `function-execution.md`; a producer that may fault dynamically still contributes its ordinary successful continuation when one is represented.

If execution inside the selected arm diverges, that concrete execution produces no runtime normal outcome and performs no normal arm, successor, or loop-transfer cleanup merely because execution remains suspended.

Divergence likewise does not alter static local normal-continuation presence under this revision. The represented explicit `fault;`, `break;`, and `continue;` statements are different: successful execution of each has no local fallthrough by definition.

## Omitted else

When no explicit else arm is present, runtime `false` selects the **omitted-else normal false outcome**.

That outcome:

- executes no body statement;
- introduces no lexical binding;
- creates no source-visible synthetic lexical scope;
- performs no cleanup; and
- carries every enclosing binding's structural ownership state and every enclosing raw-pointer binding's exact pointer origin exactly as they exist in the post-condition environment.

For normal-successor validation, the omitted-else false outcome is therefore the unchanged post-condition enclosing environment and always contributes one normal outcome.

An explicit `else {}` is an actual child lexical scope, even though its empty body yields the same enclosing structural ownership and pointer-origin outcome.

## Enclosing environment at a conditional successor

Let `E` be the post-condition function-local binding environment.

`E` contains exactly the active parameter/local binding identities that remain in the enclosing lexical environment after successful condition production. Condition evaluation cannot introduce one new function-local binding because represented conditional values are owned-value producers rather than declarations.

For each binding, `E` retains declared type, assignment-mutability classification, and structural ownership state. For every binding of exact type `RawPtr(T)`, `E` additionally retains the exact `PointerOrigin(binding)` provenance established by `raw-pointers-unsafe.md`.

Each explicit arm is source-validated from its own copy of `E`. An omitted else uses unchanged `E` as its normal false outcome.

After normal completion and local cleanup of an explicit arm that has a local normal continuation, its normal outcome contains only the binding identities belonging to `E`. An arm with no local normal continuation contributes no enclosing environment for normal-successor composition; its return, explicit fault, or loop transfer follows its own target relation.

Binding identity, declared type, and assignment-mutability classification are unchanged by conditional branching itself. Where two normal outcomes meet, the conditional therefore compares each enclosing binding's structural ownership state and, for every enclosing raw-pointer binding, its exact pointer-origin provenance.

## Conditional local normal-continuation composition

After both represented outcomes have been source-validated, compose only their **local normal** outcomes:

- **two normal outcomes:** the conditional has a normal successor only when the exact structural-ownership-state equality rule and exact raw-pointer-origin equality rule below succeed for every applicable binding in `E`; the equal common state is the one definite normal successor;
- **exactly one normal outcome:** that sole outcome is the conditional's one definite normal successor without any structural/origin equality comparison against the no-local-normal outcome;
- **zero normal outcomes:** the conditional has no local normal continuation and no normal join; and
- **omitted else:** the false outcome is always normal, so a conditional without explicit else always has at least one normal outcome.

This is only composition of the local-fallthrough relation from `function-execution.md`. It introduces no source state set, completion lattice, maybe-owned state, maybe-origin state, implicit cleanup edge, or runtime completion tag.

A sole normal outcome is not a union, intersection, widening, or normalization of branch states. It is exactly the enclosing environment produced by that one normally completing outcome after its ordinary arm-local cleanup.

A zero-local-normal conditional does not by itself imply function-activation termination. It may be activation-terminating when all paths return/fault, or it may occur inside a represented loop where all paths perform source-valid loop transfers. The enclosing control owner determines those destinations.

## Exact structural-ownership state equality for two normal outcomes

For one represented enclosing binding, two normal outcomes have equal structural ownership state exactly when their prefix-free consumed-path sets under `structural-ownership.md` are equal.

When **two** normal outcomes meet, a represented conditional has a valid normal successor only when every binding identity in `E` has equal structural ownership state on those two outcomes.

When all enclosing binding states are equal:

- that common state is the one definite structural ownership state of the binding at the normal continuation; and
- subsequent source validation proceeds through the existing single-state binding relation in `local-bindings.md` and `structural-ownership.md`.

When two normal outcomes exist and any enclosing binding has unequal normal outcome states, the conditional statement is source-invalid at its normal join boundary. No normal post-conditional ownership state is established.

The equality requirement is semantic equality of source structural state. It is not equality of runtime scalar values, record values, Core local state, parser nodes, HIR data structures, compiler hashes, or physical storage.

No equality comparison is performed between a normal outcome and an outcome that has no local normal continuation. A return, explicit fault, or loop transfer instead obeys its independently established destination/cleanup relation.

## Exact raw-pointer-origin equality for two normal outcomes

For one enclosing binding of exact type `RawPtr(T)`, two normal outcomes have equal pointer-origin state exactly when the stored raw-pointer value on each outcome carries `PointerOrigin(x)` for the same source binding identity `x` under `raw-pointers-unsafe.md`.

When **two** normal outcomes meet, a represented conditional has a valid normal successor only when every raw-pointer binding identity in `E` has equal pointer origin on those outcomes in addition to satisfying structural ownership equality.

When the origins are equal, that exact origin is the one definite pointer-origin provenance at the normal continuation. When any continuing raw-pointer binding has different origins, the conditional is source-invalid at its normal join boundary even if both pointer values have the same raw-pointer type and both target bindings would individually satisfy lexical target validity.

No pointer-origin set, alternative-origin union, maybe-origin value, runtime origin tag, lower Core pointer-target comparison, or physical-address equality substitutes for exact source binding-origin equality.

No origin comparison is performed against an outcome that has no local normal continuation; return, fault, or loop transfer instead carries its actual then-current origin state into its independently established destination/cleanup relation.

## Definite-state consequences

The exact-state rules for two normal outcomes have these required consequences.

### Equal complete consumption

If both normal outcomes consume the complete root of the same enclosing non-duplicable binding and do not reinitialize it, both consumed-path sets contain the complete root path and the binding may join as unavailable.

If two normal outcomes exist and one consumes the complete root while the other leaves it fully available, the states differ and the conditional is source-invalid.

A no-local-normal outcome is not compared against a normal sibling. Return/fault cleanup or a source-valid loop-transfer target relation consumes that path's actual then-current state as applicable.

### Equal partial consumption

If both normal outcomes consume exactly the same represented nested structural paths of one enclosing binding, the equal resulting consumed-path set may join.

If two normal outcomes leave different consumed sibling or nested paths, the states differ and the conditional is source-invalid.

No structural similarity, equal remaining-frontier shape, or equal number of consumed paths substitutes for exact consumed-path-set equality.

A no-local-normal outcome may independently reach its return/fault/loop-transfer destination after different valid structural operations; the applicable destination relation handles that path rather than normalizing it toward a normal sibling.

### Duplicable uses

A represented non-consuming duplicate use leaves the consumed-path set unchanged under `structural-ownership.md` and therefore does not by itself prevent an equal-state join when two normal outcomes meet.

Duplicating a raw-pointer value likewise preserves its exact pointer origin and therefore does not by itself prevent an exact-origin join.

### Assignment and reinitialization

Whole-binding assignment retains its existing source-first replacement semantics.

A successful assignment may begin from a fully available, partially available, or unavailable binding state and ends by establishing fresh complete structural ownership for the replacement value.

Consequently, two normal arms with different earlier ownership histories may still produce equal normal ownership states when accepted whole-binding assignments explicitly re-establish the same complete-state classification before normal arm completion.

For a mutable raw-pointer local, accepted ordinary assignment additionally installs the incoming exact pointer origin. Two normal arms may therefore retarget that pointer during their execution but can join only if the final normal origins are exactly equal. Assignment to two different target bindings does not join merely because both targets have the same source type.

The conditional does not special-case assignment and does not require runtime values assigned by different arms to be equal.

### Branch-dependent runtime values

Two normal outcomes may produce different runtime values in the same mutable enclosing binding while still having equal structural ownership state.

For a raw-pointer binding they may likewise produce different implementation-level pointer encodings only when the source-defined pointer origin remains the same; source pointer equality/representation is not defined. Different source pointer origins do not join.

Structural ownership/provenance definiteness is not runtime value equality, constant propagation, or SSA value merging.

### Omitted else

With no explicit else arm, the false normal outcome is the unchanged post-condition state.

When the then arm also completes normally, its normal enclosing structural ownership state MUST equal that unchanged post-condition state for every enclosing binding, and every enclosing raw-pointer binding's origin MUST equal its unchanged post-condition origin. Therefore a normally completing then arm may not leave one enclosing binding consumed/partially consumed or one raw-pointer local retargeted unless accepted operations inside the then arm restore the applicable post-condition state before normal completion.

When the then arm has no local normal continuation, the omitted-else false outcome is the sole normal successor and no structural/origin equality is required against that then arm. The then path independently obeys return, fault, or loop-transfer validity as applicable.

### Zero-field and zero-leaf values

Zero-field and recursively zero-leaf source values participate in consumed-path-state equality exactly like other structural owned values when two normal outcomes meet.

Their source ownership state MUST NOT be inferred from whether a Core representation has a scalar destruction leaf or emits a physical destruction operation.

## Post-successor source operations

After a conditional with one valid normal successor, every represented enclosing binding has exactly one committed structural ownership state: either the equal state of two normal outcomes or the exact state of the sole normal outcome. Every continuing raw-pointer binding likewise has exactly one committed pointer origin by the same two-outcome/sole-outcome rule.

Subsequent whole-binding use, field-value use, record-pattern use, assignment, raw-pointer operation, nested conditional, bounded `while`, lexical cleanup, return cleanup, loop transfer, or fault cleanup consumes those ordinary definite states through their existing source owners.

No post-successor operation performs a second conditional-state analysis merely because the value was previously used inside a represented conditional.

A conditional with zero local normal outcomes admits no subsequent statement in the same containing sequence under `function-execution.md`; any source-valid non-local destinations are already represented by its selected return/fault/loop-transfer paths.

## No implicit join normalization

When two normal outcomes meet, this conditional relation does not derive a common state by:

- union of consumed paths or pointer origins;
- intersection of consumed paths or pointer origins;
- prefix minimization;
- meet, join, widening, or another lattice operation;
- automatically consuming still-owned values on one branch edge;
- automatically retargeting a raw-pointer local on one branch edge;
- inventing a maybe-owned or maybe-origin state; or
- consulting lower Core liveness or pointer-target metadata.

In particular, source-invalid unequal two-normal-outcome states are not made valid by silently cleaning additional owned subvalues or retargeting a raw pointer on an outcome that retained a different state.

When exactly one normal outcome exists, using that outcome directly is not normalization and performs no branch-edge cleanup/retargeting.

This avoids introducing path-specific source destruction or pointer-retarget timing as an implicit consequence of merely reaching a branch successor.

## No conditional ownership, provenance, or completion flags

This revision introduces no source drop flag, runtime moved-state flag, hidden path tag, conditional cleanup bit, runtime pointer-origin tag, runtime completion tag, or source-visible ownership/provenance test.

Every binding that reaches a represented normal continuation has one definite structural ownership state fixed statically by the rules above, and every raw-pointer binding has one exact pointer origin. Loop-transfer destinations likewise require one exact statically established target state rather than a runtime ownership/provenance classification.

Future source control-flow forms or later extensions may introduce additional accepted relations only through their own normative changes. Their existence MUST NOT retroactively alter the semantics of two-normal-outcome conditionals already accepted by the exact-state relation.

## Constant conditions

A concrete source execution with condition value `true` executes only the then arm. A concrete source execution with condition value `false` executes only the false outcome.

Source validation nevertheless validates both represented outcomes and computes each arm's local normal-continuation presence independently of that concrete Bool value. When both outcomes are normal, the exact structural/origin equality requirements apply to both even for a constant condition. When only one outcome is normal, it is the sole static normal successor even if a particular constant Bool execution selects a no-local-normal return/fault/transfer outcome instead.

This difference between runtime selection and conservative source validation does not introduce nondeterministic source execution.

## Nested conditionals

A represented conditional may occur inside any explicit arm block because it is a body statement under `concrete-syntax.md`.

The inner conditional independently establishes zero or one definite local normal successor by the composition rules above. If it has one, the containing arm may continue from exactly that structural/pointer-origin state. If it has none, no later statement or terminal return in that same arm sequence is source-valid; any represented return/fault/loop-transfer destination remains independently controlling.

Consequently, nested conditionals compose recursively through local normal-continuation presence without a general source CFG or state-set relation.

This revision defines no direct `else if` grammar. Equivalent nested selection may be written only through the represented explicit block nesting admitted by `concrete-syntax.md`.

## Represented bounded while

One represented source `while` statement consists of exactly one represented `ConditionalValue` and exactly one explicit body `BlockStatement` under `concrete-syntax.md`.

The `while` is statement-only. It produces no source value, introduces no Unit/Void value, and is not an owned-value producer. It has no represented `else`, label, result value, iteration binding, pattern condition, iterator protocol, or unconditional-loop form. Its body may contain the bounded unlabeled `break;` and `continue;` statements defined below.

The condition consumes the exact same admission/type/producer relation defined above: its result type MUST be exactly intrinsic `Bool`, standalone record construction and direct raw-pointer value forms remain excluded by grammar, and all existing producer validation and transactional ownership rules apply unchanged.

## While validation environments H and C

Let `H` be the complete enclosing function-local binding environment immediately before validation of the loop condition. `H` includes every active enclosing parameter/local binding identity, its declared type and assignment-mutability classification, its current structural ownership state, and for every raw-pointer local its exact pointer origin.

Validate the loop condition through its existing producer owner from a copy of `H`, requiring exact intrinsic `Bool`. A failed condition validation makes the `while` source-invalid and commits no speculative condition ownership/provenance change to the surrounding environment.

On successful condition validation, apply the condition producer's selected source-state consequences exactly once to that validation copy. Call the resulting enclosing environment `C`.

`C` is the state after one successful condition production and before runtime Bool selection. The successful Bool condition transient is not a function-local binding and is not part of `C`.

The represented false outcome is always one static normal outcome carrying exactly `C`, including exact raw-pointer origins. It introduces no body scope, performs no body cleanup, and establishes the loop's definite post-loop environment when source validation succeeds.

The represented true outcome validates the explicit body from a semantically identical copy of `C` as one ordinary child lexical block under `local-bindings.md` and `function-execution.md`.

Condition evaluation introduces no new function-local binding identity. Consequently every enclosing identity in `H` is present in `C` and remains present after ordinary normal completion/cleanup of the body or after an admitted loop transfer. Body-local bindings end before ordinary backedge comparison or as part of transfer cleanup and are not target-state dimensions.

## While normal-backedge admission

If the body has a local normal continuation, the `while` admits that normal outcome as a backedge **only** when every enclosing binding identity from `H` has exactly the same structural ownership state after ordinary body-scope normal cleanup as it had in `H`, and every enclosing raw-pointer binding has exactly the same pointer origin as it had in `H`.

For structural ownership, equality is exactly equality of the prefix-free consumed-path set under `structural-ownership.md`. For raw-pointer origin, equality is exactly the `PointerOrigin(x)` identity relation from `raw-pointers-unsafe.md`. Binding identity, declared source type, and assignment-mutability classification are required to remain the same enclosing facts and are not reconstructed or merged.

The comparison is deliberately against `H`, not `C`. The condition will execute again after a normal backedge; therefore the body must restore whatever enclosing structural ownership and raw-pointer-origin state the next condition evaluation requires to begin from the same accepted loop-head state. A condition may itself transform `H` to a different `C` each iteration through its existing producer consequences.

If any enclosing binding's normal post-body structural ownership state or raw-pointer origin differs from its applicable state in `H`, the `while` is source-invalid with no admitted ordinary backedge and no committed post-loop state.

This is exact source-state equality, not equality of runtime values. A mutable enclosing ordinary binding may therefore hold a different runtime value after the body while still satisfying the backedge when its structural ownership state equals `H`. A mutable raw-pointer local may be temporarily retargeted inside the body but must have its exact `H` origin restored before a normal backedge.

A successful represented whole-binding assignment may explicitly restore complete ownership or a required raw-pointer origin before the backedge because assignment already establishes fresh complete structural state and, for a raw-pointer target, the incoming exact pointer origin under `local-bindings.md`. The loop adds no special restoration operation. An immutable binding receives no implicit restoration and cannot be assigned merely to satisfy the loop invariant.

A body-local binding never participates in this equality because ordinary normal body-scope cleanup ends it before comparison. Re-entering the same static body on a later dynamic iteration does not create a new source binding identity; it creates a new dynamic value owned by that same static binding identity while the child scope is active. The lexical pointer-target-validity rule from `raw-pointers-unsafe.md` prevents a longer-lived enclosing raw-pointer local from retaining an origin naming such a shorter-lived body-local instance.

## Nearest enclosing loop target

Every source-valid `break;` or `continue;` is lexically contained in at least one represented `while` body.

The target is exactly the nearest enclosing represented `while` whose body lexical scope contains the transfer point. Ordinary nested blocks, unsafe blocks, and conditional arm scopes do not become transfer targets. While execution is inside a nested represented loop, that inner loop is the target of an unlabeled transfer from its body and shadows any outer loop for this purpose.

Target selection is a static source lexical/control fact. It is independent of parser node identity, HIR/Core block numbering, dynamic iteration count, runtime stack layout, or physical branch structure.

An occurrence with no enclosing represented `while` is source-invalid. This revision defines no label namespace, labeled transfer, dynamic target selection, or transfer to a non-nearest enclosing loop.

## Loop-transfer exited scopes and cleanup boundary

For one admitted transfer to target loop `L`, the exited lexical scopes are every then-active source lexical scope from the scope containing the transfer point outward through and including `L`'s body lexical scope, stopping before the lexical scope containing the `while` statement itself.

Thus a transfer directly in the loop body exits exactly that body scope; a transfer in nested blocks, unsafe blocks, or a conditional arm exits those active descendant scopes innermost-first and then the target body scope; and an inner-loop transfer does not exit the outer loop body.

`function-execution.md` owns the actual cleanup ordering and remaining-frontier cleanup of those exited scopes. The transfer-state comparisons below concern only the enclosing identities represented by the selected loop's `H`/`C`; body/descendant locals are not target-state dimensions and receive no separate normalization rule.

Transfer cleanup MUST NOT consume, reset, retarget, or otherwise change an enclosing `H`/`C` binding merely to force target-state equality. Any required restoration of such a binding must have been established by an ordinary accepted source operation before the transfer.

## Continue admission

For nearest target loop `L`, let `H_L` be that loop's already-defined loop-head environment.

After the source-valid operations preceding the transfer, and independently of cleanup of body/descendant locals, `continue;` is admitted only when every binding identity belonging to `H_L` has exactly the same structural ownership state as in `H_L` and every raw-pointer binding belonging to `H_L` has exactly the same pointer origin as in `H_L`.

The equality relations are exactly the same prefix-free consumed-path-state and exact pointer-origin equalities used for an ordinary normal backedge. Binding identity, declared type, and assignment-mutability classification remain the same enclosing facts.

If any enclosing structural state or raw-pointer origin differs, the `continue;` is source-invalid. The language does not repair the state through implicit cleanup, assignment, retargeting, reset, union, intersection, widening, or a runtime ownership/provenance flag.

A mutable enclosing ordinary binding may contain a different runtime value while satisfying the target relation because target equality is structural ownership equality, not runtime value equality. A mutable raw-pointer local may likewise be retargeted temporarily only when its exact required origin is restored before the transfer. A successful ordinary whole-binding assignment may restore complete ownership or pointer origin before the transfer when source-valid. An immutable binding receives no implicit restoration.

## Break admission

For nearest target loop `L`, let `C_L` be that loop's already-defined successful post-condition environment and definite post-loop environment.

After the source-valid operations preceding the transfer, and independently of cleanup of body/descendant locals, `break;` is admitted only when every enclosing binding identity belonging to `C_L` has exactly the same structural ownership state as in `C_L` and every raw-pointer binding belonging to `C_L` has exactly the same pointer origin as in `C_L`.

If any enclosing structural state or raw-pointer origin differs, the `break;` is source-invalid. No implicit edge cleanup/reset/retargeting/normalization is performed to force equality.

The target is deliberately `C_L`, not `H_L`. The existing `while` relation already establishes `C_L` as the one definite structural-ownership/raw-pointer-origin environment after the loop's normal false exit. Requiring every explicit break path to establish exactly that same state preserves one definite post-loop source environment without adding a join, union, intersection, maybe-owned/origin state, or runtime flag.

Runtime values in mutable bindings need not equal values from a condition-false execution when the required source state equals `C_L`. `break;` does not synthesize or claim that the condition evaluated false.

## Loop-transfer execution

On successful dynamic execution of an admitted `continue;`:

1. perform the exited-scope cleanup owned by `function-execution.md` exactly once;
2. transfer to the selected loop's condition point; and
3. let the ordinary `while` relation perform the next condition evaluation exactly once.

`continue;` itself does not evaluate the condition, produce/consume a Bool, or create a source-visible iteration identity.

On successful dynamic execution of an admitted `break;`:

1. perform the exited-scope cleanup owned by `function-execution.md` exactly once; and
2. transfer directly to the selected loop's existing post-loop continuation with source environment `C_L`.

`break;` does not re-evaluate the condition and does not synthesize a false condition result. Its execution may therefore have different effects from a later condition-false exit even though both reach the same definite structural ownership/pointer-origin environment.

Both statements have no local fallthrough in their immediate source sequence. They are not return/fault termination of the current activation; the selected enclosing loop consumes their non-local destination.

## While body with no local normal continuation

If the represented body has no local normal continuation under `function-execution.md`, it contributes no ordinary normal backedge state and requires no body-wide equality comparison against `H`.

Return and explicit-fault termination use the actual then-current body/enclosing structural/provenance states through their existing activation cleanup relations. An admitted `break;` or `continue;` path instead must already have satisfied its own exact `C` or `H` target-state relation and performs transfer cleanup rather than ordinary body normal cleanup.

A producer-originating defined fault or divergence inside an otherwise locally normally continuable body remains a dynamic possibility and does not remove that body's represented local normal continuation. The ordinary backedge equality requirement still applies to the successful normal body path.

Regardless of whether the body has a local normal continuation, the represented false condition outcome remains a static normal successor with environment exactly `C`.

## While validation does not constant-prune

A represented `while` always retains its static false normal outcome, including when the condition is the literal `true`.

Source validation therefore does not infer that `while true` eliminates following code or the root normal result obligation. The body is validated even for literal `false`, and the false outcome remains represented even for literal `true`.

This conservative static relation does not assert runtime nondeterminism: at runtime a successfully produced Bool still selects exactly its semantic value.

No source constant propagation, symbolic execution, unreachable-loop-exit weakening, or value lattice is required for this rule.

## While dynamic execution

For a source-valid represented `while`, runtime execution follows `function-execution.md` and the condition producer's existing owner:

1. evaluate the condition from the current loop-head dynamic state;
2. preserve every ownership/provenance transition, transient consequence, defined-fault possibility, and divergence consequence of that evaluation;
3. on successful Bool production, consume the condition transient for selection;
4. if false, take the post-loop normal continuation without activating the body scope;
5. if true, activate and execute the ordinary child body block;
6. if the body completes normally, perform its ordinary normal child-scope cleanup and then repeat from condition evaluation;
7. if the body executes an admitted `continue;`, perform its transfer cleanup and repeat from condition evaluation without a second normal body cleanup;
8. if the body executes an admitted `break;`, perform its transfer cleanup and continue after the selected loop without re-evaluating the condition or performing normal body cleanup;
9. if the body returns or explicitly faults, follow the existing activation termination relation without a separate normal body cleanup/backedge; and
10. if condition/body execution faults through another accepted producer or diverges, preserve the existing fault/divergence semantics without inventing a normal backedge or exit.

Each successful normal or continue backedge therefore reaches one fresh condition evaluation. A direct-call condition performs a fresh dynamic call on each such visit to the condition point; no loop-invariant hoisting or memoization authority is implied.

## While post-loop environment

For every source-valid represented `while`, the definite normal post-loop environment is exactly `C`, the environment after successful condition validation and before false selection.

The backedge-restored `H` state is **not** the post-loop state. A condition producer's ownership/provenance effects remain effective on the false execution that exits the loop.

An admitted explicit `break;` also reaches the post-loop continuation only after proving exact structural ownership and raw-pointer-origin equality with `C`. The break path need not have executed a false condition and may carry different runtime values/effects while preserving that source state.

No ordinary body state is merged into `C`: a normal body state must already equal `H` to be admitted as a backedge, while a no-local-normal body contributes no ordinary backedge state. A break path independently equals `C`; a continue path independently equals `H`. Subsequent source operations consume `C` directly through their existing single-state owners.

This distinction is required for condition producers that consume or otherwise transform enclosing source state before producing Bool. It is not observable for state dimensions a condition leaves unchanged, though condition execution/effects remain semantically distinct from explicit break.

## No loop state lattice or fixed point

The bounded `while` and loop-transfer relations require no source ownership/origin state set, may-be-owned/origin state, join/meet, widening, fixed-point iteration, source CFG, SSA construction, runtime moved/drop/origin/iteration/completion flag, implicit backedge/transfer cleanup/retargeting of enclosing values, or hidden lifetime generation.

Validation checks one exact condition transition `H -> C`, one body validation from `C`, one exact source-state equality proof from each ordinary normal cleaned-body outcome back to `H`, and one exact `H`/`C` proof at each represented continue/break transfer respectively. Repeated runtime execution is justified by those invariants; source validation does not enumerate iterations.

Lower Core CFG path states and pointer-target metadata remain proving/implementation facts and do not become source loop ownership/origin authority.

## Nested conditional/while composition

A represented `IfStatement` or `WhileStatement` may occur inside any represented child block where `BodyStatement` is admitted.

An inner conditional first establishes its definite local normal successor when one exists; that structural/pointer-origin state becomes the ordinary state for later statements in the containing loop body before the outer backedge comparison. A no-local-normal inner conditional may instead consist of source-valid return/fault/loop-transfer paths and admits no later sibling in that immediate sequence.

An inner bounded `while` exposes its definite `C` post-loop state to the containing sequence whether it reaches that state through its false condition or an admitted inner break. An inner continue never reaches the outer body continuation; it returns only to the inner condition point. Neither inner transfer targets the outer loop while the inner loop remains the nearest enclosing represented loop.

After an inner loop completes to its definite `C`, later outer-body operations may independently establish the outer `H` or `C` state required by a subsequent outer continue/break.

These compositions require no general source CFG, completion lattice, ownership/origin state-set merge, or pointer-target lattice beyond the exact relations already defined here.

## Result-bearing function boundary

The represented control-flow statements consume the result-bearing normal-path completion requirement from `function-execution.md`.

A conditional whose two explicit arms both terminate the current function activation by represented return and/or explicit fault has no local normal successor and may therefore discharge the remaining result-path obligation without a redundant root terminal return after it. The two arms may both return, both explicitly fault, or use any represented return/explicit-fault mixture that leaves neither arm with local fallthrough.

A conditional with no local normal continuation because its paths instead perform loop transfers does **not** by itself terminate the function activation and does not independently discharge a result obligation. Such transfers are source-valid only inside an enclosing represented loop, which consumes their destinations; that `while` still has its represented false normal outcome.

If exactly one conditional arm has a local normal continuation, that sole continuation remains subject to the ordinary result-bearing requirement. A later source-valid result return is required before that path can reach the root closing boundary normally unless an enclosing control transfer redirects it first.

A conditional without explicit else always has the omitted-else normal false outcome and therefore cannot by itself eliminate every local normal path.

A represented bounded `while`, including `while true` and including bodies containing break/continue, always has its represented false normal outcome and therefore cannot by itself eliminate every normal root path or satisfy a missing-result obligation. A result-bearing path continuing after the loop still requires a source-valid result return before normal root completion.

## Conditional source/Core refinement

A faithful source-to-Core lowering MAY refine one source-valid represented conditional to the accepted Bool-valued `Branch` relation in [Core control flow](../core/control-flow.md).

After source validation has fixed the condition type, each arm's local normal-continuation presence, any required two-normal-outcome source structural ownership/pointer-origin equality, and any nested loop-transfer target/cleanup facts, a lowering may:

1. lower the existing condition producer exactly once under that producer's accepted lowering relation;
2. materialize its Bool result in a compiler-owned Core temporary when useful;
3. consume that temporary once as the Core `Branch` condition operand;
4. lower the then arm to one or more Core blocks;
5. lower explicit else-arm blocks when present, or use the following normal continuation directly as the false target when no else body needs lower execution;
6. for each normally completing explicit arm, refine that arm's retained source normal cleanup before its normal successor edge;
7. for each no-local-normal explicit path, preserve its existing non-local refinement: `Return` for a return, `Fault(F_explicit)` for explicit `fault;`, retained transfer cleanup plus `Goto` to the selected loop header for `continue;`, or retained transfer cleanup plus `Goto` to the selected loop post-loop block for `break;`, without first emitting ordinary normal arm cleanup;
8. when two normal outcomes exist, transfer both to one lower normal join before subsequent source statements;
9. when exactly one normal outcome exists, continue subsequent source lowering from that sole normal path, with or without a dedicated lower join block; and
10. when zero local normal outcomes exist, emit no lower normal join merely to create an unreachable local continuation.

The exact shape or number of Core blocks is not source-observable. A loop transfer's selected Core block is a lower refinement of its source lexical target, not source-semantic block identity.

A compiler temporary used for condition selection is not a source binding. Producer-internal temporaries remain governed by their producer's accepted lowering relation.

Raw-pointer origin equality is a source validation/refinement fact. A lowerer MUST retain enough information to lower each accepted raw-pointer use to its correct existing Core pointer value/target relation, but the conditional join does not require a new Core origin-merge operation, runtime origin object, or physical pointer equality check.

A result-producing return whose producer is a direct call may first create the existing call continuation block and then terminate that continuation with Core `Return`; this does not create a second source continuation.

The stable abstract Core defined-fault reason used for source `ExplicitFault` is selected by `function-execution.md` through the accepted [Core defined faults](../core/faults.md) relation. This conditional owner neither chooses an implementation string/code nor defines a second fault-lowering rule.

## Bounded-while source/Core refinement

A faithful lowering MAY refine one source-valid represented bounded `while` using only the already accepted Core CFG, value, call, pointer, and vacant initialization semantics.

After source validation has fixed exact Bool condition typing, retained condition/body HIR, body local normal-continuation presence/cleanup, any required H-state ordinary backedge structural/origin equality, and all retained loop-transfer target/cleanup facts, a lowering may:

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

The accepted Core vacant non-replacing `Init` relation and result-bearing direct-call destination relation permit one fixed compiler/source local that is Dead or otherwise wholly vacant on a later cycle to begin a new stored-value lifetime without changing its dynamic storage-instance identity. Lowering may therefore reuse statically allocated body locals and condition/result temporaries across normal or continue cycle visits when Core validation proves the required vacancy/authority. This is a lower proving fact, not source assignment mutability, source ownership restoration, or source pointer-origin inference.

No new Core operation, Core state lattice, source/Core loop flag, runtime moved/origin flag, dynamic slot identity, or hidden lifetime-generation mechanism is required. Existing Core `Goto` is sufficient for both loop transfers after source cleanup/target validity has been retained. Existing Core raw-pointer values preserve their already defined target/provenance through ordinary pointer-value transport; source exact-origin backedge validity is established before lowering.

The exact Core block identities/count remain implementation facts. The semantic requirements are the predecessor-before-header edge, per-visit condition execution, correct Branch targets, ordinary normal body cleanup before ordinary backedge, transfer cleanup before transfer `Goto`, nearest-loop target preservation, exact source structural/origin target state, absence of a synthetic ordinary backedge for no-local-normal body paths, and continuation of following source from the post-loop block.

## Lower path states do not define source control flow

Core CFG validation may preserve multiple distinct implementation states at a lower join or cycle even when source enclosing ownership/provenance is definite.

For a conditional, a source local declared only in the then-arm child scope may be represented by one Core local that is Dead after normal then-arm cleanup but Never-initialized on a false execution that never entered that arm. Those distinct lower states remain valid implementation facts under Core control flow.

For a bounded `while`, one static body local or compiler temporary may be Never-initialized before the first body/condition visit and Dead on a later cycle after its prior value was moved/dropped/cleaned. Accepted Core vacant initialization admits the later new lifetime when the selected destination is wholly vacant; source validity still comes only from the source loop and transfer relations above.

These lower differences do not make ended body/arm locals visible after their source scopes and do not create path-dependent source ownership or pointer-origin provenance for enclosing bindings.

Lowering MUST NOT:

- use Core path-state union/intersection to reconstruct source structural ownership;
- use Core raw-pointer target/provenance metadata to reconstruct or merge source pointer origins;
- accept an unequal two-normal-outcome conditional join merely because every lower continuation operation happens to validate under multiple Core states;
- accept a conditional whose raw-pointer origins differ merely because lower pointer values share a Core type or implementation representation;
- accept a bounded-`while` ordinary/continue backedge whose source enclosing structural/origin state differs from `H` merely because Core cyclic path states validate;
- accept a break whose source enclosing structural/origin state differs from `C` merely because its lower `Goto` validates;
- redirect a source loop transfer to a non-nearest loop based on lower block convenience;
- invent a source local normal successor from lower reachability after source validation determined none;
- infer source normal or transfer cleanup from lower scalar liveness; or
- turn Core worklist behavior into source semantic authority.

Source local normal-continuation presence, conditional join validity, bounded-loop ordinary/explicit transfer validity, transfer target selection, and definite source successor structural/origin states are established before lowering by this document, `function-execution.md`, `local-bindings.md`, `structural-ownership.md`, and `raw-pointers-unsafe.md`.

## Determinism

For one fixed source-valid represented program and one fixed activation state, conditional and bounded-`while` runtime selection and bounded loop-transfer target selection are deterministic.

The condition producer has its existing deterministic or otherwise accepted source behavior. Once it yields one of the two semantic Bool values, exactly one runtime outcome is selected. A selected conditional arm or `while` body may normal-complete, return, explicitly fault, execute the uniquely selected nearest-loop break/continue transfer, yield another defined fault, or diverge according to its existing owners. A false `while` condition selects only the post-loop continuation; a normal true body or continue returns only to the selected loop's next condition evaluation; break reaches only the selected loop's post-loop continuation.

Validation of both represented conditional outcomes, the represented `while` false/body relations, every exact pointer-origin equality, and every explicit transfer target-state relation is a static validity obligation, not runtime nondeterminism.

## Further boundaries

This revision does not define general expressions, grouping expressions, comparisons, logical operators, arithmetic operators, truthiness, coercions, standalone record-construction conditions, direct raw-pointer condition forms, conditional values/expressions, direct `else if`, unrestricted nonterminal-within-block return or arbitrary unreachable tails, additional loop forms (`loop`, `for`, do/while), loop `else`, loop values, match, refutable patterns, fault payloads, panic/throw syntax, catch/recovery, labels or a label namespace, labeled transfer, break/continue values, transfer to a non-nearest loop, source state/completion lattices, pointer-origin sets/unions, general loop fixed-point inference, path-dependent ownership/origin after a two-normal-outcome join or bounded-while target, automatic join/backedge/transfer normalization of enclosing bindings, drop flags, custom destructors, must-consume policy, reference/borrow semantics, raw-pointer operation semantics, lifetime inference, optimizer transformations, ABI/linkage, backend branches, Exec, Model, or stable serialized HIR/Core control-flow identity.

Those concerns require their own accepted owners or later extensions and MUST NOT be inferred from the represented conditional, bounded-`while`, or bounded unlabeled loop-transfer relations here.