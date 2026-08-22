# Source Function Execution

Status: **provisional normative; incomplete**

This document owns the represented source semantics for source function body attachment, straight-line body and nested-block execution order, dynamic direct-call activations, direct-call argument/result ownership transfer, record-construction field evaluation and transient assembly, ordinary local initialization, recursive record-destructuring declaration completion including producer-backed scrutinee evaluation and transient cleanup, whole-binding assignment RHS evaluation and replacement ordering, lexical-scope and activation cleanup, direct return, recursion/divergence, and defined-fault propagation through direct source calls.

It consumes program outcomes and recoverable-value separation from [Program behavior](../behavior.md), environment admission and realization separation from [Program lifecycle](../lifecycle.md), defined-fault identity from [Core faults](../core/faults.md), structural destruction and stored-value cleanup from [Core value and storage semantics](../core/value-storage.md), function entity/callable-signature structure from [Source callables](callables.md), source value type equality and record value shape from [Source type foundation](types.md), boolean/integer literal value production from [Source literal semantics](literals.md), structural ownership state and remaining frontiers from [Source structural ownership](structural-ownership.md), parameter/local identity, scope, lookup, assignment mutability, whole-binding use, and assignment legality from [Source function-local bindings](local-bindings.md), binding-rooted field-value production from [Source field-value access](field-access.md), and recursive exhaustive record-pattern structure, scrutinee category, binding-leaf order, per-leaf ownership consequences, and producer transient frontier selection from [Source patterns](patterns.md). It does not redefine those owners.

The represented concrete function/body/block/value/call/record-construction/field-value/record-destructuring/assignment/return grammar is owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, universal expressions, operators, general control flow, references, closures, traits, ABI, or an implementation representation.

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

- a source-valid boolean or materialized decimal integer literal from `literals.md`;
- ordinary whole-binding owned-value use from `local-bindings.md`;
- a successful result-bearing direct call under this document;
- a source-valid named-field record construction under this document and `concrete-syntax.md`; and
- a source-valid binding-rooted field-value use under `field-access.md`.

`concrete-syntax.md` exposes exactly those five families in `Value`. A record-destructuring declaration is not another `Value` producer: `patterns.md` owns its grouped production of zero or more binding-leaf values. A producer-backed record-pattern scrutinee reuses one existing producer family in a pattern-specific receiving position.

Future operator, conversion, or other expression owners MAY introduce additional owned value producers without redefining the receiving relations in this document.

Unless another accepted source owner defines a distinct rule, a producer used where this document requires a value MUST finish evaluation before that value is transferred to its receiving binding/transient owner.

Literal evaluation is effect-free, non-faulting, and non-diverging after source validation. This execution owner supplies a required source type where contextual integer materialization needs it and then transfers the resulting value.

A source-valid field-value producer may change a binding's structural ownership state before its result reaches the receiving position: a duplicable final field leaves ownership unchanged, while a non-duplicable final field consumes exactly the selected structural path under `field-access.md` and `structural-ownership.md`.

## Record construction

A source-valid represented record construction has one resolved same-module nominal record target and one named initializer for every declared field as mapped by `concrete-syntax.md`.

Construction produces exactly one owned source value of that nominal record type. The target is explicit rather than inferred. When an enclosing consumer supplies a required source type, the construction result MUST be exactly equal to that type under `types.md`.

Each initializer is associated with one selected declaration field. That field's source type is the required type supplied to the initializer's `Value` producer. The produced field value MUST have exactly that type; no conversion, coercion, widening, narrowing, defaulting, or inference is introduced.

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

A result-bearing call or nested construction used as an initializer must complete before that field transient exists and before a later initializer begins.

The completed record value may be transferred into ordinary local initialization, assignment RHS, direct-call argument, return result, enclosing construction field, or a producer-backed recursive record-pattern scrutinee. Those receiving relations keep their existing outer ordering and exact-type requirements.

This relation adds no field access/assignment, partial-field reinitialization, pattern selection, update/spread/default initialization, shorthand, positive duplicability selection, method/constructor body, or cross-module construction contract.

## Direct-call arguments

A represented direct call has exactly one ordered argument operand for each callable-signature parameter slot. Argument count MUST match exactly.

Each argument evaluation MUST produce one owned source value whose type equals exactly the corresponding parameter source type. This revision introduces no implicit conversion, coercion, widening, narrowing, subtyping, or numeric defaulting.

The parameter type is the required source type supplied to a producer that needs contextual typing. Decimal integer arguments therefore materialize under the corresponding parameter type through `literals.md`; this does not create a conversion or inference relation.

Arguments evaluate left to right.

Ordinary complete-binding argument use follows `local-bindings.md`; a binding-rooted field-value argument follows `field-access.md`. Any ownership transition occurs at that argument's evaluation position.

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

The local's declared type is the required type supplied to a producer that needs one. A represented decimal integer literal initializer therefore materializes under that declared type through `literals.md` before transfer. The produced value MUST have exactly that type.

After transfer, the local begins with complete initial structural ownership of its value under `local-bindings.md` and `structural-ownership.md`. Transfer does not duplicate the produced value.

If initializer evaluation yields a defined fault or diverges, the local never receives an owned value. Ownership transitions already performed by earlier evaluated operations remain effective.

## Record-destructuring declaration completion

A source-valid recursive record-destructuring declaration consumes the complete pattern structure, scrutinee category, exact top scrutinee type, binding-leaf order, leaf paths/types, direct-root leaf availability requirements, per-leaf duplicate-or-consume consequences, and producer-transient remaining frontier from `patterns.md` and `structural-ownership.md`.

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
2. evaluate the selected direct-call, record-construction, or field-value producer completely using the top pattern head's nominal record type as the exact required type and the pre-pattern-binding lexical environment;
3. if producer evaluation faults or diverges, perform no pattern leaf production;
4. on producer success, hold the produced record as one fully owned pattern scrutinee transient whose structural ownership state begins complete;
5. apply pattern-owned binding-leaf `Duplicate`/`Consume` productions in retained depth-first source order;
6. after every leaf production, clean the transient's remaining structural ownership frontier selected by `patterns.md` through `structural-ownership.md` exactly once;
7. only after transient cleanup completes, establish all pattern-introduced bindings in the containing lexical scope together; and
8. only then may the next body statement begin.

The pattern scrutinee transient is not a local binding and does not participate in lexical/activation cleanup after step 6.

Successful leaf production and transient cleanup introduce no new defined-fault or divergence outcome after producer success under the represented relation.

If producer evaluation yields a defined fault before transient establishment:

- no pattern leaf production occurs;
- no pattern binding enters scope;
- producer-internal transient cleanup occurs exactly as in another receiving position;
- ownership transitions already completed by producer evaluation remain effective; and
- the same fault continues through activation fault propagation.

If producer evaluation diverges before transient establishment, no pattern leaf production, pattern binding establishment, or pattern-declaration transient cleanup occurs. Producer-owned transients remain owned by the suspended producer.

For a producer-backed zero-field top pattern, successful producer evaluation yields one complete empty-record transient. There are no binding leaves; its canonical remaining frontier contains the complete empty root, whose source ownership ends before declaration completion even when lower scalar cleanup is vacuous.

This section does not redefine pattern-head/field selection, structural path validity, source duplicability, producer syntax, or frontier membership.

## Whole-binding assignment and replacement

A represented whole-binding assignment consumes assignment target legality, mutability, declared type, RHS type requirement, and binding structural lifecycle from `local-bindings.md`, together with remaining-frontier selection from `structural-ownership.md`.

The target's declared source type is the required type supplied to an RHS producer that needs one. A represented decimal integer literal RHS therefore materializes under that target type through `literals.md` before replacement execution.

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

The same ordering applies when direct-call argument evaluation, record-construction initializer evaluation, or another represented producer consumes the complete target or one of its structural subvalues before a later operation successfully produces the replacement value.

If RHS evaluation yields a defined fault, assignment performs no replacement cleanup/reset/transfer. Completed ownership transitions remain effective and activation fault cleanup uses the target's resulting current state.

If RHS evaluation diverges, assignment performs no replacement cleanup/reset/transfer. The activation remains suspended and no cleanup occurs merely because execution continues.

Core structural destruction/storage mechanics remain owned by Core; this source relation selects source ownership and source ordering.

This revision defines no field/place assignment, partial-field reinitialization, compound assignment, assignment expression, borrow/reference assignment, source interior mutability, raw-pointer assignment, or destructuring assignment.

## Straight-line body and nested-block execution

For the root function body and each represented `BlockStatement`, the applicable `BodyStatement` sequence executes strictly in concrete source order.

Root-body execution begins with its first statement after successful parameter transfer. A nested block begins when its statement is reached. A later statement begins only after the preceding statement completes normally.

For an ordinary local declaration:

1. evaluate its initializer;
2. transfer the value into the new binding and establish complete initial ownership; and
3. only then continue.

For a recursive record-destructuring declaration:

1. complete the grouped pattern declaration under the applicable direct-root or producer-backed completion relation; and
2. only after any producer transient cleanup and grouped binding establishment may the next statement begin.

For whole-binding assignment, complete RHS production, old-value cleanup, replacement transfer, and target ownership reset before continuing.

For a no-result direct-call statement, complete the call normally before continuing. A valid no-result call statement has no value to discard.

For a nested block:

1. activate its child lexical scope;
2. execute its contained `BodyStatement`s recursively in concrete order;
3. on normal completion, normally exit the child scope using lexical-scope cleanup below; and
4. only after that cleanup may the containing sequence continue.

A block statement produces no source value and introduces no Unit/Void value.

If a body statement yields a defined fault, later statements do not execute and the active function activation follows fault cleanup/propagation. A nested block exiting this way does not also perform independent normal cleanup; its child scope participates exactly once in fault cleanup.

If a body statement diverges, later statements do not execute and no termination/child-scope cleanup occurs merely because execution continues.

A root terminal return begins only after all preceding root body statements complete normally.

A represented no-result root body reaching its closing boundary without a terminal return performs normal no-result completion.

This straight-line relation introduces no branch, loop, early/nonterminal return, unreachable-statement semantics, short-circuiting, catch, defer, refutable match, or other multi-path control transfer.

## Source cleanup

For represented operations, **cleaning an owned source value** ends source execution's ownership of that value exactly once.

A binding cleanup selects its complete-root remaining ownership frontier under `structural-ownership.md`. Each frontier path denotes one maximal still-owned source subvalue. Cleaning the binding means cleaning those frontier subvalues exactly once in canonical frontier order and then ending the binding's source ownership.

A producer-backed recursive record-pattern transient uses the same structural frontier relation over its own non-binding structural owned-value root. Cleaning that transient means cleaning those frontier values exactly once in canonical frontier order and then ending all transient ownership. It does not make the transient a binding or add it to lexical/activation cleanup.

When a source value is realized in Core storage, applicable destruction-domain, stored-value-lifetime, and cleanup semantics remain owned by [Core value and storage semantics](../core/value-storage.md). This document determines only source ownership-ending selection and source order.

A value already transferred or consumed is not cleaned again by its former owner. This applies to lexical/activation cleanup, construction/argument transients, producer-backed pattern transients, and old assignment-target subvalues.

A fully available zero-field or recursively zero-leaf source subvalue remains a legitimate cleanup value. Cleaning it ends source ownership even when lower representation has no scalar destruction leaf and therefore needs no physical/Core `Drop`.

This revision introduces no custom source destructor body, source `drop` ability, must-consume policy, or general temporary-lifetime extension rule.

## Lexical-scope cleanup

When execution normally exits a represented lexical scope, consider all local bindings declared directly in that scope in **reverse local declaration order**. This includes ordinary locals and bindings introduced by recursive record-destructuring declarations.

For each binding, select its then-current complete-root remaining frontier under `structural-ownership.md` and clean every frontier member in canonical order.

For one record-destructuring declaration, `patterns.md` defines depth-first binding-leaf source order as the declaration order of the introduced bindings. Reverse local declaration cleanup therefore visits later binding leaves before earlier leaves, independently of record structural field order.

A fully available binding frontier contains only its complete root. An unavailable complete root has an empty frontier. A partial root cleans exactly the maximal still-owned disjoint subvalues and never re-cleans a consumed path.

When one source completion exits multiple active scopes, cleanup proceeds innermost to outermost. Each scope uses reverse local declaration order, and each binding uses its canonical remaining frontier.

Function parameters belong to the root activation but are not local declarations. On activation termination, after root lexical locals, process parameters in **reverse callable-signature parameter-slot order**, each using its then-current frontier.

This cleanup order is semantic and independent of physical stack layout, ABI passing, compiler/Core local numbering, or backend strategy.

## Normal return

For a source function with one result type, that result type is the required type supplied to the return-value producer. A represented decimal integer literal return therefore materializes under the declared result type through `literals.md`.

A represented return MUST first evaluate exactly one owned value producer whose type equals exactly that result type.

Result evaluation, including any structural ownership transition caused by a consuming producer, completes before return-induced scope/activation cleanup.

After successful result evaluation:

1. preserve the owned transient result outside activation-local cleanup;
2. clean all active lexical scopes innermost through root;
3. process parameters in reverse slot order, cleaning each current frontier;
4. terminate the callee activation normally; and
5. deliver the owned transient result to the caller.

Transfer to the caller does not duplicate the result.

A complete non-duplicable local consumed by result evaluation has no remaining frontier and is not cleaned again. A consumed subvalue is excluded while disjoint remaining subvalues are cleaned normally.

For a no-result function, represented `return;` performs the same scope/parameter cleanup and normal activation termination but produces no value.

Reaching the normal end of a represented no-result function body is equivalent to normal no-result completion.

A result-bearing represented body MUST NOT have a reachable normal end without a result. The current concrete grammar enforces this with a terminal value return. A later control-flow owner MUST preserve that requirement.

No Unit/Void source value is introduced by no-result completion.

## Defined-fault propagation

The represented source subset has no catch boundary.

When an applicable accepted source/Core operation yields defined fault `F` during an activation:

1. preserve every ownership transition completed before `F` was selected;
2. clean all active lexical scopes innermost through root using each binding's current remaining frontier;
3. process parameters in reverse slot order using each current frontier; and
4. terminate that activation with the same defined fault `F`.

If the fault arises from a directly called callee, the caller's direct-call evaluation yields `F`; with no catch boundary, the caller performs its own fault cleanup and propagates the same fault outward. This continues to the outermost applicable source execution.

“Same defined fault” preserves the semantic fault outcome selected by the initiating operation. This revision defines no payload representation, messages, numeric codes, exception objects, backtraces, panic syntax, or catch syntax.

This propagation is semantic unwinding of source ownership and does not require physical stack unwinding. A realization MAY use another mechanism only when it preserves every applicable cleanup and observable behavior required by the accepted source and Core contracts.

A future catch or panic owner may introduce explicit source forms and extend the applicable propagation relation at those explicit boundaries. No such boundary is represented here.

Recoverable domain/application failures represented as ordinary values remain ordinary values under `behavior.md`; they do not use this relation merely because they represent failure.

## Transient-value cleanup

Construction transients produced before a later initializer fault are cleaned in reverse construction production/source order before the same fault continues. Once transferred into a successful record result, they are no longer independently owned.

Argument transients produced before a later argument fault are cleaned in reverse production order before the fault continues in the caller.

An owned transient return result is outside callee activation-local cleanup after successful result evaluation because ownership has been separated for caller transfer.

A successfully produced assignment RHS is transferred into the target and is not independently remaining after successful assignment. If RHS production faults before success, producer-specific transient cleanup remains controlling.

A transient value produced by consuming a non-duplicable field is owned by its current transient position after production; its former binding path remains consumed and does not re-enter that binding's frontier if a later producer faults.

A producer-backed recursive record-destructuring declaration owns one pattern scrutinee transient only after producer success. Pattern binding-leaf production may consume arbitrary retained structural paths from that transient. After all leaf production, the declaration cleans exactly the canonical remaining structural frontier before new bindings enter scope. The transient then ends completely and does not participate in later lexical/activation cleanup.

A direct binding-root record pattern has no independently owned scrutinee transient; its accepted leaf productions initialize final pattern bindings directly.

This revision defines no general temporary lifetime extension, expression-statement discard, or arbitrary temporary cleanup. Only transient values required by represented record construction, direct-call argument/result transfer, assignment transfer, and producer-backed record destructuring are owned here.

## Divergence

If a record-construction initializer diverges, the construction remains suspended in that initializer. Earlier construction transients and completed ownership transitions remain; no construction/activation/scope cleanup occurs merely because execution continues.

If a directly called callee diverges, the caller remains suspended at that call and performs no return/fault cleanup merely because time passes.

Active caller/callee ownership state and any suspended producer transients persist subject to operations already completed. The same applies when a diverging call is an assignment RHS or producer-backed record-pattern scrutinee. There is no implicit source execution-step budget.

A direct binding-root record-destructuring operation has no divergence point after validation. A producer-backed operation may diverge only while evaluating its existing producer; after producer success, leaf production and transient completion are non-diverging.

## Effects boundary

Left-to-right constructor evaluation, left-to-right argument evaluation, source-first assignment RHS evaluation, producer-before-pattern evaluation, **depth-first pattern binding-leaf source order**, and concrete straight-line body/block execution fix relative source ordering for any effects that future accepted operation owners make observable.

Literal evaluation has no source-visible side effect under `literals.md`; adding literals to represented ordinary value positions therefore adds no competing effect-order relation.

Field-value production is non-faulting/non-diverging after source validation, but consuming a non-duplicable field performs its structural ownership transition at that producer position. When field-value use is a producer-backed pattern scrutinee, that transition completes before pattern transient leaf production.

Pattern binding-leaf production is non-faulting/non-diverging after source validation and any producer completion. Its source-ordered non-duplicable leaf transfers are ownership transitions whose consequences are visible to later leaves and statements.

Record assembly after successful initializer evaluation is effect-free. Initializer source order, rather than declaration field order, remains producer-effect ordering authority.

This revision defines no source effect system, purity, effect inference, speculation legality, or general transformation rules.

## Concrete grammar and implementation boundary

`concrete-syntax.md` owns represented concrete grammar. `literals.md` owns boolean/integer materialization. `structural-ownership.md` owns structural paths/state/availability/frontiers. `field-access.md` owns binding-rooted field selection/accessibility and final-field duplicate-or-consume production. `patterns.md` owns recursive record-pattern structure, binding-leaf facts/order, direct-root ownership production, producer-transient ownership transitions, and transient frontier selection. `local-bindings.md` owns binding identity/scope/lookup/mutability/lifecycle and whole-binding use/assignment legality.

Floating literals, operators, general expressions, arbitrary assignment places, branches/loops, arbitrary-receiver members, refutable/rest/shorthand pattern categories, additional producer-backed scrutinee families, and other source forms remain outside this execution relation.

The represented construction, recursive pattern, and partial-ownership cleanup relations are defined entirely by source identities, structural ownership, owned values, source order, transfer, transient ownership, and cleanup. They do not add or alter Core operations or destruction rules. Any source-to-Core lowering must refine these source requirements through accepted Core semantics rather than use Core representation behavior as source authority.

After source validation, duplicating field use may refine to projected Core `Copy`, consuming field use to projected `Move`, and whole-binding replacement to source-first Core `Assign`. A direct-root recursive pattern binding leaf may refine to a mapped source local initialized in depth-first leaf order by projected `Copy`/`Move` from the mapped source root using the retained full leaf path. A producer-backed pattern may lower its existing producer to one compiler result temporary, initialize mapped pattern locals by projected `Copy`/`Move` from retained leaf paths, and refine the retained source transient frontier through projected/aggregate Core destruction.

Remaining source cleanup may refine to Core destruction only where the lower destruction domain is non-empty. Ending ownership of a zero-leaf source value may refine to no Core `Drop`; emitting an invalid lower destruction operation merely to materialize source ownership is not required.

A compiler temporary used for a producer-backed scrutinee is not a source binding. Core path state, scalar liveness, copyability, local numbering, and destruction domains are not source pattern/structural ownership authority.

No parser, lossless syntax, typed HIR, Core MIR production lowering, runtime, or backend implementation is added or required by this semantic owner.

## Further boundaries

This revision does not define floating/other literal semantics, arithmetic/comparison/operator forms, compound assignment, assignment-as-value, branch/loop/refutable-match control flow, field assignment/partial-field reinitialization, arbitrary-receiver member/method access, refutable/rest/shorthand/wildcard/literal/guard/alternative patterns, producer-backed pattern scrutinees beyond direct calls/record constructions/field-value uses, general expression/grouping scrutinees, destructuring assignment, qualified/cross-module construction or pattern heads, field visibility modifiers, references/borrow syntax/lifetimes, indirect calls/function values/closures, generics/traits/coherence, async/tasks or Exec call semantics, effect-system completion, panic payload/catch syntax, ABI/calling convention/FFI/linkage, parser/HIR/Core MIR production code, or backend behavior.
