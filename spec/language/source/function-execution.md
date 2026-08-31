# Source Function Execution

Status: **provisional normative; incomplete**

This document owns the represented source semantics for source function body attachment, body and nested-block statement sequencing, dynamic direct-call activations, direct-call argument/result ownership transfer, bounded safe-reference producer/parameter/result/reborrow/replacement/external-referent integration, first-slice activation-local raw-pointer producer/local/assignment/unsafe-block execution integration, record-construction field evaluation and transient assembly, producer-backed field-receiver evaluation and transient cleanup, represented operator operand validation/evaluation sequencing including bounded eager-binary transient lifetime and Boolean short-circuit definite-state composition, bounded contextual grouped-value validation/evaluation transparency, bounded operation-local numeric-contract-selected-value applicability and execution transparency, ordinary local initialization, recursive record-destructuring declaration completion including producer-backed scrutinee evaluation and transient cleanup, whole-binding assignment RHS evaluation and replacement ordering, lexical-scope and activation cleanup, return execution, payload-free explicit-fault execution, bounded loop-transfer cleanup, static local normal-continuation presence, bounded-loop body execution/cleanup sequencing, recursion/divergence, and defined-fault propagation through direct source calls.

It consumes program outcomes and recoverable-value separation from [Program behavior](../behavior.md), environment admission and realization separation from [Program lifecycle](../lifecycle.md), defined-fault reason identity and explicit Core fault termination from [Core faults](../core/faults.md), structural destruction and stored-value cleanup from [Core value and storage semantics](../core/value-storage.md), function entity/callable-signature structure from [Source callables](callables.md), source value type equality and record value shape from [Source type foundation](types.md), boolean/integer/decimal floating literal value production from [Source literal semantics](literals.md), represented Boolean logical-negation/equality/inequality/short-circuit-conjunction, plain fixed-width integer-negation/integer-complement/multiplication/addition/subtraction/exclusive-or/bitwise-OR, and same-format binary floating-multiplication/floating-division/floating-addition/floating-subtraction operand/result typing, selected numeric-contract facts, semantic value transformations, and operator-local ownership facts from [Source operator semantics](operators.md), structural ownership state and remaining frontiers for binding and external-referent roots from [Source structural ownership](structural-ownership.md), parameter/local identity, scope, lookup, assignment mutability, whole-binding use, assignment legality, and raw-pointer local integration from [Source function-local bindings](local-bindings.md), safe-reference type/authority/carrier, source-validation origin provenance, root formation, complete-referent dereference, explicit reborrow, reference-relative replacement, lifetime, external-referent structural state, parameter/result-transfer, call-entry/restoration, advertised Shared-result-origin validity, cleanup, and source/Core refinement facts from [Source safe references](references.md), activation-local raw-pointer type/value/origin provenance, lexical target validity, raw address formation, unsafe RawMove/RawAssign validity, lexical unsafe admission, cleanup, and source/Core refinement facts from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md), binding-root and bounded producer-backed field-value production, receiver exact-type facts, selected field paths, duplicate-or-consume consequences, and producer-receiver remaining frontiers from [Source field-value access](field-access.md), and recursive record-pattern head/field/rest structure, scrutinee category, explicit binding-leaf order, per-leaf ownership consequences, and producer transient frontier selection from [Source patterns](patterns.md). It does not redefine those owners.

[Source control flow](control-flow.md) consumes this document's owned-value producer execution, grouped-value transparency, nested-block execution, local normal-continuation presence, return and explicit-fault termination, loop-transfer cleanup, lexical cleanup, defined-fault propagation, and divergence relations when defining represented statement-level conditionals, bounded `while`, and bounded unlabeled `break;`/`continue;`, including their definite normal successor/backedge/transfer-target relations for binding structural ownership, replacement-capable external-referent structural ownership, and raw-pointer origin provenance. This document does not redefine conditional selection, loop condition selection, conditional exact-state composition, loop backedge-state admission, or loop-transfer target/state admission.

The represented concrete function/body/block/value/grouping/numeric-contract-selection/operator/call/record-construction/field-value/record-destructuring/assignment/reference-replacement/conditional/while/break/continue/return/explicit-fault/safe-reference/raw-pointer/unsafe grammar is owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, pattern-head lookup or field accessibility, universal expressions, operator-local type/value/refinement semantics, conditional or loop selection/backedge/transfer-target validity, other general control flow, safe-reference semantics beyond the bounded relation owned by `references.md`, raw-pointer validity/provenance/unsafe-admission semantics beyond the bounded relation owned by `raw-pointers-unsafe.md`, closures, traits, ABI, or an implementation representation.

## Source function bodies

A represented source function entity MAY have one represented source body and MUST NOT have more than one.

Attaching a represented source body to a function entity is a source-semantic fact. `concrete-syntax.md` defines one concrete function form that introduces a function entity and attaches the following body. This execution relation does not require every future declaration/definition form to use that syntax.

In the represented direct-call subset, a direct source call targets exactly one resolved source function entity that has a represented body.

A source function entity without a represented body has no direct source execution relation under this document. Later FFI, external-function, intrinsic, or other callable owners may add distinct relations without redefining this one.

The represented direct call is statically bound to its resolved function entity. This revision introduces no function-operand evaluation, overload selection, indirect call, function value, virtual dispatch, or closure call.

## Dynamic source function activations

Every dynamic direct call creates one distinct **source function activation** for the target function entity.

The activation provides independent dynamic values and binding-owned structural ownership states for that function body's static parameter/local binding identities, including locals introduced by ordinary declarations and accepted recursive record patterns. Distinct simultaneous or recursive calls therefore have distinct dynamic binding state even when they execute the same declarations. When an ordinary local has raw-pointer type, its exact pointer-origin provenance is likewise activation-local validation state under `raw-pointers-unsafe.md`.

For every parameter whose exact type is `ExclusiveReplaceRef(T)`, the activation additionally has the one non-binding external referent structural root owned by `references.md`. That root is part of the activation's source-validation ownership state but is not a parameter/local binding and introduces no lexical identifier or duplicate storage domain.

Activation identity is not source-observable, a physical stack-frame address, ABI identity, task/thread identity, or required numeric runtime representation.

The callee body begins only after all argument production and safe-reference call-entry obligations succeed, parameter transfer completes, and each parameter binding has complete initial structural ownership under `local-bindings.md` and `structural-ownership.md`. A safe-reference parameter receives exactly the transferred carrier/authority relation established by `references.md`; a Shared-reference parameter additionally begins independent body validation with its distinct `ParameterOrigin(i)` provenance. Every replacement-capable parameter's external referent root begins fully available at successful call entry. These facts add no second parameter-slot category or borrowed-call pass mode. `RawPtr(T)` is not parameter-admissible under `callables.md`, so no source raw-pointer value or pointer-origin provenance is transferred into a callee parameter by this direct-call relation.

## Source producer-validation state

The represented source producer-validation state consists of all source-semantic state that an owned producer can validly change before its receiving operation commits:

- structural ownership of active parameter/local binding roots;
- structural ownership of active replacement-capable external referent roots;
- safe-reference authority/carrier/delegation and applicable Shared-result provenance state; and
- exact raw-pointer origin state where an already represented producer relation carries such a value.

Whenever this document validates a complete producer transaction speculatively, failure commits none of that transaction's speculative producer-validation state. When older operation-specific wording below speaks of cloning, committing, comparing, or preserving a binding ownership environment, the normative transaction is this complete source producer-validation state; there is no binding-only exception for nested calls, complete-referent moves, reborrows, or other newly admitted safe-reference producers.

This definition does not create a state lattice, authority-graph join, general expression effect system, or runtime state object. It only makes the already required transactional boundary complete for every represented producer family.

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
- a source-valid field-value use under `field-access.md`, using either the accepted binding-root receiver or the bounded producer receiver;
- a source-valid represented operator producer whose operator-local type/value relation is owned by `operators.md`;
- a source-valid safe root-reference formation from `references.md`, selecting Shared or replacement-capable permission from its exact required type and concrete form;
- a source-valid bounded complete-referent dereference producer from `references.md`;
- a source-valid explicit complete-referent reborrow producer from `references.md`;
- a source-valid raw-address producer from `raw-pointers-unsafe.md`; and
- a source-valid unsafe raw ownership-move producer from `raw-pointers-unsafe.md`.

`concrete-syntax.md` exposes exactly those eleven producer families through its represented ordinary `Value` grammar. `GroupedValue`, `ConditionalGroupedValue`, and a source-valid `NumericContractSelectedValue` are concrete wrappers around one already represented value producer and therefore do **not** add another producer family. Grouping is semantically transparent. A numeric-contract-selected wrapper instead qualifies exactly one eligible governed root operator occurrence while preserving that existing producer's result, ownership, and execution category. The represented operator family currently contains Boolean logical negation, plain fixed-width integer negation, plain fixed-width integer bitwise complement, plain fixed-width integer multiplication, same-format binary floating multiplication, same-format binary floating division, plain fixed-width integer addition, same-format binary floating addition, plain fixed-width integer subtraction, same-format binary floating subtraction, plain fixed-width integer exclusive-or, plain fixed-width integer bitwise OR, Boolean equality, Boolean inequality, and Boolean short-circuit conjunction; those operations do not become unrelated general expression families merely because their arities, typing rules, evaluation strategies, or numeric-contract facts differ. A record-destructuring declaration is not another `Value` producer: `patterns.md` owns its grouped production of zero or more binding-leaf values. A producer-backed record-pattern scrutinee reuses one existing producer family in a pattern-specific receiving position and remains deliberately narrower than `Value`.

`control-flow.md` reuses the concrete `ConditionalValue` receiving position for represented `if` and bounded `while`. The conditional grammar may syntactically contain a represented operator that cannot satisfy the condition's exact `Bool` requirement; such a producer is source-invalid before it can yield a condition value. Context-preserving conditional grouping wraps only the recursively admitted conditional grammar and does not create an additional producer family or alter any producer execution semantics here. `NumericContractSelectedValue`, direct safe root formation, complete-referent dereference, explicit reborrow, raw address formation, and raw ownership move are not themselves admitted as direct `ConditionalValue` forms in this revision; they may still occur inside an ordinary `Value` position nested within an otherwise admitted conditional producer when that receiving relation independently permits them, without becoming the condition's root producer.

The represented whole-binding assignment, complete-referent replacement, raw replacement, lexical unsafe block, `fault;`, `break;`, and `continue;` forms are body statements rather than owned value producers. Whole-binding assignment, reference replacement, raw replacement, and an unsafe block may complete locally normally; `fault;`, `break;`, and `continue;` have no local fallthrough. None becomes a `Value`, `ConditionalValue`, call, or return-value form merely because it is a body statement.

Future additional operator, conversion, or other expression owners MAY introduce further owned value producers without redefining the receiving relations in this document.

Unless another accepted source owner defines a distinct rule, a producer used where this document requires a value MUST finish evaluation before that value is transferred to its receiving binding/transient owner.

Literal evaluation is effect-free, non-faulting, and non-diverging after source validation. This execution owner supplies a required source type where contextual literal materialization needs it and then transfers the resulting value.

A safe root-reference producer receives its surrounding exact required safe-reference type and applies the root target, referent admission, availability, mutability where applicable, lexical extent, and canonical authority-compatibility rules from `references.md`. Root `&x` produces a fresh Shared authority/carrier; root `&mut x` produces a fresh replacement-capable exclusive authority/carrier only for an admitted mutable ordinary-local root. Either root formation changes no structural ownership state of the target and is non-faulting/non-diverging after source validation.

A bounded complete-referent dereference producer resolves its stored reference binding under `references.md` and requires surrounding exact required type equal to the referent `T`. For `SharedRef(T)` it duplicates the complete referent and leaves target ownership unchanged. For `ExclusiveReplaceRef(T)`, it duplicates a duplicable `T` under retained Shared reference-relative capability, while a non-duplicable `T` is ownership-moved under retained Exclusive reference-relative capability and consumes the complete local or external referent structural root. Dereference itself neither moves nor duplicates the stored reference carrier merely to select the target.

An explicit complete-referent reborrow producer resolves its parent safe-reference binding and applies the exact child-permission, retained-parent capability, target, lexical-validity, and fresh-authority rules from `references.md`. `&*r` creates a fresh Shared child only when the resulting `SharedRef(T)` is represented and the parent retains Shared capability; `&mut *r` creates a fresh replacement-capable child only from a parent that can delegate that capability. Reborrow moves/copies no parent carrier, changes no referent structural ownership, and creates fresh reborrow provenance rather than preserving the parent's Shared-result authority identity.

Safe-reference root formation, dereference after its required structural transition, and reborrow are non-faulting and non-diverging after their source-validity requirements hold. Their authority/carrier/delegation consequences are safe-reference state; any non-duplicable complete-referent Move is additionally the ordinary structural ownership transition selected by `references.md`.

A raw-address producer receives the surrounding exact required type `RawPtr(T)`, resolves one complete active binding root through `raw-pointers-unsafe.md`, and on success produces one owned raw-pointer value carrying that owner's exact `PointerOrigin(binding)` provenance. Address formation applies the canonical Shared direct-compatibility requirement through `references.md`; it does not read, duplicate, move, consume, replace, or otherwise access the target value, changes no target structural ownership state, creates no safe authority/carrier, and is non-faulting and non-diverging after its source-validity requirements hold.

A raw ownership-move producer receives the surrounding exact required pointee type `T`. `raw-pointers-unsafe.md` establishes the active unsafe-admission region, exact `RawPtr(T)` pointer operand and origin, continuing target extent, complete target availability, and canonical Exclusive target compatibility—therefore absence of any overlapping active safe authority—before the move is admitted. Successful execution obtains the stored pointer value non-consumingly, consumes/transfers the complete target root through the existing structural-ownership transition, leaves the pointer value/origin unchanged, and produces one owned `T`. After those source-validity requirements hold, the raw move itself is non-faulting and non-diverging. The operation remains ownership-moving even when `T` is source-duplicable.

The exact required type supplied by an existing receiving position—local initializer, whole-binding assignment RHS, reference-replacement RHS, direct-call argument, record initializer, return value, or conditional/loop condition—passes unchanged to every represented context-directed numeric operator producer and through a `NumericContractSelectedValue` wrapper. For plain integer negation, integer complement, integer addition, integer subtraction, integer exclusive-or, and integer bitwise OR, the selected operator consumes that one required type only when it is one of the eight admitted fixed-width integer types. For the shared concrete binary `*`, one of those eight integer required types selects plain integer multiplication, while exact `F16`, `F32`, or `F64` selects same-format floating multiplication; every other required type rejects the multiplication form before either operand validation may commit producer state. The syntactically distinct prefix `*r` safe-reference dereference is selected before this binary relation and is never chosen by numeric required-type dispatch. For concrete `/`, exact `F16`, `F32`, or `F64` selects same-format floating division and every other required type rejects the division form before either operand validation may commit producer state; this revision defines no integer-division selection for `/`. Same-format floating addition and floating subtraction likewise consume that one required type only when it is exactly `F16`, `F32`, or `F64`. The shared concrete `+` selects integer addition or floating addition solely from that unchanged required type, shared concrete binary `-` selects integer subtraction or floating subtraction solely from that unchanged required type, binary `*` selects integer multiplication or floating multiplication solely from that unchanged required type, and concrete `/` selects only floating division from that unchanged required type. No receiving position gains a separate operator-typing rule, and neither operand syntax nor a literal independently chooses among those semantic operations. Represented Boolean operators instead consume their intrinsic exact-`Bool` result/operand relations from `operators.md`; the receiving required type must accept that intrinsic result before the applicable Boolean-operator transaction may commit operand state.

A source-valid binding-root field-value producer may change its selected binding's structural ownership state before its result reaches the receiving position: a duplicable final field leaves ownership unchanged subject to the canonical Shared requirement, while a non-duplicable final field consumes exactly the selected structural path subject to the canonical Exclusive requirement under `field-access.md`, `references.md`, and `structural-ownership.md`.

A source-valid producer-backed field-value producer instead completes the receiver-producer execution and field-receiver transient lifetime defined below before exposing its selected result to the surrounding receiving position.

## Grouped-value validation and execution

A source-valid `GroupedValue` or `ConditionalGroupedValue` is one concrete wrapper around exactly one complete contained value producer selected by `concrete-syntax.md`. Grouping introduces no independent semantic producer, result transformation, ownership operation, or receiving position.

The surrounding receiving position's required source type passes through the grouping wrapper unchanged to the complete contained producer. Source validation then uses exactly that producer's existing owner and transaction boundary. The grouped value is source-valid exactly when the contained producer is source-valid under that unchanged requirement, and its successful source type is exactly the contained producer's successful source type.

Grouping does not add a speculative ownership transaction around an existing producer transaction and does not commit, roll back, restore, normalize, or otherwise alter source producer-validation state. If contained producer validation fails, no consequence is committed beyond what that existing producer's own source-validation relation permits. If it succeeds, exactly the contained producer's ordinary successful consequences are committed once.

Dynamic execution of one grouped value is exactly:

1. evaluate the complete contained producer exactly once under its existing producer semantics and the unchanged required source type;
2. preserve every structural ownership, safe-reference authority/carrier/delegation/provenance, raw-pointer result provenance, and producer-owned transient consequence of that evaluation;
3. if the contained producer yields defined fault `F`, produce no grouped result and propagate the same `F` through the existing producer/receiving cleanup relation with no grouping-specific cleanup layer;
4. if the contained producer diverges, the grouped evaluation remains suspended in that producer with exactly its existing live ownership and performs no cleanup merely because it is grouped; and
5. after successful producer completion, transfer the exact same produced owned source value, source type, and any value-attached source provenance through the grouping wrapper to the surrounding receiving position, without duplication or a second semantic transformation.

Grouping itself creates no source binding, structural path, place, lvalue, address, reference authority/carrier, pointer origin, storage identity, operation-owned transient, cleanup frontier, defined-fault reason, divergence point, side effect, runtime flag, or hidden source state. It has no own cleanup on success, fault, return, loop transfer, activation termination, or divergence.

A grouped represented operator retains that operator's exact value relation, required/result typing, selected numeric contract when applicable, operand evaluation order, transactional validation, held-left lifetime where applicable, short-circuit behavior where applicable, fault/divergence behavior, and result transfer from the existing operator relations. A grouped direct call, construction, field-value use, literal, whole-binding use, safe root formation, complete-referent dereference, explicit reborrow, raw address formation, or raw ownership move likewise retains exactly its existing producer relation.

When grouping occurs in a represented condition, `concrete-syntax.md` preserves `ConditionalValue` recursively inside the group. This execution relation therefore receives the same condition-context grammar and exact-Bool requirement; parentheses do not reset the condition to unrestricted ordinary `Value` or add a second control-flow rule.

After successful source validation, a faithful typed frontend MAY erase the grouping wrapper and retain only the already required typed contained `Value` facts, including exact safe-reference provenance/delegation facts or raw-pointer origin when the contained value has those categories. If source grouping is retained for diagnostics or tooling, the retained delimiter/source-location fact carries no source type, ownership, reference, pointer, operator, condition, Core, or runtime semantic identity.

## Numeric-contract-selected value validation and execution

The represented `NumericContractSelectedValue` form from `concrete-syntax.md` is one operation-local qualifier around exactly one complete ordinary `Value`. In this revision its only explicit selector is `fast`.

The surrounding receiving position's exact required source type `T` passes through the wrapper unchanged. Before validation of any operand producer beneath the selected root may commit structural ownership, safe-reference authority/carrier/delegation, or raw-operation consequences, source validation MUST establish selector applicability in this order:

1. inspect the contained value's root producer after peeling zero or more ordinary `GroupedValue` wrappers only;
2. do **not** peel another `NumericContractSelectedValue`; a selector wrapper is not transparent for locating the target of another selector;
3. require the discovered root to be a represented concrete binary `*`, binary `/`, binary `+`, or binary `-` operation candidate;
4. require the unchanged surrounding `T` to be exactly one of `F16`, `F32`, or `F64`, so that concrete binary `*` selects same-format floating multiplication rather than integer multiplication, concrete `/` selects same-format floating division, concrete `+` selects same-format floating addition rather than integer addition, or concrete binary `-` selects same-format floating subtraction rather than integer subtraction; and
5. establish selected numeric contract `fast` for exactly that discovered governed floating operation occurrence.

Only after all five facts succeed may validation execute the applicable floating-multiplication, floating-division, floating-addition, or floating-subtraction operand transaction below. Failure of root discovery, root eligibility, or the exact floating required-type requirement rejects the selected value before an operand lookup, move, field consumption, safe-reference formation/dereference/reborrow, raw address/move, call argument, nested producer, or other operand-side semantic consequence can commit merely while diagnosing the invalid selection.

Consequently a selected root that is a literal, whole-binding use, ordinary direct call, record construction, field-value use, safe root formation, complete-referent dereference, explicit reborrow, raw address formation, raw ownership move, integer operation, Boolean operation, or another currently non-governed producer is source-invalid. In particular, `@fast(1.0)` does not make decimal floating literal materialization contract-sensitive. By contrast, decimal floating literals and other source-valid ordinary `Value` producers used as operands of a valid selected floating multiplication, division, addition, or subtraction still execute normally under the exact `T` supplied by that operation.

Ordinary grouping alone is transparent for target discovery: `@fast((a * b))` selects the same root floating multiplication as `@fast(a * b)`, `@fast((a / b))` selects the same root floating division as `@fast(a / b)`, `@fast((a + b))` selects the same root floating addition as `@fast(a + b)`, and `@fast((a - b))` selects the same root floating subtraction as `@fast(a - b)` when the exact required type is floating. Stacked qualification of one root is invalid: forms equivalent to `@fast(@fast(a * b))`, `@fast((@fast(a * b)))`, `@fast(@fast(a / b))`, `@fast((@fast(a / b)))`, `@fast(@fast(a + b))`, `@fast((@fast(a + b)))`, `@fast(@fast(a - b))`, and `@fast((@fast(a - b)))` fail because the inner selector wrapper is not peeled. This rule establishes at most one explicit source selection for one governed root occurrence; it does not create conflict precedence or last-wins behavior.

Distinct nested governed operation occurrences remain independent. A selector on an outer operation does not recursively select a nested operation, and a selector on an inner operation does not change an enclosing operation. The rule that a selector wrapper is opaque during one selector's root discovery therefore rejects only stacked qualification of that same root; it does not prohibit a separately selected governed operation nested as an operand of another governed operation. Every unqualified governed floating occurrence continues to receive `standard` from the accepted Core floating fallback. Therefore mixed FloatMul/FloatDiv/FloatAdd/FloatSub trees preserve each operation occurrence's own selected/defaulted contract, including every `standard` or `reproducible` boundary required by the accepted mixed-contract composition rules. For example, `@fast((a * b) + c)` selects only the outer floating addition while the grouped inner unqualified floating multiplication remains `standard`; `@fast((a / b) + c)` selects only the outer floating addition while the grouped inner unqualified floating division remains `standard`; `@fast((a + b) * c)` selects only the outer floating multiplication while the grouped inner unqualified floating addition remains `standard`; and `@fast((a - b) + c)` selects only the outer floating addition while the grouped inner unqualified floating subtraction remains `standard`. A separately valid selector on an inner governed operation may establish `fast` for that occurrence without changing the outer operation, and a selector on the outer operation does not change an already selected inner operation. Thus an eligible Fast FloatMul consumed by an eligible Fast FloatAdd is expressible only when both participating occurrences independently carry `Fast`; one selector never supplies the other occurrence's contract. FloatDiv remains occurrence-local under the same rule but receives no new reassociation, reciprocal-replacement, or contraction permission from that locality.

After successful applicability, validation and dynamic execution are exactly those of the applicable same-format floating-multiplication, floating-division, floating-addition, or floating-subtraction producer below with selected contract `C = fast`. The wrapper adds no conversion, binding, structural path, place, lvalue, address, reference authority/carrier, pointer origin, storage identity, operation-owned transient, cleanup frontier, defined-fault reason, divergence point, side effect, dynamic caller state, runtime floating environment, or ambient numeric mode. Fault/divergence propagation, held-left lifetime, left-before-right operand sequencing, and result transfer remain those of the qualified floating operation and its operand producers.

The selection is body/operation semantics only. A caller-selected operation does not alter a callee body's operations; direct-call arguments and results transfer semantic values rather than ambient numeric contracts. A callee operation is governed only by its own explicit operation-local selection or the accepted default `standard` relation.

After successful source validation, a faithful typed frontend MAY erase the selector wrapper itself only after retaining the selected contract directly on the qualified governed floating-multiplication, floating-division, floating-addition, or floating-subtraction occurrence. Erasing the wrapper MUST NOT erase, default, infer again, or propagate that contract to a different operation. Retaining source delimiters/location for diagnostics or tooling adds no semantic wrapper identity.

## Boolean logical-negation producer validation and execution

A represented Boolean logical-negation producer consumes from `operators.md` the intrinsic result type `Bool`, operand required type `Bool`, successful semantic value transformation, immediate consumption of the successfully produced owned operand value, and absence of any operator-local structural-ownership transition.

The surrounding receiving position's required source type applies first to the operator's intrinsic result type. Source validation MUST establish that the surrounding required type is exactly `Bool` before validating an operand in a way that may commit producer state. If the surrounding required type is not `Bool`, the operator is source-invalid with result type `Bool`, and no operand consequence is committed merely while diagnosing that mismatch.

After that outer result-type requirement succeeds, validate the complete operand producer with required source type exactly `Bool` against a speculative copy of the current source producer-validation state. The speculative state is committed to the containing source-validation state only when the complete operand producer is source-valid. A failed operand lookup, type check, availability check, nested producer validation, or other source-invalid operand therefore cannot leak a speculative structural, safe-authority, or external-referent consequence merely because it appeared beneath logical negation.

A successfully validated logical-negation producer adds no further source-state change beyond the committed consequences of its operand producer.

Dynamic execution is exactly:

1. evaluate the complete operand producer exactly once under its existing producer semantics with required source type `Bool`;
2. preserve every source-state consequence completed by that operand evaluation;
3. if operand evaluation yields defined fault `F`, produce no logical-negation result and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if operand evaluation diverges, produce no logical-negation result and perform no cleanup merely because execution remains suspended;
5. after successful Bool operand production, consume that owned operand value exactly once by applying the Boolean logical-negation semantic value relation from `operators.md`; and
6. transfer the resulting distinct owned Bool exactly once to the surrounding receiving position.

The consumed operand result receives no separate cleanup after step 5. After successful operand production, the logical-negation step itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, separately cleanup-bearing transient category, storage identity, or runtime state.

When logical negation is used as a represented `if` or bounded-`while` condition, the successful post-condition producer-validation state is therefore exactly the operand producer's successful post-evaluation state. Only the owned Bool condition value is transformed before `control-flow.md` consumes it for selection.

Nested logical-negation producers apply this same relation recursively. Each nesting level validates/evaluates its complete operand once, consumes that successful operand result once, and adds no structural transition of its own.

## Plain fixed-width integer-negation producer validation and execution

A represented plain fixed-width integer-negation producer consumes from `operators.md` the context-directed exact result/operand type relation, exact mathematical-negation-plus-plain-overflow value relation, exactly-once consumption of its successful owned operand value, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is not, integer negation is source-invalid before its operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects integer negation at this outer admission point.

After `T` is admitted, validate the complete operand as one transaction:

1. clone the current source producer-validation state;
2. validate the complete operand producer with required source type exactly `T` against that clone;
3. commit the resulting speculative state to the containing source-validation state only when the complete operand producer is source-valid; and
4. add no further source-state transition for the integer-negation operation itself.

A failed operand lookup, type check, availability check, nested producer validation, or other source-invalid operand therefore commits no speculative producer-state consequence. This transaction remains required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may consume a non-duplicable binding, an external referent, or another structural path internally, for example through direct-call arguments or another nested producer.

The concrete signed-literal-versus-prefix distinction has already been established by `concrete-syntax.md` before this relation is selected. This execution owner MUST NOT reinterpret an accepted signed literal as integer negation or vice versa. In particular, under required `U8`, the signed literal `-1` fails literal materialization before this operator relation exists for that source form, whereas `-(1)` may select this operator relation with operand required type `U8`.

Dynamic execution is exactly:

1. evaluate the complete operand producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by that operand evaluation;
3. if operand evaluation yields defined fault `F`, produce no integer-negation result and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if operand evaluation diverges, produce no integer-negation result and perform no cleanup merely because execution remains suspended;
5. after successful `T` operand production, consume that owned operand value exactly once by applying the plain fixed-width integer-negation semantic value relation from `operators.md`; and
6. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The consumed operand result receives no separate cleanup after step 5. There is no held-left operand or other negation-specific transient because no later operand exists. After successful operand production, the integer-negation value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, cleanup category, storage identity, numeric-contract state, or runtime state. Plain fixed-width overflow selects the value required by `operators.md`; it does not select a fault.

Nested integer-negation producers apply this same complete producer relation recursively where the concrete prefix grammar establishes the nesting. Mixed Boolean/integer prefix syntax likewise validates recursively under each operator's exact required type and may be source-invalid even though concrete syntax represents it. Each successful integer-negation nesting level retains the same exact required type `T`; no operand-derived inference, promotion, conversion, defaulting, signed-literal rewriting, or generic arithmetic dispatch occurs.

## Plain fixed-width integer-bitwise-complement producer validation and execution

A represented plain fixed-width integer-bitwise-complement producer consumes from `operators.md` the context-directed exact result/operand type relation, canonical-width-residue/equivalent exact `-1 - v` value relation, exactly-once consumption of its successful owned operand value, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is not, integer complement is source-invalid before its operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects integer complement at this outer admission point.

After `T` is admitted, validate the complete operand as one transaction:

1. clone the current source producer-validation state;
2. validate the complete operand producer with required source type exactly `T` against that clone;
3. commit the resulting speculative state to the containing source-validation state only when the complete operand producer is source-valid; and
4. add no further source-state transition for the integer-complement operation itself.

A failed operand lookup, type check, availability check, nested producer validation, or other source-invalid operand therefore commits no speculative producer-state consequence. This transaction remains required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may change a non-duplicable binding or external referent internally.

Dynamic execution is exactly:

1. evaluate the complete operand producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by that operand evaluation;
3. if operand evaluation yields defined fault `F`, produce no integer-complement result and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if operand evaluation diverges, produce no integer-complement result and perform no cleanup merely because execution remains suspended;
5. after successful `T` operand production, consume that owned operand value exactly once by applying the plain fixed-width integer-bitwise-complement semantic value relation from `operators.md`; and
6. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The consumed operand result receives no separate cleanup after step 5. There is no held-left operand or other complement-specific transient because no later operand exists. After successful operand production, the integer-complement value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, cleanup category, storage identity, physical-representation fact, numeric-contract state, or runtime state.

Nested integer-complement producers apply this same complete producer relation recursively where the concrete prefix grammar establishes the nesting. Mixed Boolean/integer-negation/integer-complement prefix syntax likewise validates recursively under each operator's exact required type and may be source-invalid even though concrete syntax represents it. Each successful integer-complement nesting level retains the same exact required type `T`; no operand-derived inference, promotion, conversion, defaulting, physical bit-pattern reinterpretation, or generic bitwise dispatch occurs.

## Plain fixed-width integer-addition producer validation and execution

A represented plain fixed-width integer-addition producer consumes from `operators.md` the context-directed exact result/operand type relation, exact mathematical-addition-plus-plain-overflow value relation, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is not, integer addition is source-invalid before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects integer addition at this outer admission point.

After `T` is admitted, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the addition operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction is required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may consume a non-duplicable binding or external referent internally.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no addition result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no addition result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic integer value held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no addition result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the plain fixed-width integer-addition semantic value relation from `operators.md`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the same bounded scalar operation-owned transient category used by the represented Boolean eager binary operators. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, or source-visible storage identity. It exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the addition value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, or runtime state. Plain fixed-width overflow selects the value required by `operators.md`; it does not select a fault.

Nested represented additions apply this same complete producer relation recursively when the concrete grammar uses grouping to establish the nesting. Each nesting level retains its exact required type `T`; no operand-derived inference, promotion, conversion, or defaulting occurs.

## Same-format floating-addition producer validation and execution

A represented same-format floating-addition producer consumes from `operators.md` the context-directed exact result/operand type relation, its one already-established selected numeric contract `C`, the accepted same-format floating-addition value relation under `C`, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `F16`, `F32`, or `F64`. If it is not, floating addition is source-invalid before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects floating addition at this outer admission point. When `T` is one of the eight represented fixed-width integer types, the same concrete `+` form selects the distinct integer-addition producer relation above rather than this relation.

For an unqualified floating-addition occurrence, no explicit source selector establishes a different contract, so the accepted Core floating fallback establishes `C = Standard`. For a source-valid `NumericContractSelectedValue`, the applicability relation above establishes `C = Fast` for exactly the selected root occurrence before this operand transaction begins. Current source defines no `Reproducible` spelling, while the lower semantic contract domain remains unchanged.

After `T` and `C` are established, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the floating-addition operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented intrinsic floating values are duplicable, because a complete producer of floating type `T` may consume a non-duplicable binding or external referent internally while producing its floating result.

Existing literal rules compose without extension. A decimal floating literal operand materializes only under the exact required `T` supplied here. A decimal integer literal is not reclassified as floating merely because `T` is floating, no default/abstract floating type is introduced, and a signed decimal floating literal retains the signed-literal priority established before operator selection. Numeric-contract selection does not re-form, flush, or reinterpret literal materialization.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no floating-addition result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no floating-addition result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic floating value or class held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no floating-addition result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the same-format floating-addition semantic relation from `operators.md` under selected contract `C`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the existing bounded scalar operation-owned transient category. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, source-visible storage identity, NaN observation object, or numeric-contract state. Contract `C` is a static semantic fact of the floating-addition occurrence rather than an owned transient or dynamic mode. The held left value exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the floating-addition value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, defined-fault reason, numeric-contract state, NaN identity, or runtime mode. Finite values, signed zero/subnormal results, infinities, permitted semantic NaN results, and any additional result variation explicitly admitted by `C` are ordinary numerical outcomes selected by `operators.md` and its Core floating authority.

Nested represented `+` forms apply their own complete producer relations recursively according to the concrete syntax tree. Every typed nesting independently retains integer-addition or floating-addition semantic identity from its exact required type and, for each floating occurrence, its own explicit/defaulted numeric contract. An outer `Fast` occurrence does not recursively select an unqualified inner floating addition, and an inner `Fast` occurrence does not alter an unqualified outer addition. The accepted multi-operation floating rules decide whether any result-changing transformation may affect more than one occurrence. No operand-derived inference, promotion, conversion, defaulting, mixed-format arithmetic, inherited numeric mode, or generic arithmetic dispatch occurs.

## Plain fixed-width integer-subtraction producer validation and execution

A represented plain fixed-width integer-subtraction producer consumes from `operators.md` the context-directed exact result/operand type relation, exact mathematical-subtraction-plus-plain-overflow value relation, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is not, the subtraction is source-invalid before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects integer subtraction at this outer admission point.

After `T` is admitted, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the subtraction operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction is required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may consume a non-duplicable binding or external referent internally.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no subtraction result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no subtraction result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic integer value held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no subtraction result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the plain fixed-width integer-subtraction semantic value relation from `operators.md`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the same bounded scalar operation-owned transient category used by the represented Boolean eager binary operators and integer addition. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, or source-visible storage identity. It exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the subtraction value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, numeric-contract state, or runtime state. Plain fixed-width overflow selects the value required by `operators.md`; it does not select a fault.

Nested represented additive operations apply their complete producer relation recursively when the concrete grammar uses grouping to establish the nesting. Each nesting level retains its exact required type `T`; no operand-derived inference, promotion, conversion, defaulting, or unary-negation reinterpretation occurs.

## Same-format floating-subtraction producer validation and execution

A represented same-format floating-subtraction producer consumes from `operators.md` the context-directed exact result/operand type relation, its one already-established selected numeric contract `C`, the accepted same-format floating-subtraction value relation under `C`, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `F16`, `F32`, or `F64`. If it is not, floating subtraction is source-invalid before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects floating subtraction at this outer admission point. When `T` is one of the eight represented fixed-width integer types, the same concrete binary `-` form selects the distinct integer-subtraction producer relation above rather than this relation.

For an unqualified floating-subtraction occurrence, no explicit source selector establishes a different contract, so the accepted Core floating fallback establishes `C = Standard`. For a source-valid `NumericContractSelectedValue`, the applicability relation above establishes `C = Fast` for exactly the selected root occurrence before this operand transaction begins. Current source defines no `Reproducible` spelling, while the lower semantic contract domain remains unchanged.

After `T` and `C` are established, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the floating-subtraction operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented intrinsic floating values are duplicable, because a complete producer of floating type `T` may consume a non-duplicable binding or external referent internally while producing its floating result.

Existing literal rules compose without extension. A decimal floating literal operand materializes only under the exact required `T` supplied here. A decimal integer literal is not reclassified as floating merely because `T` is floating, no default/abstract floating type is introduced, and a signed decimal floating literal retains the signed-literal priority established before operator selection. This operation does not reinterpret that sign as floating unary negation. Numeric-contract selection does not re-form, flush, or reinterpret literal materialization.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no floating-subtraction result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no floating-subtraction result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic floating value or class held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no floating-subtraction result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the same-format floating-subtraction semantic relation from `operators.md` under selected contract `C`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the existing bounded scalar operation-owned transient category. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, source-visible storage identity, NaN observation object, or numeric-contract state. Contract `C` is a static semantic fact of the floating-subtraction occurrence rather than an owned transient or dynamic mode. The held left value exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the floating-subtraction value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, defined-fault reason, numeric-contract state, NaN identity, or runtime mode. Finite values, signed zero/subnormal results, infinities, permitted semantic NaN results, and any additional result variation explicitly admitted by `C` are ordinary numerical outcomes selected by `operators.md` and its Core floating authority. No floating-negation rewrite or multiply-subtract contraction is introduced.

Nested represented additive forms apply their own complete producer relations recursively according to the concrete syntax tree. Every typed nesting independently retains integer-addition, floating-addition, integer-subtraction, or floating-subtraction semantic identity from its exact required type and, for each governed floating occurrence, its own explicit/defaulted numeric contract. An outer `Fast` occurrence does not recursively select an unqualified inner governed operation, and an inner `Fast` occurrence does not alter an unqualified outer operation. The accepted multi-operation floating rules decide whether any result-changing transformation may affect more than one occurrence; this revision adds no multiply-subtract or negated-fused permission. No operand-derived inference, promotion, conversion, defaulting, mixed-format arithmetic, inherited numeric mode, or generic arithmetic dispatch occurs.

## Plain fixed-width integer-multiplication producer validation and execution

A represented plain fixed-width integer-multiplication producer consumes from `operators.md` the context-directed exact result/operand type relation, exact mathematical-multiplication-plus-plain-overflow value relation, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is one of `F16`, `F32`, or `F64`, the same concrete binary `*` form selects the distinct floating-multiplication producer relation below. Every other `T` rejects the concrete multiplication form before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects the concrete multiplication form at this outer admission point.

After integer `T` is admitted, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the multiplication operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may consume a non-duplicable binding or external referent internally.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no multiplication result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no multiplication result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic integer value held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no multiplication result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the plain fixed-width integer-multiplication semantic value relation from `operators.md`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the same bounded scalar operation-owned transient category used by the represented Boolean eager binary operators and integer addition/subtraction. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, or source-visible storage identity. It exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the multiplication value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, numeric-contract state, or runtime state. Plain fixed-width overflow selects the value required by `operators.md`; it does not select a fault.

Nested represented multiplicative operations apply this same complete producer relation recursively when grouping establishes repeated multiplication. Mixed multiplicative/additive trees execute recursively according to their concrete syntax tree. Each operation retains its exact required type `T`; no operand-derived inference, promotion, conversion, defaulting, Shared-dereference reinterpretation, or generic arithmetic dispatch occurs.

## Same-format floating-multiplication producer validation and execution

A represented same-format floating-multiplication producer consumes from `operators.md` the context-directed exact result/operand type relation, its one already-established selected numeric contract `C`, the accepted same-format floating-multiplication value relation under `C`, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `F16`, `F32`, or `F64`. If it is one of the eight represented fixed-width integer types, the same concrete binary `*` form selects the distinct integer-multiplication producer relation above. Every other `T` rejects the concrete multiplication form before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects the concrete multiplication form at this outer admission point.

For an unqualified floating-multiplication occurrence, no explicit source selector establishes a different contract, so the accepted Core floating fallback establishes `C = Standard`. For a source-valid `NumericContractSelectedValue`, the applicability relation above establishes `C = Fast` for exactly the selected root occurrence before this operand transaction begins. Current source defines no `Reproducible` spelling, while the lower semantic contract domain remains unchanged.

After floating `T` and `C` are established, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the floating-multiplication operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented intrinsic floating values are duplicable, because a complete producer of floating type `T` may consume a non-duplicable binding or external referent internally while producing its floating result.

Existing literal rules compose without extension. A decimal floating literal operand materializes only under the exact required `T` supplied here. A decimal integer literal is not reclassified as floating merely because `T` is floating, no default/abstract floating type is introduced, and a signed decimal floating literal retains the signed-literal priority established before operator selection. Numeric-contract selection does not re-form, flush, or reinterpret literal materialization.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no floating-multiplication result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no floating-multiplication result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic floating value or class held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no floating-multiplication result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the same-format floating-multiplication semantic relation from `operators.md` under selected contract `C`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the existing bounded scalar operation-owned transient category. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, source-visible storage identity, NaN observation object, or numeric-contract state. Contract `C` is a static semantic fact of the floating-multiplication occurrence rather than an owned transient or dynamic mode. The held left value exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the ordinary floating-multiplication value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, defined-fault reason, numeric-contract state, NaN identity, or runtime mode. Finite values, signed zero/subnormal results, infinities, permitted semantic NaN results including zero-times-infinity, and any additional result variation explicitly admitted by `C` are ordinary numerical outcomes selected by `operators.md` and its Core floating authority.

Nested represented multiplicative/additive forms execute their complete source producers recursively according to the concrete syntax tree. Every typed occurrence independently retains integer-multiplication, floating-multiplication, integer-addition, or floating-addition semantic identity as applicable from its exact required type and, for each governed floating occurrence, its own explicit/defaulted numeric contract. An outer `Fast` occurrence does not recursively select an unqualified inner governed operation, and an inner `Fast` occurrence does not alter an unqualified outer operation.

The accepted Core floating owner alone determines whether already-produced numerical operation results may participate in result-changing pure multiplication reassociation or eligible finite FloatMul-to-FloatAdd contraction. Those permissions do **not** relax this source execution order: every source operand producer still executes exactly once in the represented left-before-right recursive order, with all binding/external-referent ownership, calls, safe-reference authority consequences, faults, divergence, and held-left lifetimes preserved. A realization MUST NOT use numerical reassociation or contraction permission to speculate, omit, duplicate, reorder, or fuse source producer evaluation or any source-visible effect. After the required source values and operation relationships exist, the separately owned Core floating rules may alter only the numerical realization/result where their exact all-`Fast` or Fast/Fast eligibility conditions permit it. No operand-derived inference, promotion, conversion, defaulting, mixed-format arithmetic, inherited numeric mode, generic arithmetic dispatch, ambient fast-math authority, or source fused operation occurs.

## Same-format floating-division producer validation and execution

A represented same-format floating-division producer consumes from `operators.md` the context-directed exact result/operand type relation, its one already-established selected numeric contract `C`, the accepted same-format floating-division value relation under `C`, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `F16`, `F32`, or `F64`. Every other `T` rejects the concrete `/` form before either operand is validated in a way that may commit producer state. In particular, an integer required type does not select an integer-division relation, and the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects the concrete division form at this outer admission point.

For an unqualified floating-division occurrence, no explicit source selector establishes a different contract, so the accepted Core floating fallback establishes `C = Standard`. For a source-valid `NumericContractSelectedValue`, the applicability relation above establishes `C = Fast` for exactly the selected root occurrence before this operand transaction begins. Current source defines no `Reproducible` spelling, while the lower semantic contract domain remains unchanged.

After `T` and `C` are established, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the floating-division operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented intrinsic floating values are duplicable, because a complete producer of floating type `T` may consume a non-duplicable binding or external referent internally while producing its floating result.

Existing literal rules compose without extension. A decimal floating literal operand materializes only under the exact required `T` supplied here. A decimal integer literal is not reclassified as floating merely because `T` is floating, no default/abstract floating type is introduced, and a signed decimal floating literal retains the signed-literal priority established before operator selection. Numeric-contract selection does not re-form, flush, or reinterpret literal materialization.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no floating-division result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no floating-division result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic floating value or class held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no floating-division result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the same-format floating-division semantic relation from `operators.md` under selected contract `C`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the existing bounded scalar operation-owned transient category. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, source-visible storage identity, NaN observation object, or numeric-contract state. Contract `C` is a static semantic fact of the floating-division occurrence rather than an owned transient or dynamic mode. The held left value exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the floating-division value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, defined-fault reason, numeric-contract state, NaN identity, physical floating exception/status state, or runtime mode. Finite values, signed zero/subnormal results, infinities, and permitted semantic NaN results—including signed-zero divided by signed-zero and infinity divided by infinity—are ordinary numerical outcomes selected by `operators.md` and its Core floating authority. A signed-zero right operand is therefore not a source fault, panic, undefined behavior, or alternate control-flow outcome.

Nested represented multiplicative/additive forms execute their complete source producers recursively according to the concrete syntax tree. Every typed occurrence independently retains integer-multiplication, floating-multiplication, floating-division, integer-addition, floating-addition, integer-subtraction, or floating-subtraction semantic identity as applicable from its exact required type and, for each governed floating occurrence, its own explicit/defaulted numeric contract. An outer `Fast` occurrence does not recursively select an unqualified inner governed operation, and an inner `Fast` occurrence does not alter an unqualified outer operation.

The accepted Core floating owner gives FloatDiv no result-changing reassociation, reciprocal-replacement, or fused-divide permission. Source execution therefore preserves the represented division identity and complete left-before-right producer order. A realization MUST NOT use another numerical optimization permission to speculate, omit, duplicate, reorder, replace division with reciprocal multiplication, or otherwise alter source producer evaluation or any source-visible effect. No operand-derived inference, promotion, conversion, defaulting, mixed-format arithmetic, inherited numeric mode, generic arithmetic dispatch, ambient fast-math authority, or source fused operation occurs.

## Plain fixed-width integer-exclusive-or producer validation and execution

A represented plain fixed-width integer-exclusive-or producer consumes from `operators.md` the context-directed exact result/operand type relation, exact representation-neutral fixed-width exclusive-or value relation, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is not, exclusive-or is source-invalid before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects integer exclusive-or at this outer admission point.

After `T` is admitted, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the exclusive-or operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may consume a non-duplicable binding or external referent internally.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no exclusive-or result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no exclusive-or result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic integer value held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no exclusive-or result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the plain fixed-width integer-exclusive-or semantic value relation from `operators.md`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the same bounded scalar operation-owned transient category used by the represented Boolean eager binary operators and integer multiplication/addition/subtraction. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, or source-visible storage identity. It exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the exclusive-or value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, overflow classification, physical-representation fact, numeric-contract state, or runtime state.

Nested represented exclusive-or operations apply this same complete producer relation recursively when grouping establishes repeated exclusive-or. Mixed multiplicative/additive/exclusive-or trees execute recursively according to their concrete syntax tree. Each operation retains its exact required type `T`; no operand-derived inference, promotion, conversion, defaulting, physical bit-pattern reinterpretation, or generic bitwise dispatch occurs.

## Plain fixed-width integer-bitwise-OR producer validation and execution

A represented plain fixed-width integer-bitwise-OR producer consumes from `operators.md` the context-directed exact result/operand type relation, exact representation-neutral fixed-width bitwise-OR value relation, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

Let `T` be the surrounding receiving position's exact required source type. Source validation MUST first establish that `T` is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`. If it is not, bitwise OR is source-invalid before either operand is validated in a way that may commit producer state. In particular, the exact `Bool` requirement supplied by a represented `if` or bounded-`while` condition rejects integer bitwise OR at this outer admission point.

After `T` is admitted, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `T` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `T` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the bitwise-OR operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though represented fixed-width integer values are intrinsically duplicable, because a complete producer of integer type `T` may consume a non-duplicable binding or external referent internally.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `T`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no bitwise-OR result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no bitwise-OR result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned `T` value as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `T`, regardless of the semantic integer value held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no bitwise-OR result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left `T` value and the successfully produced right `T` value exactly once by applying the plain fixed-width integer-bitwise-OR semantic value relation from `operators.md`; and
11. transfer the resulting distinct owned `T` value exactly once to the surrounding receiving position.

The held left operand is the same bounded scalar operation-owned transient category used by the represented Boolean eager binary operators and integer multiplication/addition/subtraction/exclusive-or. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, or source-visible storage identity. It exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, the bitwise-OR value transformation itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, cleanup phase, overflow classification, physical-representation fact, numeric-contract state, or runtime state.

Nested represented bitwise-OR operations apply this same complete producer relation recursively when grouping establishes repeated bitwise OR. Mixed multiplicative/additive/exclusive-or/bitwise-OR trees execute recursively according to their concrete syntax tree. Each operation retains its exact required type `T`; no operand-derived inference, promotion, conversion, defaulting, physical bit-pattern reinterpretation, XOR/complement/arithmetic decomposition, or generic bitwise dispatch occurs.

## Boolean equality and inequality producer validation and execution

A represented Boolean equality or inequality producer consumes from `operators.md` its intrinsic result type `Bool`, exact left/right operand required type `Bool`, exhaustive successful truth relation, exactly-once consumption of both successful owned operand values, and absence of any operator-local structural-ownership transition.

The surrounding receiving position's required source type applies first to the operator's intrinsic result type. Source validation MUST establish that the surrounding required type is exactly `Bool` before either operand is validated in a way that may commit producer state. If the surrounding required type is not `Bool`, the operator is source-invalid with result type `Bool`, and no left or right operand consequence is committed merely while diagnosing that mismatch.

After outer result admission succeeds, validate the complete binary producer as one transaction:

1. clone the current source producer-validation state;
2. validate the complete left producer with required source type exactly `Bool` against that clone;
3. only after complete left validation succeeds, validate the complete right producer with required source type exactly `Bool` against the resulting speculative state;
4. commit the final speculative state to the containing source-validation state only when both complete operands are source-valid; and
5. add no further source-state transition for the equality/inequality operation itself.

A left validation failure therefore commits no speculative producer-state consequence. A right validation failure also commits no speculative consequence from the otherwise-valid left producer. This transaction remains required even though intrinsic Bool values are duplicable, because a complete Bool producer may consume a non-duplicable binding or external referent internally while producing its Bool result, for example through a direct-call argument or another nested producer.

Dynamic execution is eager and exactly left-to-right:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `Bool`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no equality/inequality result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after left success, hold its one produced owned Bool as the in-progress binary operator's **held left operand**;
6. evaluate the complete right producer exactly once with required source type `Bool`, regardless of the semantic Bool value held on the left;
7. preserve every source-state consequence completed by right evaluation;
8. if right evaluation yields defined fault `F`, clean the held left operand exactly once, produce no equality/inequality result, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
9. if right evaluation diverges, retain ownership of the held left operand as part of the suspended in-progress operation and perform no cleanup merely because execution remains suspended;
10. after both operands succeed, consume the held left Bool and the successfully produced right Bool exactly once by applying the selected equality/inequality semantic value relation from `operators.md`; and
11. transfer the resulting distinct owned Bool exactly once to the surrounding receiving position.

The held left operand is one bounded operation-owned transient value. It is not a source binding, place, lvalue, address, reference, argument transient, construction transient, field-receiver transient, pattern transient, condition transient, or source-visible storage identity. It exists only after left producer success and ends by cleanup on right fault, by successful operator consumption after right success, or remains owned while right evaluation diverges.

After both operand values have been produced successfully, equality/inequality itself is non-faulting and non-diverging and adds no source-visible side effect, structural transition, storage identity, or runtime state.

When equality/inequality is used as a represented `if` or bounded-`while` condition, the successful post-condition producer-validation state is exactly the sequential successful state after right producer evaluation. The operator truth relation changes only the two successfully produced Bool operands into one Bool result and introduces no successful path-dependent structural state, source state set, join, meet, widening, implicit restoration, or normalization rule.

Nested represented Boolean operators apply their own complete producer relations recursively. In particular, an equality operand may itself be a prefixed logical-negation producer, and complete nested producer execution finishes before the enclosing eager binary operation consumes its operand value.

## Boolean short-circuit conjunction producer validation and execution

A represented Boolean short-circuit conjunction producer consumes from `operators.md` its intrinsic result type `Bool`, exact left/right operand required type `Bool`, short-circuit truth relation, path-dependent successful operand consumption, and absence of any conjunction-owned structural-ownership transition.

Let `E` be the current source producer-validation state before conjunction validation. The surrounding receiving position's required source type applies first to the conjunction's intrinsic result type. Source validation MUST establish that the surrounding required type is exactly `Bool` before either operand is validated in a way that may commit producer state. If the surrounding required type is not `Bool`, the conjunction is source-invalid with result type `Bool`, and no operand consequence is committed merely while diagnosing that mismatch.

After outer result admission succeeds, validate the complete conjunction as one transaction:

1. clone `E`;
2. validate the complete left producer with required source type exactly `Bool` against that clone, yielding successful speculative post-left state `L`;
3. only after complete left validation succeeds, clone `L` and validate the complete right producer with required source type exactly `Bool` against that clone, yielding successful speculative post-right state `R`;
4. validate the complete right producer regardless of whether the left producer is a literal or another producer whose semantic Bool value could otherwise be known statically; this relation performs no value-based pruning;
5. require every active binding root and replacement-capable external referent root represented in `L` and `R` to have exactly equal complete structural ownership state, and require any persistent safe-authority state observable after successful operand completion to be exactly the same under its canonical lexical/carrier relation;
6. if either operand validation or that exact-state equality fails, reject the conjunction and commit none of the speculative conjunction transaction to the containing source-validation state;
7. only when the outer result requirement, both complete operands, and exact-state equality all succeed, commit the one common normal state `L = R` to the containing source-validation state; and
8. add no conjunction-owned source-state transition.

The exact-state equality is required because successful dynamic left-`false` execution skips the right producer and normally completes with `L`, while successful dynamic left-`true`/right-success execution normally completes with `R`. The currently represented source ownership model exposes one definite normal structural state from one successful value producer. Conjunction therefore rejects `L != R` rather than synthesizing cleanup, restoration, union, intersection, meet, join, widening, maybe-owned state, runtime ownership flags, value-dependent source validation, or an authority-graph merge.

This requirement does not demand that the right producer be effect-free. A source-valid right producer may contain accepted operations whose successful structural/authority state equals `L`, and it may independently fault or diverge at runtime according to its existing semantics. The equality constrains only the successful normal source state exposed after the conjunction. Raw-pointer origin requires no additional conjunction-local comparison in this slice because neither the direct conditional grammar nor any represented `Value` producer can retarget an existing raw-pointer binding; statement-level retargeting remains governed by `local-bindings.md` and the enclosing control-flow relations.

Dynamic execution is exactly:

1. evaluate the complete left producer exactly once under its existing producer semantics with required source type `Bool`;
2. preserve every source-state consequence completed by left evaluation;
3. if left evaluation yields defined fault `F`, produce no conjunction result, do not begin right evaluation, and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
4. if left evaluation diverges, produce no conjunction result, do not begin right evaluation, and perform no cleanup merely because execution remains suspended;
5. after successful left Bool production, consume that owned left Bool exactly once for short-circuit selection;
6. when the left Bool is `false`, do not evaluate the right producer and produce one distinct owned Bool result `false`;
7. when the left Bool is `true`, evaluate the complete right producer exactly once under its existing producer semantics with required source type `Bool`;
8. preserve every source-state consequence completed by right evaluation;
9. if right evaluation yields defined fault `F`, produce no conjunction result and continue the same `F` through the existing producer/receiving cleanup and propagation relations;
10. if right evaluation diverges, produce no conjunction result and perform no cleanup merely because execution remains suspended;
11. after successful right Bool production, consume that owned right Bool exactly once and produce one distinct owned Bool result with the same semantic truth value; and
12. transfer the successful conjunction result exactly once to the surrounding receiving position.

There is no held-left operator transient across right execution. The left Bool's ownership ends at step 5 before the right producer can begin, so a right fault performs no conjunction-specific left-result cleanup and a right divergence retains no left-result transient. Completed producer-state consequences from left and any executed portion of right evaluation remain governed by those operand producers and the ordinary activation cleanup relations.

On either successful normal path, the conjunction exposes the one definite source state established by source validation: the left-`false` path has `L`, and the left-`true`/right-success path has `R`, with source validity having proved `L = R`. The conjunction truth transformation adds no further structural transition, defined-fault reason, divergence point after a successful selected operand value, source-visible storage identity, runtime ownership flag, or hidden source state.

When conjunction is used as a represented `if` or bounded-`while` condition, `control-flow.md` therefore receives one completed owned Bool and one definite successful post-condition state from this producer relation. Conditional/loop selection does not need a conjunction-specific ownership join or restoration rule. Lexically scoped safe-reference locals or child authorities created within nested calls/blocks are ended by their existing cleanup before any surviving normal state; no additional reference-state lattice is introduced.

Nested conjunctions apply this same relation recursively where the concrete grammar uses grouping to establish nesting. Each nested conjunction must independently satisfy its own exact-state requirement before the enclosing producer may commit its transaction.

## Producer-backed field-value execution

A source-valid producer-backed field-value use consumes from `field-access.md` the already validated receiver category, complete receiver producer, exact receiver type, complete resolved non-empty field path, exact final result type, duplicate-or-consume consequence, and canonical remaining-frontier cleanup paths.

The surrounding receiving position's required source type applies to the final selected field result. It does not replace the receiver producer's own exact result type. A direct-call receiver therefore executes against the result type selected from its resolved callable signature, and a record-construction receiver executes against its explicit resolved nominal target type, whether the concrete target was unqualified or qualified.

For one producer-backed field-value use, execution is exactly:

1. evaluate the retained receiver producer exactly once under its existing direct-call or record-construction semantics and its own exact receiver type;
2. preserve every source-state transition caused while evaluating that receiver producer;
3. if receiver evaluation yields a defined fault, establish no field-receiver transient and no selected field result; the field-value producer yields that same fault to its surrounding receiving relation, while receiver-producer internal cleanup and that receiving relation's existing cleanup/propagation remain controlling;
4. if receiver evaluation diverges, establish no field-receiver transient and no selected field result, and perform no field-receiver cleanup merely because execution remains suspended;
5. after successful receiver production, transfer the complete produced record value into one fully owned **field-receiver transient** whose structural ownership state begins complete;
6. apply the source-selected final-field duplicate-or-consume consequence to the already resolved field path through the transient ownership facts owned by `field-access.md`;
7. preserve the successfully produced selected field result outside the field-receiver transient's cleanup set;
8. clean exactly the source-selected canonical remaining frontier of the field-receiver transient in canonical frontier order;
9. end the field-receiver transient completely; and
10. transfer the preserved selected field result exactly once, without duplication, to the surrounding receiving position.

The field-receiver transient is not a source binding, place, lvalue, reference, addressable object, pattern scrutinee transient, construction transient, argument transient, or held eager-binary-operator operand. It participates only in this composite field-value operation and never enters lexical/activation cleanup.

When the selected final field is duplicable, the selected result is the independent duplicate chosen by `field-access.md`; the complete receiver transient remains owned until step 8 and its original selected subvalue is cleaned with the receiver remainder. When the selected final field is non-duplicable, exactly that selected path has transferred to the preserved result and is absent from the remaining frontier cleaned at step 8.

After successful receiver production, field selection, selected-result production, remaining-frontier cleanup, and result transfer add no new defined-fault or divergence outcome under the represented source model. Zero-field and recursively zero-leaf frontier members remain real source ownership whose ending may refine to no scalar Core destruction when the lower destruction domain is empty.

The complete field-receiver transient lifecycle finishes before an enclosing local receives the result, before assignment or reference-replacement old-value cleanup begins, before a direct-call argument transient is established for that result, before a return begins activation cleanup, before an enclosing construction initializer holds its construction transient, before a represented control-flow construct owns its Bool condition transient, and before a producer-backed record pattern establishes its separate pattern scrutinee transient.

## Record construction

A source-valid represented record construction has one resolved nominal record target and one named initializer for every declared field as mapped by `concrete-syntax.md`. The target may have been selected by the accepted unqualified same-module construction lookup or by the accepted one-hop qualified cross-module lookup; target qualification has no dynamic execution role after source validation.

Before construction execution begins, source validation has already established the complete static construction boundary: target alias/member lookup where applicable, target binding accessibility and record category, exact nominal result type, every initializer field identity, direct field accessibility for every known initializer under `field-access.md`, duplicate status, exhaustive field coverage, and exact surrounding required-type equality when a receiving position supplies one. A rejection of any of those facts evaluates no initializer and commits no initializer-producer state consequence.

Construction produces exactly one owned source value of the resolved nominal record type. The target is explicit rather than inferred. When an enclosing consumer supplies a required source type, the construction result MUST be exactly equal to that type under `types.md`.

Each initializer is associated with one already resolved and accessible declaration field. That field's source type is the required type supplied to the initializer's `Value` producer. The produced field value MUST have exactly that type; no conversion, coercion, widening, narrowing, defaulting, or inference is introduced.

For a same-module target, private and exported fields are both directly accessible under `field-access.md`. For a qualified foreign target, the target record binding is already exported through qualified lookup and each explicitly initialized field must independently have exported direct accessibility. Because construction remains exhaustive, a foreign exported record with any module-private field has no source-valid qualified construction through this form. A zero-field exported foreign record has no initializer-access check and may construct directly when its target lookup/result-type requirements hold.

A represented decimal integer literal used as a field initializer materializes under that selected field type through `literals.md`. A represented decimal floating literal similarly materializes only when that selected field type is exactly `F16`, `F32`, or `F64`. These are the same required-type materialization relations used by existing value consumers and do not create conversion or inferred constructor targets.

Initializers evaluate strictly left to right in constructor source order, regardless of target record declaration order. For each initializer:

1. evaluate its `Value` producer completely;
2. preserve every source-state transition caused by that evaluation; and
3. hold the produced owned field value as one **transient construction value** associated with the selected field.

A transient construction value is semantic ownership held by the in-progress construction. It does not require source-addressable storage, a binding, field place, or other source identity.

If initializer `i` yields a defined fault before construction completes:

1. no record value is produced;
2. perform any producer-specific cleanup inside the failing initializer;
3. clean previously produced construction transients in reverse production/source order;
4. preserve structural and safe-reference state transitions already caused by evaluated initializers; and
5. continue the same defined fault.

In particular, if an earlier initializer consumed a complete non-duplicable binding, external referent, or structural subvalue, that ownership remains transferred while the transient value produced from it is cleaned exactly once after a later initializer faults. The former structural root's remaining ownership frontier excludes the consumed path.

If initializer `i` diverges, no record value is produced and no construction cleanup occurs merely because execution continues. Earlier construction transients remain owned by the suspended construction and prior source-state transitions remain effective.

Only after every initializer succeeds does assembly occur. Assembly transfers every transient construction value exactly once, without duplication, into its selected declaration field and forms one owned nominal record value. Result structural field order is declaration order; constructor source order controls evaluation/transient production.

Assembly after successful initializer evaluation is non-faulting and non-diverging. A transferred field transient is no longer independently owned and MUST NOT be cleaned separately from the completed result.

For a zero-field record there are no initializer producers/transients; successful construction directly produces the complete empty record value.

A result-bearing call, nested construction, represented operator, grouped value, numeric-contract-selected value, safe-reference producer, raw ownership-move producer, or producer-backed field-value use used as an initializer must complete before that field transient exists and before a later initializer begins. `types.md` rejects every safe-reference and raw-pointer record field, so a source-valid constructor cannot select either such field requirement. A grouping or selector wrapper adds no extra transient between its contained producer and the construction transient.

The completed record value may be transferred into ordinary local initialization, assignment RHS, reference-replacement RHS, direct-call argument, return result, enclosing construction field, a bounded producer-backed field-value receiver, or a producer-backed recursive record-pattern scrutinee. Those receiving relations keep their existing outer ordering and exact-type requirements. A qualified construction therefore composes through these existing receiving relations without defining a second execution category.

When a qualified construction is used as a producer-backed field receiver, the outer field-value transaction owned by `field-access.md` establishes its field receiver/path/result facts before this construction may commit initializer ownership. When a qualified construction appears as a record-pattern scrutinee producer, `patterns.md` has already resolved the top pattern head, whether unqualified or qualified, and its exact nominal type is the required construction result type. The construction may therefore supply that pattern exactly when both resolve to the same nominal record and their independent target/field-accessibility rules are source-valid; qualification on either side adds no execution step.

Target qualification is source lookup only. It creates no runtime module loading, dynamic access check, ABI/layout/linkage consequence, constructor function, or distinct constructed-value identity. A faithful typed representation may discard the qualified-vs-unqualified target category after retaining the resolved record identity and initializer facts.

This relation adds no field assignment, partial-field reinitialization, pattern selection, update/spread/default initialization, shorthand, positive duplicability selection, method/constructor body, public-constructor capability, or constructor-specific visibility class.

## Direct-call arguments

A represented direct call has exactly one ordered argument operand for each callable-signature parameter slot. Argument count MUST match exactly.

Each argument evaluation MUST produce one owned source value whose type equals exactly the corresponding parameter source type. This revision introduces no implicit conversion, coercion, widening, narrowing, subtyping, numeric defaulting, or reference/pass-mode conversion.

The parameter type is the required source type supplied to a producer that needs one. Decimal integer and decimal floating arguments therefore materialize under the corresponding parameter type through `literals.md`; this does not create a conversion or inference relation. A safe-reference parameter similarly supplies its exact `SharedRef(T)` or `ExclusiveReplaceRef(T)` required type to an existing safe-reference value producer under `references.md`. `RawPtr(T)` is not parameter-admissible under `callables.md`, so a direct-call argument never transports a raw-pointer value or pointer-origin provenance across this activation boundary.

Arguments evaluate left to right.

Ordinary complete-binding argument use follows `local-bindings.md`; safe root formation, complete-referent dereference, and explicit reborrow follow `references.md`; raw ownership move may produce an ordinary non-pointer argument value when the parameter's exact type is its pointee `T`; field-value arguments follow `field-access.md` and, for bounded producer-backed receivers, the complete field-receiver lifecycle above. Any structural ownership or safe-authority/carrier/delegation consequence occurs at that argument's evaluation position. A grouping or numeric-contract-selected wrapper has exactly its contained producer's evaluation position and adds no second argument-effect boundary.

Each successfully evaluated argument is held as one owned **transient argument value** until all arguments succeed. Transient ownership is semantic and does not require a materialized source storage place. When the transient argument has safe-reference type, it owns exactly the carrier contained by that produced reference value for the duration of the transient; source validation also retains any Shared-result provenance and authority-derivation facts required by `references.md`.

If argument `i` yields a defined fault before callee activation:

1. no callee activation is created;
2. earlier transient argument values are cleaned in reverse production order;
3. cleaning an earlier safe-reference transient removes exactly its carrier and lets `references.md` end that authority or transitively restore any parent capability when its last descendant branch ends;
4. source ownership and safe-reference consequences already caused by earlier arguments remain otherwise effective; and
5. the same fault continues in the caller.

If argument evaluation diverges, no callee activation is created. Earlier transient arguments remain owned by the suspended computation and no cleanup occurs merely because time passes. A retained safe-reference argument transient therefore retains its carrier and applicable authority/delegation relation while execution remains suspended.

After all arguments succeed and before callee activation is created, source validity MUST establish the call-entry obligations from `references.md` for every held safe-reference argument:

1. its complete target/referent structural root is fully available; and
2. its authority retains the complete capability promised by its exact safe-reference type.

A held `SharedRef(T)` therefore requires full Shared capability. A held `ExclusiveReplaceRef(T)` requires full replacement-capable exclusive capability. A parent reference with any active child that reduces or suspends the required capability cannot itself satisfy that call-entry obligation. There is no implicit call-site reborrow: retaining a replacement-capable parent across a nested call requires explicit `&mut *parent` or, for a Shared parameter where represented, explicit `&*parent`.

After those obligations succeed:

1. create the callee activation;
2. transfer each transient argument in parameter-slot order into its corresponding parameter binding without duplication;
3. establish every parameter binding with complete initial structural ownership of its transferred value; and
4. establish every replacement-capable parameter's non-binding external referent structural root fully available.

For Shared-reference type, transfer moves the transient's produced carrier into the callee parameter binding; ordinary prior Shared binding use may already have duplicated that carrier at argument production, but the call boundary itself creates neither another carrier nor another root/child authority. For replacement-capable reference type, transfer moves the one existing non-copyable carrier into the parameter. Reborrow arguments already carry their fresh child authority; parameter transfer does not replace that child with the parent authority.

Parameter slots remain ordinary owned-value parameter slots. Safe-reference type is a value-type dimension, not a pass-mode dimension. Raw-pointer type is not admitted as a parameter. This slice introduces no borrowed slot, raw-pointer slot, hidden outlives parameter, or alternate call mechanism.

On normal callee completion, every incoming replacement-capable external referent has already satisfied the normal restoration obligation below before activation cleanup. Every callee-owned safe-reference carrier not preserved as a valid contract-bearing Shared result is then removed by ordinary local/parameter cleanup. A valid Shared-reference result carrier is instead preserved outside that cleanup and transferred to the caller under the direct-call result relation below. Ending a reborrowed child parameter branch restores its parent's retained capability according to `references.md` when that was the final descendant branch. On defined fault, there is no external-referent restoration obligation; no result is produced and ordinary fault cleanup removes callee-owned carriers before the same fault continues into the caller. If the callee diverges, the caller remains suspended, transferred safe authorities/carriers and then-current external referent states remain live, and no result or synthetic restoration is produced. These rules compose identically for nested and recursive calls.

## Direct-call result transfer

A result-bearing direct call produces exactly one owned source result value only after the callee completes normally. Its exact result type is the target callable signature's result type.

For an ordinary non-reference result, successful result transfer is the existing owned-value transfer relation and introduces no new rule in this revision. `ExclusiveReplaceRef(T)` and `RawPtr(T)` are not result-admissible under `callables.md`, so neither replacement-capable reference carriers nor raw-pointer provenance can cross a direct-call return boundary as results.

For a target whose callable signature has result type `SharedRef(T)` and advertised Shared-reference result-origin parameter slot `j`, `references.md` guarantees that every source-valid normal callee result preserves the exact dynamic target and **same authority identity** established by the value transferred to slot `j`. The caller-side result provenance is exactly the caller-side provenance of the successfully produced argument value supplied to slot `j`.

The call/result boundary itself creates no Shared root borrow, authority, reborrow, additional reference carrier, or semantic reference `Copy`. The returned carrier is the carrier produced by valid callee result evaluation and preserved outside callee activation cleanup. A fresh callee reborrow cannot satisfy the exact result contract merely because it targets the same referent; a caller-created Shared child passed into the advertised origin slot may round-trip only when the callee preserves that exact incoming child authority.

Consequences include:

- when slot `j` receives an ordinary use of an existing Shared-reference binding, that argument production has already duplicated the caller's stored carrier; the caller's original carrier remains owned there and the distinct returned carrier may coexist with it for the same authority;
- when slot `j` receives a temporary root `&x`, successful return preserves a carrier for that same root authority after the callee parameter carrier would otherwise be cleaned, so the caller may transfer the result into an immutable Shared-reference local while `x` remains in its valid lexical extent;
- when slot `j` receives a caller-created Shared child `&*parent`, valid return preserves that exact child authority rather than deriving a new child in the callee; and
- a nested or recursive contract-bearing call exposes its result provenance from callable structure alone, allowing a surrounding call or Return to compose without expanding the nested callee body.

If callee execution yields defined fault, the direct-call producer yields that same fault and produces no result value, result carrier, or result provenance. A surrounding local/assignment/reference-replacement/return/argument receiving position is therefore not initialized or transferred a result on that path.

If the callee diverges, the caller remains suspended at the call; no direct-call result value, result carrier, provenance, receiving-position initialization, restoration, or synthetic cleanup is produced merely because execution continues.

## Ordinary local initialization

When a represented local initializer evaluates an owned value producer, evaluation completes before the produced value is transferred into the binding.

The local's declared type is the required type supplied to a producer that needs one. Represented decimal integer and decimal floating literal initializers therefore materialize under that declared type through `literals.md` before transfer. The produced value MUST have exactly that type.

After transfer, the local begins with complete initial structural ownership of its value under `local-bindings.md` and `structural-ownership.md`. Transfer does not duplicate the produced value. When the local has safe-reference type, source validity has already required the local to be immutable. Transfer moves the produced reference carrier into that binding without creating another authority; Shared provenance is preserved, while a replacement-capable reference remains non-copyable and retains its exact authority/delegation relation. When a stored child reference local later leaves scope, carrier cleanup ends that child branch as applicable and restores parent capability only through the lifecycle relation in `references.md`. When the local has raw-pointer type, `raw-pointers-unsafe.md` and `local-bindings.md` have already required the incoming pointer's exact target extent to contain the complete receiving-local extent; transfer preserves that exact pointer value and `PointerOrigin(binding)` provenance into the local without accessing the pointee.

If initializer evaluation yields a defined fault or diverges, the local never receives an owned value. Structural ownership and safe-reference authority/carrier consequences already performed by earlier evaluated operations remain effective subject to the applicable producer/transient cleanup relation. No pointer origin is installed in a local whose initialization never completes.

## Record-destructuring declaration completion

A source-valid recursive record-destructuring declaration consumes the complete pattern field/rest structure, resolved top nominal record identity, scrutinee category, exact top scrutinee type, explicit binding-leaf order, leaf paths/types, direct-root leaf availability/authority requirements, per-leaf duplicate-or-consume consequences, and producer-transient remaining frontier from `patterns.md`, `references.md`, and `structural-ownership.md`. Qualified versus unqualified pattern-head spelling and any concrete node-local rest marker have already been discharged by source validation before this execution relation begins.

### Direct binding-root completion

For the direct binding-root category:

1. complete source validation for the entire recursive pattern, including every explicit binding leaf's required full availability and canonical Shared-or-Exclusive direct-authority compatibility in one shared pre-pattern root state;
2. apply the pattern-owned direct-root binding-leaf duplicate/consume productions in retained depth-first source order;
3. each produced leaf value becomes the complete initial owned value of its corresponding not-yet-in-scope pattern binding;
4. establish all of those new bindings in the containing lexical scope together; and
5. only then may the next body statement begin.

The direct root is not evaluated through ordinary whole-binding `IdentifierUse` and no scrutinee transient exists.

Pattern leaf production is non-faulting and non-diverging after source validation.

A direct zero-field, recursively empty nested, or rest-only pattern contributes no binding leaf and therefore performs no ownership production merely for that static structure. A top-level rest-only direct pattern is an ownership no-op even when the nominal record has fields: omitted fields remain in the direct root's existing structural ownership state, and rest itself adds no whole-root availability requirement or ownership transition.

### Producer-backed completion

For a producer-backed category:

1. complete recursive pattern field/rest structure and introduced-binding validity before producer evaluation;
2. validate the selected direct-call, record-construction, or field-value producer using the top pattern head's resolved nominal record type as the exact required type and the pre-pattern-binding lexical/source producer-validation environment;
3. only after that complete producer is source-valid, evaluate it completely;
4. when that producer is a producer-backed field-value use, complete its field-receiver production, selected-result preservation, remaining-frontier cleanup, and field-receiver transient ending before the resulting owned record can become the pattern scrutinee;
5. if producer evaluation faults or diverges, perform no pattern leaf production and establish no pattern scrutinee transient;
6. on producer success, transfer the produced record into one fully owned pattern scrutinee transient whose structural ownership state begins complete;
7. apply pattern-owned binding-leaf `Duplicate`/`Consume` productions in retained depth-first source order;
8. after every explicit binding leaf has been produced, clean the transient's remaining structural ownership frontier selected by `patterns.md` through `structural-ownership.md` exactly once;
9. only after transient cleanup completes, establish all pattern-introduced bindings in the containing lexical scope together; and
10. only then may the next body statement begin.

Any accepted producer-backed scrutinee follows exactly this sequence when its result type equals the already resolved top pattern record type. Whether the producer target or receiver, the pattern head, both, or neither use represented qualification does not add a producer, transient, ownership transition, fault path, divergence path, or cleanup phase.

The pattern scrutinee transient is not a local binding and does not participate in lexical/activation cleanup after step 8. A field-receiver transient internal to the producer is a separate earlier transient and likewise never participates in pattern-transient cleanup.

Successful leaf production and pattern-transient cleanup introduce no new defined-fault or divergence outcome after producer success under the represented relation.

If producer evaluation yields a defined fault before pattern-transient establishment:

- no pattern leaf production occurs;
- no pattern binding enters scope;
- producer-internal transient cleanup occurs exactly as in another receiving position; for a producer-backed field-value producer, a receiver fault establishes no field-receiver transient;
- source-state transitions already completed by producer evaluation remain effective; and
- the same fault continues through activation fault propagation.

If producer evaluation diverges before pattern-transient establishment, no pattern leaf production, pattern binding establishment, or pattern-declaration transient cleanup occurs. Producer-owned transients remain owned by the suspended producer. For a producer-backed field-value use, no field-receiver transient exists unless its receiver producer has already completed successfully; after such success the represented field-selection/cleanup tail itself does not diverge.

For a producer-backed zero-field top pattern, successful producer evaluation yields one complete empty-record pattern transient. There are no binding leaves; its canonical remaining frontier contains the complete empty root, whose source ownership ends before declaration completion even when lower scalar cleanup is vacuous.

For a producer-backed rest-only top pattern of any represented nominal record, successful producer evaluation likewise produces no binding leaves. Because no path is consumed by leaf production, the existing canonical remaining frontier contains the complete transient root; that complete transient is cleaned exactly once before the declaration completes and no binding enters scope. This is the pattern-specific transient completion relation, not a general arbitrary-result discard operation.

Fields omitted by rest in a producer-backed pattern receive no leaf-production transition. They remain part of the transient's then-current ownership and are therefore included wherever the existing canonical remaining-frontier relation selects them after explicit leaf production.

This section does not redefine pattern-head/field/rest selection, structural path validity, source duplicability, safe-authority compatibility, producer syntax, field-receiver frontier membership, or pattern-transient frontier membership.

## Whole-binding assignment and replacement

A represented whole-binding assignment consumes assignment target legality, mutability, canonical Exclusive direct-authority compatibility where applicable, declared type, RHS type requirement, raw-pointer incoming-origin validity when applicable, and binding structural lifecycle from `local-bindings.md`, `references.md`, and `raw-pointers-unsafe.md`, together with remaining-frontier selection from `structural-ownership.md`.

The complete assignment target admission, including the requirement that no overlapping active safe authority conflicts with the target root, is established before the RHS is evaluated in a way that could commit a producer-state consequence. The target's declared source type is the required type supplied to an RHS producer that needs one. Represented decimal integer and decimal floating literal RHS values therefore materialize under that target type through `literals.md` before replacement execution. For a mutable `RawPtr(T)` target, successful RHS production must additionally yield an exact raw-pointer origin whose target extent contains the complete receiving pointer-local extent before replacement may commit.

For a source-valid assignment, execution is **source-first** with respect to replacement:

1. evaluate the RHS completely;
2. preserve every source-state consequence caused while evaluating that RHS;
3. preserve the successfully produced RHS value, including exact raw-pointer origin when applicable, outside the target's old-value cleanup set until replacement transfer;
4. only after successful RHS production and any required raw-pointer lexical target-validity check, select the target binding's then-current complete-root remaining ownership frontier under `structural-ownership.md`;
5. clean each frontier source subvalue exactly once in canonical frontier order;
6. transfer the produced RHS value into the complete target binding without duplication; and
7. establish a fresh complete structural ownership state for the replacement value and, for a raw-pointer target, install the incoming exact pointer origin as that binding's continuing provenance.

The target remains in scope during RHS evaluation. Every RHS use follows its ordinary producer semantics rather than a special self-assignment rule.

For duplicable `x = x`, RHS evaluation duplicates the complete old value, leaving the old root fully available; replacement then cleans that complete old value and transfers the duplicate into `x`. When `x` is a raw-pointer local, that duplicate retains the same exact origin and cleanup of the old pointer value has no pointee effect.

For non-duplicable `x = x`, RHS evaluation consumes the complete old value, leaving no old target-owned frontier; replacement transfers the produced value back into `x` without duplicate cleanup when source-valid under the canonical direct-authority rule.

If the RHS consumes only a non-duplicable subvalue of `x`, the target becomes partial before replacement. Its canonical remaining frontier then contains exactly the maximal still-owned disjoint source subvalues; replacement cleans those and never re-cleans the consumed path.

The same ordering applies when direct-call argument evaluation, record-construction initializer evaluation, producer-backed field-receiver evaluation, complete-referent dereference, raw ownership move, or another represented producer consumes the complete target or one of its structural subvalues before a later operation successfully produces the replacement value. A grouping or numeric-contract-selected wrapper around such a producer adds no second ownership or ordering point.

If RHS evaluation yields a defined fault, assignment performs no replacement cleanup/reset/transfer. Completed source-state transitions remain effective and activation fault cleanup uses the target's resulting current state. For a raw-pointer assignment no incoming origin is installed because replacement never completes.

If RHS evaluation diverges, assignment performs no replacement cleanup/reset/transfer. The activation remains suspended and no cleanup occurs merely because execution continues.

Core structural destruction/storage mechanics remain owned by Core; this source relation selects source ownership and source ordering.

This revision defines no field/place assignment, partial-field reinitialization, compound assignment, assignment expression, reference-relative assignment other than the bounded complete-referent replacement below, source interior mutability, raw pointee replacement through ordinary assignment, or destructuring assignment. Unsafe raw pointee replacement is the distinct operation after the safe-reference replacement relation below.

## Complete-referent replacement execution

A source-valid complete-referent replacement statement `*r = Value;` consumes from `references.md` one active binding `r` of exact type `ExclusiveReplaceRef(T)`, its current complete referent domain, the destination carrier/authority liveness and full replacement-capable authority requirement, exact RHS type `T`, source-first ordering, then-current remaining-frontier cleanup, and complete-root structural reset.

Execution is exactly:

1. resolve `r` and identify its current complete referent domain without consuming, copying, or otherwise disabling the destination reference carrier;
2. evaluate the complete RHS producer exactly once with required source type `T` under its existing producer semantics;
3. preserve every source-state and producer-transient consequence completed by that evaluation;
4. if RHS evaluation yields defined fault `F`, perform no outer referent cleanup, replacement, structural reset, or reference-carrier change and propagate the same `F` through the existing receiving/fault relation;
5. if RHS evaluation diverges, perform no outer referent cleanup, replacement, structural reset, or synthetic reference change and remain suspended in that producer;
6. after successful RHS production, require `r` still to own a live destination carrier and its authority to retain full replacement-capable exclusive reference-relative authority over the complete referent;
7. preserve the successfully produced RHS value outside the old-referent cleanup set;
8. select and clean the referent's then-current complete-root remaining ownership frontier in canonical frontier order;
9. transfer the produced exact-`T` value into the complete referent root; and
10. establish fresh complete structural ownership for that referent domain.

The destination reference carrier is neither consumed nor replaced by this statement. The referent may have been fully available, partially available, or unavailable before successful replacement. The RHS may itself ownership-move the referent—for example through `*r` when `T` is non-duplicable—so long as it does not consume, move away, or otherwise disable the destination reference carrier/authority required at step 6. This relation deliberately rejects any design that snapshots a dereference destination, lets the RHS consume the destination reference, and then commits through stale authority.

After the post-source destination-authority check and RHS success, the replacement tail is finite, non-faulting, and non-diverging under the represented source model. A failed authority/precondition is source invalidity rather than defined fault.

## Raw replacement and unsafe-block execution

A source-valid raw replacement statement consumes from `raw-pointers-unsafe.md` one active unsafe-admission region, one resolved pointer binding of exact type `RawPtr(T)`, its snapshotted exact `PointerOrigin(x)`, target lexical validity, the RHS exact type `T`, canonical post-source Exclusive target compatibility, and the source-first target-replacement relation.

Execution is exactly:

1. resolve the raw-pointer operand and snapshot its exact pointer origin before RHS evaluation;
2. evaluate the complete RHS producer exactly once with required source type `T` under its existing producer semantics;
3. preserve every source-state transition and producer-transient consequence completed by that evaluation;
4. if RHS evaluation yields defined fault `F`, perform no raw target destruction, replacement, structural reset, or pointer retargeting and propagate the same `F` through the existing receiving/fault relation;
5. if RHS evaluation diverges, perform no raw target destruction, replacement, structural reset, or pointer retargeting and remain suspended in that producer;
6. after successful RHS production, apply the post-source validity requirements from `raw-pointers-unsafe.md`, including absence of any overlapping active safe authority over `x`;
7. preserve the successfully produced RHS value outside the old-target cleanup set;
8. select and clean the target's then-current complete-root remaining ownership frontier in canonical frontier order;
9. transfer the produced exact-`T` value into the complete target root; and
10. establish the ordinary fresh complete structural ownership state for that target.

The target binding's ordinary assignment-mutability classification is not consulted by raw replacement. The pointer binding is obtained non-consumingly and is neither retargeted nor assigned; its stored pointer value and origin remain unchanged. The target may have been fully available, partially available, or unavailable before successful replacement, because the raw owner selects the then-current remaining frontier after source evaluation rather than requiring complete pre-replacement availability.

After source-validity checks and RHS success, the raw replacement tail is finite, non-faulting, and non-diverging under the represented source model. It introduces no defined fault for a failed unsafe precondition; such a failure makes the source program invalid before execution is admitted.

A represented unsafe block is one ordinary child lexical block whose body and descendant lexical blocks execute with the additional active unsafe-admission fact owned by `raw-pointers-unsafe.md`. Entering the block activates its ordinary child scope and the admission fact; entering a nested unsafe block does not add a stronger or second authority. Leaving the block normally performs the same lexical cleanup as another ordinary child block and ends only that lexical admission extent. Return, defined fault, and loop transfer from within the block use the existing non-local cleanup relations and clean the active unsafe block scope exactly once rather than performing a separate unsafe cleanup.

Unsafe-block entry and exit do not themselves read, move, duplicate, mutate, replace, clean, or retarget any source value; change structural ownership or pointer origin; create/end safe authority; change callable identity; create a runtime flag; or waive an unsafe proof obligation. An unsafe block's local-normal-continuation classification is exactly that of its contained ordinary block structure.

## Body and nested-block statement sequencing

For source validation, every represented statement or lexical block has only the minimum **local normal-continuation presence** needed by this source subset:

- **local normal continuation present** means the construct establishes exactly one definite ordinary function-local/source-external structural environment for a following statement in the same immediate sequence or for its enclosing ordinary normal continuation; and
- **no local normal continuation** means successful execution of the represented static control structure provides no fallthrough to a following statement in that same immediate sequence.

No-local-normal continuation no longer implies by definition that the current function activation terminates. A represented return or explicit `fault;` does terminate the activation, while a source-valid `break;` or `continue;` instead transfers within the nearest enclosing represented loop. The enclosing control-flow owner consumes that distinct destination.

This classification is not a source value, runtime tag, source CFG node, state set, effect, fault set, transfer-kind set, abrupt-completion lattice, ownership lattice, or pointer-origin lattice. Producer-originating defined faults and divergence remain dynamic execution outcomes and do not form additional static completion alternatives. Represented `fault;`, `break;`, and `continue;` are different because their successful execution itself has no local fallthrough. Safe-reference authority/carrier/provenance/delegation, external-referent structural state, and raw-pointer origin facts are likewise not added to a completion lattice; their validity is owned separately by `references.md`, `raw-pointers-unsafe.md`, and `control-flow.md`.

For the root function body and each represented `BlockStatement`, the applicable `BodyStatement` sequence is validated and executes strictly in concrete source order while a local normal continuation remains present. Every ordinary source-valid local declaration, record-destructuring declaration, whole-binding assignment, complete-referent replacement, raw replacement, no-result call statement, and normally completing ordinary or unsafe nested block preserves one local normal continuation. A represented explicit `fault;`, admitted `break;`, and admitted `continue;` body statement has no local normal continuation. A represented terminal return has no local normal continuation. A represented conditional exposes the local normal-continuation presence and, when present, the definite normal environment established by `control-flow.md`. A represented bounded `while` always exposes its statically represented false local normal continuation and the definite post-condition environment established by `control-flow.md`, including when its condition is the literal `true` and when some body paths transfer.

A syntactically later `BodyStatement` or terminal `ReturnStatement` in the same containing sequence after a preceding statement with no local normal continuation is source-invalid as unreachable. This semantic sequencing rule is directly observable for `fault;`, `break;`, and `continue;`, because concrete grammar permits a later body statement after them. It does not admit an otherwise unrepresented concrete tail after a terminal return in the same lexical block.

Root-body execution begins with its first statement after successful parameter transfer. A nested block begins when its statement is reached. A later statement begins only after the preceding statement completes locally normally.

For an ordinary local declaration:

1. evaluate its initializer;
2. transfer the value into the new binding and establish complete initial ownership plus any applicable safe-reference carrier/delegation/provenance or raw-pointer-origin relation; and
3. only then continue.

For a recursive record-destructuring declaration:

1. complete the grouped pattern declaration under the applicable direct-root or producer-backed completion relation; and
2. only after any producer transient cleanup and grouped binding establishment may the next statement begin.

For whole-binding assignment, complete RHS production, any raw-pointer incoming-origin check, old-value cleanup, replacement transfer, target ownership reset, and any raw-pointer origin installation before continuing.

For complete-referent replacement, complete source-first RHS production, the post-source live-destination/full-authority check, then-current referent-frontier cleanup, exact-`T` replacement, and structural reset before continuing.

For raw replacement, complete the source-first RHS evaluation, post-source raw validity checks, target remaining-frontier cleanup, and exact-`T` replacement before continuing.

For a no-result direct-call statement, complete the call normally—including every replacement-capable external-referent normal-restoration obligation—before continuing. Safe-reference arguments have already transferred into and been cleaned with the callee according to the direct-call relation above. Raw-pointer values cannot be direct-call parameters in this slice.

For represented `fault;`, apply the explicit-fault execution relation below. It has no local normal continuation and therefore never begins a following statement in the same sequence.

For represented `break;` or `continue;`, the nearest-loop target and exact target binding/external-referent structural state plus raw-pointer-origin state are already required by `control-flow.md`. Apply the loop-transfer cleanup relation below and transfer to that target. The transfer statement has no local normal continuation and therefore never begins a following statement in the same immediate sequence.

For an ordinary or unsafe nested block:

1. activate its child lexical scope and, for an unsafe block, the lexical unsafe-admission fact;
2. execute its contained sequence recursively in concrete order, including its optional terminal return when present;
3. if the nested block has a local normal continuation, normally exit the child scope using lexical-scope cleanup below and expose the resulting enclosing source state; and
4. only after that normal cleanup may the containing sequence continue.

A nested block with no local normal continuation performs no independent ordinary normal child-scope cleanup. A selected return or explicit-fault path follows the applicable activation cleanup relation, while a selected loop-transfer path follows the transfer cleanup relation below; each active child scope is cleaned exactly once by the applicable non-local completion relation. Unsafe admission adds no second cleanup layer.

A block statement produces no source value and introduces no Unit/Void value.

For a represented conditional statement, condition evaluation, selected-arm execution, explicit-arm scope composition, zero/one/two local normal outcomes, definite normal binding/external-referent structural ownership and raw-pointer origin at any local successor, and nested loop-transfer target-state validity are owned by `control-flow.md`. This sequencing relation consumes that local normal continuation only when one exists before beginning the next containing body statement. Safe-reference locals declared in an arm belong to that arm's lexical scope and have their carriers/child branches ended by its cleanup before a normal successor. No additional authority-state join is introduced.

For a represented bounded `while`, `control-flow.md` owns exact Bool admission, the pre-condition environment `H`, post-condition environment `C`, false selection, body/backedge binding/external-referent structural-state and raw-pointer-origin validity, explicit break/continue target-state validity, and the definite post-loop environment. This execution owner supplies the repeated dynamic ordering and ordinary/transfer child-scope behavior:

1. evaluate the retained condition producer under its ordinary producer execution relation;
2. after successful Bool production, when the Bool value is `false`, consume the condition result for selection and continue after the loop with no loop-body scope activation;
3. when the Bool value is `true`, consume the condition result for selection, activate the loop body's ordinary child lexical scope, and execute that block exactly once;
4. if that body reaches its local normal completion, perform its ordinary normal lexical-scope cleanup exactly once before the validated ordinary backedge returns execution to condition evaluation;
5. if execution reaches an admitted `continue;`, perform its exited-scope transfer cleanup exactly once and return execution to the selected loop's condition point without a separate ordinary body cleanup;
6. if execution reaches an admitted `break;`, perform its exited-scope transfer cleanup exactly once and continue at the selected loop's post-loop continuation without condition re-evaluation or separate ordinary body cleanup;
7. if the body returns, do not perform a separate ordinary body cleanup before return-induced activation cleanup; the active body scope is included exactly once in that termination cleanup;
8. if the body or one of its producers yields a defined fault, do not perform a separate ordinary body cleanup before defined-fault activation cleanup; the active body scope is included exactly once there; and
9. if the condition or body diverges, execution remains suspended at that operation and no normal loop/body/activation/transfer cleanup or external-referent restoration occurs merely because execution continues.

A successful ordinary or continue backedge begins a new dynamic condition evaluation, not a second source statement or a new static binding identity. Repeated execution of one static loop-body declaration creates successive dynamic binding-owned values within the same activation while retaining that declaration's one source binding identity and ordinary per-entry initialization/cleanup semantics. A safe-reference or raw-pointer local declared during one iteration is cleaned before any ordinary/continue backedge, so its carrier/delegation/origin does not become hidden loop-head state. `raw-pointers-unsafe.md` additionally prevents a longer-lived enclosing raw-pointer local from retaining an origin naming a shorter-lived loop-body binding.

If a body statement yields a defined fault, later statements do not execute and the active function activation follows fault cleanup/propagation. A nested block exiting this way does not also perform independent normal or transfer cleanup; its child scope participates exactly once in fault cleanup.

If a body statement diverges, later statements do not execute and no termination/child-scope/transfer cleanup or external-referent restoration occurs merely because execution continues.

A terminal return in the root body or a nested block begins only after every preceding statement in that same lexical sequence has completed locally normally.

A represented no-result root body reaching its closing boundary with a local normal continuation and without a terminal return performs normal no-result completion only after every incoming replacement-capable external referent is fully available. Failure of that restoration obligation makes the source body invalid rather than synthesizing repair. After the check succeeds, ordinary root-local/parameter cleanup performs normal activation termination.

This sequencing relation introduces no unrestricted mid-block return, unreachable-statement weakening, short-circuit logical operator beyond the represented conjunction producer, catch, defer, refutable match, additional loop form, labeled transfer, transfer value, or other multi-path/cyclic control transfer beyond represented terminal returns, payload-free explicit `fault;`, bounded unlabeled `break;`/`continue;`, statement-level conditional, bounded `while`, and lexical unsafe blocks consumed above.

## Explicit fault statement

The represented payload-free source `fault;` statement selects exactly one distinguished source-semantic defined-fault reason whose specification label is **`ExplicitFault`**.

`ExplicitFault` is one semantic fault-reason identity consumed by the defined-fault propagation relation below. It is not a source value or source type and has no source payload, message/string, numeric code, fault-site identity, exception object, backtrace, matching interface, catch interface, stable serialization, ABI identity, or required implementation representation.

Every source-valid execution of `fault;` selects the same distinguished source reason `ExplicitFault`. Different source locations containing `fault;` do not thereby create different fault reasons.

The statement has no operand, required value type, owned-value producer, binding target, result value, or local normal continuation. Reaching it after all preceding statements in the same sequence have completed locally normally:

1. evaluates no owned-value producer and performs no new binding read, move, duplicate, assignment, field selection, reference formation/dereference/reborrow/replacement, raw address/move/replacement, call, or value production;
2. preserves every source ownership/reference/pointer-origin transition completed before the statement;
3. selects exactly `ExplicitFault`;
4. enters the existing defined-fault propagation relation below with `F = ExplicitFault`; and
5. produces no ordinary continuation from the current activation.

The statement itself does not diverge after it is reached: it selects a defined fault. Earlier operations may independently fault or diverge before execution reaches the statement.

When `fault;` is reached inside a nested block, unsafe block, conditional arm, or bounded-`while` body, that child scope does not first perform ordinary normal or loop-transfer cleanup. The defined-fault termination relation cleans every then-active lexical scope innermost through root exactly once and then processes parameters in reverse callable-signature slot order. Cleaning safe-reference locals/parameters removes their carriers under `references.md`; cleaning raw-pointer locals has no pointee effect under `raw-pointers-unsafe.md`. Fault termination has no normal external-referent restoration obligation.

This section defines no `fault(...)`, `fault value;`, panic/throw spelling, payload/message/code, catch/recovery, fault value/type, effect signature, or programmatic inspection/comparison of `ExplicitFault`.

## Bounded loop-transfer cleanup

A source-valid `break;` or `continue;` consumes from `control-flow.md` one nearest enclosing represented `while`, its exact admitted target binding/external-referent structural and raw-pointer-origin state, and the lexical set of scopes exited by the transfer.

The transfer itself has no operand, required value type, owned-value producer, binding target, result value, or local normal continuation. Reaching it after all preceding statements in the same immediate sequence have completed locally normally performs no additional source operation before cleanup.

For the transfer's exited lexical scopes, cleanup proceeds **innermost to outermost** through and including the selected loop's body scope, stopping before the lexical scope containing the `while` statement. Within each exited scope:

1. consider local bindings declared directly in that scope in reverse local declaration order;
2. for each binding, select its then-current complete-root remaining ownership frontier under `structural-ownership.md`;
3. clean every frontier member in canonical frontier order, including removal of a safe-reference carrier/child branch when that binding has safe-reference type and target-neutral cleanup when it has raw-pointer type; and
4. end that binding's dynamic source ownership for the exited scope.

Consumed/unavailable paths are not cleaned again. A partially available binding cleans only its maximal still-owned disjoint frontier. A fully available zero-field or recursively zero-leaf source value remains real source ownership whose cleanup may refine to no Core `Drop` when the lower destruction domain is empty.

Transfer cleanup does not clean function parameters or locals belonging to an enclosing scope outside the selected loop body merely because those bindings participate in the target loop's `H`/`C` proof. It does not restore, mutate, reset, consume, retarget, or otherwise repair an enclosing binding or replacement-capable external referent merely to make that proof succeed. Source validity requires the exact target state to have been established before the transfer by ordinary accepted operations.

After the complete exited-scope cleanup:

- `continue;` transfers to the selected loop's condition point; and
- `break;` transfers to the selected loop's post-loop continuation.

No exited scope first receives an independent ordinary normal cleanup and then a second transfer cleanup. The transfer relation is the unique cleanup of those scopes for that execution.

If an operation before the transfer yields a defined fault, the transfer is never reached and the ordinary defined-fault cleanup relation controls. If execution diverges before the transfer, no transfer cleanup occurs merely because the computation remains suspended. If a return or explicit `fault;` is selected on another path, its activation-termination cleanup controls rather than loop-transfer cleanup.

Nested loops compose by target selection from `control-flow.md`: a transfer in an inner loop body exits only scopes through the inner body; it does not clean or exit an outer loop body merely because that body is active. An intervening unsafe block is an ordinary exited child scope rather than a transfer target.

## Source cleanup

For represented operations, **cleaning an owned source value** ends source execution's ownership of that value exactly once.

A binding cleanup selects its complete-root remaining ownership frontier under `structural-ownership.md`. Each frontier path denotes one maximal still-owned source subvalue. Cleaning the binding means cleaning those frontier subvalues exactly once in canonical frontier order and then ending the binding's source ownership.

For a safe-reference value, cleaning ends exactly the carrier contained by that owned reference value under `references.md`; it does not read, move, mutate, drop, restore, or otherwise clean the referent. For Shared authority, final-carrier cleanup ends the authority when no descendant exists. For replacement-capable authority, moving/cleanup remains non-copyable carrier ownership and a carrierless parent may stay active through descendants; authority ends transitively only when neither carrier nor descendant remains. Parent capability restoration is a consequence of child-branch ending, not referent cleanup.

For a raw-pointer value, cleaning ends only ownership of that pointer value and any non-observable source provenance retained with it. It does not read, move, mutate, destroy, restore, retarget, or otherwise affect the pointee and creates/ends no safe authority. A raw-pointer local's stored origin therefore ceases to matter when that pointer value's binding ownership ends; no pointee cleanup is induced.

A field-receiver transient and a producer-backed recursive record-pattern transient each use the same structural frontier relation over their own non-binding structural owned-value root. Cleaning either transient means cleaning the frontier values selected by its semantic owner exactly once in canonical frontier order and then ending all ownership held by that transient. Neither transient becomes a binding or participates in lexical/activation/loop-transfer cleanup. Record values cannot contain safe-reference or raw-pointer fields, so these structural transients do not recursively carry source reference carriers or pointer origins.

The held left operand of an in-progress represented eager binary operator is a scalar operation-owned transient rather than a structural-frontier transient. For Boolean equality/inequality it has type `Bool`; for integer multiplication/addition/subtraction/exclusive-or/bitwise OR it has the selected fixed-width integer type `T`; for same-format floating multiplication, division, addition, and subtraction it has the selected exact `F16`, `F32`, or `F64` type `T`. Cleaning it on right-operand fault ends that one owned value exactly once. If right evaluation succeeds, the applicable operator relation consumes it instead; if right evaluation diverges, it remains owned by the suspended operation. A floating operation's selected contract is not part of the held value and adds no cleanup-bearing state. Unary Boolean negation, integer negation, and integer complement create no held-left transient or other separately cleanup-bearing operator transient. Boolean short-circuit conjunction likewise creates no held-left transient across right evaluation because the successful left Bool is consumed for selection before the right producer can begin.

A grouping or numeric-contract-selected wrapper never owns the contained value separately and has no cleanup set or cleanup step. Any cleanup required while evaluating or receiving such a wrapped value is exactly the cleanup already selected by the contained producer and the surrounding receiving relation.

When a source value is realized in Core storage, applicable destruction-domain, stored-value-lifetime, and cleanup semantics remain owned by [Core value and storage semantics](../core/value-storage.md). Core carrier-aware safe-reference cleanup remains owned by [Core references](../core/references.md); Core raw-pointer value/storage semantics remain owned by [Core pointers and provenance](../core/pointers.md) and [Core value and storage semantics](../core/value-storage.md). This document determines only source ownership-ending selection and source order while `references.md` and `raw-pointers-unsafe.md` own their source-specific relations.

A value already transferred or consumed is not cleaned again by its former owner. This applies to lexical/activation/loop-transfer cleanup, construction/argument transients, held eager-binary-operator operands, field-receiver transients, producer-backed pattern transients, old assignment/reference-replacement/raw-replacement target subvalues. A transferred safe-reference carrier is likewise removed only by its current owner and never again by the former transient/binding. For raw-pointer values, transfer carries the pointer value/origin to its current owner without creating any former-owner pointee cleanup.

A fully available zero-field or recursively zero-leaf source subvalue remains a legitimate cleanup value. Cleaning it ends source ownership even when lower representation has no scalar destruction leaf and therefore needs no physical/Core `Drop`.

This revision introduces no custom source destructor body, source `drop` ability, must-consume policy, or general temporary-lifetime extension rule.

## Lexical-scope cleanup

When execution normally exits a represented lexical scope, consider all local bindings declared directly in that scope in **reverse local declaration order**. This includes ordinary locals and bindings introduced by recursive record-destructuring declarations.

For each binding, select its then-current complete-root remaining frontier under `structural-ownership.md` and clean every frontier member in canonical order. When the binding itself has safe-reference type, that cleanup additionally removes its one stored carrier under `references.md`; the target referent is not cleaned by reference destruction. When it has raw-pointer type, cleanup ends only the pointer value/origin and does not access the target.

For one record-destructuring declaration, `patterns.md` defines depth-first binding-leaf source order as the declaration order of the introduced bindings. Reverse local declaration cleanup therefore visits later binding leaves before earlier leaves, independently of record structural field order.

A fully available binding frontier contains only its complete root. An unavailable complete root has an empty frontier. A partial root cleans exactly the maximal still-owned disjoint subvalues and never re-cleans a consumed path. Shared-reference locals are duplicable and immutable and therefore normally retain complete binding structural ownership until lexical cleanup; replacement-capable reference locals are immutable but non-duplicable and may have had their carrier moved, in which case their binding root is consumed and has no carrier to clean. Raw-pointer locals are duplicable and may be mutable, but pointer cleanup remains target-neutral regardless of the stored exact origin.

When one source completion exits multiple active scopes, cleanup proceeds innermost to outermost. Ordinary normal nested-scope exit, unsafe-block exit, loop transfer, return, and defined fault each select their applicable exited-scope range; each scope uses reverse local declaration order, and each binding uses its canonical remaining frontier and any applicable reference-carrier or raw-pointer cleanup consequence.

Function parameters belong to the root activation but are not local declarations. On activation termination, after root lexical locals, process parameters in **reverse callable-signature parameter-slot order**, each using its then-current binding frontier. A safe-reference parameter loses its transferred carrier when that parameter is cleaned unless a valid Shared result carrier has already been separated outside activation cleanup. Raw-pointer parameters do not exist in this source slice. A loop transfer is not activation termination and therefore does not process parameters merely because it exits loop-body child scopes.

For normal activation termination only, every incoming replacement-capable external referent must already have passed the fully-available restoration check before this parameter cleanup begins. Parameter cleanup does not itself restore or clean that external referent; it ends the reference carrier/authority branch. Fault termination has no corresponding restoration precondition and cleans the actual carrier state without synthesizing target repair.

This cleanup order is semantic and independent of physical stack layout, ABI passing, compiler/Core local numbering, or backend strategy. Together with the source safe-reference and raw-pointer lexical validity rules, it ensures callee parameter carrier branches end before control returns/faults into a caller whose target storage remains live, ensures a reference local declared after its root/parent is cleaned before the applicable parent extent/branch may end, and ensures a valid raw-pointer local ends before the target extent that was required to contain it. A valid Shared-reference result is not a carrier targeting callee-local storage: the advertised-origin provenance/identity rule in `references.md` proves it denotes the selected incoming Shared parameter authority before activation cleanup begins.

## Normal return

A represented return may be the optional terminal return of the root body or of any represented nested lexical block admitted by `concrete-syntax.md`. Every such return terminates the current source function activation; it does not merely exit the immediately containing block.

For a source function with one result type, that result type is the required type supplied to the return-value producer. Represented decimal integer and decimal floating literal returns therefore materialize under the declared result type through `literals.md`. When the declared result is bounded `SharedRef(T)`, `callables.md` supplies the enclosing callable's advertised Shared-reference result-origin parameter slot and `references.md` owns the required provenance plus dynamic target/authority identity relation. `ExclusiveReplaceRef(T)` and `RawPtr(T)` are not result-admissible, so no source-valid normal return can export either a replacement-capable carrier or a raw-pointer value/origin from the activation.

A represented return in a result-bearing function MUST first evaluate exactly one owned value producer whose type equals exactly that result type. A represented return in a no-result function MUST contain no value. A grouping or numeric-contract-selected wrapper around a return value does not add a second producer or return phase.

Result evaluation, including any structural ownership transition, safe-reference production/duplication/reborrow, complete-referent move, raw ownership move, nested-call result consequence, or producer-specific transient cleanup, completes before the normal external-referent restoration check and return-induced scope/activation cleanup.

For a function whose result is contract-bearing `SharedRef(T)` with advertised origin slot `i`, successful result evaluation is additionally source-valid only when `references.md` proves that the produced result value has provenance exactly `ParameterOrigin(i)` and its carrier names the exact dynamic target and **same Shared authority identity** established by the incoming value for slot `i`. This check occurs after result-producer effects and before activation cleanup. A fresh root, another parameter's provenance, a local derived from a fresh root, or any fresh child reborrow—even one reaching the same root—therefore cannot begin normal return cleanup merely because its type matches. A nested exact-identity result with matching provenance remains valid.

After successful result evaluation and any required Shared-result identity/provenance check, normal return additionally requires every incoming replacement-capable external referent structural root to be fully available. This check observes the state after all return-value producer effects. No cleanup edge, implicit replacement, reset, or other repair is synthesized to satisfy it.

Only after all normal-result and restoration checks succeed:

1. preserve the owned transient result outside activation-local cleanup, including its one Shared result carrier when applicable;
2. clean all active lexical scopes innermost through root;
3. process parameters in reverse slot order, cleaning each current binding frontier and ending safe-reference parameter carriers/authority branches not themselves the already-preserved Shared result value;
4. terminate the callee activation normally; and
5. deliver the preserved owned transient result to the caller.

Transfer to the caller does not duplicate the result. For a Shared-reference result, an `IdentifierUse` or another permitted producer may already have created a duplicate carrier as part of that producer's ordinary semantics; the Return boundary itself creates no additional reference `Copy`, borrow, authority, target, or reborrow.

A complete non-duplicable local consumed by result evaluation has no remaining frontier and is not cleaned again. A consumed subvalue is excluded while disjoint remaining subvalues are cleaned normally. A Shared-reference local used to produce a valid result remains a separate stored carrier and is cleaned normally; the produced result carrier survives because it was separated before cleanup. A replacement-capable reference cannot be a result and, if its carrier was moved into an operation before return, its binding cleanup follows its then-current structural state. A raw-pointer local itself cannot be the returned result; a raw move may instead have transferred an ordinary pointee value out of its target before cleanup, in which case that target's updated structural state controls later cleanup normally.

For a no-result function, represented `return;` first requires every incoming replacement-capable external referent fully available, then performs the same active-scope/parameter cleanup and normal activation termination but produces no value.

A return reached from a nested block, unsafe block, conditional arm, or bounded-`while` body does not first perform that block's ordinary normal or loop-transfer cleanup. Return-induced activation cleanup already includes every then-active descendant scope and therefore cleans each binding exactly once.

If return-value production yields a defined fault before successful result production, or if a nested producer faults, no normal return or normal external-referent restoration check occurs and no Shared result is established. The existing defined-fault cleanup/propagation relation below handles the then-current active scopes exactly once. If return-value production diverges, no normal return cleanup, result transfer, or synthetic restoration occurs merely because execution remains suspended.

Reaching the normal end of a represented no-result function body is equivalent to normal no-result completion only after the external-referent restoration check described above, and then performs the same root-local/parameter cleanup, including safe-reference carrier ending and target-neutral raw-pointer cleanup where applicable.

A result-bearing represented body MUST NOT have a reachable normal end without a result. This is a normal-path validity requirement, not a requirement for one unconditional concrete root-terminal return. A represented path that terminates by explicit `fault;` is abnormal and therefore requires no result value or external-referent normal restoration. A conditional whose two explicit arms both terminate the activation may therefore satisfy the result obligation without a following root return whether those arms return, explicitly fault, or use a represented mixture of the two. A conditional with no local fallthrough only because its paths perform loop transfers does not terminate the activation and cannot independently satisfy the result obligation. A represented bounded `while` always retains its statically represented false local normal continuation under `control-flow.md`, including for literal `true` and regardless of admitted transfers in its body, so the loop alone does not discharge the result obligation. When any represented path still establishes a normal root continuation, that continuation must eventually encounter a source-valid result-bearing return before the root closing boundary.

No implicit result, default result, Unit, or Void source value is introduced.

## Defined-fault propagation

The represented source subset has no catch boundary.

When an applicable accepted source/Core operation yields defined fault `F`, or when represented `fault;` selects `F = ExplicitFault`, during an activation:

1. preserve every structural ownership, safe-reference, and raw-pointer-origin consequence completed before `F` was selected;
2. perform **no** normal external-referent restoration or repair merely because the activation is faulting;
3. clean all active lexical scopes innermost through root using each binding's current remaining frontier, ending safe-reference local carriers/child branches and raw-pointer local values according to their owners;
4. process parameters in reverse slot order using each current binding frontier and ending safe-reference parameter carriers/branches; and
5. terminate that activation with the same defined fault `F`.

If the fault arises from a directly called callee, the caller's direct-call evaluation yields `F`; with no catch boundary, the caller performs its own fault cleanup and propagates the same fault outward. Callee-owned reference carriers/child branches therefore end before the fault continues into the caller, while the then-current external referent structural state is not synthetically restored. Raw pointers do not cross the call boundary in this slice. This continues to the outermost applicable source execution.

A Shared-reference result-origin contract is a normal-return contract only. A defined-fault path produces no result value, Shared result carrier, or result provenance and performs no transfer/initialization of the surrounding result receiving position. A safe-reference or raw replacement whose RHS faults has not performed its outer replacement; the then-current structural/authority/pointer-origin state is simply the state produced before the fault.

“Same defined fault” preserves the semantic fault-reason identity selected by the initiating operation or explicit fault statement. For `fault;`, that identity is exactly `ExplicitFault`. This revision defines no source payload representation, messages, numeric codes, exception objects, backtraces, panic/throw syntax, or catch/recovery syntax beyond the payload-free explicit fault statement defined above.

This propagation is semantic unwinding of source ownership and reference carriers and does not require physical stack unwinding. Raw-pointer cleanup during that unwinding remains target-neutral. A realization MAY use another mechanism only when it preserves every applicable cleanup and observable behavior required by the accepted source and Core contracts.

A future catch, panic, throw, payload, or other fault owner may introduce explicit source forms and extend the applicable propagation relation at those explicit boundaries. No such boundary is represented here.

Recoverable domain/application failures represented as ordinary values remain ordinary values under `behavior.md`; they do not use this relation merely because they represent failure.

## Transient-value cleanup

Construction transients produced before a later initializer fault are cleaned in reverse construction production/source order before the same fault continues. Once transferred into a successful record result, they are no longer independently owned. Target qualification does not alter this transient lifecycle. Construction transients cannot have safe-reference or raw-pointer field type because source record fields cannot contain either.

Argument transients produced before a later argument fault are cleaned in reverse production order before the fault continues in the caller. When an argument transient has safe-reference type, cleaning it removes exactly its carrier and applies ordinary authority/child-branch ending; after successful transfer into a callee parameter, that carrier is no longer owned by the caller transient and is cleaned only with the callee parameter unless valid callee execution separately produces and preserves an exact-identity Shared result carrier for the advertised origin. Raw-pointer argument transients do not arise because raw-pointer parameters are invalid.

For each represented eager binary operator, successful left production establishes exactly one held left operand value before right evaluation begins. If right evaluation faults, that held value is cleaned exactly once before the same fault continues. If right evaluation diverges, the held value remains owned by the suspended operation. After right success, both operand values are consumed by the selected operator relation and neither remains independently cleanup-bearing. This relation applies to the held Bool of equality/inequality, the held exact fixed-width integer value of plain multiplication/addition/subtraction/exclusive-or/bitwise OR, and the held exact `F16`/`F32`/`F64` value of same-format floating multiplication, division, addition, or subtraction under that operation occurrence's independently established numeric contract. Unary Boolean negation, plain fixed-width integer negation, and plain fixed-width integer bitwise complement establish no held-left transient and consume their one produced operand immediately after successful operand production. Boolean short-circuit conjunction also establishes no held-left transient across right evaluation: its successful left Bool is consumed for selection first, so right fault/divergence has no conjunction-owned left value to clean or retain.

A field-receiver transient exists only after its receiver producer has completed successfully. Its selected field result is preserved outside its cleanup set, its source-selected canonical remaining frontier is cleaned exactly once, and the transient ends before that result transfers to the surrounding receiving position. It is never retained for later lexical, activation, argument, construction, conditional/loop-condition, return, loop-transfer, or pattern cleanup.

An owned transient return result is outside callee activation-local cleanup after successful result evaluation because ownership has been separated for caller transfer. For the bounded contract-bearing Shared-reference result, that transient owns exactly the produced result carrier whose target/authority/provenance has already satisfied `references.md`; its provenance is validation evidence and not an additional cleanup-bearing runtime object. No replacement-capable or raw-pointer return transient is source-valid in this slice.

A successfully produced whole-binding assignment, complete-referent replacement, or raw-replacement RHS is transferred into its selected target and is not independently remaining after successful replacement. For complete-referent replacement, the destination reference carrier remains independently owned throughout and after RHS transfer. When a whole-binding RHS type is raw-pointer, its exact origin transfers with that value. If RHS production faults before success, producer-specific transient cleanup remains controlling.

A raw-address producer yields one ordinary raw-pointer value with exact origin. That value remains owned by its current receiving transient until transferred into a source-valid raw-pointer local/assignment destination or cleaned by an already existing receiving cleanup relation; ending the pointer value has no pointee effect. A raw ownership move yields one ordinary owned `T` value and has no additional pointer-specific transient after the target transfer occurs. Raw replacement preserves its successful RHS outside old-target cleanup exactly like whole-binding/referent replacement and transfers it into the pointee target after that cleanup.

A transient value produced by consuming a non-duplicable binding-root field is owned by its current transient position after production; its former binding path remains consumed and does not re-enter that binding's frontier if a later producer faults. A non-duplicable `*r` result analogously owns the complete referent value after the external/local referent root becomes consumed; it does not re-enter that root if a later producer faults. For a producer-backed field-value use, the selected transferred path likewise does not re-enter the completed field-receiver transient after its result is preserved.

A producer-backed recursive record-destructuring declaration owns one pattern scrutinee transient only after producer success. Pattern binding-leaf production may consume arbitrary retained structural paths from that transient. Fields omitted by node-local rest undergo no leaf-production transition and remain in the transient's ownership. After all explicit leaf production, the declaration cleans exactly the canonical remaining structural frontier before new bindings enter scope. The transient then ends completely and does not participate in later lexical/activation/loop-transfer cleanup.

A direct binding-root record pattern has no independently owned scrutinee transient; its accepted explicit leaf productions initialize final pattern bindings directly, while fields omitted by node-local rest remain owned by the direct root according to its existing structural ownership state.

Grouping and numeric-contract selection create no transient value category. Any transient created while evaluating either wrapped value is exactly a transient of the contained producer or the surrounding receiving relation and keeps its existing ownership/lifetime/cleanup rule. A selected contract is operation semantic metadata, not transient ownership.

A temporary root `&x` or `&mut x`, and a temporary reborrow `&*r` or `&mut *r`, is ordinary producer ownership containing the produced safe-reference carrier until it transfers into its receiving local/argument position or is cleaned by that receiving relation. Reborrow carries a fresh child authority while root formation carries a fresh root authority. A valid Shared root/child passed to the advertised origin slot of a contract-bearing call may round-trip only when the callee preserves that exact incoming authority. `*r` produces an ordinary referent value; when its referent is non-duplicable through a replacement-capable reference it additionally consumes the complete referent structural root, but creates no new reference carrier category.

This revision defines no general temporary lifetime extension, expression-statement discard, or arbitrary temporary cleanup. Only transient values required by represented record construction, direct-call argument/result transfer, represented eager-binary-operator held-left execution, producer-backed field-value receivers, assignment/reference-replacement/raw-replacement transfer, producer-backed record destructuring, and the bounded safe-reference/raw-pointer producer/receiving relations are owned here. The successful Bool condition transient used by represented conditional or bounded-`while` selection is owned and ended by `control-flow.md` after this document's existing producer relation yields it. Boolean conjunction's successful operand results are consumed inside the conjunction relation and add no separate transient category.

## Divergence

If a record-construction initializer diverges, the construction remains suspended in that initializer. Earlier construction transients and completed source-state transitions remain; no construction/activation/scope/loop-transfer cleanup or external-referent restoration occurs merely because execution continues. This is identical for unqualified and qualified construction because target/accessibility validation completed before initializer evaluation began.

If a directly called callee diverges, the caller remains suspended at that call and performs no return/fault/loop-transfer cleanup merely because time passes. Safe-reference parameter carriers and corresponding authorities remain live according to their transferred/child relations; every replacement-capable external referent remains in its then-current structural state, which may be unavailable after a callee `*r` Move. No synthetic restoration occurs. A contract-bearing Shared-reference result is normal-return-only, so divergence produces no result carrier/provenance and initializes no receiving position. Raw pointers cannot be transferred to or returned from that callee in this slice.

If the right operand of a represented eager binary operator diverges after left success, the operator remains suspended in that right producer. The held left value and completed source-state consequences remain owned/effective; no operator result or held-left cleanup occurs merely because time passes. If the left operand diverges, right evaluation never begins and no held left operand exists. This applies equally to Boolean equality/inequality, plain fixed-width integer multiplication/addition/subtraction/exclusive-or/bitwise OR, and same-format floating multiplication/division/addition/subtraction under any selected contract.

For Boolean short-circuit conjunction, if the left operand diverges, right evaluation never begins. After successful left production, the left Bool is consumed for selection. A left-`false` result completes without right evaluation. A left-`true` result may begin right evaluation; if that right producer diverges, the conjunction remains suspended there with no held-left result transient, while completed source-state consequences from the left and any completed portion of the right producer remain effective.

If the sole operand of Boolean logical negation, plain fixed-width integer negation, or plain fixed-width integer bitwise complement diverges, that unary operator remains suspended in the operand producer. No operator result or operator-local transient exists, and no cleanup occurs merely because execution remains suspended.

If a direct call or record construction used as a producer-backed field receiver diverges before successful receiver production, no field-receiver transient or selected field result exists. Any earlier producer-owned transients and completed source-state transitions remain governed by that receiver producer's existing divergence relation.

Safe root formation, explicit reborrow, and an already admitted complete-referent dereference have no divergence point after source validation. A non-duplicable complete-referent dereference performs its Move at that producer position and then yields the owned value. Divergence may occur in a surrounding receiving producer after such a value or reference carrier has already become a transient; in that case its ownership/authority remains exactly where transfer had reached and is not implicitly restored or shortened.

Root raw address formation and an already admitted raw ownership move likewise have no divergence point after source validation. Divergence may occur in a surrounding receiving producer after such a value has already become a transient; its ownership/origin remains exactly where transfer had reached.

If the RHS of complete-referent replacement diverges, the operation remains suspended in that source producer. The destination reference carrier remains owned at the replacement statement, but no referent frontier cleanup, replacement, or structural reset occurs merely because time passes. Any source-state transitions already completed within the RHS remain effective.

If the RHS of raw replacement diverges, the operation remains suspended in that source producer. The snapshotted pointer origin remains the selected prospective target but no target frontier cleanup, replacement, structural reset, or pointer retargeting occurs merely because time passes. Any source-state transitions already completed within the RHS remain effective.

Active caller/callee ownership state, active safe-reference authorities/carriers/delegation, replacement-capable external referent state, raw-pointer origins, and any suspended producer transients persist subject to operations already completed. The same applies when a diverging call is an assignment RHS, represented operator operand, producer-backed field receiver, producer-backed record-pattern scrutinee, represented conditional/loop condition, return-value producer, reference-replacement RHS, or the contained producer of a grouped or numeric-contract-selected value. Neither wrapper adds a divergence point, suspended ownership, or cleanup step. There is no implicit source execution-step budget.

A direct binding-root record-destructuring operation has no divergence point after validation. A producer-backed operation may diverge only while evaluating its existing producer; after producer success, field selection/field-receiver completion or pattern leaf production/pattern-transient completion is non-diverging under the applicable owner. Rest/omission introduces no additional divergence point or post-producer failure relation.

The represented explicit `fault;`, `break;`, and `continue;` statements themselves are not divergence categories: once reached, `fault;` selects `ExplicitFault`, while admitted break/continue perform their finite transfer cleanup and transfer to the selected loop target. Entering/leaving an unsafe block likewise adds no divergence point of its own.

## Effects boundary

Left-to-right constructor evaluation, left-to-right argument evaluation plus post-production safe-reference call-entry validation, source-first ordinary-assignment RHS evaluation, source-first complete-referent-replacement RHS evaluation, source-first raw-replacement RHS evaluation, raw address formation and raw ownership move at their producer positions, safe root formation, complete-referent dereference and explicit reborrow at their producer positions, Boolean logical-negation, plain fixed-width integer-negation, and plain fixed-width integer-complement operand evaluation before their respective result transformations, eager left-to-right plain integer-multiplication/addition/subtraction/exclusive-or/bitwise-OR, same-format floating-multiplication/floating-division/floating-addition/floating-subtraction under each occurrence's selected contract, and Boolean equality/inequality operand evaluation, Boolean short-circuit conjunction left evaluation and selection before any permitted right evaluation, producer-backed field receiver evaluation before selected-field production, producer-before-pattern evaluation, **depth-first explicit pattern binding-leaf source order**, and concrete body/block statement sequencing fix relative source ordering for any effects that future accepted operation owners make observable. A node-local rest marker contributes no effect position or binding-leaf order item.

Safe root formation creates one fresh source authority/carrier at its producer position without reading, moving, mutating, or consuming the target value. Complete-referent dereference accesses the referent under the retained capability selected by `references.md`; for replacement-capable non-duplicable referents it performs the structural ownership Move at that producer position. Explicit reborrow creates a fresh child authority/carrier and delegates capability without moving/copying the parent carrier. Shared exact-identity result transfer preserves a previously produced authority/carrier relationship across the return/call boundary and introduces no additional producer/effect position. Those are semantic alias/value-production consequences but introduce no source-visible physical addressing, runtime pointer arithmetic, or source effect system.

Complete-referent replacement first evaluates its RHS while retaining the destination reference carrier and only after successful source production plus the live/full-authority check performs then-current referent-frontier cleanup and replacement. It is not authority to use a stale snapshotted destination after the RHS disables the destination reference.

Raw address formation produces an activation-local raw-pointer value/origin without accessing the target value and is admitted only under canonical Shared direct compatibility. Raw ownership move performs its target structural consumption at its producer position after canonical Exclusive target compatibility and other source-valid unsafe preconditions have been established. Raw replacement first evaluates its RHS and only after successful source production performs canonical Exclusive compatibility checking plus target remaining-frontier cleanup/replacement. Entering an unsafe block changes only lexical admission for those bounded raw operations; it does not itself create an effect, runtime mode, or proof waiver.

A grouping or numeric-contract-selected wrapper introduces no additional evaluation/effect point around its contained producer. Any ordering fact involving such a wrapped value is exactly the ordering that applies to the contained producer at that same receiving position. Numeric-contract selection can change only numerical behavior explicitly authorized by the Core floating owner; it is not evaluation-order authority. In particular, floating-multiplication reassociation or finite multiply-add contraction permission cannot reorder, speculate, omit, duplicate, or fuse the evaluation of source operand producers or their source-visible effects, and no accepted permission turns FloatDiv into reciprocal multiplication or authorizes division reassociation.

For represented conditional and bounded-`while` selection, `control-flow.md` owns condition-producer-before-selected-arm/body ordering and consumes the producer/effect ordering defined here; this execution owner does not add speculation, arm/body reordering, loop-condition hoisting, external-referent repair, or authority-state merging. Each dynamic ordinary or continue backedge reaches a fresh condition evaluation after the applicable ordinary normal or transfer cleanup. A break transfer does not evaluate the condition merely to reach the post-loop continuation.

Literal evaluation has no source-visible side effect under `literals.md`; adding literals to represented ordinary value positions therefore adds no competing effect-order relation. Represented operators similarly add no operator-local side effect after all operand values required by the selected successful semantic path have been produced under `operators.md`. Every represented eager binary operator evaluates its right producer after left success regardless of the left semantic value. Boolean short-circuit conjunction is distinct: after left success it consumes the left Bool for selection, skips the right producer when left is `false`, and evaluates the right producer exactly once only when left is `true`. Plain integer negation, integer complement, multiplication, addition, subtraction, exclusive-or, bitwise OR, and same-format floating multiplication, division, addition, and subtraction are non-faulting/non-diverging after their required operand values exist, regardless of a governed floating occurrence's selected numerical result set, just like the represented Boolean transformations on their selected successful paths. For floating division, this includes signed-zero divisors and NaN-class outcomes: those are ordinary numerical results, not fault/effect branches.

A binding-root field-value production is non-faulting/non-diverging after source validation, but consuming a non-duplicable field performs its structural ownership transition at that producer position and applies canonical Exclusive direct compatibility. A producer-backed field-value use first executes its retained receiver exactly once, then completes the selected field production and receiver-transient cleanup before its result becomes available to the surrounding consumer. When either field-value category is a producer-backed pattern scrutinee, the complete field-value operation finishes before pattern transient establishment and leaf production.

Pattern binding-leaf production is non-faulting/non-diverging after source validation and any producer completion. Its source-ordered non-duplicable leaf transfers are ownership transitions whose consequences are visible to later leaves and statements and require canonical Exclusive direct compatibility; duplicating leaves require canonical Shared compatibility. Rest itself adds no runtime producer, value transformation, or effect position; omitted producer-transient ownership ends only through the existing remaining-frontier cleanup.

Record assembly after successful initializer evaluation is effect-free. Initializer source order, rather than declaration field order, remains producer-effect ordering authority. Target qualification is resolved statically and adds no runtime effect or ordering point. Pattern-head qualification is likewise resolved before execution and adds no runtime effect or ordering point.

Reaching `fault;` selects a defined-fault reason and terminates through the existing fault relation. Reaching an admitted loop transfer performs only its then-current exited-scope cleanup before control transfer. None evaluates an ordinary producer at that statement position.

This revision defines no source effect system, purity, effect inference, speculation legality, unsafe-callable effect, or general transformation rules.

## Concrete grammar and implementation boundary

`concrete-syntax.md` owns represented concrete grammar, including bounded Shared `&T`, replacement-capable `&mut T`, root `&x`/`&mut x`, complete-referent `*r`, explicit children `&*r`/`&mut *r`, and complete-referent replacement `*r = Value;`; activation-local `raw T`, `raw &x`, contextual `raw move p`, contextual `raw assign p = Value;`, and `unsafe { ... }`; the existing `-> &T` result spelling consumed by unique-origin Shared-result admission while `-> &mut T` remains semantically result-invalid; bounded contextual ordinary/conditional grouping; the operation-local `@fast(Value)` numeric-contract-selected value; the bounded Boolean logical-negation, plain fixed-width integer-negation, and plain fixed-width integer-complement prefix placements with signed-literal priority for `-`; the bounded multiplicative tier whose binary `*` form selects plain integer multiplication or same-format floating multiplication by exact required type and whose `/` form selects same-format floating division only under exact floating required type; the bounded additive tier whose `+` form selects plain integer addition or same-format floating addition and whose binary `-` form selects plain integer subtraction or same-format floating subtraction by exact required type; the bounded plain integer-exclusive-or tier; the bounded plain integer-bitwise-OR tier; the bounded non-associative Boolean equality/inequality tier; the bounded Boolean short-circuit-conjunction tier; unqualified and qualified record-construction targets; record-pattern heads and bounded node-local rest marker; the bounded statement-level `while`; bounded unlabeled `break;`/`continue;`; and the payload-free explicit-fault statement. `references.md` owns exact safe-reference type identity/admission, root target/authority/carrier/lifetime facts, canonical compatibility, source-validation origin provenance, complete-referent dereference semantics, explicit reborrow/delegation, complete-referent replacement preconditions/effects, parameter/result-call transfer obligations, external-referent structural roots, normal restoration, advertised Shared-origin Return validity, and exact source/Core reference refinement. `raw-pointers-unsafe.md` owns raw-pointer type/value/origin validity, address formation, RawMove/RawAssign preconditions and target effects, lexical unsafe admission, and exact source/Core raw refinement. `operators.md` owns represented operator operand/result typing, selected numeric-contract facts, semantic value transformations, operator-local ownership consequences, and operation-specific source/Core refinements. `literals.md` owns boolean/integer/decimal floating materialization and the preserved signed-literal semantic boundary. `structural-ownership.md` owns structural paths/state/availability/frontiers for binding and external-referent roots. `field-access.md` owns binding-root and bounded producer-backed receiver selection, direct field accessibility consumed by field selection, construction initializers, and explicitly selected recursive pattern fields, source-selected final-field duplicate-or-consume production, and producer-receiver remaining-frontier facts. `patterns.md` owns recursive record-pattern head resolution, explicit field/rest structure, binding-leaf facts/order, direct-root ownership production, producer-transient ownership transitions, omission consequences, and pattern-transient frontier selection. `local-bindings.md` owns binding identity/scope/lookup/mutability/lifecycle and whole-binding use/assignment legality. `control-flow.md` owns represented conditional selection/arm validation/definite normal binding/external-referent structural ownership/raw-pointer origin, bounded-`while` condition selection/backedge-state admission/post-loop state, and nearest-loop break/continue target/state admission.

Additional operators beyond Boolean logical negation, plain fixed-width integer negation/bitwise-complement/multiplication/addition/subtraction/exclusive-or/bitwise-OR, same-format floating multiplication/division/addition/subtraction, Boolean equality/inequality, and Boolean short-circuit conjunction, general expressions beyond the bounded safe-reference/raw-pointer producers, grouping/selected-value wrappers and prefix/multiplicative/additive/exclusive-or/bitwise-OR/equality/logical-conjunction tiers, arbitrary assignment places, additional loop forms, labeled transfers, transfer values, unrestricted nonterminal-within-block return, arbitrary-receiver members, additional refutable/shorthand pattern categories, additional producer-backed scrutinee families, broader raw-pointer operations, unsafe callable contracts, and other source forms remain outside this execution relation.

The represented operators, bounded safe-reference producers/reborrow/replacement/result transfer, bounded raw-pointer producers/replacement/unsafe admission, grouping wrapper, numeric-contract-selected wrapper, construction, bounded producer-backed field-value, recursive pattern including bounded node-local omission, partial-ownership cleanup, return, explicit-fault, bounded-loop-transfer cleanup, bounded-`while` execution sequencing, and existing producer execution relations are defined entirely by source identities, structural ownership, safe authority/carrier/delegation identity, source Shared-result provenance where applicable, exact raw-pointer origin provenance where applicable, owned values, source order, transfer, transient ownership, local normal-continuation presence, fault-reason identity, operator-local value semantics, selected numeric-contract facts, and cleanup. Grouping, numeric-contract selection, rest, and unsafe-block admission add no new dynamic item to that list beyond their specifically owned static semantic facts. This execution owner does not redefine Core operation, destruction, reference, pointer, or unsafe semantics. Plain integer negation and plain integer complement each consume one existing Core `IntegerSub` refinement after complete operand production through `operators.md`; plain integer multiplication consumes the one represented Core `IntegerMul` relation, same-format floating multiplication consumes the distinct represented Core `FloatMul` relation with its explicit selected contract, same-format floating division consumes the distinct represented Core `FloatDiv` relation with its explicit selected contract, plain integer addition consumes the distinct represented Core `IntegerAdd` relation, same-format floating addition consumes the distinct represented Core `FloatAdd` relation with its explicit selected contract, plain integer subtraction consumes the distinct represented Core `IntegerSub` relation, same-format floating subtraction consumes the distinct represented Core `FloatSub` relation with its explicit selected contract, plain integer exclusive-or consumes the distinct represented Core `IntegerXor` relation, and plain integer bitwise OR consumes the distinct represented Core `IntegerOr` relation through `operators.md`; the represented Boolean operators and the other source features continue to consume their existing Core relations. Safe-reference forms consume exactly the accepted Core Shared/ExclusiveReplace reference and bounded identity-preserving Shared-result/external-referent relations through `references.md`. Raw forms consume exactly the accepted Core raw-pointer/unsafe/storage/alias relations through `raw-pointers-unsafe.md`. Any source-to-Core lowering must refine these source requirements and the separately owned reference/raw/operator/conditional/loop/transfer requirements through accepted Core semantics rather than use Core representation behavior as source authority.

Record-construction target qualification, record-pattern-head qualification, and the concrete rest marker are fully discharged by source validation. A faithful construction HIR requires only the resolved nominal record identity, resolved initializer field identities/types, validated initializer values, and source location already needed by construction. A faithful pattern HIR requires only the existing resolved top nominal record identity, complete explicit binding-leaf paths/types/ownership facts, scrutinee facts, producer cleanup, binding identities/order, and source location. The concrete rest marker and omitted field identities need not be retained after validation because their semantic consequences have already been discharged into the explicit leaf set, the direct-root structural ownership result, and the producer remaining cleanup frontier. Retaining rest or omitted-field facts for diagnostics/tooling adds no lower semantic requirement. Neither construction nor pattern execution needs to retain qualified versus unqualified spelling or Core module/visibility metadata merely for qualification.

A faithful typed frontend may erase a successfully validated grouping wrapper and retain the already-built contained value unchanged. It may likewise erase a successfully validated numeric-contract-selected wrapper after retaining the selected contract directly on the qualified FloatMul, FloatDiv, FloatAdd, or FloatSub occurrence. Retaining source delimiters/location for diagnostics or tooling does not require a semantic grouping or selector-wrapper HIR variant. Consequently a lowerer may lower the contained typed value through its existing value relation without any Core grouping/selector operation or extra CFG/storage step; for a selected FloatMul, FloatDiv, FloatAdd, or FloatSub, however, the retained `Fast` contract fact MUST survive that erasure and lower exactly to that Core operation's explicit contract field.

After source validation, the bounded safe-reference forms refine only through the exact mapping owned by `references.md`: `SharedRef(T)` to canonical Core Shared reference of `lower(T)`; `ExclusiveReplaceRef(T)` to canonical Core `ExclusiveReplace` reference of `lower(T)`; root `&x`/`&mut x` to the corresponding Core root-reference operation over direct storage representing `x`; ordinary Shared duplication to Core `Copy` of the carrier-bearing reference value; replacement-capable ordinary use to ownership-transferring Move of the non-copyable carrier; bounded `*r` to reference-relative Core `Copy` for Shared/duplicable access or Core `Move` for admitted replacement-capable non-duplicable complete-referent access; explicit `&*r`/`&mut *r` to fresh Core child authority construction without moving the parent carrier; `*r = Value;` to source-first reference-relative replacement using retained full `ExclusiveReplace` authority; safe-reference parameters to existing Core parameter slots; each replacement-capable parameter to the accepted Core external-referent call-entry/postcondition relation; a source-valid scalar Shared-reference result to the same Core Shared-reference result type with the advertised source parameter slot mapped exactly to the Core function's Shared-reference result-origin parameter slot; and source cleanup/result preservation to existing Core carrier-aware cleanup and result transfer. Source origin provenance, parent/child derivation, and external-referent structural state remain source validation/refinement evidence and need not become separate runtime value fields. Source binding identity, root-target eligibility, referent admission, lexical validity, contextual type admission, call-entry/full-capability validity, normal restoration, and advertised result-origin identity MUST be established before lowering and MUST NOT be reconstructed from Core `LocalId`, `StorageRegion`, `ReferenceAuthorityId`, scalar liveness, path state, or runtime machine behavior.

After source validation, the first-slice raw-pointer forms refine only through the exact mapping owned by `raw-pointers-unsafe.md`: `RawPtr(T)` to the existing Core raw-pointer type over `lower(T)`; `raw &x` to Core `AddressOf` of the complete Core local place representing `x`; ordinary raw-pointer duplication to the existing copyable Core raw-pointer value relation; ordinary raw-pointer local initialization/assignment to existing Core owned-value initialization/replacement while preserving the source-selected pointer target; source raw ownership move to Core `RawMove`; and source raw replacement to source-first Core `RawAssign` with the source-selected remaining-value cleanup/refinement. The lexical unsafe block requires no Core unsafe-mode operation or runtime flag because source validation has already discharged the represented raw-operation preconditions. Exact pointer-origin provenance remains source validation/refinement evidence and does not require a second runtime pointer-origin object. Lowering MUST NOT reconstruct source pointer validity, target structural availability, remaining frontier, safe-authority compatibility, or control-flow origin equality from Core liveness, physical address, storage-instance IDs, pointer verifier metadata, or runtime behavior.

After source validation, represented operators may refine through the operation-specific accepted-Core relations owned by `operators.md`; this execution owner does not choose a second truth/arithmetic/bitwise mapping or lower operation. Plain integer negation lowering MUST complete its one source operand producer before creating its fresh result local and emitting exactly one existing Core `IntegerSub` from an exact same-type semantic zero constant and ownership-transferring move of the operand-result local; no negation-only Core branch/join or new arithmetic operation is required. Plain integer complement lowering MUST likewise complete its one source operand producer before creating its fresh result local and emitting exactly one existing Core `IntegerSub` from the exact same-type semantic value congruent to `-1 mod 2^N` and ownership-transferring move of the operand-result local; no complement-only Core branch/join, physical bit-pattern operation, or new arithmetic/bitwise operation is required. Every represented eager binary operator lowering must preserve complete eager left-then-right producer execution and held-left fault/divergence lifetime before its operator-local Core refinement. Plain integer multiplication then uses exactly one represented Core `IntegerMul`; same-format floating multiplication uses exactly one represented Core `FloatMul` carrying the occurrence's retained explicit `Standard` or `Fast` source-selected/defaulted contract; same-format floating division uses exactly one represented Core `FloatDiv` carrying the occurrence's retained explicit `Standard` or `Fast` source-selected/defaulted contract; plain integer addition uses exactly one represented Core `IntegerAdd`; same-format floating addition uses exactly one represented Core `FloatAdd` carrying the occurrence's retained explicit `Standard` or `Fast` source-selected/defaulted contract; plain integer subtraction uses exactly one distinct represented Core `IntegerSub`; same-format floating subtraction uses exactly one distinct represented Core `FloatSub` carrying the occurrence's retained explicit `Standard` or `Fast` source-selected/defaulted contract; plain integer exclusive-or uses exactly one distinct represented Core `IntegerXor`; plain integer bitwise OR uses exactly one distinct represented Core `IntegerOr`, while Boolean equality/inequality enter their accepted comparison CFG. For FloatMul, lowering MUST preserve the retained operation identity, explicit grouping/nesting, FloatMul-result-to-FloatAdd consumption relationship, and every governed occurrence's selected/defaulted contract; it MUST NOT pre-contract, reassociate, rewrite the multiplication, or erase a `Standard`/`Reproducible` boundary. For FloatDiv, lowering MUST likewise preserve division identity, explicit grouping/nesting, producer/consumer relationships, and the occurrence's selected/defaulted contract; it MUST NOT replace division with reciprocal multiplication, pre-round a quotient, introduce a generic floating opcode, or add FloatDiv-specific CFG solely to realize the operation. The separately owned Core floating numerical rules remain the only authority for numerical result latitude. Boolean short-circuit conjunction instead lowers complete left production before branching on an ownership-transferring move of that Bool; its false Core path initializes the fresh result to `false`, its true Core path contains the complete right-producer operations followed by initialization from an ownership-transferring move of the right Bool, and both successful paths reach the result join. No new Core operation or Core state-merge rule is required for conjunction. Duplicating binding-root field use may refine to projected Core `Copy`, consuming binding-root field use to projected `Move`, and whole-binding replacement to source-first Core `Assign`. A producer-backed field-value use may lower its retained receiver producer through the existing value lowering relation, use the produced compiler-owned receiver local as the structural root, project the retained path, preserve the selected result through the retained `Copy`/`Move` consequence, and emit cleanup only for the retained source-selected receiver remaining frontier before returning the result temporary to its enclosing lower context. Lowering MUST NOT inspect Core path liveness or initialization state to choose the source duplicate/consume consequence or receiver cleanup frontier.

A direct-root recursive pattern binding leaf may refine to a mapped source local initialized in depth-first leaf order by projected `Copy`/`Move` from the mapped source root using the retained full leaf path. A rest marker emits no direct-root lower operation of its own; omitted direct-root fields remain untouched except for ordinary effects of selected explicit paths. A producer-backed pattern may lower its existing producer to one compiler result temporary, initialize mapped pattern locals by projected `Copy`/`Move` from retained explicit leaf paths, and refine the retained source transient frontier—including any still-owned omitted fields—through projected/aggregate Core destruction. A rest-only producer-backed pattern therefore may lower to existing producer evaluation followed by retained whole-transient cleanup, with no pattern binding initialization. When that producer is a producer-backed field-value use, its receiver-result temporary and cleanup complete first; the preserved field result then becomes the separate pattern-scrutinee temporary. Qualified versus unqualified pattern-head spelling and the erased rest marker do not alter this lower relation. No rest-specific Core statement, branch, path-state rule, loan, result value, runtime flag, or reference-oracle operation is introduced, and lowering MUST NOT reconstruct omitted fields or rest from Core liveness.

A source return may refine to the existing Core `Return` terminator from whichever lower block represents that return point. For the bounded Shared-reference result, this refinement is valid only after source validation has proved the required parameter-slot provenance and exact target/authority identity, and the source callable's advertised origin slot MUST map exactly to the Core function's Shared-reference result-origin parameter slot. Lowering MUST NOT introduce a synthetic reborrow, fresh authority, or return-boundary `Copy` to satisfy that contract. Replacement-capable and raw-pointer results are not source-admitted. Ordinary normal lexical-scope cleanup and ordinary normal `Goto` continuation are emitted only for source paths that actually have a local normal continuation; a returning path does not require a synthetic normal join/backedge. Source local normal-continuation presence, source cleanup selection, Shared-result origin validity, replacement-capable external-referent restoration, and any raw-pointer origin validity MUST be established before lowering and MUST NOT be reconstructed from Core reachability, path-state worklists, scalar liveness, initialization state, pointer metadata, or coincidental runtime aliasing.

A source-valid `fault;` may refine at its corresponding lower control point to the accepted Core explicit-fault terminator `Fault(F_explicit)`, where `F_explicit` is one stable represented Core semantic fault reason chosen to preserve the source reason `ExplicitFault`. This refinement emits no ordinary operand or result and no normal `Goto`/successor merely to continue source sequencing. The stable semantic reason is required; any implementation string, numeric code, allocation, or other carrier used to distinguish that Core reason remains non-normative and is not a source-visible payload.

A source-valid `continue;` may refine its retained source-selected exited-scope cleanup to existing Core destruction/reference-carrier/raw-pointer-value cleanup where applicable and then terminate the current lower path with existing Core `Goto` to the selected nearest loop's condition-header block. A source-valid `break;` may analogously refine its retained cleanup and `Goto` the selected nearest loop's post-loop block. Ending zero-leaf source ownership may erase to no Core `Drop`. The selected Core block identities are lower implementation facts rather than source loop identity.

A faithful typed HIR for a loop transfer may retain its transfer kind, source-selected cleanup sequence, source location, and enough lexical nesting/association for lowering to preserve the already validated nearest-loop destination. It need not retain `H`, `C`, Core block IDs, a source CFG, a transfer-kind lattice, or runtime completion tags. Lowering MUST NOT recompute source binding/external-referent structural or pointer-origin target-state validity from Core path/pointer state or insert destruction/reset/restoration/retargeting of enclosing values merely to make a transfer valid.

Source ownership state, safe-reference lifetime/carrier/delegation/provenance and external-referent facts, source raw-pointer origin/unsafe-validity facts, active-scope cleanup selection, and the fact that return/fault/transfer paths have no local continuation are established before lowering. Lowering MUST NOT reconstruct those source facts from Core reachability, path-state behavior, runtime reference authority tables, pointer verifier metadata, or physical storage behavior. The accepted Core `Fault(F)`, `Goto`, reference/direct-call-result, pointer, and unsafe relations are consumed rather than redefined; these source features require no new Core operation.

Remaining source cleanup may refine to Core destruction only where the lower destruction domain is non-empty. Ending ownership of a zero-leaf source value may refine to no Core `Drop`; emitting an invalid lower destruction operation merely to materialize source ownership is not required. Safe-reference scalar cleanup is not erased merely because the referent has zero scalar leaves: it still removes the reference carrier through the accepted Core reference semantics. Raw-pointer value cleanup has no pointee-destruction refinement.

Compiler temporaries used for held eager-binary-operator operands, producer-backed field receivers, producer-backed pattern scrutinees, represented operator results, represented condition results, bounded safe-reference producer/results, or bounded raw-pointer producer/results are not source bindings. A floating-multiplication, floating-division, floating-addition, or floating-subtraction held-left operand-result local is the compiler representation of the existing scalar operation-owned transient, not a source binding, numeric-contract object, or NaN identity carrier. A unary integer-negation or integer-complement operand-result local used by lowering is likewise a compiler representation of an already completed source operand, not a source binding or additional source transient. Boolean conjunction creates no held-left source transient; its lowered left-result local is consumed by Core branching before right-path execution, and its fresh result local is a compiler representation of the completed conjunction value. A temporary carrier-bearing Core safe-reference value used to realize source root formation, reborrow, a safe-reference argument, or a valid Shared-reference call/return result is lower representation of the source reference value/carrier, not a new source binding, lifetime name, or source provenance object. A temporary Core raw-pointer value used to realize `raw &x`, ordinary pointer duplication, or raw access is lower representation of the source raw-pointer value and must preserve the already validated target relation; it is not a new source binding, lifetime name, physical-address identity, or source origin object. Grouping and selector wrappers require no compiler temporary of their own. Core path state, scalar liveness, copyability, local numbering, destruction domains, vacant-storage reuse, storage-instance identities, reference-authority IDs, and pointer-verifier metadata are not source field/pattern/structural ownership/reference authority/raw origin. Accepted Core vacant initialization may be consumed by lowering to reuse those fixed compiler/source storage locations across a loop cycle, but it does not relax source mutability, source ordinary/continue backedge-state validity, source reference lifetime/delegation/restoration validity, or source raw-pointer lexical/origin validity.

No parser, lossless syntax, typed HIR, Core MIR production lowering, runtime, or backend implementation is added or required by this semantic owner.

## Further boundaries

This revision does not define other literal semantics beyond the represented boolean, decimal integer, and decimal floating families; arithmetic beyond plain same-type fixed-width integer negation/multiplication/addition/subtraction and same-format floating multiplication/division/addition/subtraction, binary bitwise operations or shifts beyond the represented unary fixed-width integer bitwise complement and binary fixed-width integer exclusive-or/bitwise OR, floating unary negation or floating remainder, standalone fused arithmetic, unary plus, increment/decrement, numeric/record/floating/pointer/general comparison or equality operators beyond exact-Bool equality/inequality, operator forms beyond Boolean negation/plain integer negation/integer complement/integer multiplication/floating multiplication/floating division/integer addition/same-format floating addition/integer subtraction/floating subtraction/integer exclusive-or/integer bitwise OR/Boolean equality/inequality/Boolean short-circuit conjunction, source `standard` or `reproducible` selector spellings, block/function/module numeric-contract scopes or defaults, lexical/dynamic contract inheritance, caller-to-callee contract propagation, generic annotations/attributes/pragmas, additional numeric-contract-selected operation families beyond represented floating multiplication/division/addition/subtraction, short-circuit logical operators beyond represented conjunction, compound assignment, assignment-as-value, conditional expressions, unequal-state/path-dependent two-normal-outcome conditional joins, unrestricted nonterminal-within-block return or arbitrary unreachable tails, additional loop forms (`loop`, `for`, do/while), loop `else`, labels or a label namespace, labeled break/continue, transfer values, loop values, general loop fixed-point inference, refutable-match control flow, field assignment/partial-field reinitialization, arbitrary value/expression field receivers beyond the bounded direct-call/record-construction receiver set, general postfix/member/method access, additional refutable/shorthand/wildcard/literal/guard/alternative patterns, producer-backed pattern scrutinees beyond direct calls/record constructions/field-value uses, grouped or other general expression scrutinees, destructuring assignment, qualified binding leaves/qualified field names/nested module paths beyond the represented alias-member pair, additional field accessibility classes beyond the represented module-private/exported relation, safe-reference semantics beyond bounded Shared and replacement-capable complete-root references (including plain Core `Exclusive`, field/path borrow targets or child reborrows, reference-relative field access, explicit source reference Drop, interior assignment beyond bounded complete-referent replacement, reference-containing record fields or aggregate results, replacement-capable results, nested references, mutable/rebindable reference locals, multiple/explicit/derived/static Shared result origins, explicit result-origin selector syntax, named lifetime/outlives syntax, or non-lexical authority shortening), raw-pointer semantics beyond the bounded activation-local `RawPtr(T)` / `raw &x` / `raw move p` / `raw assign p = Value;` / lexical `unsafe` relation (including raw-pointer parameters/results/aggregates, pointer-to-pointer/reference types, null/fabricated/numeric pointers, RawRead source exposure, non-consuming owned raw load, field/path raw addresses, pointer arithmetic/comparison/conversion, target-sized integers, physical layout/address/alignment/stability/pinning, heap/global/static raw storage, unsafe function/callable/caller contracts, user proof contracts, or reference-to-raw conversion), indirect calls/function values/closures, generics/traits/coherence, async/tasks or Exec call semantics, effect-system completion, fault payload/message/code values, panic/throw syntax, catch/recovery, ABI/calling convention/FFI/linkage, parser/HIR/Core MIR production code, or backend behavior.
