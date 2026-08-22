# Source Field-Value Access

Status: **provisional normative; incomplete**

This document owns the represented source semantics for binding-rooted dot field-path selection, the current direct record-field accessibility relation, final-path duplicate-or-consume value production, and production of one owned field value.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module identity from [Source names and modules](names-modules.md), nominal record/field identity, field source types, source type equality, and owned-value duplicability from [Source type foundation](types.md), structural source paths and path availability/consumption from [Source structural ownership](structural-ownership.md), and function-local binding lookup/lifecycle from [Source function-local bindings](local-bindings.md). It does not redefine those owners.

The represented `.` spelling and bounded field-value grammar are owned by [Source concrete syntax](concrete-syntax.md). Transfer of a successfully produced value into a local, assignment RHS, direct-call argument, return result, record-construction initializer, or producer-backed record-pattern scrutinee is owned by [Source function execution](function-execution.md). [Source patterns](patterns.md) independently consumes the direct field-accessibility relation defined here at every record-pattern field it selects; pattern structure, exhaustiveness, binding introduction, and pattern ownership consequences remain owned there.

This document does not define structural ownership mathematics, a general member system, place/lvalue grammar, field assignment, partial-field reinitialization, reference/borrow operation, general pattern semantics, physical layout, or implementation representation.

## Represented operation

A represented **field-value use** selects one non-empty structural field path rooted in exactly one parameter or ordinary local binding and produces one owned value of the final selected field type.

Conceptually:

```text
root.field
root.outer.inner
```

The root is one unqualified function-body identifier. One or more field selectors follow it. The receiver is not an arbitrary source value or expression.

Field-value use is an owned-value producer. It is not a source place, lvalue, reference, borrow, storage identity, address, method receiver, field-assignment target, or record pattern.

After static field-path selection, the final selected path must be fully available under `structural-ownership.md`. The final field type's owned-value duplicability then selects whether production duplicates that subvalue or consumes/transfers it.

## Root binding selection

The root identifier uses the unqualified function-body lookup precedence owned by `local-bindings.md`.

The selected entity MUST be one active parameter or ordinary local binding. Lookup does not bypass an active function-local binding merely because another entity would be more suitable for field selection.

Only when no active parameter/local binding resolves the root key does the existing same-module fallback occur. If that fallback selects a module declaration, the selected entity is the wrong category and the operation is source-invalid. Imported modules are not searched implicitly and source-unit module aliases do not participate in this unqualified root lookup.

The complete root value need not be fully available merely to perform static field-path selection. A partially available root may still contain fully available disjoint descendants. Final-path availability is checked only after the complete path is resolved.

## Field-path selection

Let the root binding have source type `T0`. Let the field selectors, in source order, have lexical keys `f0, f1, ... fn`.

For each selector `fi`:

1. the current source type `Ti` MUST be one nominal record type under `types.md`;
2. the record declaration defining `Ti` MUST permit direct field access to the containing function under the accessibility relation below;
3. `fi` selects exactly the unique declared field of that nominal record whose lexical field key equals `fi`;
4. the selected field's declared source type becomes `Ti+1`;
5. the selected source field identity extends the root binding's structural source path under `structural-ownership.md`; and
6. if another selector follows, selection continues from `Ti+1`.

If the current type is intrinsic rather than a nominal record, another selector is invalid.

If the current record has no field with the requested lexical key, the operation is source-invalid. Selection does not search another record/module, methods, associated items, extensions, traits, or an outer namespace merely because the field is absent.

Field declaration order is not lookup priority. Field identity remains scoped by the containing nominal record declaration under `types.md`.

Static selection through a partially available intermediate record path is permitted. Selection itself neither observes nor recreates the complete intermediate value. The operation becomes source-valid only if its final path is fully available.

## Direct record-field accessibility

The represented concrete record declaration has no field-level accessibility modifier. Under the current **direct record-field accessibility** relation, every represented record field is module-private.

A source operation that explicitly consumes this relation may directly select a field only when the record declaration containing that field belongs to the same source module as the function containing the operation.

`FieldValueUse` consumes this relation independently at every selector step. A path may select a field of a same-module record whose field type is a record defined in another module, but a later selector cannot enter that foreign record under this revision.

The represented recursive record pattern in `patterns.md` consumes the same relation independently for every selected field at every pattern depth. A same-module outer field whose type is a foreign exported record may therefore be bound as one complete leaf, but a nested pattern cannot directly open that foreign record while its fields remain module-private.

Module-level accessibility of the record type itself remains owned by `names-modules.md` and is independent of this field-accessibility rule. An exported record may be nameable in another module while its fields remain unavailable to direct field access or record-pattern selection there.

This field accessibility has no ABI, linkage, layout, serialization, reflection, or confidentiality meaning. This revision defines no public/exported/package/friend field modifier.

## Final-path availability

Let `p` be the complete non-empty structural source path selected from the root binding and let `Tf` be the source type of its final field.

The path `p` MUST be **fully available** under `structural-ownership.md` immediately before field-value production.

Consequently, an equal or ancestor consumed path, or any consumed descendant that makes `p` partial, rejects the operation. A consumed path structurally disjoint from `p` does not prevent use of `p`. A partially available root or intermediate record may therefore still be traversed to an untouched fully available descendant.

Failure of this requirement is source-invalidity. It is not a defined runtime moved-state fault.

This document consumes the availability relation; it does not redefine its equations or consumed-path state.

## Duplicable final fields

When `Tf` is duplicable under `types.md`, successful field-value use:

1. produces one new owned source value of exactly type `Tf` through the accepted duplicability capability;
2. does not consume, move, replace, or mutate the selected subvalue; and
3. leaves the root binding's structural ownership state unchanged.

Intermediate field types need not be duplicable. Their complete values are not independently produced merely because static selection passes through them.

This preserves the accepted non-consuming behavior for every source-valid duplicable final field.

## Non-duplicable final fields

When `Tf` is non-duplicable, successful field-value use:

1. produces the complete owned source value at final path `p`, of exactly type `Tf`;
2. transfers/consumes that selected subvalue exactly once through `structural-ownership.md`; and
3. does not independently consume, duplicate, replace, or mutate any ancestor or structurally disjoint sibling value.

All resulting unavailable/partial/disjoint-path consequences are exactly those of the canonical structural ownership transition. The operation introduces no clone, conversion, source equality, or runtime moved-state fault.

## Availability and mutability consequences

Field-value use does not define a second availability domain.

Assignment mutability of the root binding is irrelevant to whether an owned field subvalue may be consumed. An immutable binding may become partially available or unavailable through permitted ownership transfer. Immutability restricts assignment/reinitialization, not ownership consumption.

A mutable partially available binding may later be replaced as a complete binding under `local-bindings.md` and the source-first assignment ordering in `function-execution.md`.

This operation itself performs no assignment or reinitialization.

## Evaluation behavior

After source validation, field-value use itself is non-faulting and non-diverging.

It performs no nested value-producer evaluation and creates no construction-like intermediate transient ownership. The complete path is statically selected before the operation produces its one owned result.

For a duplicable final field, production leaves source ownership state unchanged. For a non-duplicable final field, the structural ownership transition occurs when the producer successfully produces its result and therefore precedes transfer of that result into an enclosing local, call argument, construction transient, assignment RHS, return result, or producer-backed record-pattern scrutinee.

## Required-type composition

A successful field-value use has exactly the source type of its final selected field.

When the surrounding value consumer requires a source type, the field-value result type MUST equal that required type exactly under `types.md`.

The operation introduces no inference, structural compatibility, subtyping, conversion, coercion, promotion, widening, narrowing, or numeric defaulting.

The represented result may compose with ordinary local initialization, whole-binding assignment RHS evaluation, direct-call arguments, result-bearing return, record-construction field initializers, and a producer-backed record-pattern scrutinee whose top pattern head selects exactly the same nominal record type.

Those receiving operations retain their existing ordering, transfer, replacement, cleanup, fault, and divergence authority under `function-execution.md`. If a non-duplicable field producer consumes a path before a later producer faults, the consumed value is cleaned by its then-current owner and does not re-enter the former root binding's remaining frontier.

## Operation-specific selector boundary

The field path defined here exists only to identify the source subvalue produced by `FieldValueUse` and apply the canonical structural ownership requirement to it.

It does not establish:

- a general place or lvalue;
- field assignment or partial-field reinitialization;
- an independently mutable field binding;
- a source reference or borrow;
- address-taking, pointer provenance, or physical offsets;
- arbitrary value receivers;
- method, associated-item, extension, trait, or overload lookup; or
- record-pattern binding semantics.

`patterns.md` independently consumes nominal field identities, direct accessibility, and structural ownership for its own accepted operation. That reuse does not turn a record pattern into dot field access or this operation into a general member system.

## Concrete and implementation boundary

`concrete-syntax.md` owns the represented `.` token and exact binding-rooted field-value grammar. This document does not define parser recovery, syntax-tree nodes, diagnostics, HIR representation, Core field indices, or backend behavior.

A faithful implementation may resolve source field identities to internal field indices after source validation. Existing Core structural projections and `Copy`/`Move` are suitable refinement targets only after source validation has selected the root binding, path, accessibility, final-path availability, and duplicate-or-consume consequence.

Source structural ownership MUST NOT be reconstructed from Core path state, scalar liveness, or Core copyability. Zero-field and recursively zero-leaf source subvalues remain meaningful ownership even when a faithful lower operation has no scalar effect.

Cleanup of remaining source-owned subvalues is selected by `structural-ownership.md` and sequenced by `function-execution.md`.

## Further boundaries

This revision does not define field assignment or partial-field reinitialization; arbitrary value receivers; cross-module field access or field visibility modifiers; methods/associated items; references/borrowing/lifetimes; refutable/rest/shorthand patterns; positive record duplicability-selection syntax; general expressions/operators; floating literal formation; branches/loops/control-flow joins; custom destructors; const/static semantics; panic payload/catch syntax; ABI/layout/FFI/linkage; Exec/Model source forms; or runtime/backend representation.