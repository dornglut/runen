# Source Patterns

Status: **provisional normative; incomplete**

This document owns the represented source semantics for recursive irrefutable exhaustive named-field record patterns: same-module nominal pattern-head selection, recursive field structure, binding-leaf order and production, scrutinee-category selection, direct binding-root ownership consequences, and producer-backed pattern-scrutinee transient ownership/cleanup.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), same-module declaration lookup from [Source names and modules](names-modules.md), nominal record/field identity, exact source type equality, field source types, structural field order, and owned-value duplicability from [Source type foundation](types.md), structural paths, path availability/consumption, and remaining-ownership frontiers from [Source structural ownership](structural-ownership.md), function-local binding lookup/identity/scope/shadowing/mutability from [Source function-local bindings](local-bindings.md), and direct record-field accessibility plus the completed field-value producer result boundary from [Source field-value access](field-access.md). It consumes represented producer evaluation, record-construction completion, producer-backed field-receiver completion, transient ownership termination, fault propagation, divergence, and declaration completion from [Source function execution](function-execution.md). It does not redefine those owners.

The represented concrete pattern and scrutinee spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define refutable patterns, `match`, alternatives, guards, shorthand/rest/wildcards, tuple/array/enum patterns, destructuring assignment, references or borrow binding modes, arbitrary general expressions, a general source place/lvalue abstraction, field accessibility, cross-module pattern-head syntax, or an implementation representation.

## Represented record-destructuring declaration

The represented pattern operation is one recursive, irrefutable, exhaustive named-field record-destructuring declaration.

Conceptually:

```text
let Outer {
    left: left_binding,
    inner: Inner {
        x: x_binding,
        nested: Nested {
            y: y_binding,
        },
    },
} = root;
```

The top pattern may instead receive one accepted producer-backed scrutinee:

```text
let Outer {
    left: left_binding,
    inner: Inner {
        x: x_binding,
        nested: Nested {
            y: y_binding,
        },
    },
} = make_outer();
```

Every represented **record-pattern node** has:

- one explicit unqualified nominal record head;
- one finite source-ordered sequence of explicit named fields; and
- for each selected field, exactly one target that is either:
  - one binding leaf; or
  - one nested record-pattern node.

A nested record-pattern node itself introduces no binding. Binding leaves are the only pattern elements that produce function-local bindings.

The complete tree is irrefutable because every pattern node requires exact nominal type equality and names every field of that record exactly once. Successful pattern production performs no dynamic shape test and has no mismatch outcome.

## Pattern-head selection

Every record-pattern node head is one unqualified `UserIdentifier` in the concrete form.

It resolves directly through the same-module declaration namespace owned by `names-modules.md` and MUST select one nominal record declaration. A selected function or another wrong-category declaration is invalid and MUST NOT be bypassed.

Function-local value bindings do not participate in pattern-head lookup. A local binding whose key equals a record-pattern head therefore does not change which record declaration the head denotes.

Qualified or cross-module pattern heads are not represented. The existence of a qualified `RecordConstruction` producer does not broaden this head relation.

For the top pattern node, the selected record type is the exact required scrutinee type.

For a nested pattern node selected as field target, its nominal record type MUST equal exactly the source type of that selected field. Equal field shape from another record does not satisfy this requirement.

## Recursive field structure

For every record-pattern node with selected nominal record `R`:

1. each pattern field key MUST resolve to exactly one declared field identity of `R`;
2. no declared field identity may occur more than once;
3. every declared field identity of `R` MUST occur exactly once;
4. every selected field MUST satisfy the direct record-field accessibility relation owned by `field-access.md` in the containing function; and
5. the field target MUST satisfy the target relation below.

A **binding target** contributes one binding leaf whose source type is exactly the selected field's source type.

A **nested record-pattern target** is valid only when the selected field's source type is exactly the nested node's nominal record type. The nested node then recursively satisfies this same complete field relation.

Unknown, duplicate, or missing fields at any depth reject the complete declaration.

Field presentation order is not field lookup priority. Nominal record declaration identity and structural field order remain owned by `types.md`.

## Direct field accessibility at every depth

Every selected pattern field, including fields selected inside nested nodes, independently consumes the direct field-accessibility relation from `field-access.md`.

Every represented record-pattern node head remains a same-module lookup. Therefore the nominal record opened by each represented pattern node is declared in the same source module as the containing function, and direct access to its fields is permitted by the same-module branch of `field-access.md` regardless of whether an individual field is module-private or exported.

A same-module outer record may have a field whose type is a foreign exported record. The accessible outer field may be bound as one complete binding leaf. That foreign record still cannot be recursively opened under this revision because every nested pattern head is unqualified and same-module-only, and exact nominal type equality prevents a different same-module record head from standing in for the foreign type.

Field export and qualified record construction therefore do not broaden represented record-pattern head selection or create a cross-module pattern form. A future qualified-pattern delivery may consume the same direct field-accessibility relation without redefining it.

## Binding leaves and structural paths

Each binding leaf corresponds to exactly one complete structural source path from the top pattern root.

The path is formed by appending each resolved field identity traversed from the top pattern node to that leaf. Its final type is exactly the selected leaf field's source type under `structural-ownership.md` and `types.md`.

A field target is either a binding leaf or a nested record pattern, never both. Because every record-pattern node names each field once, distinct binding-leaf paths in one valid pattern tree are pairwise structurally disjoint.

Nested record-pattern nodes are static pattern structure, not independently produced values. Their intermediate paths are not automatically duplicated or consumed merely because pattern traversal enters them.

## Binding-leaf source order

Binding-leaf source order is **depth-first traversal in concrete pattern field order**:

1. visit the current record-pattern node's fields in their written order;
2. a binding target contributes its binding leaf immediately;
3. a nested record-pattern target recursively contributes all of its binding leaves in its own field order before traversal continues to the next sibling field.

This order controls:

- pattern binding-value production order;
- pattern-introduced local declaration order; and therefore
- later reverse local-declaration cleanup order under `local-bindings.md` and `function-execution.md`.

This order does not replace record declaration structural order for remaining-ownership frontier selection.

## Complete pattern validation before ownership consequences

The complete recursive pattern tree MUST validate before any pattern-owned duplicate/consume transition and, for a producer-backed declaration, before producer validation/evaluation may acquire source ownership consequences.

Before the declaration enters its ownership-production relation, validation establishes at least:

1. every pattern head and exact nested record type relation;
2. every field identity and exhaustive field set at every node;
3. direct field accessibility at every selected field;
4. every binding leaf's complete resolved structural path and exact source type;
5. the complete depth-first binding-leaf source order;
6. pairwise uniqueness of all binding leaf lexical keys across the entire tree; and
7. absence of an overlapping function-local shadow conflict for every binding leaf key against the pre-declaration lexical environment.

A rejected recursive structure establishes no pattern binding and applies no pattern-owned ownership transition.

For a producer-backed declaration, a structurally invalid pattern does not validate/evaluate the producer merely to discover a later pattern error. Once the pattern is valid, its top nominal record type is the exact required type supplied to producer validation before producer execution begins.

Because every represented top pattern head is same-module and source-module self-import is invalid, a qualified record construction necessarily selects a foreign nominal record. That foreign record type cannot equal the same-module top pattern type. Therefore a bare qualified construction in the producer-backed scrutinee category is rejected by exact producer-result typing before any constructor initializer is evaluated or commits ownership. This consequence does not require a duplicate same-module-only constructor grammar for pattern scrutinees.

## Pattern-introduced bindings

Every valid binding leaf introduces one ordinary function-local binding under `local-bindings.md`.

For a leaf with key `b`, path `p`, and type `T = type(p)`:

- the new binding has one stable source-semantic binding identity;
- its lexical key is `b`;
- its declared source type is exactly `T`;
- it is immutable for assignment purposes in this revision; and
- its initial owned value is produced by the applicable direct-root or producer-transient leaf operation below.

All bindings introduced by one declaration enter scope together only after the **complete declaration** finishes successfully. None participates in lookup while the pattern structure, scrutinee, leaf ownership production, or producer-transient cleanup of that declaration is in progress.

Each established binding begins with complete structural ownership of its produced value under `local-bindings.md` and `structural-ownership.md`.

## Scrutinee categories and exact top type

The represented declaration has exactly two top-level scrutinee categories selected by concrete syntax. Validation MUST preserve the selected category.

### Direct binding-root scrutinee

A direct binding-root scrutinee is exactly one bare unqualified function-body identifier under `concrete-syntax.md`.

It resolves through the function-local value-binding precedence owned by `local-bindings.md` and MUST select one active parameter or ordinary local binding. Lookup MUST NOT bypass an active binding merely because a module declaration would be convenient.

Only when no active parameter/local binding resolves the key does existing same-module fallback occur. A selected module declaration is then the wrong category and the declaration is source-invalid.

The selected root binding's declared source type MUST equal exactly the nominal record type of the top pattern head.

A bare direct root is **not** ordinary whole-binding `IdentifierUse` production for this declaration. The root is not first duplicated or consumed as a complete value and no pattern scrutinee transient is created.

### Producer-backed transient scrutinee

A producer-backed scrutinee is exactly one syntactically non-bare producer admitted by `concrete-syntax.md`:

- a result-bearing direct call;
- a record construction; or
- a field-value use, using either its binding-root or bounded producer-backed receiver form.

`RecordConstruction` in this list is the one existing producer category and may use either its represented unqualified same-module target or its qualified cross-module target. This does not create a second pattern scrutinee category or a qualified pattern head.

The top pattern head's nominal record type is the exact required source type of the **complete scrutinee producer result**. Structural similarity to another record type is insufficient. Consequently a qualified construction of a foreign record fails this exact required-type relation under the current same-module pattern-head grammar before construction execution, as established above.

For a producer-backed field-value scrutinee, that top required type constrains the field-value operation's final selected field result. It does not constrain the field-value operation's internal direct-call or record-construction receiver, whose own exact receiver type remains selected and validated under `field-access.md`. A qualified construction may therefore appear **inside** such a field-value receiver when its final selected field result has the same nominal record type as the same-module top pattern; the field receiver's own foreign record type is not the pattern scrutinee type.

The producer is resolved/evaluated in the lexical environment that exists before any binding introduced by this pattern enters scope.

A successful complete producer yields one fully owned **pattern scrutinee transient** of the selected top record type. This transient:

- has no lexical key or source binding identity;
- is not source-addressable;
- does not participate in function-local lookup;
- is not an ordinary local or parameter; and
- exists only until this declaration completes.

On successful complete producer completion, the pattern transient begins as one structural owned-value root with an empty consumed-path state under `structural-ownership.md`.

When the scrutinee is a producer-backed field-value use, its internal **field-receiver transient** is not the pattern scrutinee transient. The field-value producer must first preserve its selected record result, clean and end the field-receiver transient under `field-access.md` and `function-execution.md`, and only then transfer that result into the distinct fully owned pattern scrutinee transient described here.

Boolean/integer literals are not producer-backed record scrutinee forms in this revision because their represented source types are intrinsic. This restriction does not define a general expression taxonomy.

## Direct-root prevalidation

For a direct binding-root declaration, every **binding-leaf structural path** MUST be fully available under `structural-ownership.md` in one shared pre-pattern ownership state of the selected root binding.

All binding-leaf paths are checked against that same pre-pattern state before the first pattern-owned transition.

Because valid binding-leaf paths are pairwise structurally disjoint, later source-ordered consumption of one valid leaf cannot invalidate another prevalidated leaf.

A nested record-pattern node does not independently require its complete intermediate path to be fully available merely so static pattern recursion may enter it. Ownership-producing binding leaves are the paths that require full availability.

Consequently, a nested zero-field record pattern contributes no binding leaf, performs no ownership operation, and adds no whole-path availability precondition merely because the empty record structure is named. This is the recursive analogue of the accepted top-level zero-field direct-root no-op.

If any binding-leaf path is unavailable or partially available in the pre-pattern state, the complete declaration is source-invalid and applies no pattern-owned transition.

## Direct binding-root leaf ownership

After complete recursive structure and direct-root leaf availability have validated, process binding leaves strictly in retained depth-first source order.

For each leaf path `p` of exact type `T`:

- if `T` is duplicable under `types.md`, produce one owned duplicate of the complete value at `p` and leave the root structural ownership state unchanged;
- if `T` is non-duplicable, transfer exactly the complete owned value at `p` and apply the canonical successful-consumption transition from `structural-ownership.md` to the root binding.

No ancestor of `p` is independently duplicated or consumed by this operation. Structurally disjoint paths remain governed by their own structural state.

Exhaustiveness does not imply a whole-root consume. Separately consuming every ownership-producing leaf may leave an empty remaining root frontier without synthesizing consumption of the empty root path.

A direct-root pattern has no transient cleanup phase. Later use, assignment, and cleanup of the root binding consume its resulting structural ownership state through the existing owners.

## Producer-backed transient leaf ownership

After complete pattern structure is valid and the complete producer has successfully yielded the fully owned pattern scrutinee transient root, process binding leaves strictly in retained depth-first source order.

For each leaf path `p` of exact type `T`:

- if `T` is duplicable, produce one owned duplicate of the complete value at `p` and leave the transient structural state unchanged;
- if `T` is non-duplicable, transfer exactly the complete owned value at `p` and consume `p` in the transient structural state through `structural-ownership.md`.

Pattern leaf production itself is finite, non-faulting, and non-diverging after successful complete producer completion.

No ancestor or whole transient value is independently duplicated/consumed merely to begin or continue recursive destructuring.

## Producer-backed transient remaining cleanup

After every binding leaf has been produced, the producer-backed pattern scrutinee transient ends before the declaration finishes.

Its remaining cleanup frontier is exactly `frontier([])` from `structural-ownership.md` using the transient's final consumed-path state.

Therefore:

- if no binding leaf consumed any path, the frontier contains the complete transient root;
- mixed nested consumption yields exactly the maximal still-owned disjoint structural subvalues in canonical recursive reverse record-declaration order;
- if all structurally owned subvalues have been transferred, the frontier is empty without synthesizing whole-root consumption; and
- zero-field and recursively zero-leaf frontier members remain source-owned facts even when faithful lower scalar cleanup is vacuous.

The pattern scrutinee transient frontier is cleaned exactly once by `function-execution.md` before pattern bindings enter scope. No later lexical-scope or activation cleanup owns the pattern transient.

A field-receiver transient internal to a producer-backed field-value scrutinee has already ended before this pattern transient exists. Pattern leaf consumption therefore cannot alter, enlarge, or retroactively reselect the field-receiver cleanup frontier.

The former one-level special cases “complete transient”, “direct retained fields”, and “no cleanup” are consequences of this general structural frontier for one-level patterns; they are not a second authority.

## Zero-field and zero-leaf behavior

For a top-level zero-field nominal record `Empty`, the direct-root pattern:

```text
let Empty {} = root;
```

remains valid when head/root category and exact type requirements hold. It has no binding leaf, introduces no binding, performs no ownership operation, and imposes no whole-root availability requirement. It is not implicit discard or whole-root use.

For a producer-backed top-level zero-field pattern, successful complete producer evaluation yields one complete owned empty-record pattern transient. With no leaf consumption, its remaining frontier is the complete root and that source ownership ends at declaration completion.

For a nested zero-field pattern, recursion likewise contributes no leaf and no ownership operation. If it lies inside a producer transient, that zero-field subvalue remains part of the canonical remaining frontier unless some ancestor path was transferred by another accepted operation.

A non-duplicable binding leaf whose type is a zero-field or recursively zero-leaf record remains a real source consumption. The ownership transition is retained even if faithful Core refinement has no scalar liveness/destruction event.

Pattern validity and ownership are never defined by lower scalar-leaf existence.

## Producer fault and divergence

Pattern structure is source validation and occurs before producer execution consequences.

If complete producer evaluation yields a defined fault before the pattern transient is established:

- no pattern leaf production occurs;
- no pattern-introduced binding enters scope;
- producer-internal transient cleanup follows the producer's existing owner;
- for a producer-backed field-value scrutinee, no pattern transient exists during receiver fault propagation, and any field-receiver transient that exists after receiver success is completed entirely inside that field-value producer before a result could reach this pattern;
- ownership/structural transitions already completed by producer evaluation remain effective; and
- the same defined fault continues under `function-execution.md`.

If complete producer evaluation diverges, no pattern leaf production, pattern binding establishment, or pattern-transient cleanup occurs merely because execution remains suspended. Producer-owned transients remain governed by the producer's divergence relation. A producer-backed field-value scrutinee may diverge only while its retained receiver producer is still evaluating; after receiver success its field-selection and field-receiver cleanup tail is non-diverging under the current source model.

A bare qualified foreign construction cannot reach either dynamic outcome in this pattern position because exact top-type validation rejects it before constructor execution. Qualification inside another accepted producer, such as a field-value receiver whose final result matches the top pattern type, follows that producer's ordinary fault/divergence relation.

For source validation implementations, producer-backed validation must preserve this atomic source-validity boundary: failure after tentative consuming producer validation must not leave rejected-source ownership state committed. A nested producer-backed field-value use independently preserves its own transaction boundary under `field-access.md`; pattern validity does not merge those transactions into one ownership domain.

## Declaration completion

For a valid direct-root pattern declaration:

1. complete recursive pattern structure and binding-leaf validation;
2. validate all leaf paths against one pre-pattern direct-root structural state;
3. produce every binding-leaf value in depth-first source order; and
4. establish all new bindings together.

For a valid producer-backed pattern declaration:

1. complete recursive pattern structure and binding-leaf validation;
2. validate the complete producer using the pre-pattern-binding lexical environment and the top nominal record type as the exact required type of its final result;
3. only after complete producer validation succeeds, evaluate that producer;
4. if that producer is a producer-backed field-value use, finish its separate field-receiver transient lifecycle before this step yields the owned record result;
5. transfer the produced record into the fully owned pattern scrutinee transient;
6. produce every binding-leaf value in depth-first source order;
7. clean the pattern transient's canonical remaining frontier exactly once; and
8. establish all new bindings together.

Only after the applicable sequence completes may the next body statement begin.

## HIR and lowering refinement boundary

This specification does not prescribe Rust data structures, but a faithful typed HIR must retain enough source-selected facts that lowering does not repeat source semantics.

At minimum retain:

- the top nominal record identity;
- direct-root versus producer-backed scrutinee category;
- for a direct-root scrutinee, the resolved root binding identity;
- for a producer-backed scrutinee, the validated typed complete producer, including any field-value producer's own retained receiver/path/consequence/cleanup facts through its canonical owner;
- all binding leaves in depth-first source order;
- each binding leaf's complete resolved structural field path from the top root;
- each new binding identity/key/exact type;
- each leaf's source-selected duplicate-or-consume consequence; and
- for producer-backed scrutinees, the final source-selected pattern-transient remaining cleanup frontier paths in canonical order.

Compiler temporary identity is not source-semantic pattern identity. If the retained producer is a record construction, its own source validator may discard whether its target spelling was qualified after retaining the resolved nominal record identity and initializer facts; pattern HIR requires no duplicate qualification fact.

The former one-level HIR boundary that retained only one direct field index per leaf and `None / Complete / DirectFields` transient cleanup is insufficient for recursive patterns and must not remain as parallel semantic authority.

No Core semantic change is required by this pattern relation. Existing structural projections and projected `Copy`, `Move`, and `Drop` can refine retained arbitrary leaf/cleanup paths after source validation.

Direct-root lowering can remain direct from the mapped source binding with no whole-record pattern temporary. Producer-backed lowering can reuse the existing complete producer result temporary as the pattern transient refinement; it does not need a source-visible synthetic binding. When the complete producer is a producer-backed field-value use, its receiver temporary and source-selected receiver cleanup finish first, and only its preserved selected result transfers into the separate pattern-scrutinee temporary/state.

Lowering MUST NOT reconstruct pattern exhaustiveness, binding-leaf order, source duplicability, path availability, consumed paths, field-receiver frontier membership, or pattern-transient remaining-frontier membership from Core liveness/copyability. Zero-leaf source cleanup may refine to no Core `Drop` where the lower destruction domain is empty.

## Future compatibility boundary

The explicit field-target form permits later pattern categories to extend the right side of a field entry without changing the accepted binding-leaf or nested-record spellings.

This revision does not define shorthand field binding, `_`, rest/omission, tuple/array/enum patterns, literals, alternatives, guards, refutable patterns, `match`, `if let`, loops, reference/borrow binding modes, mutable pattern bindings, destructuring assignment, arbitrary pattern scrutinees, general expressions/grouping, qualified/cross-module pattern heads, or pattern-specific visibility modifiers.

Later features must extend rather than reinterpret the direct-root and producer-backed recursive semantics accepted here. A future qualified-pattern delivery may make a foreign qualified construction directly usable as a matching scrutinee only by explicitly broadening pattern-head lookup and preserving exact nominal typing; this revision does not pre-authorize that step.

## Source/Core separation

Pattern ownership is source semantics over nominal record/field identities, structural paths, source type duplicability, binding identity, source order, and pattern-transient ownership.

A field-receiver transient used internally by a producer-backed field-value scrutinee is owned by the field-value operation, not by pattern semantics. Only the completed selected record result crosses into the pattern ownership relation.

Core projections, path liveness, scalar copyability, destruction domains, compiler local numbering, physical offsets, backend storage, and construction-target qualification are not source pattern authority.

A faithful implementation may map retained resolved paths to lower projections only after source validation has selected every source-semantic fact above.
