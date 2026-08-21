# Source Field-Value Access

Status: **provisional normative; incomplete**

This document owns the represented source semantics for binding-rooted field-path selection, direct field accessibility, final-path structural availability, final-field duplicate-or-consume ownership behavior, and production of one owned field value.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module identity from [Source names and modules](names-modules.md), nominal record and field identity, source field types, source type equality, and owned-value duplicability from [Source type foundation](types.md), and function-local binding lookup, structural source paths, and structural availability from [Source function-local bindings](local-bindings.md). It does not redefine those owners.

The represented `.` spelling and bounded field-value grammar are owned by [Source concrete syntax](concrete-syntax.md). Transfer of a successfully produced value into a local, assignment RHS, direct-call argument, return result, or record-construction initializer is owned by [Source function execution](function-execution.md).

This document does not define a general member system, place/lvalue grammar, field assignment, partial-field reinitialization, reference/borrow operation, physical layout, or implementation representation.

## Represented operation

A represented **field-value use** selects one non-empty structural field path rooted in exactly one parameter or ordinary local binding and produces one owned value of the final selected field type.

Conceptually, the represented concrete form is:

```text
root.field
root.outer.inner
```

The root is one unqualified function-body identifier. One or more field selectors follow it. The receiver is not an arbitrary source value or expression.

Field-value use is an owned-value producer. It is not a source place, lvalue, reference, borrow, storage identity, address, method receiver, or field-assignment target.

After static path selection and structural-availability validation, the final selected field's owned-value duplicability determines whether production duplicates that subvalue or consumes/transfers it exactly once.

## Root binding selection

The root identifier uses the unqualified function-body lookup precedence owned by `local-bindings.md`.

The selected entity MUST be one active parameter or ordinary local binding. Lookup does not bypass an active function-local binding merely because another entity would be more suitable for field selection.

Only when no active parameter/local binding resolves the root key does the existing same-module fallback lookup occur. If that fallback selects a module declaration, the selected entity is the wrong category for field-value use and the operation is source-invalid. Imported modules are not searched implicitly and source-unit module aliases do not participate in this unqualified root lookup.

The complete root binding value need not be fully available merely to perform static field-path selection. A partially available root may still contain fully available disjoint descendants. The final selected path's structural availability is validated only after the complete field path has been resolved as defined below.

An unavailable complete root cannot contain a fully available descendant, so every field-value path rooted there is invalid by the final-path availability rule without requiring a separate whole-root check.

## Field-path selection

Let the root binding have source type `T0`. Let the field selectors, in source order, have lexical keys `f0, f1, ... fn`.

For each selector `fi`:

1. the current source type `Ti` MUST be one nominal record source type under `types.md`;
2. the record declaration defining `Ti` MUST belong to the same source module as the function containing this field-value use;
3. `fi` selects exactly the unique declared field of that nominal record whose lexical field key equals `fi` under `lexical.md`;
4. the selected field's declared source type becomes `Ti+1`;
5. the selected source field identity extends the root binding's structural source path under `local-bindings.md`; and
6. if another selector follows, selection continues from that source type.

If the current type is intrinsic rather than a nominal record, another selector is invalid.

If the current record has no field with the requested lexical key, the operation is source-invalid. Selection does not search another record, another module, methods, associated items, extensions, traits, or an outer namespace merely because the requested field is absent.

Field declaration order is not lookup priority. Field identity remains scoped by the containing nominal record declaration under `types.md`.

Static selection through a partially available intermediate record path is permitted. Selection itself neither observes nor recreates a complete value for that intermediate path. The operation becomes source-valid only if the final selected path passes the structural-availability requirement below.

## Direct field accessibility

The represented concrete record declaration has no field-level accessibility modifier. For this direct field-value operation, each represented record field is **module-private**.

A field may be selected only when the record declaration containing that field belongs to the same source module as the function containing the field-value use.

This requirement applies independently at every selector step. Consequently, a path may select a field of a same-module record whose field type is a record defined in another module, but a later selector cannot enter that foreign record under this revision.

Module-level accessibility of the record type itself remains owned by `names-modules.md` and is independent of this field-accessibility rule. An exported record may therefore be nameable in another module while its fields remain unavailable to direct field-value use there.

This field accessibility has no ABI, linkage, layout, serialization, reflection, or confidentiality meaning.

This revision defines no public/exported/package/friend field modifier. A later accepted field-accessibility mechanism may broaden the direct-access domain without changing the same-module cases defined here.

## Final-path structural availability

Let `p` be the complete non-empty structural source path selected from the root binding, and let `Tf` be the source type of its final selected field.

The final path `p` MUST be **fully available** under the structural availability relation in `local-bindings.md` immediately before field-value production.

Therefore the operation is source-invalid when:

- `p` was already consumed;
- an ancestor of `p` was already consumed; or
- any descendant of `p` was already consumed, making the complete value at `p` only partially available.

A consumed path that is structurally disjoint from `p` does not prevent use of `p`.

In particular, a partially available root or intermediate record may still be traversed to a final path that is fully available when prior consumption occurred only in a disjoint branch.

Failure of this requirement is a source-validation failure. It is not a defined runtime `Fault` and MUST NOT be deferred to a physical moved-state check.

## Duplicable final fields

When `Tf` is duplicable under the source owned-value duplicability classification in `types.md`, successful field-value use:

1. produces one new owned source value of exactly type `Tf` preserving the selected final field's source semantic value through `Tf`'s accepted duplicability capability;
2. does not consume, move, replace, or mutate the final selected subvalue;
3. leaves the root binding's consumed-path set unchanged; and
4. changes no other binding's structural availability.

Intermediate field types need not be duplicable. Their complete values are not independently produced, consumed, or duplicated merely because static selection passes through them.

This preserves the previously represented non-consuming field-value behavior exactly for every source-valid duplicable final field.

## Non-duplicable final fields

When `Tf` is non-duplicable, successful field-value use:

1. produces the complete owned source value at final path `p`, of exactly type `Tf`;
2. transfers/consumes that selected source subvalue exactly once;
3. records `p` as consumed under `local-bindings.md`;
4. does not consume, duplicate, replace, or mutate any structurally disjoint sibling subvalue; and
5. does not implicitly consume the complete root or any proper ancestor value as a separate ownership operation.

After that transition:

- the path `p` and every descendant path are unavailable;
- every proper ancestor of `p`, including the complete root, is partially available unless a separate accepted transition later replaces the complete binding;
- fully available structurally disjoint paths remain usable; and
- ordinary whole-binding use of the root or another partially available ancestor is invalid because its complete value is not fully available.

The produced value is transferred ownership, not a clone or conversion. The operation introduces no source equality, comparison, custom clone, factory, or runtime moved-state fault.

## Repeated, ancestor, descendant, and disjoint use

Structural availability yields the following consequences without a separate field-use state machine:

- repeated field-value use of one consumed non-duplicable path is invalid;
- consuming a descendant makes later use of the complete ancestor value invalid while that ancestor is only partially available;
- consuming an ancestor makes every descendant path unavailable;
- consuming one field does not invalidate a structurally disjoint sibling path;
- a duplicable sibling may still be duplicated after another sibling was consumed;
- a non-duplicable sibling may still be consumed after another sibling was consumed; and
- nested access may pass through a partially available intermediate record to an untouched disjoint descendant whose complete path remains fully available.

These are source ownership consequences. They do not introduce member-level mutation or make structural paths independently assignable bindings.

## Availability and mutability consequences

Field-value use consumes the structural availability relation owned by `local-bindings.md`; it does not define a second availability domain.

Assignment mutability of the root binding is irrelevant to whether a field subvalue may be consumed. An immutable binding may become partially available or unavailable through permitted owned-value consumption. Immutability restricts whole-binding assignment/reinitialization, not ownership transfer.

A mutable partially available binding may later be replaced as a complete binding under the assignment relation in `local-bindings.md` and the source-first execution ordering in `function-execution.md`.

This operation itself performs no assignment or reinitialization.

## Evaluation behavior

After source validation, field-value use itself is non-faulting and non-diverging.

It performs no nested value-producer evaluation and creates no construction-like intermediate transient ownership. The complete field path is statically selected before the operation produces its one owned result.

For a duplicable final field, production is non-consuming and effect-free with respect to source ownership state. For a non-duplicable final field, production performs exactly the structural ownership transfer defined above. That source ownership transition occurs when the producer successfully produces its result and therefore precedes transfer of that result into an enclosing local, argument transient, construction transient, assignment RHS transient, or return result.

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

Those receiving operations retain their existing ordering, transfer, replacement, cleanup, fault, and divergence authority under `function-execution.md`. When a non-duplicable field-value producer has consumed its selected path before a later producer faults, that ownership transition remains effective and the transferred value is cleaned by its then-current owner rather than by the former root binding.

## Operation-specific selector boundary

The represented field path exists only to identify the source subvalue produced by this operation and to apply structural availability to that selected subvalue.

It does not establish:

- a general place or lvalue;
- field assignment or partial-field reinitialization;
- an independently mutable field binding;
- a source reference or borrow;
- address-taking, pointer provenance, or physical offsets;
- arbitrary value receivers;
- method, associated-item, extension, trait, or overload lookup;
- destructuring or pattern binding.

A later operation may consume compatible syntax only through its own accepted semantic owner.

## Concrete and implementation boundary

`concrete-syntax.md` owns the represented `.` token and the exact binding-rooted field-value grammar. This document does not define parser recovery, syntax-tree nodes, diagnostics, HIR representation, Core field indices, or backend behavior.

A faithful implementation may resolve accepted source field identities to implementation field indices after source validation, but those indices are not source semantic identity.

Existing Core structural place projection and `Copy`/`Move` capabilities are suitable implementation targets only after source validation has selected the root binding, exact structural field path, final-path availability, and duplicate-or-consume ownership consequence under this document and `local-bindings.md`.

Source structural availability MUST NOT be reconstructed from Core place/path state, scalar liveness, or a Core destruction domain. In particular, zero-field or recursively zero-leaf source subvalues remain meaningful source ownership even where consuming them has no lower scalar-liveness transition.

Cleanup of remaining source-owned subvalues after partial consumption is owned by `local-bindings.md` and `function-execution.md`, not by this field-value owner.

## Further boundaries

This revision does not define field assignment or partial-field reinitialization; arbitrary value receivers; cross-module field access; field visibility modifiers; methods or associated items; references, borrowing, or lifetimes; patterns or destructuring; positive record duplicability-selection syntax; general expressions or operators; floating literal formation; branches/loops or control-flow joins; custom destructors; const/static semantics; panic payload/catch syntax; ABI/layout/FFI/linkage; Exec/Model source forms; or runtime/backend representation.
