# Source Function Execution

Status: **provisional normative; incomplete**

This document owns the represented source semantics for source function body attachment, dynamic direct-call activations, direct-call argument and result ownership transfer, lexical-scope and activation cleanup, direct return, recursion and divergence, and defined-fault propagation through direct source calls.

It consumes program outcomes and recoverable-value separation from [Program behavior](../behavior.md), environment admission and realization separation from [Program lifecycle](../lifecycle.md), defined-fault identity from [Core faults](../core/faults.md), structural destruction and stored-value cleanup from [Core value and storage semantics](../core/value-storage.md), function entity and callable-signature structure from [Source callables](callables.md), source value type equality from [Source type foundation](types.md), and parameter/local binding identity, scope, availability, and ordinary whole-binding owned use from [Source function-local bindings](local-bindings.md). It does not redefine those owners.

This document does not define concrete function/body/call/return syntax, a universal expression taxonomy, literals, operators, general control flow, references, closures, traits, ABI, or an implementation representation.

## Source function bodies

A represented source function entity MAY have one represented source body. It MUST NOT have more than one represented source body.

Attaching a represented source body to a function entity is a source-semantic fact. This revision does not define concrete declaration or definition grammar and does not require a declaration and body to be written in one source construct.

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

- ordinary whole-binding owned-value use from `local-bindings.md`; and
- a successful result-bearing direct call under this document.

Future literal, operator, record-construction, conversion, member-access, or other expression owners MAY introduce additional owned value producers without redefining the execution relation in this document.

Unless another accepted source owner defines a distinct rule for its producer, a producer used where this document requires a value MUST finish evaluation before that value is transferred to the receiving binding or transient ownership position.

## Direct-call arguments

A represented direct call has exactly one ordered argument operand for each callable-signature parameter slot. Argument count MUST match the callable signature exactly.

Each argument evaluation MUST produce one owned source value whose source type is exactly equal under `types.md` to the corresponding parameter source type. This revision introduces no implicit conversion, coercion, widening, narrowing, subtyping, or numeric defaulting.

Arguments are evaluated left to right in their ordered call/signature sequence.

When argument evaluation uses a complete parameter or local binding, that use is ordinary whole-binding owned-value use under `local-bindings.md` unless another accepted source owner explicitly defines a different context. Any resulting duplicate-or-consume transition therefore occurs in the same left-to-right order as argument evaluation.

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

The produced value's source type MUST be exactly equal under `types.md` to the local binding's declared source type.

After the transfer completes, the local binding becomes available under `local-bindings.md`. The transfer does not duplicate the produced value.

If initializer evaluation yields a defined fault or diverges, that local binding never becomes available. Ownership and availability transitions already performed by earlier evaluated operations remain effective.

## Source cleanup

For the represented operations in this document, **cleaning an owned source value** ends the source execution's ownership of that value exactly once.

When such a source value is realized in Core storage, applicable destruction-domain, stored-value-lifetime, and cleanup semantics remain owned by `core/value-storage.md`; this document determines only the source-owned value or binding selected for cleanup and the source order in which those selections occur.

A source value that has already been transferred or consumed is not cleaned again by its former owner.

This revision introduces no custom source destructor body, source `drop` ability, or general temporary-lifetime extension rule.

## Lexical-scope cleanup

When execution normally exits a represented lexical scope, each still-available ordinary local binding declared directly in that scope is cleaned in **reverse local declaration order**. An unavailable binding owns no value to clean.

When one source completion exits multiple active lexical scopes, cleanup proceeds from the innermost active scope outward. Each scope uses reverse local declaration order.

Function parameters belong to the root activation under `local-bindings.md` but are not ordinary local declarations. On source function activation termination, after the root lexical scope's still-available ordinary locals have been cleaned, still-available parameter bindings are cleaned in **reverse callable-signature parameter-slot order**.

This cleanup ordering is semantic and independent of physical stack layout, ABI passing, compiler local numbering, Core local numbering, or backend cleanup strategy.

## Normal return

For a source function whose callable signature has one result type, a represented return MUST first evaluate exactly one owned value producer whose source type is exactly equal under `types.md` to that result type.

Result evaluation completes before any return-induced lexical-scope or activation cleanup.

After successful result evaluation:

1. preserve the owned transient result value outside the activation-local cleanup set;
2. clean all active lexical scopes from innermost through the root lexical scope using the rules above;
3. clean still-available parameter bindings in reverse parameter-slot order;
4. terminate the callee activation normally; and
5. deliver the owned transient result value to the caller as the successful result of the direct call.

The transfer to the caller does not duplicate the result value.

Consequently, when return evaluation obtains a non-duplicable local through ordinary whole-binding owned use, that binding becomes unavailable before cleanup and is not cleaned again as a remaining local value.

For a source function whose callable signature specifies no result value, a represented no-result return performs the same scope and parameter cleanup and normal activation termination but produces no source value.

Reaching the normal end of a represented no-result function body is equivalent to normal no-result completion.

A result-bearing represented function body MUST NOT have a reachable normal end that produces no result. A later body/control-flow owner MUST validate that condition for the concrete constructs it introduces.

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

Transient argument values already produced when a later argument yields a defined fault are cleaned in reverse production order before the defined fault continues in the caller activation.

An owned transient return result is not part of callee activation-local cleanup after successful result evaluation because ownership has already been separated for transfer to the caller.

This revision does not define general temporary lifetime extension, expression-statement discard, or arbitrary temporary cleanup. Only transient values required by represented direct-call argument and return transfer are owned here.

## Divergence

If a directly called callee diverges, the caller remains suspended at that direct call and does not perform return or fault cleanup merely because execution continues indefinitely.

Active caller and callee ownership state, together with any transient values retained by the suspended evaluation, persists subject to operations already performed. There is no implicit source execution-step budget.

## Effects boundary

Left-to-right argument evaluation fixes relative source ordering for any effects that applicable future expression owners make observable.

This revision does not define a source effect system, purity, effect inference, speculation legality, or general transformation rules.

## Grammar and implementation boundary

General literals, arithmetic or comparison operators, assignment, branches, loops, record construction, member access, and concrete function/body/call/return grammar are not defined here.

Those features are not prerequisites to the represented activation and ownership relation because ordinary whole-binding owned use and nested result-bearing direct calls already supply represented owned values.

No parser, lossless-syntax representation, typed HIR, Core MIR production lowering, runtime implementation, or backend implementation is added or required by this semantic owner.

After this semantic relation is accepted, selection of the next source-language milestone requires a fresh continuation audit. This document does not itself authorize a concrete grammar or frontend slice.

## Further boundaries

This revision does not define concrete function/parameter/local/body/call/return syntax, punctuation, keywords, parser recovery, literal spellings or default literal typing, arithmetic/comparison/operator forms, branch/loop/pattern control flow, assignment/replacement expressions, field/member access, record construction, references/borrow syntax/lifetimes, indirect calls/function values/closures, generics/traits/coherence, async/tasks or Exec call semantics, effect-system completion, panic payload/catch syntax, ABI/calling convention/FFI/linkage, parser/lossless syntax/HIR/Core MIR production code, or backend behavior.
