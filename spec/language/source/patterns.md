# Source Patterns

Status: **provisional normative; incomplete**

This document owns the represented source semantics for recursive irrefutable exhaustive named-field record patterns: same-module nominal pattern-head selection, recursive field structure, binding-leaf order and production, scrutinee-category selection, direct binding-root ownership consequences, and producer-backed transient ownership/cleanup.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), same-module declaration lookup from [Source names and modules](names-modules.md), nominal record/field identity, exact source type equality, field source types, structural field order, and owned-value duplicability from [Source type foundation](types.md), structural paths, path availability/consumption, and remaining-ownership frontiers from [Source structural ownership](structural-ownership.md), function-local binding lookup/identity/scope/shadowing/mutability from [Source function-local bindings](local-bindings.md), and direct same-module record-field accessibility from [Source field-value access](field-access.md). It consumes represented producer evaluation, transient ownership termination, fault propagation, divergence, and declaration completion from [Source function execution](function-execution.md). It does not redefine those owners.

The represented concrete pattern and scrutinee spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define refutable patterns, `match`, alternatives, guards, shorthand/rest/wildcards, tuple/array/enum patterns, destructuring assignment, references or borrow binding modes, arbitrary general expressions, a general source place/lvalue abstraction, cross-module field visibility, or an implementation representation.

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

Qualified or cross-module pattern heads are not represented.

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

For the currently represented record declaration form, direct fields are module-private. Therefore recursive pattern selection may open only record nodes whose fields are directly accessible to the containing function.

A same-module outer record may have a field whose type is a foreign exported record. The outer field itself may be selected and bound as one complete binding leaf because the outer field is accessible. A nested record pattern may not open that foreign record under this revision because its fields remain module-private to the foreign module.

This relation does not introduce field-visibility syntax or broaden construction/field access across modules.

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

For a producer-backed declaration, a structurally invalid pattern does not validate/evaluate the producer merely to discover a later pattern error.

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
- a binding-rooted field-value use.

The top pattern head's nominal record type is the producer's exact required source type. Structural similarity to another record type is insufficient.

The producer is resolved/evaluated in the lexical environment that exists before any binding introduced by this pattern enters scope.

A successful producer yields one fully owned **pattern scrutinee transient** of the selected top record type. This transient:

- has no lexical key or source binding identity;
- is not source-addressable;
- does not participate in function-local lookup;
- is not an ordinary local or parameter; and
- exists only until this declaration completes.

On successful producer completion, the transient begins as one structural owned-value root with an empty consumed-path state under `structural-ownership.md`.

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

After complete pattern structure is valid and the producer has successfully yielded the fully owned transient root, process binding leaves strictly in retained depth-first source order.

For each leaf path `p` of exact type `T`:

- if `T` is duplicable, produce one owned duplicate of the complete value at `p` and leave the transient structural state unchanged;
- if `T` is non-duplicable, transfer exactly the complete owned value at `p` and consume `p` in the transient structural state through `structural-ownership.md`.

Pattern leaf production itself is finite, non-faulting, and non-diverging after successful producer completion.

No ancestor or whole transient value is independently duplicated/consumed merely to begin or continue recursive destructuring.

## Producer-backed transient remaining cleanup

After every binding leaf has been produced, the producer-backed pattern scrutinee transient ends before the declaration finishes.

Its remaining cleanup frontier is exactly `frontier([])` from `structural-ownership.md` using the transient's final consumed-path state.

Therefore:

- if no binding leaf consumed any path, the frontier contains the complete transient root;
- mixed nested consumption yields exactly the maximal still-owned disjoint structural subvalues in canonical recursive reverse record-declaration order;
- if all structurally owned subvalues have been transferred, the frontier is empty without synthesizing whole-root consumption; and
- zero-field and recursively zero-leaf frontier members remain source-owned facts even when faithful lower scalar cleanup is vacuous.

The transient frontier is cleaned exactly once by `function-execution.md` before pattern bindings enter scope. No later lexical-scope or activation cleanup owns the transient.

The former one-level special cases “complete transient”, “direct retained fields”, and “no cleanup” are consequences of this general structural frontier for one-level patterns; they are not a second authority.

## Zero-field and zero-leaf behavior

For a top-level zero-field nominal record `Empty`, the direct-root pattern:

```text
let Empty {} = root;
```

remains valid when head/root category and exact type requirements hold. It has no binding leaf, introduces no binding, performs no ownership operation, and imposes no whole-root availability requirement. It is not implicit discard or whole-root use.

For a producer-backed top-level zero-field pattern, successful producer evaluation yields one complete owned empty-record transient. With no leaf consumption, its remaining frontier is the complete root and that source ownership ends at declaration completion.

For a nested zero-field pattern, recursion likewise contributes no leaf and no ownership operation. If it lies inside a producer transient, that zero-field subvalue remains part of the canonical remaining frontier unless some ancestor path was transferred by another accepted operation.

A non-duplicable binding leaf whose type is a zero-field or recursively zero-leaf record remains a real source consumption. The ownership transition is retained even if faithful Core refinement has no scalar liveness/destruction event.

Pattern validity and ownership are never defined by lower scalar-leaf existence.

## Producer fault and divergence

Pattern structure is source validation and occurs before producer execution consequences.

If producer evaluation yields a defined fault before the transient is established:

- no pattern leaf production occurs;
- no pattern-introduced binding enters scope;
- producer-internal transient cleanup follows the producer's existing owner;
- ownership/structural transitions already completed by producer evaluation remain effective; and
- the same defined fault continues under `function-execution.md`.

If producer evaluation diverges, no pattern leaf production, pattern binding establishment, or pattern-transient cleanup occurs merely because execution remains suspended. Producer-owned transients remain governed by the producer's divergence relation.

For source validation implementations, producer-backed validation must preserve this atomic source-validity boundary: failure after tentative consuming producer validation must not leave rejected-source ownership state committed.

## Declaration completion

For a valid direct-root pattern declaration:

1. complete recursive pattern structure and binding-leaf validation;
2. validate all leaf paths against one pre-pattern direct-root structural state;
3. produce every binding-leaf value in depth-first source order; and
4. establish all new bindings together.

For a valid producer-backed pattern declaration:

1. complete recursive pattern structure and binding-leaf validation;
2. evaluate the producer using the pre-pattern-binding lexical environment;
3. establish the fully owned transient root;
4. produce every binding-leaf value in depth-first source order;
5. clean the transient's canonical remaining frontier exactly once; and
6. establish all new bindings together.

Only after the applicable sequence completes may the next body statement begin.

## HIR and lowering refinement boundary

This specification does not prescribe Rust data structures, but a faithful typed HIR must retain enough source-selected facts that lowering does not repeat source semantics.

At minimum retain:

- the top nominal record identity;
- direct-root versus producer-backed scrutinee category;
- for a direct-root scrutinee, the resolved root binding identity;
- for a producer-backed scrutinee, the validated typed producer;
- all binding leaves in depth-first source order;
- each binding leaf's complete resolved structural field path from the top root;
- each new binding identity/key/exact type;
- each leaf's source-selected duplicate-or-consume consequence; and
- for producer-backed scrutinees, the final source-selected remaining cleanup frontier paths in canonical order.

Compiler temporary identity is not source-semantic pattern identity.

The former one-level HIR boundary that retained only one direct field index per leaf and `None / Complete / DirectFields` transient cleanup is insufficient for recursive patterns and must not remain as parallel semantic authority.

No Core semantic change is required by this pattern relation. Existing structural projections and projected `Copy`, `Move`, and `Drop` can refine retained arbitrary leaf/cleanup paths after source validation.

Direct-root lowering can remain direct from the mapped source binding with no whole-record pattern temporary. Producer-backed lowering can reuse the existing producer result temporary as the transient refinement; it does not need a source-visible synthetic binding.

Lowering MUST NOT reconstruct pattern exhaustiveness, binding-leaf order, source duplicability, path availability, consumed paths, or transient remaining-frontier membership from Core liveness/copyability. Zero-leaf source cleanup may refine to no Core `Drop` where the lower destruction domain is empty.

## Future compatibility boundary

The explicit field-target form permits later pattern categories to extend the right side of a field entry without changing the accepted binding-leaf or nested-record spellings.

This revision does not define shorthand field binding, `_`, rest/omission, tuple/array/enum patterns, literals, alternatives, guards, refutable patterns, `match`, `if let`, loops, reference/borrow binding modes, mutable pattern bindings, destructuring assignment, arbitrary pattern scrutinees, general expressions/grouping, qualified pattern heads, cross-module field visibility, or field visibility modifiers.

Later features must extend rather than reinterpret the direct-root and producer-backed recursive semantics accepted here.

## Source/Core separation

Pattern ownership is source semantics over nominal record/field identities, structural paths, source type duplicability, binding identity, source order, and transient ownership.

Core projections, path liveness, scalar copyability, destruction domains, compiler local numbering, physical offsets, and backend storage are not source pattern authority.

A faithful implementation may map retained resolved paths to lower projections only after source validation has selected every source-semantic fact above.
