# Source Field-Value Access

Status: **provisional normative; incomplete**

This document owns the represented source semantics for bounded dot field-path selection, the current direct record-field accessibility relation, binding-root and producer-backed field receiver categories, final-path duplicate-or-consume value production, producer-receiver transient ownership and remaining-frontier selection, and production of one owned field value.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module identity and qualified/unqualified module lookup from [Source names and modules](names-modules.md), nominal record/field identity, field source types, source type equality, and owned-value duplicability from [Source type foundation](types.md), function signatures and result presence from [Source callables](callables.md), structural source paths and path availability/consumption/frontiers from [Source structural ownership](structural-ownership.md), and function-local binding lookup/lifecycle from [Source function-local bindings](local-bindings.md). It does not redefine those owners.

The represented `.` spelling, binding-root/producer-receiver grammar, direct-call form, record-construction form, and receiving positions are owned by [Source concrete syntax](concrete-syntax.md). Evaluation of a producer receiver, dynamic field-receiver transient lifetime, transient cleanup sequencing, and transfer of a successfully produced field result into a local, assignment RHS, direct-call argument, return result, record-construction initializer, conditional, or producer-backed record-pattern scrutinee are owned by [Source function execution](function-execution.md). [Source patterns](patterns.md) independently consumes the direct field-accessibility relation defined here at every record-pattern field it selects and may receive a completed field-value result as a producer-backed scrutinee; pattern structure, exhaustiveness, binding introduction, and pattern ownership consequences remain owned there.

This document does not define structural ownership mathematics, a general member/postfix system, place/lvalue grammar, field assignment, partial-field reinitialization, reference/borrow operation, general pattern semantics, physical layout, or implementation representation.

## Represented operation

A represented **field-value use** selects one non-empty structural field path from exactly one represented receiver and produces one owned value of the final selected field type.

The represented receiver categories are exactly:

1. a **binding-root receiver**: one active parameter or ordinary local binding selected by one unqualified function-body identifier; or
2. a **producer receiver**: exactly one represented result-bearing direct call or one represented record construction whose successful result becomes the field receiver.

Conceptually:

```text
root.field
root.outer.inner
make().field
make().outer.inner
Record { field: value }.other
```

At least one field selector follows every receiver. A bare identifier therefore remains ordinary whole-binding use, a bare direct call remains a direct-call producer, and a bare record construction remains a record-construction producer.

A field-value use is one owned-value producer. It is not a source place, lvalue, reference, borrow, storage identity, address, method receiver, field-assignment target, or record pattern.

The producer-receiver category is deliberately not an arbitrary source value/expression receiver. This revision does not admit a literal, bare binding value, another generic expression form, parenthesized value, method result, reference, place, or arbitrary postfix expression as a receiver. Nested composition occurs only through already represented direct-call arguments, record-construction initializers, and the static selector chain of this operation.

After static receiver/type/field-path selection, the final selected type's owned-value duplicability selects whether production duplicates that subvalue or consumes/transfers it. Binding-root receivers additionally require the selected path to be fully available in the selected binding. Producer receivers instead begin dynamic field selection from one fresh, fully owned receiver transient after successful producer completion.

## Binding-root receiver selection

The binding-root identifier uses the unqualified function-body lookup precedence owned by `local-bindings.md`.

The selected entity MUST be one active parameter or ordinary local binding. Lookup does not bypass an active function-local binding merely because another entity would be more suitable for field selection.

Only when no active parameter/local binding resolves the root key does the existing same-module fallback occur. If that fallback selects a module declaration, the selected entity is the wrong category and the operation is source-invalid. Imported modules are not searched implicitly and source-unit module aliases do not participate in this unqualified root lookup.

The complete root value need not be fully available merely to perform static field-path selection. A partially available root may still contain fully available disjoint descendants. Final-path availability is checked only after the complete path is resolved.

The binding-root category creates no receiver transient. Its duplicate/consume consequence applies directly to the selected binding's canonical structural ownership state as defined below.

## Producer receiver selection and exact receiver type

A producer receiver is exactly one `DirectCall` or `RecordConstruction` from `concrete-syntax.md` followed by at least one field selector.

Its receiver source type is selected independently of the surrounding field-value required type:

- for a `DirectCall`, resolve the call target through the existing direct-call lookup relation and require the selected source function signature to have exactly one result value; that declared result type is the receiver type;
- for a `RecordConstruction`, resolve its explicit same-module nominal record target through the existing constructor-target relation; that nominal record type is the receiver type.

A no-result direct call cannot be a source-valid producer receiver because it produces no receiver value.

The surrounding receiving position does **not** supply its required type to the producer receiver. It requires the final selected field result type instead. The receiver producer is validated against its own statically selected receiver type and its existing argument/initializer requirements.

The producer receiver does not introduce a source-visible temporary binding, inferred receiver type, conversion, coercion, or hidden member lookup.

## Transactional producer-receiver source validation

Source validation of a producer-backed field-value use is one composite transaction with respect to function-local structural ownership state.

Before committing ownership consequences caused by receiver arguments, receiver construction initializers, or their nested producers, source validation MUST establish the operation's static receiver/selector/result facts:

1. resolve the receiver category and target relation;
2. determine its exact receiver result type as above;
3. require the first selector to begin from a nominal record type;
4. resolve the complete non-empty selector path under the field-selection/accessibility relation below;
5. determine the exact final selected field type;
6. require that final type to equal the surrounding receiving position's required source type when one exists; and
7. select the final field's duplicate-or-consume consequence from its accepted owned-value duplicability.

The receiver producer is then source-validated using its existing producer rules against its own exact receiver type and the pre-operation function-local environment. Only when the complete producer-backed field-value use is source-valid are ownership consequences from that receiver validation committed to the enclosing source-validation environment.

A rejected receiver target, no-result call, non-record selector step, inaccessible/unknown field, final required-type mismatch, invalid receiver argument/initializer, or other invalid receiver producer MUST NOT leave speculative receiver-producer ownership committed into later source validation.

This transaction boundary belongs only to this composite producer. It does not redefine the validation transaction of a direct call, record construction, or another producer when used in another receiving position.

## Field-path selection

Let the receiver have source type `T0`. Let the field selectors, in source order, have lexical keys `f0, f1, ... fn`.

For each selector `fi`:

1. the current source type `Ti` MUST be one nominal record type under `types.md`;
2. the record declaration defining `Ti` MUST permit direct field access to the containing function under the accessibility relation below;
3. `fi` selects exactly the unique declared field of that nominal record whose lexical field key equals `fi`;
4. the selected field's declared source type becomes `Ti+1`;
5. the selected source field identity extends the operation's structural source path under `structural-ownership.md`; and
6. if another selector follows, selection continues from `Ti+1`.

If the current type is intrinsic rather than a nominal record, another selector is invalid.

If the current record has no field with the requested lexical key, the operation is source-invalid. Selection does not search another record/module, methods, associated items, extensions, traits, or an outer namespace merely because the field is absent.

Field declaration order is not lookup priority. Field identity remains scoped by the containing nominal record declaration under `types.md`.

For a binding-root receiver, static selection through a partially available intermediate record path is permitted. Selection itself neither observes nor recreates the complete intermediate value. The operation becomes source-valid only if its final path is fully available.

For a producer receiver, static path selection occurs before dynamic receiver evaluation. Successful receiver production later establishes complete structural ownership of the transient root before the selected path's duplicate-or-consume consequence is applied.

## Direct record-field accessibility

The represented concrete record declaration has no field-level accessibility modifier. Under the current **direct record-field accessibility** relation, every represented record field is module-private.

A source operation that explicitly consumes this relation may directly select a field only when the record declaration containing that field belongs to the same source module as the function containing the operation.

Every represented field-value use consumes this relation independently at every selector step, regardless of receiver category. A path may select a field of a same-module record whose field type is a record defined in another module, but a later selector cannot enter that foreign record under this revision.

A qualified direct-call receiver may legally call an exported function from another module. If that function's result is an exported foreign record, the record type can be the receiver result type but its fields remain inaccessible to the caller under the current module-private direct field relation. Exporting the record type does not export its fields.

The represented recursive record pattern in `patterns.md` consumes the same relation independently for every selected field at every pattern depth. A same-module outer field whose type is a foreign exported record may therefore be bound as one complete leaf, but a nested pattern cannot directly open that foreign record while its fields remain module-private.

Module-level accessibility of the record type itself remains owned by `names-modules.md` and is independent of this field-accessibility rule. An exported record may be nameable in another module while its fields remain unavailable to direct field access or record-pattern selection there.

This field accessibility has no ABI, linkage, layout, serialization, reflection, or confidentiality meaning. This revision defines no public/exported/package/friend field modifier. A later accepted field-accessibility mechanism may broaden the direct-access domain without changing the same-module cases defined here.

## Binding-root final-path availability

For a binding-root receiver, let `p` be the complete non-empty structural source path selected from the root binding and let `Tf` be the source type of its final field.

The path `p` MUST be **fully available** under `structural-ownership.md` immediately before field-value production.

Consequently, an equal or ancestor consumed path, or any consumed descendant that makes `p` partial, rejects the operation. A consumed path structurally disjoint from `p` does not prevent use of `p`. A partially available root or intermediate record may therefore still be traversed to an untouched fully available descendant.

Failure of this requirement is source-invalidity. It is not a defined runtime moved-state fault.

This document consumes the availability relation; it does not redefine its equations or consumed-path state.

A producer receiver has no pre-existing binding-root availability state to consult. Its successful receiver transient begins complete, so the statically selected path is initially fully available by construction.

## Binding-root duplicable final fields

For a binding-root receiver, when the final selected type `Tf` is duplicable under `types.md`, successful field-value use:

1. produces one new owned source value of exactly type `Tf` through the accepted duplicability capability;
2. does not consume, move, replace, or mutate the selected subvalue; and
3. leaves the root binding's structural ownership state unchanged.

Intermediate field types need not be duplicable. Their complete values are not independently produced merely because static selection passes through them.

This preserves the accepted non-consuming behavior for every source-valid duplicable final field.

## Binding-root non-duplicable final fields

For a binding-root receiver, when `Tf` is non-duplicable, successful field-value use:

1. produces the complete owned source value at final path `p`, of exactly type `Tf`;
2. transfers/consumes that selected subvalue exactly once through `structural-ownership.md`; and
3. does not independently consume, duplicate, replace, or mutate any ancestor or structurally disjoint sibling value.

All resulting unavailable/partial/disjoint-path consequences are exactly those of the canonical structural ownership transition. The operation introduces no clone, conversion, source equality, or runtime moved-state fault.

## Producer receiver transient ownership

After a source-valid producer receiver finishes successfully, its complete produced record value becomes one fully owned **field-receiver transient**.

The transient:

- has exactly the statically selected receiver record type;
- begins with the complete structural ownership state under `structural-ownership.md`;
- is not a parameter/local binding, source place, addressable object, or new lookup identity; and
- exists only for this field-value operation until the selected result has been preserved and the transient's remaining source ownership has ended.

Let `p` be the complete selected non-empty field path and `Tf` its final type.

When `Tf` is duplicable:

1. produce one owned duplicate of the selected final subvalue;
2. leave the transient structural ownership state complete; and
3. preserve the produced duplicate outside the transient's cleanup set.

When `Tf` is non-duplicable:

1. transfer the complete selected subvalue at `p` exactly once into the produced field result;
2. consume exactly path `p` in the transient structural ownership state; and
3. preserve the transferred result outside the transient's cleanup set.

No ancestor of `p` is independently produced, duplicated, or consumed merely because selection traverses it. Structurally disjoint siblings retain their ordinary transient ownership until cleanup.

After selected-result production, select the transient's canonical remaining ownership frontier from its then-current consumed-path state under `structural-ownership.md`. Every frontier member remains owned by the transient and MUST end exactly once before the field-value operation transfers its preserved result to the surrounding receiving position.

Consequences:

- a duplicable selected final field leaves the complete transient root as the canonical remaining ownership when the root is otherwise complete; the original selected subvalue is cleaned as part of that transient while the duplicate result remains independently owned;
- a non-duplicable selected path is absent from the remaining frontier; only maximal still-owned disjoint source subvalues are cleaned;
- nested partial frontiers retain the canonical reverse structural source order from `structural-ownership.md`; and
- zero-field or recursively zero-leaf source subvalues remain real ownership/frontier members even when faithful Core scalar destruction has no physical/scalar effect.

The field-receiver transient introduces no second structural ownership algebra. Its state and frontier are ordinary applications of `structural-ownership.md` to a transient root rather than a lexical binding.

## Availability and mutability consequences

Field-value use does not define a second binding availability domain.

For a binding-root receiver, assignment mutability of the root binding is irrelevant to whether an owned field subvalue may be consumed. An immutable binding may become partially available or unavailable through permitted ownership transfer. Immutability restricts assignment/reinitialization, not ownership consumption.

A mutable partially available binding may later be replaced as a complete binding under `local-bindings.md` and the source-first assignment ordering in `function-execution.md`.

For a producer receiver, duplicate/consume affects the receiver transient rather than creating or mutating a lexical binding. Ownership transitions caused while evaluating the receiver producer remain ordinary transitions of whatever existing bindings that producer uses.

This operation itself performs no assignment or reinitialization.

## Evaluation boundary

Static receiver/category/type/path/accessibility/result validation is complete before runtime field selection.

For a binding-root receiver, field-value production itself is non-faulting and non-diverging. It performs no nested value-producer evaluation and creates no receiver transient.

For a producer receiver, dynamic receiver evaluation is owned by `function-execution.md` and may have exactly the fault/divergence/transient behavior already associated with that direct call or record construction and its nested producers. No field-receiver transient or selected field result exists until the receiver producer succeeds.

After receiver producer success, establishment of the complete field-receiver transient, static-path selected-field production, canonical remaining-frontier cleanup, and completion of the field-value producer add no new defined-fault or divergence outcome under the current source model.

The exact runtime ordering and transfer points are owned by `function-execution.md`; this document owns the receiver/transient/path/ownership facts that ordering consumes.

## Required-type composition

A successful field-value use has exactly the source type of its final selected field.

When the surrounding value consumer requires a source type, the field-value result type MUST equal that required type exactly under `types.md`.

For a producer receiver, that final required type does not become the receiver producer's required type. The receiver producer is validated/evaluated against its own exact result type selected from its function signature or explicit construction target.

The operation introduces no inference, structural compatibility, subtyping, conversion, coercion, promotion, widening, narrowing, or numeric defaulting.

The represented result may compose with ordinary local initialization, whole-binding assignment RHS evaluation, direct-call arguments, result-bearing return, record-construction field initializers, represented conditional evaluation when its exact final type is `Bool`, and a producer-backed record-pattern scrutinee whose top pattern head selects exactly the same nominal record type.

Those receiving operations retain their existing ordering, transfer, replacement, cleanup, fault, divergence, and conditional/pattern authority under `function-execution.md`, `control-flow.md`, and `patterns.md`.

When a producer-backed field result becomes a record-pattern scrutinee, the field-receiver transient completes first: its selected record result is preserved, its remaining frontier is cleaned, and the resulting owned record is then transferred into the distinct pattern scrutinee transient. The two transient states are sequential and MUST NOT be merged.

If receiver evaluation or a binding-root non-duplicable field producer consumes a path before a later enclosing producer faults, the consumed value is cleaned by its then-current owner and does not re-enter the former owner/source binding's remaining frontier.

## Operation-specific selector boundary

The field path defined here exists only to identify the source subvalue produced by `FieldValueUse` and apply the canonical structural ownership requirement to it.

It does not establish:

- a general place or lvalue;
- field assignment or partial-field reinitialization;
- an independently mutable field binding;
- a source reference or borrow;
- address-taking, pointer provenance, or physical offsets;
- arbitrary value/expression receivers beyond the explicitly represented direct-call and record-construction producer receivers;
- general postfix chaining, grouping, or an expression precedence system;
- method, associated-item, extension, trait, or overload lookup; or
- record-pattern binding semantics.

`patterns.md` independently consumes nominal field identities, direct accessibility, and structural ownership for its own accepted operation. Reusing a completed field-value result as one producer-backed pattern scrutinee does not turn a record pattern into dot field access or this operation into a general member system.

## Concrete and implementation boundary

`concrete-syntax.md` owns the represented `.` token and exact binding-root/producer-backed field-value grammar. This document does not define parser recovery, syntax-tree nodes, diagnostics, HIR representation, Core field indices, or backend behavior.

A faithful implementation MUST retain enough source-selected information to refine the accepted operation without re-running source ownership semantics. At minimum the retained information must distinguish binding-root from producer-backed receiver, retain a validated producer for a producer receiver, retain the exact receiver type, complete resolved field path, final result type, duplicate-or-consume consequence, and for a producer receiver the canonical remaining-frontier cleanup paths.

Implementation storage/recursion may use indirection. That representation does not create a source general expression tree, place, lvalue, synthetic binding, or hidden receiver identity.

Existing Core structural projections, `Copy`/`Move`, call continuations, partial initialization, and `Drop` are suitable refinement targets only after source validation has selected the receiver category, path, accessibility, ownership consequence, and producer-receiver cleanup frontier.

For a producer receiver, faithful lowering may materialize the existing receiver producer result in Core storage, project the retained path, preserve the selected result through `Copy` or `Move`, and then lower only the HIR-retained/source-selected remaining frontier. Core liveness/path state MUST NOT be inspected to choose source duplicate/consume or cleanup.

Zero-field and recursively zero-leaf source subvalues remain meaningful ownership even when a faithful lower operation has no scalar effect.

Cleanup ordering and transfer into the surrounding consumer are sequenced by `function-execution.md`.

## Further boundaries

This revision does not define field assignment or partial-field reinitialization; arbitrary value/expression receivers beyond the bounded direct-call/record-construction receiver set; a general postfix/member or expression grammar; cross-module field access or field visibility modifiers; methods/associated items; references/borrowing/lifetimes; refutable/rest/shorthand patterns; positive record duplicability-selection syntax; general operators/conversions; floating literal formation; loops/backedges or new control-flow joins; custom destructors; const/static semantics; panic payload/catch syntax; ABI/layout/FFI/linkage; Exec/Model source forms; or runtime/backend representation.
