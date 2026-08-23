# Source Function Execution

Status: **provisional normative; incomplete**

This document owns the represented source semantics for source function body attachment, body and nested-block statement sequencing, dynamic direct-call activations, direct-call argument/result ownership transfer, record-construction field evaluation and transient assembly, producer-backed field-receiver evaluation and transient cleanup, Boolean logical-negation operand validation/evaluation sequencing, ordinary local initialization, recursive record-destructuring declaration completion including producer-backed scrutinee evaluation and transient cleanup, whole-binding assignment RHS evaluation and replacement ordering, lexical-scope and activation cleanup, return execution, payload-free explicit-fault execution, bounded loop-transfer cleanup, static local normal-continuation presence, bounded-loop body execution/cleanup sequencing, recursion/divergence, and defined-fault propagation through direct source calls.

It consumes program outcomes and recoverable-value separation from [Program behavior](../behavior.md), environment admission and realization separation from [Program lifecycle](../lifecycle.md), defined-fault reason identity and explicit Core fault termination from [Core faults](../core/faults.md), structural destruction and stored-value cleanup from [Core value and storage semantics](../core/value-storage.md), function entity/callable-signature structure from [Source callables](callables.md), source value type equality and record value shape from [Source type foundation](types.md), boolean/integer/decimal floating literal value production from [Source literal semantics](literals.md), Boolean logical-negation operand/result typing and semantic value transformation from [Source operator semantics](operators.md), structural ownership state and remaining frontiers from [Source structural ownership](structural-ownership.md), parameter/local identity, scope, lookup, assignment mutability, whole-binding use, and assignment legality from [Source function-local bindings](local-bindings.md), binding-root and bounded producer-backed field-value production, receiver exact-type facts, selected field paths, duplicate-or-consume consequences, and producer-receiver remaining frontiers from [Source field-value access](field-access.md), and recursive exhaustive record-pattern head/field structure, scrutinee category, binding-leaf order, per-leaf ownership consequences, and producer transient frontier selection from [Source patterns](patterns.md). It does not redefine those owners.

[Source control flow](control-flow.md) consumes this document's owned-value producer execution, nested-block execution, local normal-continuation presence, return and explicit-fault termination, loop-transfer cleanup, lexical cleanup, defined-fault propagation, and divergence relations when defining represented statement-level conditionals, bounded `while`, and bounded unlabeled `break;`/`continue;`, including their definite normal successor/backedge/transfer-target relations. This document does not redefine conditional selection, loop condition selection, conditional ownership joins, loop backedge-state admission, or loop-transfer target/state admission.

The represented concrete function/body/block/value/operator/call/record-construction/field-value/record-destructuring/assignment/conditional/while/break/continue/return/explicit-fault grammar is owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, pattern-head lookup or field accessibility, universal expressions, operator-local type/value/refinement semantics, conditional or loop selection/backedge/transfer-target validity, other general control flow, references, closures, traits, ABI, or an implementation representation.

## Source function bodies

A represented source function entity MAY have one represented source body and MUST NOT have more than one.

Attaching a represented source body to a function entity is a source-semantic fact. `concrete-syntax.md` defines one concrete function form that introduces a function entity and attaches the following body. This execution relation does not require every future declaration/definition form to use that syntax.

In the represented direct-call subset, a direct source call targets exactly one resolved source function entity that has a represented body.

A source function entity without a represented body has no direct source execution relation under this document. Later FFI, external-function, intrinsic, or other callable owners may add distinct relations without redefining this one.

The represented direct call is statically bound to its resolved function entity. This revision introduces no function-operand evaluation, overload selection, indirect call, function value, virtual dispatch, or closure call.

## Dynamic source function activations

Every dynamic direct call creates one distinct **source function activation** for the target function entity.

The activation provides independent dynamic values and binding-owned structural ownership states for that function body's static parameter/local binding identities, including locals introduced by ordinary declarations and accepted recursive record patterns. Distinct simultaneous or recursive calls therefore have distinct dynamic binding state even when they execute the same declarations.

Activation identity is not source-observable, a physical stack-frame address, ABI identity, task/thread identity, or required numeric runtime representation.

The callee body begins only after parameter transfer completes and each parameter binding has complete initial structural ownership under `local-bindings.md` and `structural-ownership.md`.

## Recursion

Direct and mutual recursion are source-valid. Every recursive direct call creates a fresh activation.

A recursive execution may diverge.

A physical target limitation MUST NOT retroactively make otherwise valid Runen source invalid. When an applicable hard target/environment requirement cannot realize the accepted recursion semantics, environment admission may reject that target/environment combination or a legal realization may emulate the source semantics. A backend MUST NOT silently alter the source call relation.

This document defines no recursion capability name, stack-size guarantee, tail-call guarantee, physical call stack, or concrete admission syntax.

## Owned value producers

This revision does not define a universal source expression taxonomy.

An **owned value producer** is an applicable accepted source operation whose successful evaluation yields exactly one owned source value.

The represented producer families are:

- a source-valid boolean, materialized decimal integer, or materialized decimal floating literal from `literals.md`;
- ordinary whole-binding owned-value use from `local-bindings.md`;
- a successful result-bearing direct call under this document;
- a source-valid named-field record construction under this document and `concrete-syntax.md`;
- a source-valid field-value use under `field-access.md`, using either the accepted binding-root receiver or the bounded producer receiver; and
- a source-valid Boolean logical-negation producer whose operator-local type/value relation is owned by `operators.md`.

`concrete-syntax.md` exposes exactly those six families in `Value`. A record-destructuring declaration is not another `Value` producer: `patterns.md` owns its grouped production of zero or more binding-leaf values. A producer-backed record-pattern scrutinee reuses one existing producer family in a pattern-specific receiving position and remains deliberately narrower than `Value`.

`control-flow.md` reuses a concrete subset of these existing producer families in its `ConditionalValue` receiving position for represented `if` and bounded `while`. That use does not create an additional producer family or alter any producer execution semantics here.

The represented `fault;`, `break;`, and `continue;` forms are body statements with no local fallthrough, not owned value producers, `Value`, `ConditionalValue`, expressions, calls, or return-value forms. None evaluates a producer merely to select its termination/transfer reason or target.

Future additional operator, conversion, or other expression owners MAY introduce further owned value producers without redefining the receiving relations in this document.

Unless another accepted source owner defines a distinct rule, a producer used where this document requires a value MUST finish evaluation before that value is transferred to its receiving binding/transient owner.

Literal evaluation is effect-free, non-faulting, and non-diverging after source validation. This execution owner supplies a required source type where contextual literal materialization needs it and then transfers the resulting value.

A source-valid binding-root field-value producer may change its selected binding's structural ownership state before its result reaches the receiving position: a duplicable final field leaves ownership unchanged, while a non-duplicable final field consumes exactly the selected structural path under `field-access.md` and `structural-ownership.md`.

A source-valid producer-backed field-value producer instead completes the receiver-producer execution and field-receiver transient lifetime defined below before exposing its selected result to the surrounding receiving position.

## Boolean logical-negation producer validation and execution

A represented Boolean logical-negation producer consumes from `operators.md` the intrinsic result type `Bool`, operand required type `Bool`, successful semantic value transformation, and absence of any operator-local binding structural-ownership transition.

The surrounding receiving position's required source type applies first to the operator's intrinsic result type. Source validation MUST establish that the surrounding required type is exactly `Bool` before validating an operand in a way that may commit a binding ownership consequence. If the surrounding required type is not `Bool`, the operator is source-invalid with result type `Bool`, and no operand ownership consequence is committed merely while diagnosing that mismatch.

After that outer result-type requirement succeeds, validate the complete operand producer with required source type exactly `Bool` against a speculative copy of the current binding structural-ownership environment. The speculative operand environment is committed to the containing source-validation environment only when the complete operand producer is source-valid. A failed operand lookup, type check, availability check, nested producer validation, or other source-invalid operand therefore cannot leak a speculative binding consumption merely because it appeared beneath logical negation.

A successfully validated logical-negation producer adds no further binding structural-ownership change beyond the committed consequences of its operand producer.

Dynamic execution is exactly:

1. evaluate the complete operand producer exactly once under its existing producer semantics with required source type `Bool`;
2. preserve every binding ownership consequence completed by that operand evaluation;
3. if operand evaluation yields defined fault `F`, produce no logical-negation result and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if operand evaluation diverges, produce no logical-negation result and perform no cleanup merely because execution remains suspended;
5. after successful Bool operand production, apply the Boolean logical-negation semantic value relation from `operators.md` exactly once; and
6. transfer the resulting owned Bool exactly once to the surrounding receiving position.

After successful operand production, the logical-negation step itself is non-faulting and non-diverging and adds no source-visible side effect, binding transition, transient cleanup phase, storage identity, or runtime state.

When logical negation is used as a represented `if` or bounded-`while` condition, the successful post-condition binding environment is therefore exactly the operand producer's successful post-evaluation environment. Only the owned Bool condition value is transformed before `control-flow.md` consumes it for selection.

Nested logical-negation producers apply this same relation recursively. Each nesting level validates/evaluates its complete operand once and adds no binding structural-ownership transition of its own.

## Producer-backed field-value execution

A source-valid producer-backed field-value use consumes from `field-access.md` the already validated receiver category, complete receiver producer, exact receiver type, complete resolved non-empty field path, exact final result type, duplicate-or-consume consequence, and canonical remaining-frontier cleanup paths.

The surrounding receiving position's required source type applies to the final selected field result. It does not replace the receiver producer's own exact result type. A direct-call receiver therefore executes against the result type selected from its resolved callable signature, and a record-construction receiver executes against its explicit resolved nominal target type, whether the concrete target was unqualified or qualified.

For one producer-backed field-value use, execution is exactly:

1. evaluate the retained receiver producer exactly once under its existing direct-call or record-construction semantics and its own exact receiver type;
2. preserve every binding ownership transition caused while evaluating that receiver producer;
3. if receiver evaluation yields a defined fault, establish no field-receiver transient and no selected field result; the field-value producer yields that same fault to its surrounding receiving relation, while receiver-producer internal cleanup and that receiving relation's existing cleanup/propagation remain controlling;
4. if receiver evaluation diverges, establish no field-receiver transient and no selected field result, and perform no field-receiver cleanup merely because execution remains suspended;
5. after successful receiver production, transfer the complete produced record value into one fully owned **field-receiver transient** whose structural ownership state begins complete;
6. apply the source-selected final-field duplicate-or-consume consequence to the already resolved field path through the transient ownership facts owned by `field-access.md`;
7. preserve the successfully produced selected field result outside the field-receiver transient's cleanup set;
8. clean exactly the source-selected canonical remaining frontier of the field-receiver transient in canonical frontier order;
9. end the field-receiver transient completely; and
10. transfer the preserved selected field result exactly once, without duplication, to the surrounding receiving position.

The field-receiver transient is not a source binding, place, lvalue, reference, addressable object, pattern scrutinee transient, construction transient, or argument transient. It participates only in this composite field-value operation and never enters lexical/activation cleanup.

When the selected final field is duplicable, the selected result is the independent duplicate chosen by `field-access.md`; the complete receiver transient remains owned until step 8 and its original selected subvalue is cleaned with the receiver remainder. When the selected final field is non-duplicable, exactly that selected path has transferred to the preserved result and is absent from the remaining frontier cleaned at step 8.

After successful receiver production, field selection, selected-result production, remaining-frontier cleanup, and result transfer add no new defined-fault or divergence outcome under the represented source model. Zero-field and recursively zero-leaf frontier members remain real source ownership whose ending may refine to no scalar Core destruction when the lower destruction domain is empty.

The complete field-receiver transient lifecycle finishes before an enclosing local receives the result, before assignment old-value cleanup begins, before a direct-call argument transient is established for that result, before a return begins activation cleanup, before an enclosing construction initializer holds its construction transient, before a represented control-flow construct owns its Bool condition transient, and before a producer-backed record pattern establishes its separate pattern scrutinee transient.

## Record construction

A source-valid represented record construction has one resolved nominal record target and one named initializer for every declared field as mapped by `concrete-syntax.md`. The target may have been selected by the accepted unqualified same-module construction lookup or by the accepted one-hop qualified cross-module lookup; target qualification has no dynamic execution role after source validation.

Before construction execution begins, source validation has already established the complete static construction boundary: target alias/member lookup where applicable, target binding accessibility and record category, exact nominal result type, every initializer field identity, direct field accessibility for every known initializer under `field-access.md`, duplicate status, exhaustive field coverage, and exact surrounding required-type equality when a receiving position supplies one. A rejection of any of those facts evaluates no initializer and commits no initializer-producer ownership consequence.

Construction produces exactly one owned source value of the resolved nominal record type. The target is explicit rather than inferred. When an enclosing consumer supplies a required source type, the construction result MUST be exactly equal to that type under `types.md`.

Each initializer is associated with one already resolved and accessible declaration field. That field's source type is the required type supplied to the initializer's `Value` producer. The produced field value MUST have exactly that type; no conversion, coercion, widening, narrowing, defaulting, or inference is introduced.

For a same-module target, private and exported fields are both directly accessible under `field-access.md`. For a qualified foreign target, the target record binding is already exported through qualified lookup and each explicitly initialized field must independently have exported direct accessibility. Because construction remains exhaustive, a foreign exported record with any module-private field has no source-valid qualified construction through this form. A zero-field exported foreign record has no initializer-access check and may construct directly when its target lookup/result-type requirements hold.

A represented decimal integer literal used as a field initializer materializes under that selected field type through `literals.md`. This is the same required-type materialization relation used by existing value consumers and does not create a conversion or inferred constructor target.

Initializers evaluate strictly left to right in constructor source order, regardless of target record declaration order. For each initializer:

1. evaluate its `Value` producer completely;
2. preserve every ownership transition caused by that evaluation; and
3. hold the produced owned field value as one **transient construction value** associated with the selected field.

A transient construction value is semantic ownership held by the in-progress construction. It does not require source-addressable storage, a binding, field place, or other source identity.

If initializer `i` yields a defined fault before construction completes:

1. no record value is produced;
2. perform any producer-specific cleanup inside the failing initializer;
3. clean previously produced construction transients in reverse production/source order;
4. preserve binding structural ownership transitions already caused by evaluated initializers; and
5. continue the same defined fault.

In particular, if an earlier initializer consumed a complete non-duplicable binding or one non-duplicable structural subvalue, that ownership remains transferred while the transient value produced from it is cleaned exactly once after a later initializer faults. The former binding's remaining ownership frontier excludes the consumed path.

If initializer `i` diverges, no record value is produced and no construction cleanup occurs merely because execution continues. Earlier construction transients remain owned by the suspended construction and prior ownership transitions remain effective.

Only after every initializer succeeds does assembly occur. Assembly transfers every transient construction value exactly once, without duplication, into its selected declaration field and forms one owned nominal record value. Result structural field order is declaration order; constructor source order controls evaluation/transient production.

Assembly after successful initializer evaluation is non-faulting and non-diverging. A transferred field transient is no longer independently owned and MUST NOT be cleaned separately from the completed result.

For a zero-field record there are no initializer producers/transients; successful construction directly produces the complete empty record value.

A result-bearing call, nested construction, producer-backed field-value use, or Boolean logical-negation producer used as an initializer must complete before that field transient exists and before a later initializer begins.

The completed record value may be transferred into ordinary local initialization, assignment RHS, direct-call argument, return result, enclosing construction field, a bounded producer-backed field-value receiver, or a producer-backed recursive record-pattern scrutinee. Those receiving relations keep their existing outer ordering and exact-type requirements. A qualified construction therefore composes through these existing receiving relations without defining a second execution category.

When a qualified construction is used as a producer-backed field receiver, the outer field-value transaction owned by `field-access.md` establishes its field receiver/path/result facts before this construction may commit initializer ownership. When a qualified construction appears as a record-pattern scrutinee producer, `patterns.md` has already resolved the top pattern head, whether unqualified or qualified, and its exact nominal type is the required construction result type. The construction may therefore supply that pattern exactly when both resolve to the same nominal record and their independent target/field-accessibility rules are source-valid; qualification on either side adds no execution step.

Target qualification is source lookup only. It creates no runtime module loading, dynamic access check, ABI/layout/linkage consequence, constructor function, or distinct constructed-value identity. A faithful typed representation may discard the qualified-vs-unqualified target category after retaining the resolved record identity and initializer facts.

This relation adds no field assignment, partial-field reinitialization, pattern selection, update/spread/default initialization, shorthand, positive duplicability selection, method/constructor body, public-constructor capability, or constructor-specific visibility class.

## Direct-call arguments

A represented direct call has exactly one ordered argument operand for each callable-signature parameter slot. Argument count MUST match exactly.

Each argument evaluation MUST produce one owned source value whose type equals exactly the corresponding parameter source type. This revision introduces no implicit conversion, coercion, widening, narrowing, subtyping, or numeric defaulting.

The parameter type is the required source type supplied to a producer that needs one. Decimal integer arguments therefore materialize under the corresponding parameter type through `literals.md`; this does not create a conversion or inference relation.

Arguments evaluate left to right.

Ordinary complete-binding argument use follows `local-bindings.md`; field-value arguments follow `field-access.md` and, for bounded producer-backed receivers, the complete field-receiver lifecycle above. Boolean logical-negation arguments complete their operand evaluation and logical-negation relation above before their produced Bool is held as the argument transient. Any ownership transition occurs at the applicable producer's evaluation position.

Each successfully evaluated argument is held as one owned **transient argument value** until all arguments succeed. Transient ownership is semantic and does not require a materialized source storage place.

If argument `i` yields a defined fault before callee activation:

1. no callee activation is created;
2. earlier transient argument values are cleaned in reverse production order;
3. source ownership transitions already caused by earlier arguments remain effective; and
4. the same fault continues in the caller.

If argument evaluation diverges, no callee activation is created. Earlier transient arguments remain owned by the suspended computation and no cleanup occurs merely because time passes.

After all arguments succeed:

1. create the callee activation;
2. transfer each transient argument in parameter-slot order into its corresponding parameter binding; and
3. establish every parameter binding with complete initial structural ownership of its transferred value.

Transfer does not duplicate a transient argument value.

Parameter slots are owned-value parameters in this represented relation. This rule neither introduces nor prohibits future reference, borrow, or other pass-mode parameter forms; such forms require their own accepted authority.

## Ordinary local initialization

When a represented local initializer evaluates an owned value producer, evaluation completes before the produced value is transferred into the binding.

The local's declared type is the required type supplied to a producer that needs one. A represented decimal integer literal initializer therefore materializes under that declared type through `literals.md` before transfer. A Boolean logical-negation initializer first establishes that the declared required type is exactly Bool, then validates/evaluates its operand under the relation above. The produced value MUST have exactly the local's declared type.

After transfer, the local begins with complete initial structural ownership of its value under `local-bindings.md` and `structural-ownership.md`. Transfer does not duplicate the produced value.

If initializer evaluation yields a defined fault or diverges, the local never receives an owned value. Ownership transitions already performed by earlier evaluated operations remain effective.

## Record-destructuring declaration completion

A source-valid recursive record-destructuring declaration consumes the complete pattern structure, resolved top nominal record identity, scrutinee category, exact top scrutinee type, binding-leaf order, leaf paths/types, direct-root leaf availability requirements, per-leaf duplicate-or-consume consequences, and producer-transient remaining frontier from `patterns.md` and `structural-ownership.md`. Qualified versus unqualified pattern-head spelling has already been discharged by source validation before this execution relation begins.

### Direct binding-root completion

For the direct binding-root category:

1. complete source validation for the entire recursive pattern, including every binding leaf's required full availability in one shared pre-pattern root state;
2. apply the pattern-owned direct-root binding-leaf duplicate/consume productions in retained depth-first source order;
3. each produced leaf value becomes the complete initial owned value of its corresponding not-yet-in-scope pattern binding;
4. establish all of those new bindings in the containing lexical scope together; and
5. only then may the next body statement begin.

The direct root is not evaluated through ordinary whole-binding `IdentifierUse` and no scrutinee transient exists.

Pattern leaf production is non-faulting and non-diverging after source validation.

A direct zero-field or recursively empty nested pattern contributes no binding leaf and therefore performs no ownership production merely for that empty static structure. A top-level zero-field direct pattern remains the accepted ownership no-op even when the complete root is not fully available.

### Producer-backed completion

For a producer-backed category:

1. complete recursive pattern structure and introduced-binding validity before producer evaluation;
2. validate the selected direct-call, record-construction, or field-value producer using the top pattern head's resolved nominal record type as the exact required type and the pre-pattern-binding lexical environment;
3. only after that complete producer is source-valid, evaluate it completely;
4. when that producer is a producer-backed field-value use, complete its field-receiver production, selected-result preservation, remaining-frontier cleanup, and field-receiver transient ending before the resulting owned record can become the pattern scrutinee;
5. if producer evaluation faults or diverges, perform no pattern leaf production and establish no pattern scrutinee transient;
6. on producer success, transfer the produced record into one fully owned pattern scrutinee transient whose structural ownership state begins complete;
7. apply pattern-owned binding-leaf `Duplicate`/`Consume` productions in retained depth-first source order;
8. after every binding leaf has been produced, clean the transient's remaining structural ownership frontier selected by `patterns.md` through `structural-ownership.md` exactly once;
9. only after transient cleanup completes, establish all pattern-introduced bindings in the containing lexical scope together; and
10. only then may the next body statement begin.

Any accepted producer-backed scrutinee follows exactly this sequence when its result type equals the already resolved top pattern record type. Boolean logical negation is deliberately not one of the accepted producer-backed record-pattern scrutinee categories. Whether the accepted producer target or receiver, the pattern head, both, or neither use represented qualification does not add a producer, transient, ownership transition, fault path, divergence path, or cleanup phase.

The pattern scrutinee transient is not a local binding and does not participate in lexical/activation cleanup after step 8. A field-receiver transient internal to the producer is a separate earlier transient and likewise never participates in pattern-transient cleanup.

Successful leaf production and pattern-transient cleanup introduce no new defined-fault or divergence outcome after producer success under the represented relation.

If producer evaluation yields a defined fault before pattern-transient establishment:

- no pattern leaf production occurs;
- no pattern binding enters scope;
- producer-internal transient cleanup occurs exactly as in another receiving position; for a producer-backed field-value producer, a receiver fault establishes no field-receiver transient;
- ownership transitions already completed by producer evaluation remain effective; and
- the same fault continues through activation fault propagation.

If producer evaluation diverges before pattern-transient establishment, no pattern leaf production, pattern binding establishment, or pattern-declaration transient cleanup occurs. Producer-owned transients remain owned by the suspended producer. For a producer-backed field-value use, no field-receiver transient exists unless its receiver producer has already completed successfully; after such success the represented field-selection/cleanup tail itself does not diverge.

For a producer-backed zero-field top pattern, successful producer evaluation yields one complete empty-record pattern transient. There are no binding leaves; its canonical remaining frontier contains the complete empty root, whose source ownership ends before declaration completion even when lower scalar cleanup is vacuous.

This section does not redefine pattern-head/field selection, structural path validity, source duplicability, producer syntax, field-receiver frontier membership, or pattern-transient frontier membership.

## Whole-binding assignment and replacement

A represented whole-binding assignment consumes assignment target legality, mutability, declared type, RHS type requirement, and binding structural lifecycle from `local-bindings.md`, together with remaining-frontier selection from `structural-ownership.md`.

The target's declared source type is the required type supplied to an RHS producer that needs one. A represented decimal integer literal RHS therefore materializes under that target type through `literals.md` before replacement execution. A Boolean logical-negation RHS must satisfy the exact Bool receiving and operand relation above before replacement begins.

For a source-valid assignment, execution is **source-first** with respect to replacement:

1. evaluate the RHS completely;
2. preserve every ownership transition caused while evaluating that RHS;
3. preserve the successfully produced RHS value outside the target's old-value cleanup set until replacement transfer;
4. only after successful RHS production, select the target binding's then-current complete-root remaining ownership frontier under `structural-ownership.md`;
5. clean each frontier source subvalue exactly once in canonical frontier order;
6. transfer the produced RHS value into the complete target binding without duplication; and
7. establish a fresh complete structural ownership state for the replacement value.

The target remains in scope during RHS evaluation. Every RHS use follows its ordinary producer semantics rather than a special self-assignment rule.

For duplicable `x = x`, RHS evaluation duplicates the complete old value, leaving the old root fully available; replacement then cleans that complete old value and transfers the duplicate into `x`.

For non-duplicable `x = x`, RHS evaluation consumes the complete old value, leaving no old target-owned frontier; replacement transfers the produced value back into `x` without duplicate cleanup.

If the RHS consumes only a non-duplicable subvalue of `x`, the target becomes partial before replacement. Its canonical remaining frontier then contains exactly the maximal still-owned disjoint source subvalues; replacement cleans those and never re-cleans the consumed path.

The same ordering applies when direct-call argument evaluation, record-construction initializer evaluation, producer-backed field-receiver evaluation, Boolean logical-negation operand evaluation, or another represented producer consumes the complete target or one of its structural subvalues before a later operation successfully produces the replacement value.

If RHS evaluation yields a defined fault, assignment performs no replacement cleanup/reset/transfer. Completed ownership transitions remain effective and activation fault cleanup uses the target's resulting current state.

If RHS evaluation diverges, assignment performs no replacement cleanup/reset/transfer. The activation remains suspended and no cleanup occurs merely because execution continues.

Core structural destruction/storage mechanics remain owned by Core; this source relation selects source ownership and source ordering.

This revision defines no field/place assignment, partial-field reinitialization, compound assignment, assignment expression, borrow/reference assignment, source interior mutability, raw-pointer assignment, or destructuring assignment.

## Body and nested-block statement sequencing

For source validation, every represented statement or lexical block has only the minimum **local normal-continuation presence** needed by this source subset:

- **local normal continuation present** means the construct establishes exactly one definite ordinary function-local binding environment for a following statement in the same immediate sequence or for its enclosing ordinary normal continuation; and
- **no local normal continuation** means successful execution of the represented static control structure provides no fallthrough to a following statement in that same immediate sequence.

No-local-normal continuation no longer implies by definition that the current function activation terminates. A represented return or explicit `fault;` does terminate the activation, while a source-valid `break;` or `continue;` instead transfers within the nearest enclosing represented loop. The enclosing control-flow owner consumes that distinct destination.

This classification is not a source value, runtime tag, source CFG node, state set, effect, fault set, transfer-kind set, abrupt-completion lattice, or ownership lattice. Producer-originating defined faults and divergence remain dynamic execution outcomes and do not form additional static completion alternatives. Represented `fault;`, `break;`, and `continue;` are different because their successful execution itself has no local fallthrough.

For the root function body and each represented `BlockStatement`, the applicable `BodyStatement` sequence is validated and executes strictly in concrete source order while a local normal continuation remains present. Every ordinary source-valid local declaration, record-destructuring declaration, assignment, no-result call statement, and normally completing nested block preserves one local normal continuation. A represented explicit `fault;`, admitted `break;`, and admitted `continue;` body statement has no local normal continuation. A represented terminal return has no local normal continuation. A represented conditional exposes the local normal-continuation presence and, when present, the definite normal environment established by `control-flow.md`. A represented bounded `while` always exposes its statically represented false local normal continuation and the definite post-condition environment established by `control-flow.md`, including when its condition is the literal `true` and when some body paths transfer.

A syntactically later `BodyStatement` or terminal `ReturnStatement` in the same containing sequence after a preceding statement with no local normal continuation is source-invalid as unreachable. This semantic sequencing rule is directly observable for `fault;`, `break;`, and `continue;`, because concrete grammar permits a later body statement after them. It does not admit an otherwise unrepresented concrete tail after a terminal return in the same lexical block.

Root-body execution begins with its first statement after successful parameter transfer. A nested block begins when its statement is reached. A later statement begins only after the preceding statement completes locally normally.

For an ordinary local declaration:

1. evaluate its initializer;
2. transfer the value into the new binding and establish complete initial ownership; and
3. only then continue.

For a recursive record-destructuring declaration:

1. complete the grouped pattern declaration under the applicable direct-root or producer-backed completion relation; and
2. only after any producer transient cleanup and grouped binding establishment may the next statement begin.

For whole-binding assignment, complete RHS production, old-value cleanup, replacement transfer, and target ownership reset before continuing.

For a no-result direct-call statement, complete the call normally before continuing. A valid no-result call statement has no value to discard.

For represented `fault;`, apply the explicit-fault execution relation below. It has no local normal continuation and therefore never begins a following statement in the same sequence.

For represented `break;` or `continue;`, the nearest-loop target and exact target structural state are already required by `control-flow.md`. Apply the loop-transfer cleanup relation below and transfer to that target. The transfer statement has no local normal continuation and therefore never begins a following statement in the same immediate sequence.

For a nested block:

1. activate its child lexical scope;
2. execute its contained sequence recursively in concrete order, including its optional terminal return when present;
3. if the nested block has a local normal continuation, normally exit the child scope using lexical-scope cleanup below and expose the resulting enclosing binding environment; and
4. only after that normal cleanup may the containing sequence continue.

A nested block with no local normal continuation performs no independent ordinary normal child-scope cleanup. A selected return or explicit-fault path follows the applicable activation cleanup relation, while a selected loop-transfer path follows the transfer cleanup relation below; each active child scope is cleaned exactly once by the applicable non-local completion relation.

A block statement produces no source value and introduces no Unit/Void value.

For a represented conditional statement, condition evaluation, selected-arm execution, explicit-arm scope composition, zero/one/two local normal outcomes, definite normal ownership at any local successor, and nested loop-transfer target-state validity are owned by `control-flow.md`. This sequencing relation consumes that local normal continuation only when one exists before beginning the next containing body statement.

For a represented bounded `while`, `control-flow.md` owns exact Bool admission, the pre-condition environment `H`, post-condition environment `C`, false selection, body/backedge structural-state validity, explicit break/continue target-state validity, and the definite post-loop environment. This execution owner supplies the repeated dynamic ordering and ordinary/transfer child-scope behavior:

1. evaluate the retained condition producer under its ordinary producer execution relation;
2. after successful Bool production, when the Bool value is `false`, consume the condition result for selection and continue after the loop with no loop-body scope activation;
3. when the Bool value is `true`, consume the condition result for selection, activate the loop body's ordinary child lexical scope, and execute that block exactly once;
4. if that body reaches its local normal completion, perform its ordinary normal lexical-scope cleanup exactly once before the validated ordinary backedge returns execution to condition evaluation;
5. if execution reaches an admitted `continue;`, perform its exited-scope transfer cleanup exactly once and return execution to the selected loop's condition point without a separate ordinary body cleanup;
6. if execution reaches an admitted `break;`, perform its exited-scope transfer cleanup exactly once and continue at the selected loop's post-loop continuation without condition re-evaluation or separate ordinary body cleanup;
7. if the body returns, do not perform a separate ordinary body cleanup before return-induced activation cleanup; the active body scope is included exactly once in that termination cleanup;
8. if the body or one of its producers yields a defined fault, do not perform a separate ordinary body cleanup before defined-fault activation cleanup; the active body scope is included exactly once there; and
9. if the condition or body diverges, execution remains suspended at that operation and no normal loop/body/activation/transfer cleanup occurs merely because execution continues.

A successful ordinary or continue backedge begins a new dynamic condition evaluation, not a second source statement or a new static binding identity. Repeated execution of one static loop-body declaration creates successive dynamic binding-owned values within the same activation while retaining that declaration's one source binding identity and ordinary per-entry initialization/cleanup semantics.

If a body statement yields a defined fault, later statements do not execute and the active function activation follows fault cleanup/propagation. A nested block exiting this way does not also perform independent normal or transfer cleanup; its child scope participates exactly once in fault cleanup.

If a body statement diverges, later statements do not execute and no termination/child-scope/transfer cleanup occurs merely because execution continues.

A terminal return in the root body or a nested block begins only after every preceding statement in that same lexical sequence has completed locally normally.

A represented no-result root body reaching its closing boundary with a local normal continuation and without a terminal return performs normal no-result completion.

This sequencing relation introduces no unrestricted mid-block return, unreachable-statement weakening, short-circuit operator, catch, defer, refutable match, additional loop form, labeled transfer, transfer value, or other multi-path/cyclic control transfer beyond represented terminal returns, payload-free explicit `fault;`, bounded unlabeled `break;`/`continue;`, statement-level conditional, and bounded `while` owners consumed above.

## Explicit fault statement

The represented payload-free source `fault;` statement selects exactly one distinguished source-semantic defined-fault reason whose specification label is **`ExplicitFault`**.

`ExplicitFault` is one semantic fault-reason identity consumed by the defined-fault propagation relation below. It is not a source value or source type and has no source payload, message/string, numeric code, fault-site identity, exception object, backtrace, matching interface, catch interface, stable serialization, ABI identity, or required implementation representation.

Every source-valid execution of `fault;` selects the same distinguished source reason `ExplicitFault`. Different source locations containing `fault;` do not thereby create different fault reasons.

The statement has no operand, required value type, owned-value producer, binding target, result value, or local normal continuation. Reaching it after all preceding statements in the same sequence have completed locally normally:

1. evaluates no owned-value producer and performs no new binding read, move, duplicate, assignment, field selection, call, or value production;
2. preserves every source ownership transition completed before the statement;
3. selects exactly `ExplicitFault`;
4. enters the existing defined-fault propagation relation below with `F = ExplicitFault`; and
5. produces no ordinary continuation from the current activation.

The statement itself does not diverge after it is reached: it selects a defined fault. Earlier operations may independently fault or diverge before execution reaches the statement.

When `fault;` is reached inside a nested block, conditional arm, or bounded-`while` body, that child scope does not first perform ordinary normal or loop-transfer cleanup. The defined-fault termination relation cleans every then-active lexical scope innermost through root exactly once and then processes parameters in reverse callable-signature slot order.

This section defines no `fault(...)`, `fault value;`, panic/throw spelling, payload/message/code, catch/recovery, fault value/type, effect signature, or programmatic inspection/comparison of `ExplicitFault`.

## Bounded loop-transfer cleanup

A source-valid `break;` or `continue;` consumes from `control-flow.md` one nearest enclosing represented `while`, its exact admitted target structural state, and the lexical set of scopes exited by the transfer.

The transfer itself has no operand, required value type, owned-value producer, binding target, result value, or local normal continuation. Reaching it after all preceding statements in the same immediate sequence have completed locally normally performs no additional source operation before cleanup.

For the transfer's exited lexical scopes, cleanup proceeds **innermost to outermost** through and including the selected loop's body scope, stopping before the lexical scope containing the `while` statement. Within each exited scope:

1. consider local bindings declared directly in that scope in reverse local declaration order;
2. for each binding, select its then-current complete-root remaining ownership frontier under `structural-ownership.md`;
3. clean every frontier member in canonical frontier order; and
4. end that binding's dynamic source ownership for the exited scope.

Consumed/unavailable paths are not cleaned again. A partially available binding cleans only its maximal still-owned disjoint frontier. A fully available zero-field or recursively zero-leaf source value remains real source ownership whose cleanup may refine to no Core `Drop` when the lower destruction domain is empty.

Transfer cleanup does not clean function parameters or locals belonging to an enclosing scope outside the selected loop body merely because those bindings participate in the target loop's `H`/`C` structural-state proof. It does not mutate/reset/consume an enclosing binding to make that proof succeed. Source validity requires the exact target state to have been established before the transfer by ordinary accepted operations.

After the complete exited-scope cleanup:

- `continue;` transfers to the selected loop's condition point; and
- `break;` transfers to the selected loop's post-loop continuation.

No exited scope first receives an independent ordinary normal cleanup and then a second transfer cleanup. The transfer relation is the unique cleanup of those scopes for that execution.

If an operation before the transfer yields a defined fault, the transfer is never reached and the ordinary defined-fault cleanup relation controls. If execution diverges before the transfer, no transfer cleanup occurs merely because the computation remains suspended. If a return or explicit `fault;` is selected on another path, its activation-termination cleanup controls rather than loop-transfer cleanup.

Nested loops compose by target selection from `control-flow.md`: a transfer in an inner loop body exits only scopes through the inner body; it does not clean or exit an outer loop body merely because that body is active.

## Source cleanup

For represented operations, **cleaning an owned source value** ends source execution's ownership of that value exactly once.

A binding cleanup selects its complete-root remaining ownership frontier under `structural-ownership.md`. Each frontier path denotes one maximal still-owned source subvalue. Cleaning the binding means cleaning those frontier subvalues exactly once in canonical frontier order and then ending the binding's source ownership.

A field-receiver transient and a producer-backed recursive record-pattern transient each use the same structural frontier relation over their own non-binding structural owned-value root. Cleaning either transient means cleaning the frontier values selected by its semantic owner exactly once in canonical frontier order and then ending all ownership held by that transient. Neither transient becomes a binding or participates in lexical/activation/loop-transfer cleanup.

When a source value is realized in Core storage, applicable destruction-domain, stored-value-lifetime, and cleanup semantics remain owned by [Core value and storage semantics](../core/value-storage.md). This document determines only source ownership-ending selection and source order.

A value already transferred or consumed is not cleaned again by its former owner. This applies to lexical/activation/loop-transfer cleanup, construction/argument transients, field-receiver transients, producer-backed pattern transients, and old assignment-target subvalues.

A fully available zero-field or recursively zero-leaf source subvalue remains a legitimate cleanup value. Cleaning it ends source ownership even when lower representation has no scalar destruction leaf and therefore needs no physical/Core `Drop`.

This revision introduces no custom source destructor body, source `drop` ability, must-consume policy, or general temporary-lifetime extension rule.

## Lexical-scope cleanup

When execution normally exits a represented lexical scope, consider all local bindings declared directly in that scope in **reverse local declaration order**. This includes ordinary locals and bindings introduced by recursive record-destructuring declarations.

For each binding, select its then-current complete-root remaining frontier under `structural-ownership.md` and clean every frontier member in canonical order.

For one record-destructuring declaration, `patterns.md` defines depth-first binding-leaf source order as the declaration order of the introduced bindings. Reverse local declaration cleanup therefore visits later binding leaves before earlier leaves, independently of record structural field order.

A fully available binding frontier contains only its complete root. An unavailable complete root has an empty frontier. A partial root cleans exactly the maximal still-owned disjoint subvalues and never re-cleans a consumed path.

When one source completion exits multiple active scopes, cleanup proceeds innermost to outermost. Ordinary normal nested-scope exit, loop transfer, return, and defined fault each select their applicable exited-scope range; each scope uses reverse local declaration order, and each binding uses its canonical remaining frontier.

Function parameters belong to the root activation but are not local declarations. On activation termination, after root lexical locals, process parameters in **reverse callable-signature parameter-slot order**, each using its then-current frontier. A loop transfer is not activation termination and therefore does not process parameters merely because it exits loop-body child scopes.

This cleanup order is semantic and independent of physical stack layout, ABI passing, compiler/Core local numbering, or backend strategy.

## Normal return

A represented return may be the optional terminal return of the root body or of any represented nested lexical block admitted by `concrete-syntax.md`. Every such return terminates the current source function activation; it does not merely exit the immediately containing block.

For a source function with one result type, that result type is the required type supplied to the return-value producer. A represented decimal integer literal return therefore materializes under the declared result type through `literals.md`. A Boolean logical-negation return must satisfy the exact Bool receiving and operand relation above before activation cleanup begins.

A represented return in a result-bearing function MUST first evaluate exactly one owned value producer whose type equals exactly that result type. A represented return in a no-result function MUST contain no value.

Result evaluation, including any structural ownership transition or producer-specific transient cleanup, completes before return-induced scope/activation cleanup.

After successful result evaluation:

1. preserve the owned transient result outside activation-local cleanup;
2. clean all active lexical scopes innermost through root;
3. process parameters in reverse slot order, cleaning each current frontier;
4. terminate the callee activation normally; and
5. deliver the owned transient result to the caller.

Transfer to the caller does not duplicate the result.

A complete non-duplicable local consumed by result evaluation has no remaining frontier and is not cleaned again. A consumed subvalue is excluded while disjoint remaining subvalues are cleaned normally.

For a no-result function, represented `return;` performs the same active-scope/parameter cleanup and normal activation termination but produces no value.

A return reached from a nested block, conditional arm, or bounded-`while` body does not first perform that block's ordinary normal or loop-transfer cleanup. Return-induced activation cleanup already includes every then-active descendant scope and therefore cleans each binding exactly once.

If return-value production yields a defined fault before successful result production, no normal return occurs. The existing defined-fault cleanup/propagation relation below handles the then-current active scopes exactly once. If return-value production diverges, no normal return cleanup occurs merely because execution remains suspended.

Reaching the normal end of a represented no-result function body is equivalent to normal no-result completion.

A result-bearing represented body MUST NOT have a reachable normal end without a result. This is a normal-path validity requirement, not a requirement for one unconditional concrete root-terminal return. A represented path that terminates by explicit `fault;` is abnormal and therefore requires no result value. A conditional whose two explicit arms both terminate the activation may therefore satisfy the result obligation without a following root return whether those arms return, explicitly fault, or use a represented mixture of the two. A conditional with no local fallthrough only because its paths perform loop transfers does not terminate the activation and cannot independently satisfy the result obligation. A represented bounded `while` always retains its statically represented false local normal continuation under `control-flow.md`, including for literal `true` and regardless of admitted transfers in its body, so the loop alone does not discharge the result obligation. When any represented path still establishes a normal root continuation, that continuation must eventually encounter a source-valid result-bearing return before the root closing boundary.

No implicit result, default result, Unit, or Void source value is introduced.

## Defined-fault propagation

The represented source subset has no catch boundary.

When an applicable accepted source/Core operation yields defined fault `F`, or when represented `fault;` selects `F = ExplicitFault`, during an activation:

1. preserve every ownership transition completed before `F` was selected;
2. clean all active lexical scopes innermost through root using each binding's current remaining frontier;
3. process parameters in reverse slot order using each current frontier; and
4. terminate that activation with the same defined fault `F`.

If the fault arises from a directly called callee, the caller's direct-call evaluation yields `F`; with no catch boundary, the caller performs its own fault cleanup and propagates the same fault outward. This continues to the outermost applicable source execution.

“Same defined fault” preserves the semantic fault-reason identity selected by the initiating operation or explicit fault statement. For `fault;`, that identity is exactly `ExplicitFault`. This revision defines no source payload representation, messages, numeric codes, exception objects, backtraces, panic/throw syntax, or catch/recovery syntax beyond the payload-free explicit fault statement defined above.

This propagation is semantic unwinding of source ownership and does not require physical stack unwinding. A realization MAY use another mechanism only when it preserves every applicable cleanup and observable behavior required by the accepted source and Core contracts.

A future catch, panic, throw, payload, or other fault owner may introduce explicit source forms and extend the applicable propagation relation at those explicit boundaries. No such boundary is represented here.

Recoverable domain/application failures represented as ordinary values remain ordinary values under `behavior.md`; they do not use this relation merely because they represent failure.

## Transient-value cleanup

Construction transients produced before a later initializer fault are cleaned in reverse construction production/source order before the same fault continues. Once transferred into a successful record result, they are no longer independently owned. Target qualification does not alter this transient lifecycle.

Argument transients produced before a later argument fault are cleaned in reverse production order before the fault continues in the caller.

A field-receiver transient exists only after its receiver producer has completed successfully. Its selected field result is preserved outside its cleanup set, its source-selected canonical remaining frontier is cleaned exactly once, and the transient ends before that result transfers to the surrounding receiving position. It is never retained for later lexical, activation, argument, construction, conditional/loop-condition, return, loop-transfer, or pattern cleanup.

An owned transient return result is outside callee activation-local cleanup after successful result evaluation because ownership has been separated for caller transfer.

A successfully produced assignment RHS is transferred into the target and is not independently remaining after successful assignment. If RHS production faults before success, producer-specific transient cleanup remains controlling.

A transient value produced by consuming a non-duplicable binding-root field is owned by its current transient position after production; its former binding path remains consumed and does not re-enter that binding's frontier if a later producer faults. For a producer-backed field-value use, the selected transferred path analogously does not re-enter the completed field-receiver transient after its result is preserved.

A producer-backed recursive record-destructuring declaration owns one pattern scrutinee transient only after producer success. Pattern binding-leaf production may consume arbitrary retained structural paths from that transient. After all leaf production, the declaration cleans exactly the canonical remaining structural frontier before new bindings enter scope. The transient then ends completely and does not participate in later lexical/activation/loop-transfer cleanup.

A direct binding-root record pattern has no independently owned scrutinee transient; its accepted leaf productions initialize final pattern bindings directly.

Boolean logical negation creates no additional cleanup-bearing transient category. Its operand producer retains every transient lifecycle already required by that producer, and the successful Bool result transfers directly to the surrounding receiving relation after the operator relation is applied.

This revision defines no general temporary lifetime extension, expression-statement discard, or arbitrary temporary cleanup. Only transient values required by represented record construction, direct-call argument/result transfer, producer-backed field-value receivers, assignment transfer, and producer-backed record destructuring are owned here. The successful Bool condition transient used by represented conditional or bounded-`while` selection is owned and ended by `control-flow.md` after this document's existing producer relation yields it.

## Divergence

If a record-construction initializer diverges, the construction remains suspended in that initializer. Earlier construction transients and completed ownership transitions remain; no construction/activation/scope/loop-transfer cleanup occurs merely because execution continues. This is identical for unqualified and qualified construction because target/accessibility validation completed before initializer evaluation began.

If a directly called callee diverges, the caller remains suspended at that call and performs no return/fault/loop-transfer cleanup merely because time passes.

If a direct call or record construction used as a producer-backed field receiver diverges before successful receiver production, no field-receiver transient or selected field result exists. Any earlier producer-owned transients and completed ownership transitions remain governed by that receiver producer's existing divergence relation.

Active caller/callee ownership state and any suspended producer transients persist subject to operations already completed. The same applies when a diverging call is an assignment RHS, Boolean logical-negation operand, producer-backed field receiver, producer-backed record-pattern scrutinee, represented conditional/loop condition, or return-value producer. There is no implicit source execution-step budget.

A direct binding-root record-destructuring operation has no divergence point after validation. A producer-backed operation may diverge only while evaluating its existing producer; after producer success, Boolean logical negation, field selection/field-receiver completion, or pattern leaf production/pattern-transient completion is non-diverging under the applicable owner.

The represented explicit `fault;`, `break;`, and `continue;` statements themselves are not divergence categories: once reached, `fault;` selects `ExplicitFault`, while admitted break/continue perform their finite transfer cleanup and transfer to the selected loop target.

## Effects boundary

Left-to-right constructor evaluation, left-to-right argument evaluation, source-first assignment RHS evaluation, Boolean logical-negation operand evaluation before its result transformation, producer-backed field receiver evaluation before selected-field production, producer-before-pattern evaluation, **depth-first pattern binding-leaf source order**, and concrete body/block statement sequencing fix relative source ordering for any effects that future accepted operation owners make observable.

For represented conditional and bounded-`while` selection, `control-flow.md` owns condition-producer-before-selected-arm/body ordering and consumes the producer/effect ordering defined here; this execution owner does not add speculation, arm/body reordering, or loop-condition hoisting authority. Each dynamic ordinary or continue backedge reaches a fresh condition evaluation after the applicable ordinary normal or transfer cleanup. A break transfer does not evaluate the condition merely to reach the post-loop continuation.

Literal evaluation has no source-visible side effect under `literals.md`; adding literals to represented ordinary value positions therefore adds no competing effect-order relation. Boolean logical negation similarly adds no operator-local side effect after its operand producer completes under `operators.md`.

A binding-root field-value production is non-faulting/non-diverging after source validation, but consuming a non-duplicable field performs its structural ownership transition at that producer position. A producer-backed field-value use first executes its retained receiver exactly once, then completes the selected field production and receiver-transient cleanup before its result becomes available to the surrounding consumer. When either field-value category is a producer-backed pattern scrutinee, the complete field-value operation finishes before pattern transient establishment and leaf production.

Pattern binding-leaf production is non-faulting/non-diverging after source validation and any producer completion. Its source-ordered non-duplicable leaf transfers are ownership transitions whose consequences are visible to later leaves and statements.

Record assembly after successful initializer evaluation is effect-free. Initializer source order, rather than declaration field order, remains producer-effect ordering authority. Target qualification is resolved statically and adds no runtime effect or ordering point. Pattern-head qualification is likewise resolved before execution and adds no runtime effect or ordering point.

Reaching `fault;` selects a defined-fault reason and terminates through the existing fault relation. Reaching an admitted loop transfer performs only its then-current exited-scope cleanup before control transfer. None evaluates an ordinary producer at that statement position.

This revision defines no source effect system, purity, effect inference, speculation legality, or general transformation rules.

## Concrete grammar and implementation boundary

`concrete-syntax.md` owns represented concrete grammar, including the bounded Boolean logical-negation prefix placements, unqualified and qualified record-construction targets, record-pattern heads, the bounded statement-level `while`, bounded unlabeled `break;`/`continue;`, and the payload-free explicit-fault statement. `operators.md` owns Boolean logical-negation operand/result typing, semantic value transformation, operator-local ownership neutrality, and operation-specific source/Core refinement. `literals.md` owns boolean/integer/decimal floating materialization. `structural-ownership.md` owns structural paths/state/availability/frontiers. `field-access.md` owns binding-root and bounded producer-backed receiver selection, direct field accessibility consumed by field selection, construction initializers, and recursive pattern fields, source-selected final-field duplicate-or-consume production, and producer-receiver remaining-frontier facts. `patterns.md` owns recursive record-pattern head resolution and structure, binding-leaf facts/order, direct-root ownership production, producer-transient ownership transitions, and pattern-transient frontier selection. `local-bindings.md` owns binding identity/scope/lookup/mutability/lifecycle and whole-binding use/assignment legality. `control-flow.md` owns represented conditional selection/arm validation/definite normal ownership, bounded-`while` condition selection/backedge-state admission/post-loop ownership, and nearest-loop break/continue target/state admission.

Additional operators, general expressions, arbitrary assignment places, additional loop forms, labeled transfers, transfer values, unrestricted nonterminal-within-block return, arbitrary-receiver members, refutable/rest/shorthand pattern categories, additional producer-backed scrutinee families, and other source forms remain outside this execution relation.

The represented Boolean logical-negation, construction, bounded producer-backed field-value, recursive pattern, partial-ownership cleanup, return, explicit-fault, bounded-loop-transfer cleanup, bounded-`while` execution sequencing, and existing producer execution relations are defined entirely by source identities, structural ownership, owned values, source order, transfer, transient ownership, local normal-continuation presence, fault-reason identity, operator-local value semantics, and cleanup. They do not add or alter Core operations or destruction rules. Any source-to-Core lowering must refine these source requirements and the separately owned operator/conditional/loop/transfer requirements through accepted Core semantics rather than use Core representation behavior as source authority.

Record-construction target qualification and record-pattern-head qualification are fully discharged by source validation. A faithful construction HIR requires only the resolved nominal record identity, resolved initializer field identities/types, validated initializer values, and source location already needed by construction. A faithful pattern HIR requires only the existing resolved top nominal record identity, complete binding-leaf paths/types/ownership facts, scrutinee facts, producer cleanup, and source location. Neither operation needs to retain qualified versus unqualified spelling or Core module/visibility metadata merely for qualification.

After source validation, Boolean logical negation may refine through the operation-specific accepted-Core relation owned by `operators.md`; this execution owner does not choose a second truth mapping or lower operation. Duplicating binding-root field use may refine to projected Core `Copy`, consuming binding-root field use to projected `Move`, and whole-binding replacement to source-first Core `Assign`. A producer-backed field-value use may lower its retained receiver producer through the existing value lowering relation, use the produced compiler-owned receiver local as the structural root, project the retained path, preserve the selected result through the retained `Copy`/`Move` consequence, and emit cleanup only for the retained source-selected receiver remaining frontier before returning the result temporary to its enclosing lower context. Lowering MUST NOT inspect Core path liveness or initialization state to choose the source duplicate/consume consequence or receiver cleanup frontier.

A direct-root recursive pattern binding leaf may refine to a mapped source local initialized in depth-first leaf order by projected `Copy`/`Move` from the mapped source root using the retained full leaf path. A producer-backed pattern may lower its existing producer to one compiler result temporary, initialize mapped pattern locals by projected `Copy`/`Move` from retained leaf paths, and refine the retained source transient frontier through projected/aggregate Core destruction. When that producer is a producer-backed field-value use, its receiver-result temporary and cleanup complete first; the preserved field result then becomes the separate pattern-scrutinee temporary. Qualified versus unqualified pattern-head spelling does not alter this lower relation.

A source return may refine to the existing Core `Return` terminator from whichever lower block represents that return point. Ordinary normal lexical-scope cleanup and ordinary normal `Goto` continuation are emitted only for source paths that actually have a local normal continuation; a returning path does not require a synthetic normal join/backedge. Source local normal-continuation presence and source cleanup selection MUST be established before lowering and MUST NOT be reconstructed from Core reachability, path-state worklists, scalar liveness, or initialization state.

A source-valid `fault;` may refine at its corresponding lower control point to the accepted Core explicit-fault terminator `Fault(F_explicit)`, where `F_explicit` is one stable represented Core semantic fault reason chosen to preserve the source reason `ExplicitFault`. This refinement emits no ordinary operand or result and no normal `Goto`/successor merely to continue source sequencing. The stable semantic reason is required; any implementation string, numeric code, allocation, or other carrier used to distinguish that Core reason remains non-normative and is not a source-visible payload.

A source-valid `continue;` may refine its retained source-selected exited-scope cleanup to existing Core destruction where applicable and then terminate the current lower path with existing Core `Goto` to the selected nearest loop's condition-header block. A source-valid `break;` may analogously refine its retained cleanup and `Goto` the selected nearest loop's post-loop block. Ending zero-leaf source ownership may erase to no Core `Drop`. The selected Core block identities are lower implementation facts rather than source loop identity.

A faithful typed HIR for a loop transfer may retain its transfer kind, source-selected cleanup sequence, source location, and enough lexical nesting/association for lowering to preserve the already validated nearest-loop destination. It need not retain `H`, `C`, Core block IDs, a source CFG, a transfer-kind lattice, or runtime completion tags. Lowering MUST NOT recompute source target-state validity from Core path state or insert destruction/reset of enclosing values merely to make a transfer valid.

Source ownership state, active-scope cleanup selection, and the fact that return/fault/transfer paths have no local continuation are established before lowering. Lowering MUST NOT reconstruct those source facts from Core reachability or path-state behavior. The accepted Core `Fault(F)` and `Goto` relations are consumed rather than redefined; these source features require no new Core operation.

Remaining source cleanup may refine to Core destruction only where the lower destruction domain is non-empty. Ending ownership of a zero-leaf source value may refine to no Core `Drop`; emitting an invalid lower destruction operation merely to materialize source ownership is not required.

Compiler temporaries used for producer-backed field receivers, producer-backed pattern scrutinees, represented operator results, or represented condition results are not source bindings. Core path state, scalar liveness, copyability, local numbering, destruction domains, and vacant-storage reuse are not source field/pattern/structural ownership authority. Accepted Core vacant initialization may be consumed by lowering to reuse those fixed compiler/source storage locations across a loop cycle, but it does not relax source mutability or source ordinary/continue backedge-state validity.

No parser, lossless syntax, typed HIR, Core MIR production lowering, runtime, or backend implementation is added or required by this semantic owner.

## Further boundaries

This revision does not define other literal semantics beyond the represented boolean, decimal integer, and decimal floating families; arithmetic/comparison operators or operator forms beyond the represented Boolean logical negation; compound assignment; assignment-as-value; conditional expressions; unequal-state/path-dependent two-normal-outcome conditional joins; unrestricted nonterminal-within-block return or arbitrary unreachable tails; additional loop forms (`loop`, `for`, do/while); loop `else`; labels or a label namespace; labeled break/continue; transfer values; loop values; general loop fixed-point inference; refutable-match control flow; field assignment/partial-field reinitialization; arbitrary value/expression field receivers beyond the bounded direct-call/record-construction receiver set; general postfix/member/method access; refutable/rest/shorthand/wildcard/literal/guard/alternative patterns; producer-backed pattern scrutinees beyond direct calls/record constructions/field-value uses; general expression/grouping scrutinees; destructuring assignment; qualified binding leaves/qualified field names/nested module paths beyond the represented alias-member pair; additional field accessibility classes beyond the represented module-private/exported relation; references/borrow syntax/lifetimes; indirect calls/function values/closures; generics/traits/coherence; async/tasks or Exec call semantics; effect-system completion; fault payload/message/code values; panic/throw syntax; catch/recovery; ABI/calling convention/FFI/linkage; parser/HIR/Core MIR production code; or backend behavior.
