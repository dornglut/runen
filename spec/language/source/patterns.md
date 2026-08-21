# Source Patterns

Status: **provisional normative; incomplete**

This document owns the represented source semantics for the first irrefutable record-pattern category: same-module nominal record pattern selection, exhaustive named field-to-binding mapping, pattern binding production order, and pattern-specific duplicate-or-consume ownership consequences.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), same-module record declaration lookup from [Source names and modules](names-modules.md), nominal record and field identity, exact source type equality, field source types, and owned-value duplicability from [Source type foundation](types.md), function-local binding lookup, binding identity, lexical scope, assignment mutability, structural source paths, and structural availability from [Source function-local bindings](local-bindings.md), and direct same-module record-field accessibility from [Source field-value access](field-access.md). It does not redefine those owners.

The represented concrete spelling is owned by [Source concrete syntax](concrete-syntax.md). Straight-line declaration completion and later lexical/activation cleanup composition are owned by [Source function execution](function-execution.md).

This document does not define general patterns, arbitrary scrutinee expressions, refutable matching, destructuring assignment, references or borrow binding modes, a general source place/lvalue abstraction, or an implementation representation.

## Represented record-destructuring declaration

The first represented pattern operation is one binding-rooted, irrefutable, exhaustive record-destructuring declaration.

Conceptually:

```text
let RecordName {
    field_a: binding_a,
    field_b: binding_b,
} = root;
```

The operation has exactly:

- one nominal record-pattern head;
- one finite source-ordered sequence of explicit field-to-binding entries;
- one existing root parameter/local binding as scrutinee; and
- zero or more newly introduced ordinary local bindings.

The pattern is one level only. Every pattern entry selects one direct field of the record head and introduces one binding for that selected field value. The right side of a pattern entry is not another pattern in this revision.

The operation is **irrefutable** because source validation requires the root to have exactly the selected nominal record type and the pattern to name every declared field exactly once. Successful execution therefore performs no dynamic shape test and has no mismatch outcome.

The operation is not an ordinary owned-value use of the complete root. It does not first duplicate or consume a whole record value and then unpack a temporary. It applies the structural field ownership relation defined below directly to the existing root binding.

## Pattern-head selection

The record-pattern head is one unqualified `UserIdentifier` in the concrete form.

It resolves directly through the same-module declaration namespace owned by `names-modules.md`. It MUST select one nominal record declaration. A selected function or another wrong-category declaration is invalid for this pattern and MUST NOT be bypassed to search another entity.

Function-local value bindings do not participate in pattern-head lookup. A local binding whose lexical key equals the record-pattern head therefore does not change which same-module record declaration the head denotes.

Qualified or cross-module pattern heads are not represented by this revision.

## Root binding selection and exact type

The scrutinee root is one unqualified function-body identifier.

It resolves through the function-local value-binding lookup precedence owned by `local-bindings.md` and MUST select one active parameter or ordinary local binding. Lookup MUST NOT bypass an active local binding merely because a module declaration would be a more convenient scrutinee category.

Only when no active parameter/local binding resolves the key does the existing same-module fallback lookup occur. A selected module declaration is then the wrong category for a root scrutinee and the declaration is source-invalid.

The selected root binding's declared source type MUST be exactly the nominal record source type denoted by the pattern head under `types.md`. Equal field keys, equal field types, or equal structural shape from another nominal record do not satisfy this requirement.

The root binding does not need a separate whole-value-fully-available precondition merely to resolve the pattern. Each selected field path instead obeys the canonical structural availability requirement defined below.

## Direct record-field accessibility

Every field selected by the represented record pattern MUST satisfy the direct record-field accessibility relation owned by `field-access.md`.

For the current record declaration form this means the containing nominal record declaration belongs to the same source module as the function containing the pattern.

An exported record type may therefore be nameable in another source module while its fields remain unavailable to direct record-pattern binding under this revision.

Pattern accessibility does not define ABI visibility, linkage, physical layout, serialization, reflection, confidentiality, or a new field-level visibility modifier.

## Complete pattern structure validation

Before the declaration may apply any ownership transition or establish any new binding, source validation resolves and validates the **complete** pattern structure.

Let the selected nominal record have declaration fields `F`. Let the pattern entries, in concrete source order, select field keys and introduce binding keys.

The declaration is structurally valid only when all of the following hold:

1. every pattern field key resolves to exactly one field identity in `F`;
2. no record field identity occurs more than once in the pattern;
3. every field identity in `F` occurs exactly once in the pattern;
4. no introduced binding lexical key occurs more than once in the pattern;
5. no introduced binding key would violate the overlapping function-local shadowing prohibition in `local-bindings.md` at the declaration point;
6. the root binding and exact root type requirements above hold; and
7. every selected field is directly accessible as required above.

An unknown field key, duplicate field, missing field, duplicate introduced binding key, overlapping-shadow conflict, wrong pattern-head category, wrong root category, wrong nominal root type, or inaccessible field makes the declaration source-invalid.

A declaration rejected by this complete structural validation:

- establishes no pattern-introduced binding;
- applies no duplicate or consume operation to any root field path; and
- leaves all pre-existing binding structural-availability facts unchanged by the rejected pattern itself.

Pattern field presentation order is not field lookup priority. The selected nominal record declaration continues to own field identity and structural field order.

## Pattern-introduced bindings

Every valid pattern entry introduces one ordinary function-local binding.

For a pattern entry selecting record field `f` and naming new binding key `b`:

- the new binding has one stable source-semantic binding identity under `local-bindings.md`;
- its lexical key is `b`;
- its declared source type is exactly the declared source type of `f`;
- its assignment-mutability classification is **immutable** in this revision; and
- its initial owned value is produced by the pattern ownership operation below.

All bindings introduced by one destructuring declaration enter scope together only after the **complete declaration** finishes successfully. None of those new bindings participates in lookup while the pattern head, root, field structure, accessibility, structural availability, or ownership consequences of the same declaration are being validated or applied.

The concrete source order of pattern entries is the source declaration order of the introduced bindings for lexical ordering and cleanup composition. If the source writes fields in an order different from record declaration order, that pattern source order still controls the order of the new binding declarations.

This source declaration order does not alter the nominal record's structural field order.

## Pattern structural ownership

After the complete pattern has passed all non-ownership structural/type/accessibility/name checks above, process its pattern entries strictly in pattern source order.

For an entry selecting direct record field identity `f`, let `p = [f]` be the corresponding one-field structural source path rooted at the scrutinee binding, and let `T` be `f`'s declared source type.

The entry is source-valid only when `p` is **fully available** immediately before that entry's ownership production under `local-bindings.md`.

If `T` is duplicable under `types.md`:

1. produce one new owned source value of exactly type `T` using the accepted non-consuming duplicability capability;
2. use that produced value as the initial owned value of the entry's new binding; and
3. leave the root binding's consumed-path set unchanged.

If `T` is non-duplicable:

1. produce the complete owned source value at path `p`, of exactly type `T`;
2. transfer/consume exactly that selected field subvalue once;
3. record `p` as consumed under `local-bindings.md`; and
4. use the transferred value as the initial owned value of the entry's new binding.

No proper ancestor of `p` is independently duplicated or consumed by this operation. Structurally disjoint sibling paths remain governed solely by their own current structural availability.

Because the pattern is exhaustive but field ownership is determined independently per selected field type, exhaustiveness does **not** imply an implicit whole-root consume. In particular, separately consuming every top-level non-duplicable field may leave the root with an empty remaining ownership frontier while the empty path itself was never consumed.

The pattern owner consumes, rather than redefines, the structural availability relation and owned-value duplicability capability.

## Availability consequences

The accepted structural availability relation yields the pattern consequences directly:

- a duplicable selected field leaves its path and root availability unchanged;
- a consumed non-duplicable selected field becomes unavailable and may make its proper ancestors, including the complete root, partially available;
- a consumed path does not invalidate a disjoint sibling field path;
- a later pattern entry selecting a disjoint fully available field remains valid;
- if a selected path is unavailable or partially available when its entry is processed, the declaration is source-invalid under the final-path requirement;
- an all-duplicable pattern leaves the complete root fully available;
- a pattern that consumes at least one top-level field normally leaves the complete root partially available unless a prior accepted transition already made a selected path invalid; and
- later whole-binding use, disjoint field-value use, assignment/reinitialization, and cleanup follow `local-bindings.md`, `field-access.md`, and `function-execution.md` without a pattern-specific second state machine.

An immutable root binding may legally lose ownership of non-duplicable field subvalues through a source-valid pattern. Assignment mutability restricts later assignment/reinitialization and is independent of owned-value transfer.

A mutable partially available root may later be whole-replaced by the existing assignment relation; successful replacement resets its consumed-path set exactly as already defined.

## Zero-field records

For a zero-field nominal record `Empty`, the exhaustive represented pattern is:

```text
let Empty {} = root;
```

When the pattern head/root category and exact type requirements hold, this declaration is valid.

It contains no pattern entry, introduces no local binding, duplicates no value, consumes no structural path, and leaves the root consumed-path set unchanged.

The empty exhaustive pattern is therefore an ownership no-op. It is not an implicit discard and does not consume the empty root path merely because the record has no fields.

## Zero-leaf field ownership

A selected non-duplicable field whose source type is a zero-field or recursively zero-leaf record is still a source-owned value.

Consuming that field through the represented pattern records its one-field source structural path as consumed and transfers source ownership to the new pattern binding exactly as for another non-duplicable field.

This ownership transition remains meaningful even when a faithful lower structural storage model has no scalar leaf whose liveness changes and no physical destruction operation to emit later.

Pattern validity and ownership MUST NOT be defined through lower scalar-leaf existence.

## Evaluation, fault, and divergence

The represented scrutinee is an already-existing binding. The represented pattern entries contain no nested source value producers.

After successful source validation, pattern ownership production itself is non-faulting and non-diverging. It performs the finite source-ordered duplicate/consume transfers defined above and then establishes all pattern-introduced bindings.

A later body statement begins only after that complete declaration finishes successfully under `function-execution.md`.

Because this first pattern has no arbitrary owned-value scrutinee, it introduces no call/constructor scrutinee staging, pattern transient ownership, producer fault ordering, or divergence interaction.

A later pattern revision may admit an arbitrary owned-value producer as scrutinee only by defining that additional evaluation/ownership boundary without changing the binding-rooted behavior accepted here.

## Relation to field-value access

The record pattern and `FieldValueUse` are distinct source operations.

They may consume the same nominal field identities, same-module direct field-accessibility relation, source structural paths, structural availability, and type duplicability capability, but:

- record-pattern syntax is not dot field-value syntax;
- the pattern establishes new lexical bindings;
- `FieldValueUse` produces one owned value for an enclosing value consumer;
- this pattern produces zero or more initial binding values as one grouped declaration; and
- neither operation creates a general place/lvalue/member abstraction merely because both identify record fields.

## Concrete and implementation boundary

`concrete-syntax.md` owns the exact represented spelling and parser-level distinction from ordinary local declarations. This document does not define syntax-tree node kinds, parser recovery, HIR structs, Core field indices, or backend representation.

A faithful typed HIR must retain the already-resolved source decisions strongly enough that lowering does not repeat source pattern semantics. At minimum the semantic information includes:

- resolved nominal record identity;
- resolved root binding identity;
- resolved field identity for every pattern entry;
- every new binding identity, lexical key, and exact source type;
- pattern source order; and
- the source-selected duplicate-or-consume ownership consequence of each entry.

A faithful lowering may refine each pattern entry in pattern source order to initialization of the mapped new source local from the mapped root's projected field:

- duplicating entry -> existing projected Core `Copy`;
- consuming entry -> existing projected Core `Move`.

The mapped pattern bindings remain ordinary source locals and must participate in the existing lexical source-local declaration/cleanup ordering. Existing Core projected `Init`/`Copy`/`Move` semantics are refinement targets only after source validation.

Core place/path state, scalar liveness, local numeric identifiers, field projection indices, and destruction domains are not source pattern authority. No new Core normative operation is required by this represented pattern.

## Further boundaries

This revision does not define:

- shorthand field binding;
- wildcard/ignore patterns;
- rest or omitted fields;
- partial/non-exhaustive record patterns;
- nested record patterns;
- tuple, array, enum, variant, scalar-literal, range, or other pattern categories;
- refutable patterns, `match`, guards, conditionals, loops, or control-flow joins;
- arbitrary value/call/constructor scrutinees;
- destructuring assignment;
- mutable pattern-binding modifiers;
- qualified/cross-module record patterns or broader field visibility;
- references, borrow binding modes, lifetime syntax, or source `unsafe`;
- positive record duplicability-selection syntax, traits, generics, or coherence;
- closures/function values;
- floating literals, operators, conversions, or general expressions;
- custom destruction, panic/catch payload forms, const/static semantics, ABI/layout/FFI/linkage, Exec/Model source forms, or runtime/backend representation.

A later pattern feature must extend this accepted relation explicitly rather than silently reinterpret the binding-rooted exhaustive form.