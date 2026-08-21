# Source Patterns

Status: **provisional normative; incomplete**

This document owns the represented source semantics for the first irrefutable record-pattern category: same-module nominal record-pattern selection, exhaustive named field-to-binding mapping, scrutinee-category selection, pattern binding production order, direct binding-root structural ownership consequences, and producer-backed transient field ownership and remaining-cleanup selection.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), same-module record declaration lookup from [Source names and modules](names-modules.md), nominal record and field identity, exact source type equality, field source types, and owned-value duplicability from [Source type foundation](types.md), function-local binding lookup, binding identity, lexical scope, assignment mutability, structural source paths, and structural availability from [Source function-local bindings](local-bindings.md), and direct same-module record-field accessibility from [Source field-value access](field-access.md). It consumes represented owned-value producer evaluation, transient producer ownership, cleanup, fault propagation, divergence, and straight-line declaration completion from [Source function execution](function-execution.md). It does not redefine those owners.

The represented concrete spelling and syntactic distinction between direct binding-root and producer-backed scrutinees are owned by [Source concrete syntax](concrete-syntax.md). Later lexical/activation cleanup composition is owned by `function-execution.md`.

This document does not define general patterns, arbitrary general expressions, refutable matching, destructuring assignment, references or borrow binding modes, a general source place/lvalue abstraction, or an implementation representation.

## Represented record-destructuring declaration

The first represented pattern operation is one one-level, irrefutable, exhaustive record-destructuring declaration.

The accepted direct binding-root form is conceptually:

```text
let RecordName {
    field_a: binding_a,
    field_b: binding_b,
} = root;
```

The represented producer-backed extension may instead use one syntactically non-bare record-valued producer:

```text
let RecordName {
    field_a: binding_a,
    field_b: binding_b,
} = make_record();
```

Every represented declaration has exactly:

- one nominal record-pattern head;
- one finite source-ordered sequence of explicit field-to-binding entries;
- one scrutinee from exactly one of the two scrutinee categories below; and
- zero or more newly introduced ordinary local bindings.

The pattern is one level only. Every pattern entry selects one direct field of the record head and introduces one binding for that selected field value. The right side of a pattern entry is not another pattern in this revision.

The operation is **irrefutable** because source validation requires the scrutinee to have exactly the selected nominal record type and the pattern to name every declared field exactly once. Successful pattern ownership production therefore performs no dynamic shape test and has no mismatch outcome.

A direct binding-root scrutinee is not an ordinary owned-value use of the complete root. It does not first duplicate or consume a whole record value and then unpack a temporary. It applies the direct structural field ownership relation below to the existing root binding exactly as accepted before producer-backed scrutinees were introduced.

A producer-backed scrutinee is different: its already-represented producer first yields one owned transient record value under `function-execution.md`. Pattern field ownership is then applied to that transient without creating a source binding for it.

## Pattern-head selection

The record-pattern head is one unqualified `UserIdentifier` in the concrete form.

It resolves directly through the same-module declaration namespace owned by `names-modules.md`. It MUST select one nominal record declaration. A selected function or another wrong-category declaration is invalid for this pattern and MUST NOT be bypassed to search another entity.

Function-local value bindings do not participate in pattern-head lookup. A local binding whose lexical key equals the record-pattern head therefore does not change which same-module record declaration the head denotes.

Qualified or cross-module pattern heads are not represented by this revision.

## Scrutinee categories and exact type

The represented record-destructuring declaration has exactly two scrutinee categories selected by concrete syntax. Source semantic validation MUST preserve that category; it MUST NOT reinterpret one category as the other merely because both ultimately provide a value of the selected record type.

### Direct binding-root scrutinee

A direct binding-root scrutinee is exactly one bare unqualified function-body identifier under `concrete-syntax.md`.

It resolves through the function-local value-binding lookup precedence owned by `local-bindings.md` and MUST select one active parameter or ordinary local binding. Lookup MUST NOT bypass an active local binding merely because a module declaration would be a more convenient scrutinee category.

Only when no active parameter/local binding resolves the key does the existing same-module fallback lookup occur. A selected module declaration is then the wrong category for a direct root scrutinee and the declaration is source-invalid.

The selected root binding's declared source type MUST be exactly the nominal record source type denoted by the pattern head under `types.md`. Equal field keys, equal field types, or equal structural shape from another nominal record do not satisfy this requirement.

The root binding does not need a separate whole-value-fully-available precondition merely to resolve the pattern. Each selected field path instead obeys the canonical structural availability requirement below.

A bare root is **not** passed through ordinary `IdentifierUse` owned-value production for this declaration. In particular, a non-duplicable root is not consumed as a whole merely because the same lexical spelling could denote `IdentifierUse` in another concrete `Value` position.

### Producer-backed transient scrutinee

A producer-backed scrutinee is exactly one syntactically non-bare producer admitted for this pattern by `concrete-syntax.md`:

- a result-bearing direct call;
- a record construction; or
- a binding-rooted field-value use.

The nominal record type selected by the pattern head is the producer's exact required source type. The producer MUST successfully produce exactly that nominal record type under its existing owner. Structural similarity to another record type does not satisfy this requirement.

The producer is resolved and evaluated in the lexical environment that exists before any binding introduced by this pattern enters scope. Pattern-introduced keys therefore do not participate in producer lookup.

A successful producer yields exactly one fully owned **pattern scrutinee transient** of the selected nominal record type. This transient:

- has no source binding identity or lexical key;
- is not source-addressable;
- does not participate in function-local lookup;
- is not an ordinary local or parameter binding; and
- exists only until this producer-backed record-destructuring declaration completes.

The transient begins with complete ownership of the produced record value. It has no pre-existing binding consumed-path state whose availability must be checked before pattern field production.

Boolean/integer literals are not producer-backed record scrutinee forms in this revision because their accepted source types are intrinsic rather than nominal record types. This restriction does not define a general expression taxonomy.

## Direct record-field accessibility

Every field selected by the represented record pattern MUST satisfy the direct record-field accessibility relation owned by `field-access.md`.

For the current record declaration form this means the containing nominal record declaration belongs to the same source module as the function containing the pattern.

An exported record type may therefore be nameable in another source module while its fields remain unavailable to direct record-pattern binding under this revision.

Pattern accessibility does not define ABI visibility, linkage, physical layout, serialization, reflection, confidentiality, or a new field-level visibility modifier.

## Complete pattern validation before scrutinee ownership consequences

Before a represented declaration may apply pattern-owned field production or establish any new binding, source validation resolves and validates the **complete pattern structure**.

Let the selected nominal record have declaration fields `F`. Let the pattern entries, in concrete source order, select field keys and introduce binding keys. For each resolved direct field identity `f`, let `[f]` denote the corresponding one-field structural source path when the scrutinee category has a binding root.

The common pattern structure is source-valid only when all of the following hold before any pattern-owned field production:

1. every pattern field key resolves to exactly one field identity in `F`;
2. no record field identity occurs more than once in the pattern;
3. every field identity in `F` occurs exactly once in the pattern;
4. no introduced binding lexical key occurs more than once in the pattern;
5. no introduced binding key would violate the overlapping function-local shadowing prohibition in `local-bindings.md` at the declaration point;
6. the scrutinee category satisfies its exact nominal record type requirement; and
7. every selected field is directly accessible as required above.

A direct binding-root declaration additionally requires, before any pattern ownership transition:

8. every selected one-field path `[f]` is **fully available** under `local-bindings.md` in the root's pre-pattern structural-availability state.

The direct selected top-level field paths are pairwise structurally disjoint because the pattern is exhaustive and each record field identity appears exactly once. Therefore validating all of their availability against the one pre-pattern state is equivalent, for every source-valid direct-root declaration, to requiring each path to remain fully available before its later source-ordered ownership production. This prevalidation avoids giving a rejected direct-root pattern any partial ownership consequence.

A producer-backed transient begins fully owned only after its producer has successfully completed, so it requires no pre-existing selected-path availability check. Pattern structure and introduced-binding validity are nevertheless established before the declaration enters the producer execution relation. A declaration with an invalid pattern structure therefore has no source execution that evaluates its producer merely to discover a later pattern failure.

An unknown field key, duplicate field, missing field, duplicate introduced binding key, overlapping-shadow conflict, wrong pattern-head category, invalid scrutinee category, wrong nominal scrutinee type, inaccessible field, or—where applicable—unavailable/partially available direct-root selected field path makes the declaration source-invalid.

A rejected declaration establishes no pattern-introduced binding and applies no pattern-owned duplicate or consume operation. For the direct-root category it leaves all pre-existing binding structural-availability facts unchanged by the rejected pattern itself.

Pattern field presentation order is not field lookup priority. The selected nominal record declaration continues to own field identity and structural field order.

## Pattern-introduced bindings

Every valid pattern entry introduces one ordinary function-local binding.

For a pattern entry selecting record field `f` and naming new binding key `b`:

- the new binding has one stable source-semantic binding identity under `local-bindings.md`;
- its lexical key is `b`;
- its declared source type is exactly the declared source type of `f`;
- its assignment-mutability classification is **immutable** in this revision; and
- its initial owned value is produced by the applicable pattern field ownership operation below.

All bindings introduced by one destructuring declaration enter scope together only after the **complete declaration** finishes successfully. None participates in lookup while the pattern structure or scrutinee is being resolved/evaluated, while direct-root availability is being validated, while pattern field values are being produced, or while a producer-backed scrutinee transient is being completed.

The concrete source order of pattern entries is the source declaration order of the introduced bindings for lexical ordering and cleanup composition. If the source writes fields in an order different from record declaration order, that pattern source order still controls the order of the new binding declarations.

This source declaration order does not alter the nominal record's structural field order.

## Direct binding-root structural ownership

After the complete direct-root pattern, including every selected field's structural availability, has passed validation, process its pattern entries strictly in pattern source order.

For an entry selecting direct record field identity `f`, let `p = [f]` be the corresponding one-field structural source path rooted at the selected binding, and let `T` be `f`'s declared source type.

Because every selected path was fully available in the pre-pattern state and the selected direct paths are pairwise disjoint, ownership transitions performed for earlier entries cannot invalidate the current entry's path.

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

The direct-root pattern consumes, rather than redefines, the structural availability relation and owned-value duplicability capability.

## Producer-backed transient field ownership

After the complete pattern structure is valid and the producer has successfully yielded its fully owned pattern scrutinee transient, process the same pattern entries strictly in pattern source order.

For each entry selecting direct field identity `f` of declared source type `T`:

- if `T` is duplicable, produce one owned value of type `T` using its accepted non-consuming duplicability capability, use that produced value as the corresponding new binding's initial value, and retain complete ownership of field `f` in the transient;
- if `T` is non-duplicable, transfer exactly the complete direct field value `f` from the transient into the corresponding new binding and cease transient ownership of that direct field.

The transient starts with every direct field wholly owned, and this one-level exhaustive pattern consumes only complete direct fields. Earlier field transfers therefore cannot make a distinct later direct field partially available. Pattern field production itself remains finite, non-faulting, and non-diverging after successful producer evaluation.

No whole transient record value is independently duplicated or consumed by pattern field production. In particular, transferring every non-duplicable direct field does not synthesize a whole-root consume.

## Producer-backed transient remaining cleanup

After every pattern field value has been produced, the pattern scrutinee transient ends before the declaration finishes and before the next body statement may begin.

Because the represented pattern is one-level and exhaustive, its transient remaining ownership has this bounded deterministic frontier:

1. if no direct field was consumed—including a zero-field record—the frontier contains exactly the complete transient record value;
2. if at least one direct field was consumed, the frontier contains exactly the complete direct fields still owned by the transient, selected in **reverse record declaration order**;
3. consequently, if every direct field was consumed, the frontier is empty.

For the second case, the still-owned direct fields are exactly those whose pattern entries used `Duplicate`. Pattern source order does not replace record structural order for this cleanup selection.

This bounded frontier is consistent with the accepted structural remaining-ownership behavior for a value whose only consumed paths are distinct direct fields, but it does not generalize `local-bindings.md`, make the transient a binding, or define nested-pattern ownership.

A retained direct field may itself be a zero-field or recursively zero-leaf record value. It remains one complete owned frontier member even when lower scalar destruction has no physical effect. Likewise, the complete transient of a zero-field record is one owned source value whose ownership ends here despite containing no scalar leaf.

`function-execution.md` owns execution of this selected transient cleanup frontier. No later lexical-scope or activation cleanup owns the transient after this completion boundary.

## Direct-root availability consequences

The accepted structural availability relation yields the direct binding-root consequences exactly as before:

- a duplicable selected field leaves its path and root availability unchanged;
- a consumed non-duplicable selected field becomes unavailable and may make its proper ancestors, including the complete root, partially available;
- a consumed path does not invalidate a structurally disjoint sibling field path;
- every selected field path has already been proven fully available before the first direct-root pattern ownership transition;
- an all-duplicable direct-root pattern leaves the complete root fully available;
- a source-valid direct-root pattern that consumes at least one top-level field leaves the complete root partially available; and
- later whole-binding use, disjoint field-value use, assignment/reinitialization, and cleanup follow `local-bindings.md`, `field-access.md`, and `function-execution.md` without a pattern-specific second binding state machine.

An immutable root binding may legally lose ownership of non-duplicable field subvalues through a source-valid direct-root pattern. Assignment mutability restricts later assignment/reinitialization and is independent of owned-value transfer.

A mutable partially available root may later be whole-replaced by the existing assignment relation; successful replacement resets its consumed-path set exactly as already defined.

Producer-backed transient ownership does not add a persistent function-local structural-availability state after declaration completion. Any pre-existing binding transitions caused while evaluating the producer remain governed by that producer's existing relation.

## Zero-field records

For a zero-field nominal record `Empty`, the direct-root exhaustive pattern remains:

```text
let Empty {} = root;
```

When the pattern head/root category and exact type requirements hold, this declaration is valid.

It contains no pattern entry, introduces no local binding, has no selected non-empty field path whose availability must be checked, duplicates no value, consumes no structural path, and leaves the root consumed-path set unchanged.

The direct empty pattern is therefore an ownership no-op. It is not an implicit discard and does not consume or require ordinary whole-value use of the empty root merely because the record has no fields.

For a producer-backed zero-field pattern, successful producer evaluation instead yields one complete owned empty-record transient. There are no field bindings or field transfers. The transient's remaining cleanup frontier is the complete empty value, and its ownership ends at declaration completion under `function-execution.md`. This source ownership termination remains meaningful even when faithful Core refinement emits no scalar destruction operation.

## Zero-leaf field ownership

A selected non-duplicable field whose source type is a zero-field or recursively zero-leaf record is still a source-owned value.

Consuming that field through a direct-root pattern records its one-field source structural path as consumed and transfers source ownership to the new pattern binding exactly as before. Consuming such a field from a producer-backed transient likewise transfers the complete direct field value into its new binding and removes that field from transient ownership.

These ownership transitions remain meaningful even when a faithful lower structural storage model has no scalar leaf whose liveness changes and no physical destruction operation to emit later.

Pattern validity and ownership MUST NOT be defined through lower scalar-leaf existence.

## Evaluation, fault, and divergence

A direct binding-root scrutinee is an already-existing binding. Its represented pattern entries contain no nested source value producers. After successful source validation, direct-root pattern ownership production itself is non-faulting and non-diverging.

For a producer-backed scrutinee, the selected producer evaluates completely before any pattern field production begins and before any pattern-introduced binding enters scope. Producer evaluation follows the existing ordering, ownership, transient, fault, and divergence rules owned by `function-execution.md`, `field-access.md`, and the producer's other applicable owner.

If producer evaluation yields a defined fault:

1. no pattern field production occurs;
2. no pattern-introduced binding enters scope;
3. cleanup belonging to an in-progress direct call or record construction remains exactly the cleanup already defined for that producer; and
4. the same defined fault continues under `function-execution.md`.

Ownership or structural-availability transitions already caused while evaluating the producer remain effective exactly as for the same producer in another receiving position.

If producer evaluation diverges, no pattern field production occurs, no pattern binding enters scope, and no pattern-declaration cleanup is triggered merely because execution remains suspended in the producer. Producer-owned transient values remain governed by that producer's divergence relation.

After successful producer evaluation, pattern field production and transient remaining cleanup complete before all new pattern bindings enter scope together and before the next body statement begins. Under the current accepted cleanup relation this completion adds no new defined fault or divergence outcome.

## Relation to field-value access

The record pattern and `FieldValueUse` are distinct source operations even when a `FieldValueUse` serves as a producer-backed pattern scrutinee.

They may consume the same nominal field identities, same-module direct field-accessibility relation, source structural paths, structural availability, and type duplicability capability, but:

- record-pattern syntax is not dot field-value syntax;
- the pattern establishes new lexical bindings;
- `FieldValueUse` produces one owned value for an enclosing value consumer, including the producer-backed pattern position represented here;
- the record pattern then produces zero or more initial binding values as one grouped declaration; and
- neither operation creates a general place/lvalue/member abstraction merely because both identify record fields.

A record-valued `FieldValueUse` used as a producer-backed scrutinee first applies its own duplicate-or-consume consequence to its source binding. The resulting transient is then independent of that source binding for pattern field production and transient cleanup.

## Concrete and implementation boundary

`concrete-syntax.md` owns the exact represented spelling and parser-level distinction from ordinary local declarations and between direct-root versus producer-backed scrutinees. This document does not define syntax-tree node kinds, parser recovery, HIR structs, compiler temporary identities, Core field indices, or backend representation.

A faithful typed HIR must retain the already-resolved source decisions strongly enough that lowering does not repeat source pattern semantics. At minimum the semantic information includes:

- resolved nominal record identity;
- scrutinee category;
- for a direct-root scrutinee, the resolved root binding identity;
- for a producer-backed scrutinee, the typed producer plus the source-selected transient remaining-cleanup frontier and order;
- resolved field identity for every pattern entry;
- every new binding identity, lexical key, and exact source type;
- pattern source order; and
- the source-selected duplicate-or-consume ownership consequence of each entry.

Compiler temporary identity used to refine a producer-backed transient is not source-semantic identity.

A faithful direct-root lowering may continue to refine each pattern entry in pattern source order directly from the mapped source root's projected field:

- duplicating entry -> existing projected Core `Copy`;
- consuming entry -> existing projected Core `Move`.

It MUST NOT insert a whole-record temporary merely because the producer-backed category exists.

A faithful producer-backed lowering may:

1. lower the retained producer using the existing owned-value lowering relation to one compiler result temporary;
2. initialize mapped pattern source locals in pattern source order using projected Core `Copy` or `Move` from that temporary according to the retained `OwnedUse` decision; and
3. refine the HIR-retained transient cleanup frontier through existing Core `Drop` effects where the selected source value has represented scalar destruction consequences.

A lower-vacuous zero-leaf cleanup effect may be erased only as a refinement fact. Core scalar liveness/copyability MUST NOT be used to decide source pattern validity, source `Duplicate` versus `Consume`, or the source transient cleanup frontier.

The mapped pattern bindings remain ordinary source locals and participate in the existing lexical source-local declaration/cleanup ordering. Existing Core projected `Init`/`Copy`/`Move`/`Drop` semantics are refinement targets only after source validation. No new Core normative operation is required by this represented extension.

## Further boundaries

This revision does not define:

- shorthand field binding;
- wildcard/ignore patterns;
- rest or omitted fields;
- partial/non-exhaustive record patterns;
- nested record patterns;
- tuple, array, enum, variant, scalar-literal, range, or other pattern categories;
- refutable patterns, `match`, guards, conditionals, loops, or control-flow joins;
- a bare binding scrutinee interpreted as ordinary whole-value production;
- producer-backed boolean/integer literal scrutinees;
- arbitrary general expressions or grouping as pattern scrutinees;
- destructuring assignment;
- mutable pattern-binding modifiers;
- qualified/cross-module record-pattern heads or broader field visibility;
- references, borrow binding modes, lifetime syntax, or source `unsafe`;
- positive record duplicability-selection syntax, traits, generics, or coherence;
- closures/function values;
- floating literals, operators, conversions, or general expressions;
- custom destruction, panic/catch payload forms, const/static semantics, ABI/layout/FFI/linkage, Exec/Model source forms, or runtime/backend representation.

A later pattern feature must extend this accepted relation explicitly rather than silently reinterpret either accepted scrutinee category.