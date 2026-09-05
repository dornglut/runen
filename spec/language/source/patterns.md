# Source Patterns

Status: **provisional normative; incomplete**

This document owns the represented source semantics for recursive named-field record patterns: the existing irrefutable record-destructuring declaration with bounded node-local rest/omission, and one bounded single-success refutable record-selection pattern with exact `Bool`/fixed-width-integer literal-test leaves. It owns unqualified same-module and qualified cross-module nominal pattern-head selection, recursive field/rest structure, binding-leaf and literal-test order, scrutinee-category selection, direct binding-root ownership consequences, producer-backed pattern-scrutinee transient ownership/cleanup, and the pattern-local success/mismatch relation before control-flow arm execution.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), same-module declaration lookup and qualified cross-module lookup from [Source names and modules](names-modules.md), nominal record/field identity, exact source type equality, field source types, structural field order, and owned-value duplicability from [Source type foundation](types.md), Boolean and decimal-integer literal materialization from [Source literal semantics](literals.md), exact Boolean and fixed-width integer equality value relations from [Source operator semantics](operators.md), structural paths, path availability/consumption, and remaining-ownership frontiers from [Source structural ownership](structural-ownership.md), function-local binding lookup/identity/scope/shadowing/mutability from [Source function-local bindings](local-bindings.md), the canonical direct safe-authority compatibility relation from [Source safe references](references.md), and direct record-field accessibility plus the completed field-value producer result boundary from [Source field-value access](field-access.md). It consumes represented producer evaluation, record-construction completion, producer-backed field-receiver completion, transient ownership termination, fault propagation, divergence, and declaration/pattern-transient completion from [Source function execution](function-execution.md). It does not redefine those owners.

The represented concrete pattern, rest-marker, scrutinee, and bounded `if let` spellings are owned by [Source concrete syntax](concrete-syntax.md). Success/mismatch arm execution and definite normal-successor composition are owned by [Source control flow](control-flow.md).

This document does not define multi-arm `match`/`case` selection, pattern alternatives, guards, ranges, shorthand or wildcard bindings, tuple/array/enum patterns, destructuring assignment, reference/borrow binding modes, safe-reference formation/reborrow, arbitrary general expressions, a general source place/lvalue abstraction, field accessibility, nested module paths beyond the represented alias/member pair, or an implementation representation.

## Represented record-destructuring declaration

The represented pattern operation is one recursive, irrefutable named-field record-destructuring declaration. Every record-pattern node is either exhaustive through its explicit fields or uses the bounded rest marker to omit every declared field not explicitly selected at that node.

Conceptually, the existing exhaustive form remains represented:

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

The bounded rest form may instead select only explicit fields and omit the remainder:

```text
let Outer {
    left: left_binding,
    inner: Inner {
        x: x_binding,
        ..
    },
    ..
} = root;
```

The top pattern may receive one accepted producer-backed scrutinee with the same node-local relation:

```text
let Outer {
    left: left_binding,
    inner: Inner {
        x: x_binding,
        ..
    },
    ..
} = make_outer();
```

A pattern may also open only accessible fields of an exported foreign record explicitly through the represented two-part module-qualified head:

```text
let dep::Outer {
    exported_field: value,
    ..
} = root;
```

Every represented **irrefutable record-pattern node** has:

- one explicit nominal record head, either unqualified or one represented two-part qualified module member;
- one finite source-ordered sequence of zero or more explicit named fields;
- for each explicit field, exactly one target that is either:
  - one binding leaf; or
  - one nested irrefutable record-pattern node; and
- zero or one node-local rest marker that, when present, omits every declared field identity not selected by an explicit field in that node.

A nested irrefutable record-pattern node itself introduces no binding. Binding leaves are the only pattern elements that produce function-local bindings. The rest marker introduces no binding, structural path, value, type, safe-authority access, or runtime operation.

The complete tree is irrefutable because every pattern node requires exact nominal type equality and no represented node performs a runtime shape test. A node without rest is exhaustive exactly as before. A node with rest statically accepts its exact nominal record while selecting only its explicit fields and omitting the remainder. Successful pattern production has no mismatch outcome.

## Pattern-head selection

Every represented irrefutable or refutable record-pattern node head is either one unqualified `UserIdentifier` or one represented `QualifiedModuleMember` in the concrete form.

An **unqualified pattern head** resolves directly through the same-module declaration namespace owned by `names-modules.md` and MUST select one nominal record declaration. A selected function or another wrong-category declaration is invalid and MUST NOT be bypassed.

A **qualified pattern head** resolves only through the source-unit module-alias and qualified cross-module lookup relation owned by `names-modules.md`. That lookup therefore requires the selected target module binding to be exported. The resolved binding MUST denote one nominal record declaration. An unresolved alias/member, inaccessible target binding, or wrong-category exported binding is invalid and MUST NOT be bypassed merely because pattern context requires a record.

Function-local value bindings do not participate in either pattern-head lookup relation. A local binding whose key equals an unqualified record-pattern head therefore does not change which same-module record declaration that head denotes, and local bindings never participate in a syntactically qualified head.

Qualification is source name resolution only. After a source-valid head has selected its nominal record declaration, qualified versus unqualified spelling does not create a distinct record type, pattern category, runtime module operation, or lower execution fact.

For the top pattern node, the selected record type is the exact required scrutinee type.

For a nested pattern node selected as an applicable field target, its nominal record type MUST equal exactly the source type of that selected field. Equal field shape from another record does not satisfy this requirement.

## Recursive field and rest structure

For every irrefutable record-pattern node with selected nominal record `R`:

1. each explicit pattern field key MUST resolve to exactly one declared field identity of `R`;
2. no declared field identity may occur in more than one explicit field;
3. every explicit selected field MUST satisfy the direct record-field accessibility relation owned by `field-access.md` in the containing function;
4. every explicit field target MUST satisfy the irrefutable target relation below;
5. the node contains at most one rest marker; and
6. if the node has no rest marker, every declared field identity of `R` MUST occur exactly once as an explicit field, while if the node has a rest marker, every declared field identity not explicitly selected is omitted by that marker.

A **binding target** contributes one binding leaf whose source type is exactly the selected field's source type.

A **nested irrefutable record-pattern target** is valid only when the selected field's source type is exactly the nested node's nominal record type. The nested node then recursively satisfies this same field/rest relation independently; a rest marker in one node does not omit fields of another node.

An omitted field is not a selected pattern field. It contributes no target, binding leaf, structural path, field accessibility requirement, safe-authority compatibility requirement, ownership production, or source-order item merely because it exists in the nominal record.

Unknown or duplicate explicit fields reject the complete declaration. Missing fields reject a node only when that node has no rest marker. Concrete-syntax violations such as duplicate or non-final rest are rejected before this semantic relation is selected.

Field presentation order is not field lookup priority. Nominal record declaration identity and structural field order remain owned by `types.md`.

## Direct field accessibility at every depth

Every explicitly selected pattern field, including fields selected inside nested nodes, independently consumes the direct field-accessibility relation from `field-access.md`.

For a pattern operation in a function belonging to source module `C`, every resolved record-pattern node may denote a record declared in `C` or in another source module:

- when the node's nominal record is declared in `C`, its explicitly selected fields are directly accessible through the same-module branch of `field-access.md` regardless of whether an individual field is module-private or exported;
- when the node's nominal record is declared in another module, the record binding must already be exported through qualified head lookup and every explicitly selected field must independently have exported direct accessibility.

Field identity is resolved before accessibility. An explicit field key not declared by the resolved nominal record is an unknown field; a known explicitly selected field that fails the direct-accessibility relation is inaccessible. One invalidity class does not stand in for the other.

A field omitted by a node-local rest marker is not explicitly opened and therefore requires no direct field-accessibility check merely to be omitted. Consequently an exported foreign record containing module-private fields may be opened by a qualified rest-bearing pattern that selects only accessible exported fields. Explicitly naming one of that foreign record's module-private fields remains invalid. Without rest, the existing exhaustive relation still makes such a foreign record impossible to open when any required field is inaccessible. An exported zero-field foreign record requires no field-access check and may be opened with a source-valid qualified empty pattern or qualified rest-only pattern.

Recursive patterns may cross source-module boundaries repeatedly. A same-module outer record may contain a foreign exported record and open it with a qualified nested head. A foreign record may contain a record from the containing function's own module and open that field with the ordinary unqualified same-module head when exact nominal typing holds. A foreign record may also contain a record from a third module, which may be opened only through an applicable source-unit alias and exported qualified head. At every node, accessibility is recomputed from the actual current record's defining module and each explicitly selected field relative to the containing function module; no root-wide visibility decision is inherited by descendants. A node-local rest may omit inaccessible fields at that node without changing another node's lookup or accessibility obligations.

Pattern-head qualification does not qualify field names, import fields into a module namespace, or create a second pattern-specific visibility relation.

## Binding leaves and structural paths

Each binding leaf corresponds to exactly one complete structural source path from the top pattern root.

The path is formed by appending each resolved explicit field identity traversed from the top pattern node to that leaf. Its final type is exactly the selected leaf field's source type under `structural-ownership.md` and `types.md`.

In the irrefutable declaration, an explicit field target is either a binding leaf or a nested irrefutable record pattern, never both. Because no declared field may be explicitly selected twice, distinct binding-leaf paths in one valid pattern tree are pairwise structurally disjoint. Omitted fields and rest markers contribute no binding-leaf path.

Nested record-pattern nodes are static pattern structure, not independently produced values. Their intermediate paths are not automatically duplicated or consumed merely because pattern traversal enters them. A rest marker likewise creates no owned-value or safe-authority access operation.

## Binding-leaf source order

Binding-leaf source order is **depth-first traversal in concrete explicit pattern field order**:

1. visit the current record-pattern node's explicit fields in their written order;
2. a binding target contributes its binding leaf immediately;
3. a nested record-pattern target recursively contributes all of its binding leaves in its own explicit field order before traversal continues to the next sibling field; and
4. a rest marker contributes no binding leaf and no position to this order.

This order controls:

- pattern binding-value production order;
- pattern-introduced local declaration order; and therefore
- later reverse local-declaration cleanup order under `local-bindings.md` and `function-execution.md`.

This order does not replace record declaration structural order for remaining-ownership frontier selection. Omitted fields remain governed by structural declaration order only where existing remaining-frontier cleanup later selects them.

## Complete pattern validation before ownership consequences

The complete recursive pattern tree MUST validate before any pattern-owned duplicate/consume transition and, for a producer-backed declaration, before producer validation/evaluation may acquire source ownership consequences.

Before the declaration enters its ownership-production relation, validation establishes at least:

1. every unqualified/qualified pattern-head lookup, target accessibility/category, and exact top/nested nominal record type relation;
2. every explicit field identity, uniqueness fact, and either complete no-rest exhaustiveness or valid node-local rest authorization at every node;
3. direct field accessibility at every explicitly selected field;
4. every binding leaf's complete resolved structural path and exact source type;
5. the complete depth-first binding-leaf source order from explicit fields;
6. pairwise uniqueness of all binding leaf lexical keys across the entire tree; and
7. absence of an overlapping function-local shadow conflict for every binding leaf key against the pre-declaration lexical environment.

For a direct binding-root scrutinee, the additional availability and safe-authority compatibility prevalidation below is also completed against one shared pre-pattern state before any pattern-owned transition.

Omitted fields need no field-accessibility or safe-authority validation and produce no binding fact. A rejected recursive structure establishes no pattern binding and applies no pattern-owned ownership transition.

For a producer-backed declaration, a structurally invalid pattern does not validate/evaluate the producer merely to discover a later pattern error. Once the pattern is valid, its top nominal record type is the exact required type supplied to producer validation before producer execution begins.

A qualified record construction used as the producer-backed scrutinee is therefore accepted or rejected by the same exact nominal relation as any other producer. When the construction target and qualified top pattern head resolve to the same nominal foreign record and their independent target/field-accessibility requirements are source-valid, the construction may directly supply that pattern. The constructor remains exhaustive under its own owner even when the receiving pattern uses rest. When the nominal types differ, validation rejects before constructor initializer evaluation or ownership commitment.

## Pattern-introduced bindings

Every valid explicit binding leaf introduces one ordinary function-local binding under `local-bindings.md`.

For a leaf with key `b`, path `p`, and type `T = type(p)`:

- the new binding has one stable source-semantic binding identity;
- its lexical key is `b`;
- its declared source type is exactly `T`;
- it is immutable for assignment purposes in this revision; and
- its initial owned value is produced by the applicable direct-root or producer-transient leaf operation below.

A rest marker introduces no binding and participates in no duplicate-binding or local-shadow check.

All bindings introduced by one declaration enter scope together only after the **complete declaration** finishes successfully. None participates in lookup while the pattern structure, scrutinee, leaf ownership production, or producer-transient cleanup of that declaration is in progress.

Each established binding begins with complete structural ownership of its produced value under `local-bindings.md` and `structural-ownership.md`.

## Scrutinee categories and exact top type

The represented declaration has exactly two top-level scrutinee categories selected by concrete syntax. Validation MUST preserve the selected category.

### Direct binding-root scrutinee

A direct binding-root scrutinee is exactly one bare unqualified function-body identifier under `concrete-syntax.md`.

It resolves through the function-local value-binding precedence owned by `local-bindings.md` and MUST select one active parameter or ordinary local binding. Lookup MUST NOT bypass an active binding merely because a module declaration would be convenient.

Only when no active parameter/local binding resolves the key does existing same-module fallback occur. A selected module declaration is then the wrong category and the declaration is source-invalid.

The selected root binding's declared source type MUST equal exactly the nominal record type of the top pattern head, whether that head was resolved by same-module or qualified lookup.

A bare direct root is **not** ordinary whole-binding `IdentifierUse` production for this declaration. The root is not first duplicated or consumed as a complete value and no pattern scrutinee transient is created.

### Producer-backed transient scrutinee

A producer-backed scrutinee is exactly one syntactically non-bare producer admitted by `concrete-syntax.md`:

- a result-bearing direct call;
- a record construction; or
- a field-value use, using either its binding-root or bounded producer-backed receiver form.

`RecordConstruction` in this list is the one existing producer category and may use either its represented unqualified same-module target or its qualified cross-module target. Pattern-head qualification and pattern rest do not create a second construction or pattern scrutinee category.

The top pattern head's nominal record type is the exact required source type of the **complete scrutinee producer result**. Structural similarity to another record type is insufficient. A qualified construction of a foreign record may therefore satisfy a qualified top pattern exactly when both resolve to the same nominal record and the construction and pattern are independently source-valid.

For a producer-backed field-value scrutinee, that top required type constrains the field-value operation's final selected field result. It does not constrain the field-value operation's internal direct-call or record-construction receiver, whose own exact receiver type remains selected and validated under `field-access.md`. A qualified construction may therefore appear inside such a field-value receiver regardless of whether the final record result consumed by the pattern is same-module or foreign, provided the final selected type exactly equals the resolved top pattern type.

The producer is resolved/evaluated in the lexical environment that exists before any binding introduced by this pattern enters scope.

A successful complete producer yields one fully owned **pattern scrutinee transient** of the selected top record type. This transient:

- has no lexical key or source binding identity;
- is not source-addressable or a safe-reference target in this slice;
- does not participate in function-local lookup;
- is not an ordinary local or parameter; and
- exists only until the consuming pattern operation completes its success cleanup or mismatch cleanup.

On successful complete producer completion, the pattern transient begins as one structural owned-value root with an empty consumed-path state under `structural-ownership.md`.

When the scrutinee is a producer-backed field-value use, its internal **field-receiver transient** is not the pattern scrutinee transient. The field-value producer must first preserve its selected record result, clean and end the field-receiver transient under `field-access.md` and `function-execution.md`, and only then transfer that result into the distinct fully owned pattern scrutinee transient described here.

Boolean/integer literals are not producer-backed record scrutinee forms in this revision because their represented source types are intrinsic. This restriction does not define a general expression taxonomy.

## Direct-root prevalidation

For a direct binding-root declaration, every **explicit binding-leaf structural path** MUST be fully available under `structural-ownership.md` in one shared pre-pattern ownership state of the selected root binding.

Every leaf additionally consumes the canonical direct safe-authority compatibility relation from `references.md` in that same pre-pattern authority state:

- a leaf whose exact type is duplicable requires the **Shared requirement** for its complete path; and
- a leaf whose exact type is non-duplicable requires the **Exclusive requirement** for its complete path.

All binding-leaf availability and safe-authority requirements are checked against the same pre-pattern state before the first pattern-owned transition. Omitted fields and rest markers create no path-availability or authority-compatibility precondition.

Because valid binding-leaf paths are pairwise structurally disjoint, later source-ordered consumption of one valid leaf cannot invalidate another prevalidated leaf. Safe-authority compatibility is likewise prevalidated per leaf against the same incoming active-authority set; the pattern itself creates no safe authority.

A root safe authority overlaps every descendant binding-leaf path. Therefore a live replacement-capable root authority blocks direct pattern production from the original target according to the applicable Shared/Exclusive requirement even when a Shared child has reduced the parent's retained reference-relative authority.

A nested record-pattern node does not independently require its complete intermediate path to be fully available or satisfy a direct safe-authority requirement merely so static pattern recursion may enter it. Ownership-producing binding leaves are the paths that require both checks.

Consequently, a nested zero-field or rest-only record pattern contributes no binding leaf, performs no ownership/access operation, and adds no whole-path availability or safe-authority precondition merely because the record structure is named. This is the recursive analogue of the accepted top-level zero-field/rest-only direct-root no-op.

If any binding-leaf path is unavailable/partial or fails its applicable safe-authority requirement in the pre-pattern state, the complete declaration is source-invalid and applies no pattern-owned transition.

## Direct binding-root leaf ownership

After complete recursive structure and direct-root leaf availability/authority prevalidation have succeeded, process binding leaves strictly in retained depth-first source order.

For each leaf path `p` of exact type `T`:

- if `T` is duplicable under `types.md`, its prevalidated Shared requirement permits one owned duplicate of the complete value at `p`, leaving the root structural ownership state unchanged;
- if `T` is non-duplicable, its prevalidated Exclusive requirement permits transfer of exactly the complete owned value at `p` and the canonical successful-consumption transition from `structural-ownership.md` to the root binding.

No ancestor of `p` is independently duplicated or consumed by this operation. Structurally disjoint paths remain governed by their own structural state.

A field omitted by rest receives no pattern-owned duplicate or consume transition and no direct safe-authority access. Its ownership remains exactly whatever the direct root's pre-pattern structural state already establishes, subject only to independent selected leaf consumption on structurally related paths. Rest does not synthesize whole-root or ancestor consumption.

Exhaustive no-rest patterns retain the existing consequence that separately consuming every ownership-producing leaf may leave an empty remaining root frontier without synthesizing consumption of the empty root path.

A rest-only direct-root pattern has no binding leaf and performs no ownership or authority-requiring access transition. A direct-root pattern has no transient cleanup phase. Later use, assignment, and cleanup of the root binding consume its resulting structural ownership state through the existing owners.

## Producer-backed transient leaf ownership

After complete pattern structure is valid and the complete producer has successfully yielded the fully owned pattern scrutinee transient root, process explicit binding leaves strictly in retained depth-first source order.

For each leaf path `p` of exact type `T`:

- if `T` is duplicable, produce one owned duplicate of the complete value at `p` and leave the transient structural state unchanged;
- if `T` is non-duplicable, transfer exactly the complete owned value at `p` and consume `p` in the transient structural state through `structural-ownership.md`.

The pattern transient is not a safe-reference target in this slice, so these transient-local leaf operations introduce no additional direct-root safe-authority compatibility check. Any authority effects caused while evaluating the producer have already been validated by that producer's canonical owner.

Pattern leaf production itself is finite, non-faulting, and non-diverging after successful complete producer completion.

No ancestor or whole transient value is independently duplicated/consumed merely to begin or continue recursive destructuring. Omitted fields receive no leaf-production transition and remain owned in the transient until the canonical remaining cleanup below.

## Producer-backed transient remaining cleanup

After every explicit binding leaf has been produced, the producer-backed pattern scrutinee transient ends before the declaration finishes.

Its remaining cleanup frontier is exactly `frontier([])` from `structural-ownership.md` using the transient's final consumed-path state.

Therefore:

- if no binding leaf consumed any path—including a rest-only pattern—the frontier contains the complete transient root;
- fields omitted by rest remain represented in that frontier according to the existing recursive frontier relation;
- duplicable explicit leaves also leave their source-owned paths in the transient and therefore remain represented in the frontier where applicable;
- mixed nested consumption yields exactly the maximal still-owned disjoint structural subvalues in canonical recursive reverse record-declaration order;
- if all structurally owned subvalues have been transferred, the frontier is empty without synthesizing whole-root consumption; and
- zero-field and recursively zero-leaf frontier members remain source-owned facts even when faithful lower scalar cleanup is vacuous.

The pattern scrutinee transient frontier is cleaned exactly once by `function-execution.md` before the consuming pattern operation transfers control beyond that transient: before declaration bindings enter scope, before bounded-refutable success bindings enter their success block, or before bounded-refutable mismatch control begins. Rest introduces no second cleanup category, source order, or lifetime. A rest-only producer-backed irrefutable pattern therefore evaluates its accepted producer, establishes the ordinary fully owned pattern transient, produces no leaves, cleans that complete remaining transient through this existing relation, and introduces no bindings. This is a pattern-specific omission/cleanup relation, not a general arbitrary-value discard expression or statement.

A field-receiver transient internal to a producer-backed field-value scrutinee has already ended before this pattern transient exists. Pattern leaf consumption or omission therefore cannot alter, enlarge, or retroactively reselect the field-receiver cleanup frontier.

The former one-level special cases “complete transient”, “direct retained fields”, and “no cleanup” are consequences of this general structural frontier for one-level patterns; they are not a second authority.

## Zero-field and zero-leaf behavior

For a top-level zero-field nominal record `Empty`, the direct-root patterns:

```text
let Empty {} = root;
let Empty { .. } = root;
```

are both valid when head/root category and exact type requirements hold. Qualified zero-field foreign records may analogously use either spelling when qualified head lookup and exact root typing hold. Either form has no binding leaf, introduces no binding, performs no ownership/access operation, and imposes no whole-root availability or safe-authority requirement. Neither is implicit discard or whole-root use.

For a producer-backed top-level zero-field pattern, successful complete producer evaluation yields one complete owned empty-record pattern transient. With no leaf consumption, its remaining frontier is the complete root and that source ownership ends at declaration completion whether or not the node spelled rest.

For a nested zero-field or rest-only pattern, recursion likewise may contribute no leaf and no ownership/access operation. If it lies inside a producer transient, that zero-field or omitted subvalue remains part of the canonical remaining frontier unless some selected ancestor path was transferred by another accepted operation.

A non-duplicable binding leaf whose type is a zero-field or recursively zero-leaf record remains a real source consumption and requires the same direct-root Exclusive compatibility when sourced from a binding root. The ownership transition is retained even if faithful Core refinement has no scalar liveness/destruction event.

Pattern validity and ownership are never defined by lower scalar-leaf existence.

## Producer fault and divergence

Pattern structure is source validation and occurs before producer execution consequences.

If complete producer evaluation yields a defined fault before the pattern transient is established:

- no pattern leaf production occurs;
- no pattern-introduced binding enters scope;
- producer-internal transient cleanup follows the producer's existing owner;
- for a producer-backed field-value scrutinee, no pattern transient exists during receiver fault propagation, and any field-receiver transient that exists after receiver success is completed entirely inside that field-value producer before a result could reach this pattern;
- ownership/structural/reference-authority transitions already completed by producer evaluation remain effective; and
- the same defined fault continues under `function-execution.md`.

If complete producer evaluation diverges, no pattern leaf production, pattern binding establishment, or pattern-transient cleanup occurs merely because execution remains suspended. Producer-owned transients remain governed by the producer's divergence relation. A producer-backed field-value scrutinee may diverge only while its retained receiver producer is still evaluating; after receiver success its field-selection and field-receiver cleanup tail is non-diverging under the current source model.

Rest and omission add no producer evaluation step, fault reason, divergence point, or post-producer failure relation. After successful producer completion, explicit leaf production and remaining-frontier cleanup retain their existing finite/non-diverging classifications.

A qualified construction used as the scrutinee follows its ordinary construction fault/divergence relation after complete pattern and producer source validation. Qualified versus unqualified spelling of the already resolved pattern head does not add a fault, divergence, or cleanup path.

For source validation implementations, producer-backed validation must preserve this atomic source-validity boundary: failure after tentative consuming producer validation must not leave rejected-source ownership or authority state committed. A nested producer-backed field-value use independently preserves its own transaction boundary under `field-access.md`; pattern validity does not merge those transactions into one ownership domain.

## Declaration completion

For a valid direct-root pattern declaration:

1. complete recursive pattern field/rest structure and binding-leaf validation;
2. validate all explicit leaf paths and applicable Shared/Exclusive direct safe-authority requirements against one pre-pattern state;
3. produce every binding-leaf value in depth-first explicit-field source order; and
4. establish all new bindings together.

For a valid producer-backed pattern declaration:

1. complete recursive pattern field/rest structure and binding-leaf validation;
2. validate the complete producer using the pre-pattern-binding lexical environment and the top nominal record type as the exact required type of its final result;
3. only after complete producer validation succeeds, evaluate that producer;
4. if that producer is a producer-backed field-value use, finish its separate field-receiver transient lifecycle before this step yields the owned record result;
5. transfer the produced record into the fully owned pattern scrutinee transient;
6. produce every explicit binding-leaf value in depth-first source order;
7. clean the pattern transient's canonical remaining frontier—including omitted fields—exactly once; and
8. establish all new bindings together.

Only after the applicable sequence completes may the next body statement begin.

## Bounded single-success refutable record selection

The represented refutable pattern operation is one recursive named-field record pattern consumed only by the bounded single-success selection statement from `control-flow.md` and `concrete-syntax.md`. It reuses the exact nominal head lookup, field identity/accessibility, node-local rest/omission, structural path, binding-leaf, scrutinee-category, producer transaction, and qualified-name relations above. It does not change the existing irrefutable declaration.

A **refutable record-pattern node** has the same explicit nominal record head, explicit-field uniqueness/accessibility relation, no-rest exhaustiveness/rest-authorized omission relation, and optional final rest marker as an irrefutable record-pattern node. Its explicit field target is exactly one of:

- one ordinary binding leaf with the same binding identity/type relation as above;
- one nested refutable record-pattern node satisfying the same exact nominal field-type requirement as above; or
- one **literal-test leaf**.

A literal-test leaf is exactly one represented Boolean literal or one represented decimal integer literal. It introduces no binding. No one field target both binds and tests the same path in this revision. A source-valid refutable pattern MUST contain at least one literal-test leaf somewhere in the complete recursive tree; a zero-test tree is rejected in this operation rather than becoming an always-success selector for capability already owned by irrefutable destructuring.

Every literal-test leaf corresponds to the complete resolved structural path formed by its explicit field ancestry. Let `T` be that path's exact selected field type.

- a Boolean literal test is valid exactly when `T` is intrinsic `Bool`;
- a decimal integer literal test is valid exactly when `T` is one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64` and `literals.md` successfully materializes that literal under exact required type `T`;
- every other field type rejects the test leaf.

The selected field type supplies the literal's required type directly. This relation creates no default integer type, coercion, promotion, conversion, comparison-local type inference, or general pattern inference. The materialized literal is a static semantic constant with no ownership state and no runtime producer evaluation.

Literal-test source order is depth-first traversal in explicit field order, independently of binding-leaf production order: visit explicit fields in written order, contribute a literal-test target immediately, recursively contribute tests from a nested refutable node before the next sibling, and contribute nothing for a binding target or rest marker. Binding-leaf order remains the existing depth-first binding-only order above, with literal-test targets contributing no binding leaf or position to that binding order.

### Complete refutable-pattern validation before scrutinee effects

The complete refutable pattern tree MUST validate before any pattern-owned runtime test/duplicate/consume transition and, for a producer-backed scrutinee, before producer validation/evaluation may acquire source ownership consequences.

Validation establishes at least:

1. every head lookup/category/accessibility and exact top/nested nominal record relation;
2. every explicit field identity, uniqueness fact, direct accessibility fact, and no-rest exhaustiveness or rest-authorized omission relation;
3. every binding leaf path/type/key, complete binding-leaf order, lexical-key uniqueness, and shadowing fact;
4. every literal-test path, exact field type, admitted literal category, successful static literal materialization, and complete literal-test order; and
5. presence of at least one literal-test leaf.

A static pattern or literal failure commits no scrutinee producer-validation state and establishes no binding. Once the tree is valid, the top nominal record type supplies the exact complete producer result type exactly as for the irrefutable producer-backed pattern relation.

For a direct-root refutable selection, every explicit binding-leaf path retains the existing fully-available plus Shared/Exclusive compatibility requirement above. Every literal-test path additionally MUST be fully available and satisfy the canonical **Shared requirement** from `references.md`, all against the same pre-selection structural/authority state. Testing reads one duplicable intrinsic scalar and therefore does not consume the selected path. Nested static pattern nodes and omitted fields add no independent availability/authority requirement merely because traversal enters or omits them.

All direct-root binding and test path preconditions are discharged before the first dynamic literal test or binding transfer. Because the complete valid tree cannot select the same field identity twice at one node and nested targets remain structurally inside their selected field, a literal-test path cannot also be a binding path or contain a nested binding target beneath the same tested scalar. Distinct ownership-producing/test leaves are therefore structurally disjoint where simultaneous path operations exist.

### Two-phase match execution and no rollback

After successful complete scrutinee acquisition, execute the refutable pattern in exactly two pattern-local phases.

**Phase 1 — literal tests only.** Process literal-test leaves in retained literal-test source order. For each test path:

1. obtain one non-consuming duplicate of the exact `Bool` or fixed-width-integer path value;
2. compare it with the statically materialized literal through the accepted exact Boolean or fixed-width integer equality value relation;
3. leave the scrutinee structural ownership state unchanged;
4. if the values differ, select mismatch immediately and perform no later literal test or binding production;
5. if they are equal, continue to the next literal test.

After the complete scrutinee exists, each bounded literal test is finite, non-faulting, non-diverging, and ownership-neutral. A mismatch therefore occurs before any pattern binding value has been duplicated or transferred.

**Phase 2 — binding production after full match only.** Only when every literal test succeeds, process ordinary binding leaves in the existing binding-leaf source order and apply exactly the existing direct-root or producer-transient duplicate-versus-consume relation above. No second testing pass occurs.

This ordering is the canonical no-rollback boundary. Mismatch never undoes a binding transfer, recreates consumed structural ownership, copies a non-duplicable value, or creates a runtime moved-state/drop-flag mechanism.

### Refutable direct-root success and mismatch

For a direct-root scrutinee, literal testing performs only non-consuming duplicates under the prevalidated Shared requirements.

On mismatch:

- no binding leaf has been produced;
- no pattern binding exists;
- the pattern operation contributes no structural ownership transition to the direct root; and
- the direct-root structural/authority state is exactly the state that existed immediately before dynamic pattern testing.

On full match, binding leaves are produced through the existing direct-root relation. Duplicable binding leaves preserve root structural state; non-duplicable binding leaves consume exactly their prevalidated paths. Those successful ownership transitions are real enclosing state and are not undone when the success block later ends.

### Refutable producer-backed success and mismatch

A producer-backed refutable selection reuses exactly the existing producer-backed scrutinee categories and evaluates the selected complete producer exactly once. Any producer-internal field-receiver transient finishes before the resulting record enters the distinct pattern scrutinee transient, exactly as above.

If complete producer evaluation faults or diverges before the pattern transient exists, the existing producer fault/divergence relation applies: no literal test, binding production, selection arm, or pattern-transient cleanup begins merely because this is a refutable receiving position.

After successful producer completion, the pattern transient begins fully owned. Literal tests are non-consuming duplicates from that transient.

On mismatch, no binding transfer has occurred, so the transient still has its initial empty consumed-path set. Its remaining frontier is therefore exactly the complete root. `function-execution.md` ends that transient exactly once before mismatch control begins.

On full match, ordinary binding leaves duplicate/consume through the existing producer-transient relation, then the final remaining frontier is selected from the post-binding consumed-path state and cleaned exactly once by `function-execution.md` before success bindings enter their block.

Producer effects completed before the pattern transient exists remain effective on both match outcomes. The pattern operation performs no rollback or producer re-evaluation.

### Refutable binding scope and pattern-local completion

All success binding identities are statically established during pattern validation, but none enters lexical scope during scrutinee evaluation or literal testing. After full match, all binding-leaf values are produced, applicable producer-transient cleanup completes, and then all success bindings enter scope together for exactly the success child block owned by `control-flow.md`/`local-bindings.md`.

Mismatch establishes no success binding. An explicit mismatch block is a sibling child scope and cannot resolve a success-only binding merely because its static identity was known during validation. With omitted mismatch, no synthetic binding scope exists.

The pattern operation itself supplies only `match` versus `mismatch` plus the exact pattern-owned state described here. `control-flow.md` owns arm execution, abnormal completion, omitted mismatch fallthrough, and exact normal-successor composition. In particular, this pattern owner does not normalize a direct-root success state to equal mismatch after a non-duplicable transfer.

## HIR and lowering refinement boundary

This specification does not prescribe implementation data structures, but a faithful typed HIR must retain enough source-selected facts that lowering does not repeat source semantics.

At minimum retain:

- the top nominal record identity;
- direct-root versus producer-backed scrutinee category;
- for a direct-root scrutinee, the resolved root binding identity;
- for a producer-backed scrutinee, the validated typed complete producer, including any field-value producer's own retained receiver/path/consequence/cleanup facts through its canonical owner;
- all explicit binding leaves in depth-first source order;
- each binding leaf's complete resolved structural field path from the top root;
- each new binding identity/key/exact type;
- each leaf's source-selected duplicate-or-consume consequence; and
- for producer-backed scrutinees, the final source-selected pattern-transient remaining cleanup frontier paths in canonical order.

For direct-root patterns, source validation MUST discharge each leaf's applicable Shared/Exclusive compatibility requirement before lowering. Lower representation MUST NOT reconstruct source direct-access legality from Core alias state.

The concrete rest marker and the set of omitted field identities need not survive in typed HIR after complete source validation. Their semantic consequences are already discharged into the accepted explicit leaf set, direct-root ownership result, and producer cleanup frontier. Retaining the marker or omitted identities for diagnostics/tooling does not create a lower semantic requirement.

Pattern-head qualification is discharged by source validation. Nested pattern heads are static validation structure; after every nested exact nominal type relation is proven, their resolved identities are already represented by the retained top record identity, full leaf paths, and retained leaf types. A faithful HIR therefore need not retain qualified versus unqualified head spelling or an additional nested-head identity solely for this feature.

Compiler temporary identity is not source-semantic pattern identity. If the retained producer is a record construction, its own source validator may discard whether its target spelling was qualified after retaining the resolved nominal record identity and initializer facts; pattern HIR requires no duplicate qualification fact.

The former one-level HIR boundary that retained only one direct field index per leaf and `None / Complete / DirectFields` transient cleanup is insufficient for recursive patterns and must not remain as parallel semantic authority.

No Core semantic change is required by this pattern relation. Existing structural projections and projected `Copy`, `Move`, and `Drop` can refine retained arbitrary leaf/cleanup paths after source validation.

Direct-root lowering can remain direct from the mapped source binding with no whole-record pattern temporary and emits projected source operations only for retained explicit binding leaves. Producer-backed lowering can reuse the existing complete producer result temporary as the pattern transient refinement; it does not need a source-visible synthetic binding. When the complete producer is a producer-backed field-value use, its receiver temporary and source-selected receiver cleanup finish first, and only its preserved selected result transfers into the separate pattern-scrutinee temporary/state. Producer-backed lowering then refines the already retained remaining cleanup frontier, including any omitted paths, through existing destruction.

For bounded refutable selection, typed HIR additionally MUST retain every literal-test leaf in literal-test source order with its complete resolved path, exact admitted scalar type, and statically materialized semantic literal value; whether the pattern has the bounded refutable-selection category; and enough producer-transient cleanup facts to distinguish complete-root mismatch cleanup from post-binding success cleanup. Source validation MUST discharge every direct-root literal-test Shared requirement before lowering.

A faithful bounded-refutable lowering performs all retained literal tests before emitting any retained binding-leaf `Copy`/`Move`. Fixed-width integer tests refine through an applicable projected/direct scalar `Copy`, exactly one accepted typed Core `IntegerEq`, and existing Bool `Branch`/`Goto` control flow. Boolean tests may refine through the existing Bool equality/branch relation without a new predicate operation. Producer mismatch cleanup uses the complete transient frontier; producer success cleanup uses the retained post-binding frontier. No Core `Match`, switch, generic predicate, pattern-test operation, `IntegerNe`, rollback operation, runtime discriminant object, or source state lattice is required.

Lowering MUST NOT reconstruct pattern-head lookup/accessibility, whether a source node used rest, omitted field identities, no-rest exhaustiveness, binding-leaf order, literal-test order/type/value, source duplicability, path availability, consumed paths, direct safe-authority compatibility, field-receiver frontier membership, pattern-transient remaining-frontier membership, or match/mismatch cleanup selection from Core liveness/copyability/alias state. Zero-leaf source cleanup may refine to no Core `Drop` where the lower destruction domain is empty.

## Future compatibility boundary

The explicit field-target form permits later pattern categories to extend the right side of a field entry without changing the accepted binding-leaf or nested-record spellings. The bounded rest marker occupies only the node-level omission role defined here and does not become a field target.

This revision does not define shorthand field binding, `_` wildcard/ignore bindings, tuple/array/enum patterns, top-level scalar patterns, floating literal tests, alternatives, guards, multi-arm `match`/`case`, reference/borrow binding modes, mutable pattern bindings, destructuring assignment, arbitrary pattern scrutinees, general expressions/grouping, qualified binding leaves, qualified field names, nested module paths beyond the represented alias/member pair, pattern-specific visibility modifiers, ranges, constructor spread/update, or general spread syntax.

Later pattern features must extend rather than reinterpret the direct-root and producer-backed recursive semantics accepted here. A future variant/range/multi-arm pattern feature must preserve the exact nominal and qualified-lookup semantics of represented record heads and the accepted no-rollback ownership boundary rather than silently replacing them with structural/runtime member lookup or maybe-owned state. A future wildcard or spread feature must not reinterpret this node-local rest marker as a produced value or binding.

## Source/Core separation

Pattern ownership is source semantics over nominal record/field identities, structural paths, source type duplicability, binding identity, binding/test source order, node-local omission, direct safe-authority compatibility, literal-test equality, match/mismatch state, and pattern-transient ownership.

A field-receiver transient used internally by a producer-backed field-value scrutinee is owned by the field-value operation, not by pattern semantics. Only the completed selected record result crosses into the pattern ownership relation.

Core projections, path liveness, scalar copyability, lower alias state, destruction domains, compiler local numbering, physical offsets, backend storage, construction-target qualification, pattern-head qualification, and the erased concrete rest marker are not source pattern authority.

A faithful implementation may map retained resolved paths to lower projections only after source validation has selected every source-semantic fact above.