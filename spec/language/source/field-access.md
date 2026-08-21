# Source Field-Value Access

Status: **provisional normative; incomplete**

This document owns the represented source semantics for binding-rooted field-path selection, direct field accessibility, final-field duplicability admissibility, and production of an owned field value.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module identity from [Source names and modules](names-modules.md), nominal record and field identity, source field types, source type equality, and owned-value duplicability from [Source type foundation](types.md), and function-local binding lookup and whole-binding availability from [Source function-local bindings](local-bindings.md). It does not redefine those owners.

The represented `.` spelling and bounded field-value grammar are owned by [Source concrete syntax](concrete-syntax.md). Transfer of a successfully produced value into a local, assignment RHS, direct-call argument, return result, or record-construction initializer is owned by [Source function execution](function-execution.md).

This document does not define a general member system, place/lvalue grammar, partial field move relation, reference/borrow operation, field mutation, physical layout, or implementation representation.

## Represented operation

A represented **field-value use** selects one field path rooted in exactly one parameter or ordinary local binding and, when the final selected field type is duplicable, produces one new owned value preserving that final field's source semantic value.

Conceptually, the represented concrete form is:

```text
root.field
root.outer.inner
```

The root is one unqualified function-body identifier. One or more field selectors follow it. The receiver is not an arbitrary source value or expression.

Field-value use is an owned-value producer. It is not a source place, lvalue, reference, borrow, storage identity, address, method receiver, or field-assignment target.

## Root binding selection

The root identifier uses the unqualified function-body lookup precedence owned by `local-bindings.md`.

The selected entity MUST be one active parameter or ordinary local binding. Lookup does not bypass an active function-local binding merely because another entity would be more suitable for field selection.

Only when no active parameter/local binding resolves the root key does the existing same-module fallback lookup occur. If that fallback selects a module declaration, the selected entity is the wrong category for field-value use and the operation is source-invalid. Imported modules are not searched implicitly and source-unit module aliases do not participate in this unqualified root lookup.

The selected root binding MUST be definitely **available** under `local-bindings.md` before field selection begins.

Field-value use does not introduce another whole-binding availability state.

## Field-path selection

Let the root binding have source type `T0`. Let the field selectors, in source order, have lexical keys `f0, f1, ... fn`.

For each selector `fi`:

1. the current source type `Ti` MUST be one nominal record source type under `types.md`;
2. the record declaration defining `Ti` MUST belong to the same source module as the function containing this field-value use;
3. `fi` selects exactly the unique declared field of that nominal record whose lexical field key equals `fi` under `lexical.md`;
4. the selected field's declared source type becomes `Ti+1`;
5. if another selector follows, selection continues from that source type.

If the current type is intrinsic rather than a nominal record, another selector is invalid.

If the current record has no field with the requested lexical key, the operation is source-invalid. Selection does not search another record, another module, methods, associated items, extensions, traits, or an outer namespace merely because the requested field is absent.

Field declaration order is not lookup priority. Field identity remains scoped by the containing nominal record declaration under `types.md`.

## Direct field accessibility

The represented concrete record declaration has no field-level accessibility modifier. For this direct field-value operation, each represented record field is **module-private**.

A field may be selected only when the record declaration containing that field belongs to the same source module as the function containing the field-value use.

This requirement applies independently at every selector step. Consequently, a path may select a field of a same-module record whose field type is a record defined in another module, but a later selector cannot enter that foreign record under this revision.

Module-level accessibility of the record type itself remains owned by `names-modules.md` and is independent of this field-accessibility rule. An exported record may therefore be nameable in another module while its fields remain unavailable to direct field-value use there.

This field accessibility has no ABI, linkage, layout, serialization, reflection, or confidentiality meaning.

This revision defines no public/exported/package/friend field modifier. A later accepted field-accessibility mechanism may broaden the direct-access domain without changing the same-module cases defined here.

## Final-field duplicability

Let `Tf` be the source type of the final selected field.

`Tf` MUST be duplicable under the source owned-value duplicability classification in `types.md`.

Intermediate field types need not be duplicable. Their values are not independently produced, consumed, or duplicated merely because the path passes through them.

When `Tf` is duplicable, successful field-value use:

1. produces one new owned source value of exactly type `Tf` preserving the selected final field's source semantic value through `Tf`'s accepted duplicability capability;
2. does not consume, move, replace, or mutate the final field;
3. does not consume or partially consume any containing record;
4. leaves the root binding definitely available; and
5. changes no other binding availability.

The operation does not define source equality or comparison. Duplicating the selected source semantic value does not imply bitwise copying, physical field loading, shared storage identity, physical representation equality, or a particular realization strategy.

## Non-duplicable final fields

If `Tf` is non-duplicable, the field-value use is source-invalid under this revision.

It MUST NOT implicitly:

- move or consume the selected field;
- consume the complete root value;
- make the root or one field partially unavailable;
- invoke a clone, conversion, or factory operation; or
- yield a defined runtime fault in place of source validation.

This is a bounded admissibility rule for the currently represented operation, not a permanent claim that the concrete field path can only ever denote non-consuming access.

A later accepted partial-field ownership relation MAY admit consuming a non-duplicable selected field using compatible concrete syntax after that relation defines structural source availability, disjoint-field use, whole-value use after extraction, reinitialization, cleanup, nested paths, and borrow/reference interaction. Such an extension MUST preserve the behavior of every duplicable field-value use defined here.

## Availability and mutability consequences

Because represented field-value use is non-consuming, successful evaluation leaves the complete root binding available under `local-bindings.md`.

Repeated field-value use of the same or another duplicable field is therefore valid whenever the root remains otherwise available.

A later ordinary whole-binding owned use continues to follow `local-bindings.md`: it may duplicate or consume the complete root value according to the root source type. Field-value use does not change that later rule.

Assignment mutability of the root binding is irrelevant to this operation because field-value use performs no assignment or replacement.

An unavailable root binding is source-invalid for field-value use. This remains a source-validation failure, not a runtime moved-state fault.

No field/member availability state is introduced.

## Evaluation behavior

After source validation, field-value use itself is effect-free, non-faulting, and non-diverging.

It performs no nested value-producer evaluation and creates no construction-like intermediate transient ownership. The complete field path is statically selected before the operation produces its one owned result.

The successful result may then be consumed by any currently represented receiving value context under `function-execution.md`.

## Required-type composition

A successful field-value use has exactly the source type of its final selected field.

When the surrounding value consumer requires a source type, the field-value result type MUST equal that required type exactly under `types.md`.

The operation introduces no inference, structural compatibility, subtyping, conversion, coercion, promotion, widening, narrowing, or numeric defaulting.

The represented producer may therefore compose with:

- ordinary local initialization;
- whole-binding assignment RHS evaluation;
- direct-call arguments;
- result-bearing return; and
- record-construction field initializers.

Those receiving operations retain their existing ordering, transfer, replacement, cleanup, fault, and divergence authority under `function-execution.md`.

## Operation-specific selector boundary

The represented field path exists only to identify the source subvalue duplicated by this operation.

It does not establish:

- a general place or lvalue;
- field assignment or interior mutation;
- an independently mutable field binding;
- a source reference or borrow;
- address-taking, pointer provenance, or physical offsets;
- arbitrary value receivers;
- method, associated-item, extension, trait, or overload lookup;
- destructuring or pattern binding.

A later operation may consume compatible syntax only through its own accepted semantic owner.

## Concrete and implementation boundary

`concrete-syntax.md` owns the represented `.` token and the exact binding-rooted field-value grammar. This document does not define parser recovery, syntax-tree nodes, diagnostics, HIR representation, Core field indices, or backend behavior.

A faithful implementation may resolve the accepted source field identities to implementation field indices for lowering, but those indices are not source semantic identity.

Existing Core structural place projection and `Copy` capability are suitable implementation targets only after source validation has selected the root binding, exact field path, and final duplicability under this document. Core place/path state remains implementation/proving authority and MUST NOT be imported as a source partial-availability model.

## Further boundaries

This revision does not define moving or consuming a non-duplicable field; partial field/member availability; field assignment; arbitrary value receivers; cross-module field access; field visibility modifiers; methods or associated items; references, borrowing, or lifetimes; patterns or destructuring; positive record duplicability-selection syntax; general expressions or operators; floating literal formation; branches/loops or control-flow joins; const/static semantics; panic payload/catch syntax; ABI/layout/FFI/linkage; Exec/Model source forms; or runtime/backend representation.
