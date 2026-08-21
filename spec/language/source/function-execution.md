# Source Function Execution

Status: **provisional normative; incomplete**

This document owns the represented source semantics for source function body attachment, straight-line body and nested-block execution order, dynamic direct-call activations, direct-call argument and result ownership transfer, record-construction field evaluation and transient assembly, local initialization, whole-binding assignment RHS evaluation and replacement ordering, lexical-scope and activation cleanup, direct return, recursion and divergence, and defined-fault propagation through direct source calls.

It consumes program outcomes and recoverable-value separation from [Program behavior](../behavior.md), environment admission and realization separation from [Program lifecycle](../lifecycle.md), defined-fault identity from [Core faults](../core/faults.md), structural destruction and stored-value cleanup from [Core value and storage semantics](../core/value-storage.md), function entity and callable-signature structure from [Source callables](callables.md), source value type equality and record value shape from [Source type foundation](types.md), boolean/integer literal value production from [Source literal semantics](literals.md), parameter/local binding identity, scope, lookup, assignment mutability, availability, ordinary whole-binding owned use, and assignment legality from [Source function-local bindings](local-bindings.md), and binding-rooted field-value production from [Source field-value access](field-access.md). It does not redefine those owners.

The represented concrete function/body/block/value/call/record-construction/field-value/assignment/return spellings and grammar are owned by [Source concrete syntax](concrete-syntax.md). Literal spelling, mathematical integer formation, required-type materialization, and representability are owned by `concrete-syntax.md` and `literals.md`. Field-path selection, direct field accessibility, and final-field duplicability are owned by `field-access.md`. This document owns execution consequences where those forms feed the receiving operations defined here; it does not own concrete spelling, literal typing, field selection, or parser representation.

This document does not define a universal expression taxonomy, operators, general control flow, references, closures, traits, ABI, or an implementation representation.

## Source function bodies

A represented source function entity MAY have one represented source body. It MUST NOT have more than one represented source body.

Attaching a represented source body to a function entity is a source-semantic fact. `concrete-syntax.md` defines one concrete function form that introduces a function entity and attaches the following concrete body to it. This execution relation does not require all future declaration and definition forms to use that same concrete construct.

In the represented direct-call subset, a direct source call targets exactly one resolved source function entity that has a represented source body.

A source function entity without a represented source body has no direct source execution relation under this document. Later FFI, external-function, intrinsic, or other callable owners may add distinct execution relations without redefining this one.

The represented direct call is statically bound to its resolved source function entity. This revision introduces no function-operand evaluation, overload selection, indirect call, function value, virtual dispatch, or closure call.

## Dynamic source function activations

Every dynamic direct call creates one distinct **source function activation** for the target source function entity.

The activation provides independent dynamic value and availability state for that function body's static parameter and ordinary local binding identities. Distinct simultaneously active or recursively nested calls therefore have distinct dynamic binding state even when they execute the same static declarations.

Activation identity exists only to distinguish dynamic executions. It is not source-observable, not a physical stack-frame address, not ABI identity, not task or thread identity, and not required to have a numeric runtime representation.

The callee body begins execution only after the parameter transfer defined below has completed and the corresponding parameter bindings are available under `local-bindings.md`.

## Recursion

Direct and mutual recursion are source-valid under this revision. Every recursive direct call creates a fresh source function activation.

A recursive execution may diverge.

A physical target limitation MUST NOT retroactively make otherwise valid Runen source invalid. When an applicable hard target or environment requirement cannot realize the accepted recursion semantics, environment admission may reject that target/environment combination or a legal realization may emulate the source semantics. A backend MUST NOT silently alter the source call relation.

This document does not define a recursion capability name, stack-size guarantee, tail-call guarantee, physical call stack, or concrete admission syntax.

## Owned value producers

This revision does not define a universal source expression taxonomy.

An **owned value producer** for this execution relation is an applicable accepted source operation whose successful evaluation yields exactly one owned source value.

The represented owned value producers sufficient for this revision are:

- a source-valid boolean or materialized decimal integer literal from `literals.md`;
- ordinary whole-binding owned-value use from `local-bindings.md`;
- a successful result-bearing direct call under this document;
- a source-valid named-field record construction under this document and `concrete-syntax.md`; and
- a source-valid binding-rooted field-value use under `field-access.md`.

`concrete-syntax.md` exposes exactly those five producer families in its current `Value` grammar. Future operator, conversion, consuming-field, or other expression owners MAY introduce additional owned value producers without redefining the receiving relations in this document.

Unless another accepted source owner defines a distinct rule for its producer, a producer used where this document requires a value MUST finish evaluation before that value is transferred to the receiving binding or transient ownership position.

Literal evaluation itself is effect-free, non-faulting, and non-diverging after source validation under `literals.md`. Represented field-value use is likewise effect-free, non-faulting, and non-diverging after source validation under `field-access.md`. This execution owner supplies the consuming position's required source type where applicable and then transfers the resulting owned value under the ordinary receiving rules below.

## Record construction

A source-valid represented record construction has one resolved same-module nominal record target and one named initializer for every declared field as mapped by `concrete-syntax.md`.

Construction produces exactly one owned source value of that nominal record type. The target is explicit rather than inferred from an outer required type. When an enclosing value consumer supplies a required source type, the construction result MUST be exactly equal to that required type under `types.md`.

Each initializer is associated with the selected declaration field identified by `concrete-syntax.md`. That declaration field's source type is the required source type supplied to the initializer's `Value` producer. The produced field value MUST have exactly that type under `types.md`; this relation introduces no conversion, coercion, widening, narrowing, defaulting, or inference.

In particular, a represented decimal integer literal used as a field initializer materializes under that selected field type through `literals.md`. This is the same required-type materialization relation used by the existing value consumers and does not create a conversion or an inferred constructor target.

Initializers evaluate strictly left to right in their concrete constructor source order, regardless of the target record's declaration field order. For each initializer in turn:

1. evaluate its `Value` producer completely under that producer's accepted semantics;
2. preserve every ownership and binding-availability transition caused by that evaluation; and
3. hold the successfully produced owned field value as one **transient construction value** associated with the selected declaration field.

A transient construction value is semantic ownership held by the in-progress construction. It does not require an independently source-addressable temporary, source binding, storage extent, field place, or other source identity.

If initializer `i` yields a defined fault before construction completes:

1. no record value is produced;
2. apply any producer-specific cleanup required within the failing initializer evaluation itself;
3. clean transient construction values already produced by earlier initializers in reverse production, and therefore reverse constructor-source, order;
4. preserve all binding availability and ownership transitions already caused by evaluated initializers; and
5. continue the same defined fault under the fault-propagation rules below.

In particular, if an earlier initializer consumed a non-duplicable binding, that binding remains unavailable while the transient value produced from it is cleaned exactly once after a later initializer faults.

If initializer `i` diverges, no record value is produced and no construction cleanup occurs merely because execution continues indefinitely. Transient construction values produced by earlier initializers remain owned by the suspended construction, and ownership or binding-availability transitions already performed remain effective.

Only after every initializer has completed successfully does assembly occur. Assembly transfers every transient construction value exactly once, without duplication, into its selected declaration field and thereby forms one owned record value with the declaration-defined field/value shape from `types.md`. The resulting structural field order is the record declaration order; constructor source order controls evaluation and transient production, not the semantic record field sequence.

Assembly itself is effect-free after successful initializer evaluation and introduces no additional defined fault or divergence. After a field transient is transferred into the record result, it is no longer independently owned by the construction and MUST NOT be cleaned separately from that result.

For a zero-field record construction there are no initializer producers or construction transients. Successful construction immediately produces the owned empty record value of the resolved nominal type.

A result-bearing direct call used as a field initializer must complete successfully before its field transient exists and before any later initializer begins. A nested record construction used as a field initializer recursively follows this same relation before producing its field value.

The completed record value is otherwise an ordinary owned value producer result. It may be transferred by the existing local-initialization, whole-binding assignment RHS, direct-call argument, return-result, or enclosing record-construction field relations. Those consuming relations keep their existing outer ordering and exact-type requirements.

This represented construction relation itself adds no field-value access, field assignment, partial field move or availability relation, destructuring, update/spread/default initialization, field-init shorthand, positive duplicability selection, constructor or method body, or cross-module construction contract. Represented binding-rooted duplicable field-value access is independently owned by `field-access.md`.

## Direct-call arguments

A represented direct call has exactly one ordered argument operand for each callable-signature parameter slot. Argument count MUST match the callable signature exactly.

Each argument evaluation MUST produce one owned source value whose source type is exactly equal under `types.md` to the corresponding parameter source type. This revision introduces no implicit conversion, coercion, widening, narrowing, subtyping, or numeric defaulting.

The corresponding parameter source type is the required source type supplied to an argument producer whose canonical semantics require one. In particular, a represented decimal integer literal argument materializes under that parameter type through `literals.md`; this does not create a conversion or inference relation.

Arguments are evaluated left to right in their ordered call/signature sequence.

When argument evaluation uses a complete parameter or local binding, that use is ordinary whole-binding owned-value use under `local-bindings.md` unless another accepted source owner explicitly defines a different context. Any resulting duplicate-or-consume transition therefore occurs in the same left-to-right order as argument evaluation. A represented field-value argument instead follows `field-access.md` and leaves its root binding available.

Each successfully evaluated argument is held as one owned **transient argument value** until all arguments have evaluated successfully. Transient ownership is semantic and does not require a materialized temporary storage place.

If evaluation of argument `i` yields a defined fault before callee activation:

1. no callee activation is created;
2. transient argument values already produced for earlier arguments are cleaned in reverse production order;
3. binding availability changes already caused while evaluating earlier arguments remain in effect; and
4. the same defined fault continues in the caller activation under the fault-propagation rules below.

If evaluation of an argument diverges, no callee activation is created and the caller remains suspended in that argument evaluation. Earlier transient argument values remain owned by the suspended computation; passage of time alone does not trigger cleanup.

After all arguments evaluate successfully:

1. create the callee activation;
2. transfer each transient argument value, in parameter-slot order, into the corresponding parameter binding of that activation; and
3. make each parameter binding available with that transferred value under `local-bindings.md`.

The transfer does not duplicate a transient argument value.

Parameter slots in this represented direct-call relation are owned-value parameters. This rule neither introduces nor prohibits future reference, borrow, or other pass-mode parameter forms.

## Local initialization

When a represented ordinary local initializer evaluates an owned value producer, evaluation completes before the produced value is transferred into the local binding.

The local binding's declared source type is the required source type supplied to an initializer producer whose canonical semantics require one. A represented decimal integer literal initializer therefore materializes under that declared type through `literals.md` before transfer.

The produced value's source type MUST be exactly equal under `types.md` to the local binding's declared source type.

After the transfer completes, the local binding becomes available under `local-bindings.md`. The transfer does not duplicate the produced value.

If initializer evaluation yields a defined fault or diverges, that local binding never becomes available. Ownership and availability transitions already performed by earlier evaluated operations remain effective.

## Whole-binding assignment and replacement

A represented whole-binding assignment consumes the assignment target legality, mutability, declared type, RHS type requirement, and pre/post-assignment availability relation defined by `local-bindings.md`.

The selected assignment target's declared source type is the required source type supplied to an RHS producer whose canonical semantics require one. A represented decimal integer literal RHS therefore materializes under that target type through `literals.md` before replacement execution.

For a source-valid assignment, execution is **source-first** with respect to replacement:

1. evaluate the assignment RHS completely as the owned value producer required by the source-valid assignment relation;
2. preserve every ownership and availability transition caused while evaluating that RHS;
3. preserve the successfully produced owned RHS value outside the target's old-value cleanup set until replacement transfer completes;
4. only after successful RHS value production, observe whether the target binding is then available;
5. if the target is then available, clean its old owned value exactly once;
6. if the target is then unavailable, there is no old target-owned value to clean;
7. transfer the produced RHS value into the target binding without duplication; and
8. make the target binding available as required by `local-bindings.md`.

The assignment target remains in scope during RHS evaluation. Consequently, ordinary RHS use of the target follows the existing whole-binding owned-use relation rather than a special self-assignment rule. A field-value RHS rooted at the target follows `field-access.md` and does not consume or partially consume the target before replacement.

For a duplicable mutable target `x`, `x = x` duplicates the old value during RHS evaluation and leaves `x` available; replacement then cleans that old target-owned value and transfers the duplicate into `x`.

For a non-duplicable mutable target `x`, `x = x` consumes the old value during RHS evaluation and leaves `x` unavailable; replacement therefore has no old target-owned value to clean and transfers the produced owned value back into `x`.

The same ordering applies when RHS direct-call argument evaluation consumes the assignment target before that call successfully produces the replacement value.

If RHS evaluation yields a defined fault, assignment performs no replacement cleanup and no replacement transfer. Ownership and availability transitions already caused during RHS evaluation remain effective, and the same fault then follows the activation cleanup and propagation rules below.

If RHS evaluation diverges, assignment performs no replacement cleanup or replacement transfer. The enclosing function activation remains suspended in the RHS evaluation, and no cleanup occurs merely because execution has continued indefinitely.

Cleaning an old target-owned value under this relation uses the source cleanup rule below. Applicable structural destruction domains, stored-value lifetime endings, storage state, storage extent, and storage-instance identity remain owned by [Core value and storage semantics](../core/value-storage.md). This source relation selects source ownership and ordering; it does not reproduce Core storage state or destruction mechanics.

This revision defines no field/member/place assignment, compound assignment, assignment expression value, borrow/reference assignment, source interior mutability, raw-pointer assignment, or destructuring assignment.

## Straight-line body and nested-block execution

For the root function-body form and each represented `BlockStatement` in `concrete-syntax.md`, the applicable `BodyStatement` sequence executes strictly in concrete source order.

Root-body execution begins with its first body statement after successful parameter transfer. A nested block begins execution when that block statement is reached in its containing statement sequence. In either sequence, a later body statement begins only after the preceding statement completes normally.

For a represented ordinary local declaration statement:

1. evaluate its initializer under the local-initialization and owned-value rules above;
2. transfer the produced value into the new local binding and make that binding available; and
3. only after that transfer completes normally may the next body statement begin.

For a represented whole-binding assignment statement:

1. evaluate and complete the assignment under the whole-binding assignment/replacement rules above; and
2. only after the replacement value has been transferred and the target is available may the next body statement begin.

For a represented no-result direct-call statement:

1. evaluate the direct call under the call rules in this document;
2. require the called function's signature to specify no result value as mapped by `concrete-syntax.md`; and
3. only after the call completes normally may the next body statement begin.

A valid no-result call statement has no produced source value and therefore performs no arbitrary result discard.

For a represented nested block statement:

1. the child lexical scope established for that block under `local-bindings.md` is active while its contained `BodyStatement` sequence executes;
2. execute that sequence recursively under the same straight-line statement rules in concrete source order;
3. if every contained statement completes normally, normally exit the child lexical scope using the lexical-scope cleanup relation below; and
4. only after that child-scope cleanup completes may the next statement in the containing sequence begin.

A represented block statement produces no source value and introduces no Unit, Void, or equivalent value.

If execution of a body statement yields a defined fault, later body statements in the active sequence do not execute and the fault cleanup/propagation rules below apply to the active function activation. A nested block that exits this way does not also perform a separate normal child-scope cleanup; its active child scope participates exactly once in the defined-fault cleanup relation.

If execution of a body statement diverges, later body statements in the active sequence do not execute and no termination or child-scope cleanup occurs merely because execution continues indefinitely. A diverging operation inside a nested block leaves that child scope active in the suspended computation.

When the concrete root body has a terminal return statement, that return begins only after every preceding root body statement has completed normally and then follows the normal-return rules below.

When a represented no-result concrete root body reaches its closing body boundary without a terminal return, it performs the accepted normal no-result completion described below.

This straight-line relation introduces no branch, loop, early/nonterminal return, unreachable-statement, short-circuit, catch, defer, or other multiple-path/control-transfer semantics.

## Source cleanup

For the represented operations in this document, **cleaning an owned source value** ends the source execution's ownership of that value exactly once.

When such a source value is realized in Core storage, applicable destruction-domain, stored-value-lifetime, and cleanup semantics remain owned by `core/value-storage.md`; this document determines only the source-owned value or binding selected for cleanup and the source order in which those selections occur.

A source value that has already been transferred or consumed is not cleaned again by its former owner. This applies equally to lexical-scope/activation cleanup, transient construction/argument cleanup, and old target-owned values selected for assignment replacement.

This revision introduces no custom source destructor body, source `drop` ability, or general temporary-lifetime extension rule.

## Lexical-scope cleanup

When execution normally exits a represented lexical scope, each still-available ordinary local binding declared directly in that scope is cleaned in **reverse local declaration order**. An unavailable binding owns no value to clean.

When one source completion exits multiple active lexical scopes, cleanup proceeds from the innermost active scope outward. Each scope uses reverse local declaration order.

Function parameters belong to the root activation under `local-bindings.md` but are not ordinary local declarations. On source function activation termination, after the root lexical scope's still-available ordinary locals have been cleaned, still-available parameter bindings are cleaned in **reverse callable-signature parameter-slot order**.

This cleanup ordering is semantic and independent of physical stack layout, ABI passing, compiler local numbering, Core local numbering, or backend cleanup strategy.

## Normal return

For a source function whose callable signature has one result type, that result type is the required source type supplied to a return-value producer whose canonical semantics require one. A represented decimal integer literal return therefore materializes under the declared result type through `literals.md`.

A represented return MUST first evaluate exactly one owned value producer whose source type is exactly equal under `types.md` to that result type.

Result evaluation completes before any return-induced lexical-scope or activation cleanup.

After successful result evaluation:

1. preserve the owned transient result value outside the activation-local cleanup set;
2. clean all active lexical scopes from innermost through the root lexical scope using the rules above;
3. clean still-available parameter bindings in reverse parameter-slot order;
4. terminate the callee activation normally; and
5. deliver the owned transient result value to the caller as the successful result of the direct call.

The transfer to the caller does not duplicate the result value.

Consequently, when return evaluation obtains a non-duplicable local through ordinary whole-binding owned use, that binding becomes unavailable before cleanup and is not cleaned again as a remaining local value. A represented field-value return leaves its root binding available, so the complete root remains selected for ordinary activation cleanup after the duplicated field result has been preserved for transfer.

For a source function whose callable signature specifies no result value, a represented no-result return performs the same scope and parameter cleanup and normal activation termination but produces no source value.

Reaching the normal end of a represented no-result function body is equivalent to normal no-result completion.

A result-bearing represented function body MUST NOT have a reachable normal end that produces no result. The current concrete body grammar enforces that requirement by requiring a terminal value return for a result-bearing concrete function. A later body/control-flow owner MUST preserve the requirement for any constructs it adds.

No Unit, Void, or equivalent source value is introduced by no-result completion.

## Defined-fault propagation

The represented source subset has no catch boundary.

When an applicable accepted source or Core operation yields a defined fault `F` during one source function activation:

1. clean all active lexical scopes in that activation from innermost through the root lexical scope using the same available-binding rules;
2. clean still-available parameter bindings in reverse parameter-slot order; and
3. terminate that activation with the same defined fault `F`.

If the defined fault arises from a directly called callee, the caller's direct-call evaluation yields `F`. Because no represented catch boundary exists, the caller then performs its own fault cleanup and propagates `F` outward. This continues until the outermost applicable source execution denotes the defined-fault outcome under `behavior.md`.

“Same defined fault” preserves the semantic defined-fault outcome selected by the initiating operation. This revision does not define fault payload representation, strings or messages, numeric fault codes, physical exception objects, backtraces, panic syntax, or catch syntax.

This propagation is semantic unwinding of source ownership. It does not require physical stack unwinding. A realization MAY use another mechanism only when it preserves every applicable cleanup and observable behavior required by the accepted source and Core contracts.

A future catch or panic owner may introduce explicit source forms and extend the applicable propagation relation at those explicit boundaries. No such boundary is represented here.

Recoverable domain or application failures represented as ordinary source values remain ordinary values under `behavior.md`; they do not use this defined-fault propagation relation merely because they represent failure.

## Transient-value cleanup

Transient construction values already produced when a later record initializer yields a defined fault are cleaned in reverse constructor production/source order before the same fault continues in the enclosing activation. Once all construction transients have been transferred into a successful record result, they are no longer independently remaining transient values.

Transient argument values already produced when a later argument yields a defined fault are cleaned in reverse production order before the defined fault continues in the caller activation.

An owned transient return result is not part of callee activation-local cleanup after successful result evaluation because ownership has already been separated for transfer to the caller.

A successfully produced assignment RHS value is transferred into the assignment target and therefore is not an independently remaining transient after successful assignment completion. If RHS production faults before successful value production, existing producer-specific transient cleanup rules remain controlling.

Represented field-value use creates no producer-specific intermediate transient ownership. Once its one duplicated field value is successfully produced, the receiving operation owns the ordinary transient/transfer consequence required by its context.

This revision does not define general temporary lifetime extension, expression-statement discard, or arbitrary temporary cleanup. Only transient values required by represented record construction, direct-call argument, return, and assignment transfer are owned here.

## Divergence

If a record-construction initializer diverges, the enclosing construction remains suspended in that initializer evaluation. Earlier transient construction values and ownership/availability changes remain as defined above; no construction, activation, or lexical-scope cleanup occurs merely because execution continues indefinitely.

If a directly called callee diverges, the caller remains suspended at that direct call and does not perform return or fault cleanup merely because execution continues indefinitely.

Active caller and callee ownership state, together with any transient values retained by the suspended evaluation, persists subject to operations already performed. The same applies when the diverging call is the RHS of a represented assignment. There is no implicit source execution-step budget.

Field-value use itself cannot diverge after source validation under `field-access.md`.

## Effects boundary

Left-to-right record-initializer evaluation, left-to-right argument evaluation, source-first assignment RHS evaluation, and concrete straight-line body/nested-block execution fix relative source ordering for any effects that applicable future expression or operation owners make observable.

Literal evaluation has no source-visible side effect under `literals.md`; represented field-value use likewise has no source-visible side effect under `field-access.md`. Adding either producer to these positions therefore adds no competing effect-order relation.

Record assembly after successful initializer evaluation is itself effect-free under the represented construction relation. Initializer source order, rather than record declaration order, remains the ordering authority for producer effects.

This revision does not define a source effect system, purity, effect inference, speculation legality, or general transformation rules.

## Concrete grammar and implementation boundary

`concrete-syntax.md` owns the currently represented concrete record/function/type/local/value/literal/call/record-construction/field-value/assignment/block/return grammar and its mapping to the semantic relations used here. This execution owner does not duplicate those spellings or punctuation rules. `literals.md` owns represented boolean and integer literal value/materialization semantics. `field-access.md` owns represented binding-rooted field-path selection, direct field accessibility, and final-field duplication semantics.

Floating and other unrepresented literals, arithmetic or comparison operators, assignment expressions or general assignment places, branches, loops, consuming/partial field access, arbitrary-receiver members, and other concrete source forms remain outside the represented execution relation.

The source record-construction relation is defined entirely by source record/field identity, owned values, source evaluation order, transient ownership, transfer, and cleanup. It does not add or alter a Core operation, aggregate initialization rule, destruction-domain rule, or cleanup rule. The source field-value relation likewise does not make Core projections or copyability source authority. Any source-to-Core lowering must refine these source requirements through accepted Core semantics rather than using Core representation behavior as source authority.

No parser, lossless-syntax representation, typed HIR, Core MIR production lowering, runtime implementation, or backend implementation is added or required by this semantic owner.

## Further boundaries

This revision does not define literal spelling or materialization, floating/other literal semantics, arithmetic/comparison/operator forms, compound assignment, assignment-as-value, branch/loop/pattern control flow, consuming/non-duplicable field access, partial-field moves/availability, field assignment, arbitrary-receiver member/method access, destructuring, qualified/cross-module record construction, record update/default/shorthand forms, references/borrow syntax/lifetimes, indirect calls/function values/closures, generics/traits/coherence, async/tasks or Exec call semantics, effect-system completion, panic payload/catch syntax, ABI/calling convention/FFI/linkage, parser/lossless syntax/HIR/Core MIR production code, or backend behavior.
