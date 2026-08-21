# Source Record Construction

Status: **provisional normative; incomplete**

This document owns the represented source semantics for constructing one complete owned value of an already-declared nominal record type. It owns constructor-target selection, constructor-field identity and completeness, field-initializer required typing, initializer evaluation order, transient field-value ownership, construction-specific fault and divergence consequences, and successful completed-record formation.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), same-module module binding and lookup from [Source names and modules](names-modules.md), nominal record identity, field identity/order/type, source type equality, and owned-value duplicability from [Source type foundation](types.md), literal value production from [Source literal semantics](literals.md), ordinary whole-binding owned-value use from [Source function-local bindings](local-bindings.md), and general owned-value transfer, cleanup, direct-call, return, fault propagation, and straight-line execution from [Source function execution](function-execution.md). The represented concrete constructor spelling is owned by [Source concrete syntax](concrete-syntax.md). This document does not redefine those owners.

This revision does not define member access, field extraction, field assignment, partial field moves, member-level availability, a general source place relation, patterns, destructuring, field accessibility across modules, or an implementation representation.

## Record-construction value producer

A **record construction** selects exactly one already-declared nominal record source type and, after all required field initializers have completed successfully, produces exactly one complete owned source value of that nominal type.

Record construction is an owned-value producer. A successful construction may therefore be consumed by any represented source context whose grammar admits a `Value` and whose semantic owner accepts the constructed record type, including local initialization, whole-binding assignment, direct-call arguments, result-bearing return, and another record field initializer.

Construction does not create an anonymous structural record type, an untyped aggregate value, a conversion, a coercion, a defaulting relation, or a separate record value category. The produced value has exactly the selected nominal record type under `types.md`.

## Constructor target selection

The constructor target is one lexical identifier key supplied by the concrete `RecordConstruction` form in `concrete-syntax.md`.

For this represented construction relation, target selection performs **same-module module-scope lookup** directly under `names-modules.md` for the source unit containing the construction.

The selected module binding MUST denote one nominal record source type. If the selected binding denotes another module-level entity category, construction is source-invalid. Lookup MUST NOT skip that selected binding to search for a record declaration preferred by the constructor context.

The constructor target does not participate in the function-local value-binding lookup relation owned by `local-bindings.md`. Consequently, an active parameter or ordinary local binding whose lexical key equals the constructor target key does not hide, replace, or otherwise alter the same-module record target selected for `Name { ... }`.

Same-module lookup remains order-independent under `names-modules.md`. Reordering source units, record declarations, function declarations, or function-local declarations therefore MUST NOT change which same-module record declaration a constructor target denotes.

This revision defines no qualified `alias::Record { ... }` construction. Although `names-modules.md` defines exported module bindings and qualified cross-module lookup, no accepted source owner yet defines whether or how another module may initialize a record's fields. Cross-module record construction therefore remains undefined rather than inferring field-construction accessibility from module-level record accessibility.

## Constructor field identity

After one nominal record type has been selected, every constructor-field key is resolved only within that selected record declaration's field sequence from `types.md`.

A constructor-field key identifies the unique field of the selected record whose lexical identifier key is equal under `lexical.md`. Field identity remains scoped by the containing nominal record. Equal field keys in another record declaration do not participate in this resolution.

Constructor-field resolution is not module lookup, member lookup, method lookup, associated-item lookup, overload resolution, or function-local value lookup.

The source presentation position of a constructor field is not its semantic field identity. The semantic field identity remains the field identity established by the selected record declaration.

## Complete named-field construction

A source-valid record construction MUST supply exactly one initializer for every field declared by the selected nominal record type.

Therefore:

- an initializer key that denotes no field of the selected record is source-invalid;
- two or more initializers that denote the same selected-record field are source-invalid;
- omission of any declared field is source-invalid;
- every declared field is present exactly once in a valid non-empty construction;
- a record declaration with zero fields is constructed by the represented empty constructor form with zero initializers.

Constructor fields are named, not positional. Their concrete source presentation order MAY differ from record declaration-field order.

The completed record value nevertheless has the exact field identities, field source types, and semantic structural order established by the nominal record declaration under `types.md`. Reordering constructor entries does not reorder the record declaration or create a distinct record type.

This revision defines no positional constructor, implicit field shorthand, default field value, omitted-field inference, record update/spread/base operation, anonymous record value, or structural compatibility rule.

## Field-initializer required types

For each constructor field, the field's declared source type under `types.md` is the unique **required source type** supplied to that field's initializer producer.

The initializer MUST successfully produce exactly that source type under the applicable producer's accepted semantics.

Consequently:

- a represented boolean literal is valid only when the selected field requires `Bool`, under `literals.md`;
- a represented decimal integer literal materializes directly under the selected field's required fixed-width integer type through `literals.md` and is invalid for a non-integer required type;
- ordinary whole-binding owned-value use remains governed by `local-bindings.md` and MUST produce exactly the selected field type;
- a result-bearing direct call MUST produce exactly the selected field type under `function-execution.md`;
- another represented record construction may recursively serve as a field initializer and intrinsically produces exactly its own selected nominal record type.

The completed construction has exactly the selected outer nominal record type. Any receiving local, assignment target, direct-call parameter, return result, or outer record field continues to require exact source type equality under its existing owner.

Field-initializer contextual typing introduces no type inference, integer defaulting, subtyping, conversion, coercion, promotion, widening, narrowing, or structural record compatibility.

## Initializer evaluation order

Constructor-field **source presentation order is semantic initializer evaluation order**.

For constructor entries presented as `e0`, `e1`, ..., `en`:

1. evaluate `e0` first;
2. begin `ei+1` only after `ei` has completed successfully and produced its owned field value;
3. preserve every source-semantic ownership or binding-availability transition caused while evaluating `ei` before evaluating `ei+1`;
4. continue until all presented initializers have completed successfully or one initializer faults or diverges.

Record declaration-field order MUST NOT reorder initializer evaluation. Declaration-field order remains the completed record's structural order under `types.md`; constructor presentation order independently controls evaluation effects and transient production order.

A record construction with zero fields performs no field-initializer evaluation.

## Transient field values

Every successfully evaluated field initializer produces exactly one owned **transient field value** of that field's declared source type.

Until all field initializers have completed successfully, each previously produced transient field value remains independently owned by the in-progress construction.

A transient field value:

- is not yet a field of a completed source record value;
- is not a parameter or ordinary local binding;
- is not source-addressable storage;
- does not create a source place, member, or field-access capability;
- cannot be named or otherwise selected by a later initializer merely because it has already been produced.

The in-progress construction owns all such transient field values until they are either cleaned because construction terminates abnormally or transferred exactly once into the completed record value after all initializers succeed.

Transient field ownership is semantic. It does not require a compiler or runtime to materialize one physical temporary object for each field.

## Successful record formation

After every required field initializer has completed successfully, construction performs one semantic aggregate-formation boundary:

1. associate every independently owned transient field value with the exact selected-record field identity resolved for its initializer;
2. transfer each transient field value exactly once into that field of one new complete owned record value;
3. end the construction's independent ownership of those transient field values; and
4. produce the complete owned value of the selected nominal record type.

Aggregate formation does not duplicate a transient field value.

Aggregate formation after successful initializer evaluation is non-faulting and non-diverging in this represented relation. It is not specified as a sequence of source-visible field assignments or writes, and this document defines no source-observable ordering among the conceptual transfers performed by aggregate formation.

The record declaration's field order remains the semantic structural order of the completed value. Constructor presentation order remains only the already-completed evaluation/production order.

For an empty nominal record, successful construction directly produces its complete empty record value.

## Defined fault during construction

If evaluation of constructor initializer `ei` yields a defined fault before successful aggregate formation:

1. no completed record value is produced;
2. no later constructor initializer is evaluated;
3. transient field values already produced by earlier initializers are cleaned exactly once in **reverse production order**;
4. binding availability and other ownership transitions already caused while evaluating earlier initializers remain effective; and
5. the same defined fault continues under the fault-propagation relation in `function-execution.md`.

Reverse production order is determined by constructor source presentation order, not by the selected record's declaration-field order, because no completed aggregate exists at this point.

Cleaning a transient field value uses the source cleanup relation owned by `function-execution.md`; this document selects the construction-specific values and their cleanup order but does not redefine structural destruction mechanics or custom destructors.

This revision introduces no constructor-specific fault identity, payload, exception object, catch boundary, or recovery form.

## Divergence during construction

If evaluation of constructor initializer `ei` diverges:

- no completed record value is produced;
- no later constructor initializer is evaluated;
- previously produced transient field values remain owned by the suspended construction;
- binding availability and other ownership transitions already caused by earlier initializers remain effective; and
- no cleanup occurs merely because execution has continued indefinitely.

This relation does not introduce an implicit step budget or timeout.

## Completed-record ownership and duplicability

After successful formation, the record construction yields an ordinary owned source value of the selected nominal record type.

Its owned-value duplicability classification is exactly the classification owned by `types.md`. Construction neither grants nor removes duplicability.

The represented concrete record declaration currently makes no positive duplicability selection. Consequently, a record value constructed from such a declaration is non-duplicable under the existing no-selection rule even when every field type is duplicable.

When a completed record value is later used through ordinary whole-binding owned-value use, `local-bindings.md` remains authoritative for duplicate-or-consume behavior. When it is transferred through a local initializer, assignment, direct call, or return, `function-execution.md` remains authoritative for the receiving transfer relation.

When a completed record value is later selected for source cleanup, `function-execution.md` owns source cleanup selection and ordering. Applicable structural destruction domains and reverse declaration-field destruction order in a Core realization remain owned by [Core value and storage semantics](../core/value-storage.md). Construction does not define a custom destructor or a source-level field cleanup operation.

## Composition with receiving value contexts

Record construction owns only its producer-specific evaluation and ownership relation. Existing consumers retain their authority.

### Local initialization

A record construction used as an ordinary local initializer completes successfully before its produced record value is transferred into the new binding under `function-execution.md`.

The local's declared source type remains the receiving required type and MUST exactly equal the constructed nominal record type.

### Whole-binding assignment

A record construction used as an assignment RHS completes its complete construction relation before the existing source-first replacement boundary in `function-execution.md` begins.

A field initializer may itself duplicate or consume the assignment target through ordinary whole-binding use. Those availability transitions occur during construction and remain effective when assignment later determines whether an old target-owned value remains to clean.

If construction faults, the assignment performs no replacement cleanup or replacement transfer beyond the ownership transitions already caused while evaluating the constructor fields, as required by the existing assignment relation.

### Direct-call arguments

A record construction used as one direct-call argument completes before its produced record value becomes the call relation's transient argument value.

If a later call argument faults, cleanup of that already-completed record argument is governed by the existing direct-call transient-argument cleanup relation. The construction-specific reverse field-transient cleanup rule applies only while the record itself has not yet completed formation.

### Return

A record construction used as a return value completes before the existing return cleanup and transfer boundary. The produced record value is then the owned transient result governed by `function-execution.md`.

### Nested record construction

A record construction may be used as another construction's field initializer. The inner construction completes first under this document. Its completed owned record value then becomes one transient field value of the outer construction.

If a later outer initializer faults, the completed inner record value is cleaned as one already-produced outer field transient; its own successful construction is not retroactively reverted.

## Member-access and partial-availability boundary

This revision deliberately defines construction without member access.

It does not define `value.field`, field extraction, field borrowing, field assignment, a general source place path, or another member-selection form.

In particular, this revision does not introduce a temporary member-access rule restricted to duplicable fields. Such a restriction would assign semantics to a future general member form without defining the non-duplicable case and would prejudge later move/borrow interactions.

A future ownership-moving extraction of a non-duplicable field requires independently accepted source semantics for at least:

- the containing binding's source state after partial extraction;
- availability of disjoint fields;
- whole-record use after partial extraction;
- assignment or reinitialization of the complete binding after partial extraction;
- cleanup of fields that remain owned;
- nested field paths; and
- interaction with future borrow/reference member access.

Core partial initialization, field projections, and sub-place moves demonstrate lower-level representability but are not source-language authority for those relations.

## Cross-module construction boundary

This revision defines only same-module record construction.

A future qualified constructor form may consume the existing module-alias and exported-binding relation from `names-modules.md`, but it MUST also establish an explicit source rule for whether and how the target record's fields are constructible from another module.

Module-level `export` of a record declaration does not, by itself, define field-construction accessibility. This document therefore does not infer cross-module construction permission from record export status.

This boundary does not introduce a general field-visibility mechanism for same-module construction, member access, methods, ABI layout, or another concern.

## Implementation boundary

This document does not prescribe parser nodes, syntax-tree identity, HIR representation, Core MIR operations, compiler temporaries, physical aggregate layout, runtime storage, ABI passing, or backend realization.

An implementation MUST preserve the semantic distinction between:

- constructor source presentation order, which governs initializer evaluation and transient production order; and
- nominal record declaration-field order, which governs completed record structural order.

An implementation MUST NOT move an earlier produced source field value into a partially completed aggregate in a way that changes the required construction-specific cleanup behavior if a later initializer faults or diverges.

Existing lower-level structural aggregate operations may be used only when their refinement preserves the complete source relation above.

## Further boundaries

This revision does not define:

- qualified cross-module construction or a general field-accessibility system;
- member access, field extraction, field assignment, partial moves, member-level availability, or general source places/lvalues;
- positional construction, field shorthand, default fields, record update/spread/base forms, or anonymous structural records;
- positive record duplicability-selection syntax;
- patterns or destructuring;
- floating literals;
- unary or binary operators, precedence, grouping, or a general expression grammar;
- nested blocks, branches, loops, joins, or early return;
- references, borrowing syntax, lifetimes, source interior mutability, raw pointers, or source `unsafe`;
- indirect calls, function values, closures, or captures;
- generics, traits, coherence, methods, or overloads;
- const/static semantics;
- panic payload or catch syntax;
- ABI, layout, FFI, or linkage;
- Exec or Model source forms;
- package/filesystem/dependency mapping;
- runtime or backend behavior.

Those concerns require their own canonical owners when an accepted consumer requires them.
