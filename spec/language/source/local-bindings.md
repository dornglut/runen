# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for body-local binding identity, lexical scope and local-first lookup precedence, binding assignment mutability, binding lifecycle, ordinary whole-binding owned-value use, whole-binding assignment legality, bounded binding-root field assignment/reinitialization legality, safe-reference/raw-pointer/function-value/closure binding contextual integration, and the points at which a binding's structural ownership state begins, persists, resets, or ends.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), concrete source value types and owned-value duplicability from [Source type foundation](types.md), captureless function-value use/call-target facts from [Source function values and indirect calls](function-values.md), dedicated closure/capture binding establishment and opaque-closure call-target facts from [Source closures and explicit by-value capture](closures.md), module-level constant binding/value semantics from [Source constants](constants.md), immutable persistent static binding/read semantics from [Source immutable execution statics](statics.md), first-slice abstract type-parameter expressions and their capability-conservative whole-value use rule from [Source generics](generics.md), structural paths, structural ownership state, path availability, consumption, remaining-ownership frontiers, complete-root replacement reset, bounded non-empty subpath installation, opaque abstract roots, and opaque closure roots from [Source structural ownership](structural-ownership.md), callable parameter-slot type expressions from [Source callables](callables.md), safe-reference target/authority/carrier/lifetime and direct safe-authority compatibility rules from [Source safe references](references.md), and raw-pointer contextual admission, pointer-origin provenance, lexical target validity, and raw pointee operations from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md). It does not redefine those owners.

Represented binding-rooted field-path selection, direct field accessibility, final-field duplicate-or-consume value production, and bounded assignment-target field-path resolution/accessibility are owned by [Source field-value access](field-access.md). Represented recursive record-pattern selection, including bounded node-local rest/omission, and pattern-specific binding production are owned by [Source patterns](patterns.md). Represented source-function body attachment, source-function activations, shared call/producer transactions, owned argument/result transfer including safe-reference carriers and replacement-capable external referents, local initialization, whole-binding and bounded field-assignment replacement ordering, reference-relative replacement ordering, normal-continuation presence, lexical-scope and activation cleanup, return, recursion, divergence, defined-fault propagation, raw-operation execution ordering, and constant-value producer integration are owned by [Source function execution](function-execution.md). Closure activation creation, held-environment transfer, capture-binding establishment timing, closure-specific cleanup composition, and closure-call snapshot ordering are owned by `closures.md` while consuming the existing execution relations. Generic application, exact substitution, and the non-observable generic activation substitution context are owned separately by [Source generics](generics.md); the ordinary execution relation consumes the instantiated type facts supplied by that owner. Represented conditional selection, zero/one/two normal-outcome composition, bounded `while` condition/body selection, structural-state joins/backedges including active closure/capture roots and external referent roots, and raw-pointer-origin joins/backedges are owned by [Source control flow](control-flow.md). Concrete parameter/local/pattern/closure/value/call/field-value/assignment/block/conditional/while/return/reference/raw-pointer/unsafe/generic/constant spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, closure-site/type/capture semantics, constant declaration/value production, generic parameter identity/substitution/capability semantics, safe-reference formation/dereference/reborrow/replacement/authority semantics, raw-pointer formation/pointee access/unsafe admission semantics, normal-continuation presence, conditional or loop selection/successor composition, field lookup/accessibility, pattern structure, general expression evaluation, traits, ABI, Core liveness, or an implementation representation.

## Binding identities and body instances

A represented source-function body and a represented closure body each instantiate the lexical binding/scope relation defined by this document with their own root scope and active binding set. This is reuse of the existing relation in two concrete body cases; it does **not** introduce a third generic `BodyBinding`, `CallableBinding`, or `SourceCallableActivation` semantic object.

When a represented source function entity has a body under `function-execution.md`, that body has exactly one **parameter binding** corresponding to each callable-signature ordinary parameter slot.

When a represented closure site is invoked under `closures.md`, its closure body has exactly one explicit **parameter binding** corresponding to each explicit closure callable-interface parameter slot. These bindings use the same parameter-binding lifecycle/lookup rules below, but their containing activation is a closure activation rather than a source-function activation.

Each parameter binding has:

- exactly the source parameter type expression of its corresponding callable-interface slot, which may be one admitted abstract type parameter only inside a generic source-function body;
- one lexical identifier key governed by `lexical.md`;
- one stable source-semantic binding identity; and
- immutable assignment status.

Parameter lexical keys MUST be unique within their body root. For a closure body they additionally MUST be pairwise distinct from all capture-binding keys supplied by `closures.md`.

Parameter lexical keys, binding identities, and assignment-mutability classifications are body-local value facts. They are not callable-signature/interface identity or equality dimensions and are distinct from generic type-parameter slot identities/keys under `generics.md`.

A represented binding identity is independent of original identifier spelling, token/source offset, parser node, physical address, compiler collection index, HIR/Core identifier choice, runtime storage identity, generic type-parameter identity, closure-site identity, source safe-reference authority identity, or source raw-pointer origin provenance.

For source functions, concrete parameter source order maps to callable parameter-slot order. For closures, concrete explicit-parameter source order maps to the explicit closure-interface slot order from `closures.md`/`callables.md`. Every represented parameter binding is immutable for assignment purposes, including parameters whose concrete types are `SharedRef(T)` or `ExclusiveReplaceRef(T)` and source-function parameters whose declared type expression is an abstract generic type parameter. `RawPtr(T)` and opaque closure-site types are not parameter-admissible under `callables.md` in this slice.

A replacement-capable reference parameter's binding mutability remains immutable even though its reference permission permits complete-referent replacement. Parameter binding assignment and referent replacement are distinct semantic operations.

## Ordinary local declarations

A represented ordinary local declaration:

- belongs to exactly one lexical scope in the current source-function or closure body;
- introduces exactly one lexical identifier key and one stable local binding identity;
- has exactly one admitted declared source type expression;
- has exactly one initializer; and
- classifies the binding as immutable or mutable for assignment purposes.

An admitted declared source type expression is either one existing concrete ordinary-local-admissible source value type under the applicable type/reference/raw-pointer/function-value owners or, inside a generic source-function body, one bare in-scope abstract type parameter admitted by `generics.md`. An opaque closure-site type is not an ordinary declared local type because it has no general `Type` spelling and is established only by the dedicated closure declaration relation.

Uninitialized ordinary local declarations are not represented.

The initializer is resolved and typed in the lexical environment that exists before the new binding is introduced. `function-execution.md` owns initializer evaluation and transfer. The binding enters scope only after successful initialization completes and therefore cannot be selected by lookup from its own initializer.

The concrete forms in `concrete-syntax.md` establish immutable `let name: Type = Value;` and mutable `let mut name: Type = Value;` bindings. In a generic source function an admitted bare type-parameter key may fill that declared `Type` position according to `generics.md`; this remains an explicit declared type and does not introduce inference. The separate `let name = fn [captures] ...` closure declaration is not an inferred ordinary local. This revision defines no inferred ordinary local type or uninitialized local form.

Safe-reference local admission is deliberately immutable-only:

- `let name: &T = Value;` may establish an immutable ordinary local when `SharedRef(T)` is valid under `references.md`;
- `let name: &mut T = Value;` may establish an immutable ordinary local when `ExclusiveReplaceRef(T)` is valid under `references.md`;
- `let mut name: &T = Value;` is source-invalid; and
- `let mut name: &mut T = Value;` is source-invalid.

This immutable-only rule is a source reference-lifetime/authority boundary. It does not redefine the general assignment-mutability classification of non-reference locals and does not imply a hidden `const` or type-level mutability dimension. In particular, `&mut T` denotes replacement capability over the referent, not mutability/rebinding of the reference local that stores the carrier. An abstract type parameter is not a safe-reference type expression in this slice, so an ordinary source-function local declared with that abstract type may use either ordinary assignment-mutability class.

For represented raw-pointer types, both ordinary local mutability classes are admitted when `RawPtr(T)` is valid under `raw-pointers-unsafe.md`:

- `let name: raw T = Value;` establishes an immutable raw-pointer local; and
- `let mut name: raw T = Value;` establishes a mutable raw-pointer local whose stored pointer value may later be replaced by ordinary whole-binding assignment.

Raw-pointer local mutability applies only to the stored pointer value. It does not grant pointee mutation authority. Every raw-pointer initializer additionally MUST satisfy the pointer-origin lexical target-validity relation from `raw-pointers-unsafe.md` for the complete static extent of the receiving local.

A concrete captureless function-value type from `function-values.md` is an ordinary non-reference local type and may use either assignment-mutability class. Its initializer, whole-binding use, reassignment, and cleanup consume the ordinary relations here; function-value-specific identity/formation/indirect-call behavior remains owned by `function-values.md`.

After successful initializer transfer, the new local begins with one complete structural owned-value root of its declared type expression and the initial empty consumed-path state from `structural-ownership.md`. For an abstract type parameter that root is the opaque complete root defined by `structural-ownership.md` while consuming `generics.md`. When the initialized value is a safe reference, the local additionally stores the produced reference carrier whose authority/lifetime consequence is owned by `references.md`; when it is a raw pointer, source validation additionally retains the exact pointer-origin provenance owned by `raw-pointers-unsafe.md`. Neither relation introduces a second structural ownership state for the reference or pointer binding itself.

A replacement-capable reference value may also target a structural root external to the current activation. That external referent root is separate non-binding validation state owned by `references.md`; it is not the structural ownership state of the parameter/local binding that stores the reference carrier.

## Pattern-introduced local bindings

One source-valid record-destructuring declaration under `patterns.md` may introduce zero or more ordinary body-local bindings as one grouped declaration boundary in either a source-function body or a closure body.

For every pattern binding leaf, `patterns.md` supplies:

- the introduced lexical key;
- the exact selected source type; and
- the duplicate-or-consume production consequence that yields the binding's initial owned value.

`patterns.md` also supplies the complete declaration's retained binding-leaf source order. This binding owner uses that order as the declaration order of the introduced bindings.

This binding owner supplies each introduced binding with one stable source-semantic binding identity and classifies it as immutable for assignment purposes.

Before any pattern binding is introduced:

- all introduced lexical keys MUST be pairwise distinct across the complete pattern tree;
- every introduced key MUST satisfy the overlapping-shadow prohibition below against the pre-declaration lexical environment in the current body; and
- the complete declaration MUST have passed the pattern structure/type/accessibility validation owned by `patterns.md`.

If any introduced key is invalid, the complete declaration is rejected. It introduces no subset of the intended bindings and does not create a partially extended lexical environment.

All bindings introduced by one successful record-destructuring declaration enter scope **together after the complete declaration finishes**, including any producer-backed transient completion required by `patterns.md` and `function-execution.md`. None participates in lookup while that same declaration is validating or producing its binding values.

The binding-leaf source order defined by `patterns.md` is the declaration order of the introduced bindings for lexical cleanup composition. Pattern structure does not change nominal record structural field order from `types.md`.

Each successfully established pattern binding begins with one complete structural owned-value root of its exact binding type and the initial empty consumed-path state from `structural-ownership.md`.

Represented nominal record fields cannot have `SharedRef(T)`, `ExclusiveReplaceRef(T)`, `RawPtr(T)`, a captureless function-value type, an opaque closure-site type, or abstract function type-parameter type, so the represented record pattern relation cannot introduce a safe-reference, raw-pointer, function-value, closure-valued, or abstract-generic pattern binding in this slice. No borrow-binding, pointer-binding, closure-binding, or generic-pattern binding mode is implied.

A pattern with no binding leaves introduces no body-local binding and therefore does not change the lookup environment by itself.

## Dedicated closure bindings

A successful closure declaration under `closures.md` introduces exactly one **dedicated closure binding** in the ordinary non-generic source-function body containing that declaration.

The dedicated closure binding:

- has the exact opaque closure-site type established by that declaration;
- has one stable binding identity and one lexical key;
- is immutable for ordinary assignment;
- begins with one complete opaque structural root after successful transactional capture formation;
- enters scope only after all captures have been successfully produced and the closure value is established; and
- remains in scope through the end of its containing source-function lexical scope unless its whole value is consumed earlier.

It is not an ordinary local declaration, has no declared `Type` syntax, and does not create general type inference. It is not a source function entity or module binding.

A dedicated closure binding may be selected by `closures.md` as a bounded closure-call target. It may also be selected as an explicit by-value capture target of a later closure declared in the same ordinary source-function body. It is not a whole-binding assignment target, bounded field-assignment root, safe-reference referent, raw-pointer pointee, pattern-record root, or ordinary generally typed value receiving category merely because it owns a source value.

## Closure-activation capture bindings

Each bounded closure invocation under `closures.md` establishes one immutable **capture binding** for every capture slot in the new closure activation, in exact capture-list order after ordinary explicit parameter transfer and before body statement execution.

Each capture binding:

- has the lexical key written by the capture occurrence;
- has a stable closure-body-local binding identity distinct from the outer binding that supplied the captured value;
- has the exact captured concrete source type;
- begins with one complete structural owned-value root after transfer from the held closure environment; and
- is immutable for ordinary assignment.

Capture bindings are not ordinary local declarations and have no declared-type syntax. They participate in local lookup and ordinary owned-value use only where the consuming operation owner admits their category and exact type.

A capture binding of exact nominal-record type may be an immutable field-value receiver or direct pattern root under `field-access.md`/`patterns.md`. A capture binding of exact Shared-referent-admissible intrinsic/record type may be a Shared root under `references.md`. A capture binding of exact raw-pointee-admissible intrinsic/record type may be a raw-address target under `raw-pointers-unsafe.md`. Replacement-capable root formation and ordinary assignment remain unavailable because the capture binding is immutable.

A capture binding of exact captureless function-value type retains that exact type and may be a bounded-indirect call target under `function-values.md`. A capture binding of opaque closure-site type, produced by later-closure capture of an earlier dedicated closure value, is **not** a bounded closure-call target in this slice. That selected binding remains category-final and does not fall through to a same-named module function.

## Lexical scopes in source-function and closure bodies

A represented source-function body has one root lexical scope. A represented closure body has a **separate fresh root lexical scope** for each closure site/activation under `closures.md`. The two body cases instantiate the same lexical-tree relation but are not one shared lexical ancestry.

A represented nested block establishes one child lexical scope of its containing body/scope. The resulting lexical scopes form one finite rooted tree per body.

The root body braces in `concrete-syntax.md` delimit the root lexical scope. Each concrete `BlockStatement` establishes exactly one child lexical scope containing its enclosed `BodyStatement` sequence and optional terminal `ReturnStatement`, and ending at that block's closing boundary. Recursively nested block statements therefore establish descendant lexical scopes. An `unsafe` block is one such ordinary child lexical block plus the separate unsafe-admission fact owned by `raw-pointers-unsafe.md`.

Each explicit represented conditional arm is one ordinary `BlockStatement` and therefore one child lexical scope. A then arm and explicit else arm of the same conditional are sibling scopes. An omitted else introduces no synthetic lexical scope under `control-flow.md`. Each represented `while` body is likewise one ordinary `BlockStatement` child scope; repeated dynamic iterations re-enter that same static source scope rather than creating new source binding identities.

The semantic scope tree does not prescribe parser nodes, source ranges, HIR scope identifiers, Core blocks, physical storage lifetime, or a physical address for a borrow/raw-pointer target.

A parameter binding belongs to the current body's root scope and is in scope throughout that body, including descendant lexical scopes while those scopes are active.

In a source-function body, ordinary locals, pattern bindings, and dedicated closure bindings are in scope from immediately after successful establishment through the end of the containing lexical scope. In a closure body, ordinary locals and pattern bindings have the same rule; capture bindings are seeded in the closure root by activation establishment and remain active for the closure activation unless consumed as owned values.

An uncaptured creator source-function binding is **not** an ancestor binding of the fresh closure-body root and never participates in closure-body local lookup. A closure parameter/body local may therefore reuse such a lexical key if no binding active in the closure's own scope tree has that key.

A return terminates the current activation under `function-execution.md`/`closures.md` rather than creating a later point in the ended scope. Closure-body return terminates the closure activation, not the creator source-function activation.

These containment/cleanup relations are consumed by `references.md` to prove represented implicit lexical safe-reference lifetime and by `raw-pointers-unsafe.md` to require that every pointer local's target extent contain that pointer local's complete extent. Child safe-reference locals end before their earlier parent/reference target extent. Creator reference/raw-pointer/unsafe scope is not inherited by a closure activation merely because the closure was declared inside it.

This binding owner does not add lifetime names, authority sets, pointer-origin sets, generic type-parameter scopes, or a third generalized scope tree; generic type-parameter scope is owned separately by `generics.md`.

## Overlapping shadowing and key reuse

**Overlapping body-local shadowing is forbidden within one body root/scope tree.**

A parameter, ordinary local, pattern binding, dedicated closure binding, or closure capture binding establishment MUST NOT introduce a lexical identifier key equal to the key of another active binding whose lexical scope contains the establishment point, except that capture and explicit-parameter uniqueness is prevalidated as one closure-root establishment boundary under `closures.md`.

For one grouped record-destructuring declaration, this requirement applies to every binding leaf against the pre-declaration lexical environment, and all binding-leaf keys in that declaration MUST also be pairwise distinct.

Consequently within one body:

- a local cannot shadow a parameter or active capture binding;
- a nested local cannot shadow an enclosing local or dedicated closure binding;
- a dedicated closure binding cannot reuse an overlapping parameter/local/pattern/closure key;
- capture keys and explicit closure parameter keys must be pairwise distinct;
- two sequential locals in the same continuing lexical scope cannot reuse one key;
- two bindings introduced by one pattern cannot share a key; and
- disjoint sibling lexical scopes, including explicit sibling conditional arms, MAY independently introduce the same key because their binding scopes do not overlap.

Because a closure body has a fresh root and uncaptured creator bindings are absent from it, an uncaptured creator key may be reused by a closure explicit parameter or body local without violating this rule. Explicitly captured keys are present through capture bindings and therefore do participate in the closure-body no-overlap rule.

This prohibition applies only inside the current body's local value-binding domain. A body-local binding key MAY equal a module-level declaration key, including a module constant/function key, and MAY equal a source-unit module-alias key. Module-level equality is handled by local-first lookup below; an alias is used only by explicit qualified lookup and is not shadowed by the unqualified value domain. In a generic source-function body a value-binding key MAY also equal an in-scope generic type-parameter key because generic type parameters participate only in the distinct type-position lookup domain owned by `generics.md`.

## Local-first lookup precedence

Within a represented source-function body or closure body, an **unqualified body identifier lookup** that participates in the local value-binding domain consults active bindings in that body's own lexical tree first.

If exactly one active binding has the requested lexical key, lookup resolves to that binding. The consuming source form then determines whether that selected binding category/type is valid for the operation.

Only when no active binding in the current body resolves the key does lookup fall through to the accepted same-module relation in `names-modules.md`, using the current body's module context. A closure body uses the declaration site's source module retained by `closures.md`. Explicit qualified `alias::member` lookup instead uses the retained declaration-site source unit directly under `names-modules.md` and is not blocked by a same-named local key.

Lookup MUST NOT skip an active body-local binding merely because the consuming context would prefer a module-level entity of another category.

The concrete bare identifier value form, admitted binding-rooted `FieldValueUse` root, direct binding-root pattern scrutinee, whole-binding or bounded binding-root field assignment target, unqualified call target, safe-reference root/reborrow/dereference/reference-replacement operands, raw-address target, raw-move pointer operand, and raw-assign pointer operand consume this precedence subject to each operation owner's category boundary. A wrong-category selected entity is rejected rather than bypassed.

For the bare identifier value form, a selected active source-function parameter/ordinary local/pattern binding or closure explicit parameter/ordinary local/pattern/capture binding uses the ordinary binding-use relation below when the surrounding operation admits that value category. A dedicated opaque closure binding is not thereby a general value producer; `closures.md` owns its bounded use for invocation or later closure capture. Only when no active binding resolves the key may same-module lookup select a module declaration: `constants.md` owns constant value production, while `function-values.md` owns context-typed function-value formation when an exact required function-value type selects a non-generic function entity. A wrong-category module binding is final.

For an unqualified call target, this precedence is decisive:

- an exact function-value binding delegates to `function-values.md` and may become bounded-indirect invocation, including a closure capture binding of exact function-value type;
- a **dedicated closure binding** delegates to `closures.md` and may become bounded closure invocation;
- an opaque-closure-valued capture binding is non-callable in this slice and is category-final;
- any other selected local binding is final wrong-category failure; and
- only when no local resolves may same-module lookup select the existing direct source-function target.

A generic argument list does not bypass a selected body-local binding. `generics.md` owns generic arguments only after direct function target selection; `function-values.md` and `closures.md` own early rejection for their respective local callable branches.

For a bounded binding-root field assignment target, the first identifier resolves through this relation but successful assignment requires a **mutable ordinary local**. `field-access.md` then resolves the one-or-more field selectors from that local's declared type. Parameters, pattern bindings, dedicated closure bindings, and closure capture bindings are immutable/non-assignment categories and are rejected without module fallback. No qualified assignment root, arbitrary receiver, or general lvalue lookup is introduced.

For Shared root `&x` or `&x.field...`, `references.md` may select an active parameter, ordinary local, or independently admissible closure capture binding and owns selection of the complete root or bounded structural field path plus the exact Shared referent/accessibility/availability requirements. Dedicated opaque closure bindings remain inadmissible because closure types are not Shared referents. For replacement-capable root `&mut x` or `&mut x.field...`, `references.md` requires one active **mutable ordinary local** and owns selection of the complete root or bounded structural field path plus the exact replacement-referent/accessibility/availability and Exclusive-authority requirements. Capture bindings are immutable and therefore cannot be replacement roots. Because `generics.md` does not admit an abstract type parameter as a safe-reference referent, these forms cannot acquire reference authority over an abstract-typed binding merely from later substitution.

For `*r`, `&*r`, `&mut *r`, and the destination reference binding in `*r = Value;`, `references.md` requires the selected binding to be one active safe-reference parameter/local binding with the exact permission/referent required by that operation. First-slice closure captures cannot have safe-reference type, so no capture-binding case is added for these carrier operations.

For raw address formation, `raw-pointers-unsafe.md` may select one active parameter, ordinary local, or independently admissible closure capture binding of a first-slice raw-pointee-admissible type and selects only its complete root. For raw move/assign, that owner requires the pointer operand binding to have exact type `RawPtr(T)` and consumes its retained exact pointer origin. First-slice closure capture bindings themselves cannot have raw-pointer type, but ordinary raw-pointer locals inside a closure body remain represented. An abstract type parameter is not a first-slice raw pointee under `generics.md`/`raw-pointers-unsafe.md`.

A nominal record-pattern head is not a body-local value-binding lookup. `patterns.md` defines each represented record-pattern head through same-module or qualified nominal-record declaration lookup independently of active local bindings with equal keys, using the closure declaration-site module/source-unit context where applicable.

Source-unit module aliases remain the distinct qualified-lookup mechanism owned by `names-modules.md`. The concrete `alias::member` qualified call target and qualified constant/function-value candidate resolve through that mechanism rather than this unqualified lookup. Qualified calls remain direct module-function calls; an exact required function-value type may instead select qualified formation from an exported non-generic function. Body-local bindings do not block either explicitly qualified value relation.

Generic type-parameter lookup is also distinct from this value-binding precedence. `generics.md` owns bare type-position lookup in a generic source-function body and consults its in-scope type-parameter keys before same-module nominal-type lookup; it does not consult or shadow this value-binding domain. A generic type-parameter key therefore does not block an otherwise valid module constant lookup in a value position. Closure bodies have no generic binder in this slice.

Beyond the represented two-part module alias/member qualification, operation-specific field selectors, bounded record-pattern field selection, bounded binding-root field assignment, bounded safe-reference root/dereference/reborrow/replacement lookup, bounded raw-pointer root/pointer-operand lookup, bounded closure capture/target lookup, constant-value lookup consumed by `constants.md`, and the distinct generic type-position lookup above, this revision defines no arbitrary member lookup, nested module paths, labels, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented binding category has one exact assignment-mutability consequence:

- source-function and closure explicit parameters are immutable;
- ordinary locals are immutable unless their concrete declaration carries `mut`;
- every binding introduced by a represented record pattern is immutable;
- every dedicated closure binding is immutable; and
- every closure-activation capture binding is immutable.

Assignment mutability is a binding property independent of concrete source type identity, abstract generic type-parameter identity, opaque closure-site identity, structural ownership state, source owned-value duplicability, callable-signature/interface identity/equality, safe-reference alias authority/permission, and raw-pointer origin provenance.

Consuming an owned value from an immutable binding, including a represented structural subvalue when `field-access.md` or `patterns.md` permits that consumption, is valid when the applicable safe-authority compatibility requirement is also satisfied. Immutability restricts ordinary whole-binding and binding-root field assignment/reinitialization; it does not require the binding to retain ownership of every subvalue and it is not raw target-access authority.

No parameter-mutability, pattern-binding-mutability, dedicated-closure-mutability, or capture-binding-mutability form is represented. Therefore the current concrete whole-binding and bounded field-assignment forms can successfully target only a mutable ordinary local.

Every represented ordinary local whose declared concrete type is `SharedRef(T)` or `ExclusiveReplaceRef(T)` MUST be immutable; the otherwise represented mutable-local form is invalid for either safe-reference type.

An ordinary local whose declared concrete type is `RawPtr(T)` MAY be immutable or mutable. A mutable raw-pointer local may be ordinarily assigned another exact `RawPtr(T)` value only when the incoming pointer origin satisfies the lexical target-validity rule from `raw-pointers-unsafe.md` for the complete receiving-local extent.

Assignment to any immutable binding is source-invalid regardless of whether its complete structural root or a selected structural subpath is fully available, partially available, or unavailable.

Binding mutability does not itself replace a value or restore ownership. Replacement is an explicit ordinary assignment operation under the rules below. Replacement capability carried by `ExclusiveReplaceRef(T)` separately permits `*r = Value;` to replace the referent even though the reference binding `r` itself is immutable. Unsafe raw replacement through a pointer may likewise replace an immutable **pointee target**, including admissible capture storage, because `raw-pointers-unsafe.md` deliberately does not consume ordinary target-binding assignment mutability as a precondition. Neither operation is whole-binding or binding-root field assignment to the reference/pointer binding.

## Binding structural ownership state

Every in-scope represented binding owns exactly one structural owned-value root under `structural-ownership.md` whose root type expression is the binding's exact source type expression.

For a binding declared with one abstract type parameter, `structural-ownership.md` provides exactly the opaque complete root consumed by `generics.md`; no represented non-empty structural path exists through that abstract root. For a dedicated closure binding, it provides the opaque closure complete root whose only source-visible path is `[]`. A closure capture binding of nominal-record type retains normal record structural paths; a capture binding of opaque closure type remains path-opaque.

This document owns only the binding lifecycle around that structural state:

- successful parameter transfer establishes an explicit parameter with complete initial ownership;
- successful ordinary local initialization establishes the local with complete initial ownership;
- successful pattern binding production establishes each new pattern binding with complete initial ownership;
- successful closure capture formation establishes a dedicated closure binding with complete opaque-root ownership under `closures.md`;
- successful closure activation transfer establishes each capture binding with complete initial ownership in capture-list order under `closures.md`;
- represented consuming/duplicating operations act on binding structural state only through their canonical operation owners and `structural-ownership.md`;
- successful ordinary whole-binding replacement establishes a fresh complete structural ownership state for the replacement value;
- successful bounded ordinary-local field assignment applies the canonical non-empty subpath-installation transition from `structural-ownership.md` to the binding's existing root state; and
- lexical/activation termination ends whatever binding ownership remains according to `function-execution.md` and, for closure environment/capture composition, `closures.md`.

Safe-reference authority/carrier state, replacement-capable external referent structural state, generic activation substitution context, closure held-environment state, and raw-pointer origin provenance are deliberately distinct facts. `references.md` owns safe authority and external-referent state; `generics.md` owns generic substitution context; `closures.md` owns held closure environments; `raw-pointers-unsafe.md` owns pointer origin. Root safe-reference formation and raw address formation leave the target binding's structural ownership state unchanged. Dereference Move through a replacement-capable reference updates the actual local-root structural state at the reference's exact selected target path when the reference targets a local binding. Raw ownership move and raw replacement likewise alter the target structural state only through the explicit transitions consumed by their canonical owners.

Entering or normally exiting a child lexical scope does not itself change structural ownership state, external-referent state, safe authority, generic substitution context, or pointer-origin provenance of an ancestor/enclosing domain. Valid ownership transitions, reference child lifecycle, pointer retargeting, or assignment affecting an enclosing domain inside the child remain in force at the following parent-scope program point when the applicable control-flow relation admits that normal continuation. Child reference-local cleanup may end a child authority and thereby restore parent reference-relative authority before the enclosing normal outcome is formed.

A closure-call boundary is different: the creator activation is suspended while a distinct closure activation executes. Closure-body bindings belong to the fresh closure root and do not mutate creator binding state merely by lexical lookup. Explicit parameter/reference operations may still affect caller-visible external referents only through the existing call/reference contracts.

Structural source paths, prefix-free consumed-path state, fully/partially/unavailable classification, path consumption, bounded subpath-installation state, recursive remaining-frontier selection, and opaque-root boundaries are defined only by `structural-ownership.md`. They are not redefined here.

For represented statement-level conditionals and bounded `while`, `control-flow.md` owns normal-continuation composition over every active continuing binding root in the **current activation**, including dedicated closure/capture roots where present, together with replacement-capable external referent state and raw-pointer origins. This binding owner adds no union, intersection, widening, automatic restoration, or closure-specific join rule. A non-duplicable dedicated closure consumed on only one continuing branch/backedge therefore fails the same exact-state requirements unless an independently represented restoration exists; dedicated closure bindings are immutable, so this slice provides none.

Safe-reference authority/delegation state does not introduce a general control-flow lattice. Immutable reference locals, non-copyable replacement-capable carrier movement, explicit reborrow, lexical child cleanup, and the no-field/no-result/no-rebinding restrictions make persistent carrier/authority consequences definite through existing binding ownership plus the sequential reference relation. `control-flow.md` owns the exact composition boundary.

Raw-pointer origin is an additional exact provenance fact, not a structural ownership path set. The raw-pointer/control-flow relation likewise adds no origin union, set, maybe-origin state, widening, fixed point, or NLL lattice.

Generic activation substitution context is fixed for one source-function activation under `generics.md`; it is not a branch-varying ownership fact and therefore adds no conditional/loop join lattice. Closure activations have no generic substitution context in this slice.

Future refutable matches, catch/recovery forms, additional loop forms, or other control-flow forms require their own accepted definite-state relations; this document adds none beyond the represented conditional/while relations owned by `control-flow.md`.

## Ordinary whole-binding owned-value use

A represented **ordinary whole-binding owned-value use** applies to the empty structural path of one selected active binding when the consuming operation owner admits that binding category.

The complete root path MUST be fully available under `structural-ownership.md` immediately before the use.

If the binding's type expression is one abstract type parameter under `generics.md`:

1. require the canonical Exclusive safe-authority compatibility relation for direct access to that complete binding root;
2. produce the complete owned value through the capability-conservative consuming transfer/Move selected by `generics.md`; and
3. apply the canonical successful-consumption transition from `structural-ownership.md`.

This abstract branch is not a classification of the later substituted concrete type as non-duplicable. Generic-body validation lacks positive duplicability evidence, and every concrete realization MUST preserve the selected Move even when the activation substitution supplies a concrete duplicable type.

Otherwise, if the binding's concrete source type is duplicable under `types.md`:

1. require the canonical Shared safe-authority compatibility relation for direct access to that binding root;
2. produce another owned source value of that complete type through the accepted duplicability capability; and
3. leave the binding's structural ownership state unchanged.

Otherwise the binding's concrete source type is non-duplicable, and ordinary use:

1. requires the canonical Exclusive safe-authority compatibility relation for direct access to that binding root;
2. transfers/consumes the complete owned value through the empty structural path; and
3. applies the canonical successful-consumption transition from `structural-ownership.md`.

The safe-authority check concerns authorities targeting the binding being used. Moving or copying a safe-reference **carrier binding** is not direct access to that carrier's referent and does not conflict with the authority carried by its own value merely because that reference is active. First-slice generic abstract roots cannot themselves be safe-reference targets because `generics.md` does not admit abstract referent types; the Exclusive requirement on an abstract Move nevertheless preserves the canonical direct-access rule rather than creating an exception.

For a binding of exact concrete type `SharedRef(T)`, the type is duplicable. The successful duplicate therefore has the carrier consequence owned by `references.md`: it creates another carrier naming the same Shared authority/target while retaining the stored source carrier. This is not a reborrow or new root authority.

For a binding of exact concrete type `ExclusiveReplaceRef(T)`, the type is non-duplicable. Successful ordinary use moves the one stored carrier into the produced value and consumes the reference binding root, without copying the carrier, ending the authority when an active descendant still keeps it alive, or accessing the referent.

For a binding of exact concrete type `RawPtr(T)`, the type is duplicable. The successful duplicate preserves the exact raw-pointer value and pointer-origin provenance owned by `raw-pointers-unsafe.md`; it does not access the pointee or create any reference authority.

For a dedicated or captured binding of opaque closure-site type, `types.md`/`closures.md` supply duplicability from the all-captures rule. This whole-root relation supplies the selected Duplicate/Consume consequence only when `closures.md` invokes it for dedicated closure snapshot/later closure capture or for other specifically admitted closure ownership transport. It does not make an opaque closure value a general bare-value producer or callable from an opaque-closure-valued capture binding.

Ordinary whole-binding use of a partially available or unavailable complete root is source-invalid, not a defined runtime moved-state fault.

The concrete bare `UserIdentifier` value shape maps to this operation only when local-first lookup resolves a binding whose category/type is admitted by the surrounding value operation. If no active binding resolves that key and same-module lookup instead selects a source constant, the same concrete shape maps to the constant-value producer owned by `constants.md`; if lookup instead selects a source static, it maps to the persistent scalar-read producer owned by `statics.md`. This binding-use relation creates no hidden local for either module entity. In a closure body, uncaptured creator locals are absent and therefore do not suppress module fallback.

This relation does not define constant value production, static persistent reads/roots, closure capture/invocation, field-value production, record-pattern ownership, safe-reference dereference/reborrow, raw address formation, or raw pointee access. Those owners may use their own bounded receiving relations without first applying ordinary whole-binding use to the complete target root.

## Whole-binding assignment and reinitialization

A represented whole-binding assignment target MUST resolve through local-first lookup and MUST denote one **mutable ordinary local binding**. Parameters, pattern bindings, dedicated closure bindings, and closure capture bindings are not whole-binding assignment targets in this slice.

Before RHS consequences can commit, the target must satisfy the canonical Exclusive safe-authority compatibility requirement from `references.md`: no active overlapping safe authority may target that complete root.

The RHS MUST produce exactly one owned source value whose type expression is exactly equal to the target binding's declared source type expression under the applicable concrete equality from `types.md` or abstract generic equality from `generics.md`.

When the target binding has concrete type `RawPtr(T)`, the produced RHS raw-pointer value additionally MUST carry an exact pointer origin whose target binding extent contains the complete receiving pointer-local extent under `raw-pointers-unsafe.md`. This source validity requirement applies before the new pointer value/origin becomes the target binding's continuing state.

The target may have a fully available, partially available, or unavailable complete structural root when assignment begins. An abstract generic root can only be fully available or unavailable because it has no represented non-empty path. Successful assignment always replaces/reinitializes the complete ordinary-local value. Safe authority is a separate rejection condition even when the current structural root would otherwise be replaceable.

The target remains in scope during RHS evaluation. Every RHS use observes the target's current structural ownership and safe-authority state. A consuming RHS may therefore change structural ownership state before replacement completes. A safe authority established or retained during RHS evaluation likewise remains controlling at the replacement point. For a raw-pointer assignment, RHS production similarly determines the exact incoming pointer origin before replacement commits.

After successful RHS production, satisfaction of any raw-pointer lexical target-validity requirement, and a second satisfaction of the canonical Exclusive safe-authority requirement at the actual replacement point, `function-execution.md` owns source-first replacement ordering:

1. select and end ownership of the target's then-current remaining old-value frontier through `structural-ownership.md`;
2. transfer the successfully produced replacement value into the target; and
3. establish a fresh complete structural ownership state with an empty consumed-path set.

For a `RawPtr(T)` target, the successful transfer also replaces the stored pointer-origin provenance with the incoming exact origin. Ending the old pointer value has no pointee effect under `raw-pointers-unsafe.md`.

Thus a mutable ordinary local may be reinitialized from any represented structural ownership state only when the complete-root Exclusive safe-authority requirement succeeds both at admission and after RHS evaluation, while every immutable/non-assignment binding category may not be ordinarily assigned in any state.

Safe-reference locals themselves cannot be whole-binding assignment targets because reference locals are immutable. Reference-relative `*r = Value;` remains a separate operation owned by `references.md`/`function-execution.md`; unsafe raw replacement of a pointee, including admissible capture storage, remains a separate operation and does not use target-binding mutability.

A defined fault or divergence during RHS evaluation performs no replacement/reset merely because assignment was intended. Ownership, safe-reference authority/carrier, external-referent, and pointer-origin transitions that completed while evaluating the RHS remain in force under their existing owners.

This whole-binding relation defines no general source place/lvalue, plain-Exclusive replacement, interior mutability, raw pointee replacement, destructuring assignment, or closure rebinding.

## Bounded binding-root field assignment and reinitialization

A represented **bounded binding-root field assignment** targets one non-empty structural field path `p` under one selected **mutable ordinary local** binding root.

The root identifier MUST resolve through local-first lookup and MUST denote a mutable ordinary local. Parameters, pattern bindings, dedicated closure bindings, and closure capture bindings are not field-assignment roots. This is intentionally narrower than field-value access, which may admit an immutable nominal-record capture binding.

`field-access.md` resolves the one-or-more field selectors from the ordinary local's exact declared concrete type, requires every selector step to select one declared nominal-record field with the existing direct accessibility, and supplies the exact non-empty structural path `p` and final type `type(p)`. An abstract generic binding or opaque closure root exposes no non-empty structural path and therefore cannot satisfy this operation. This operation does not first produce, duplicate, consume, or otherwise evaluate an intermediate field value merely to select the target.

The RHS MUST produce exactly one owned source value whose type is exactly equal under `types.md` to `type(p)`. No conversion, coercion, inferred target type, structural record equivalence, method/property setter, or independently mutable field relation is introduced.

Before RHS consequences can commit, the exact selected structural target `p` MUST satisfy the canonical Exclusive direct safe-authority compatibility requirement from `references.md`. An overlapping authority targeting an ancestor, equal path, or descendant blocks this direct replacement as that canonical relation requires; a structurally disjoint sibling authority does not spuriously block it.

The selected binding and field path remain statically identified while RHS evaluation proceeds. Every RHS use observes the binding's then-current structural ownership and safe-authority state. A consuming RHS may therefore consume the exact target, one or more descendants of the target, or structurally disjoint paths before replacement commits.

On the RHS producer's normal successful continuation immediately before replacement, let `C` be the binding root's resulting consumed-path set. The assignment MUST satisfy the bounded non-empty subpath-installation admission relation from `structural-ownership.md`: no member of `C` may be a strict ancestor of `p`.

Therefore the post-RHS target may be:

- fully available, for ordinary replacement;
- exactly consumed at `p`, for reinitialization; or
- partially available because strict descendants of `p` are consumed, for reconstruction.

If a strict ancestor of `p` remains consumed on that successful continuation, the assignment is source-invalid. It does not split that consumed ancestor into sibling/complement consumed paths, implicitly reconstruct the ancestor, or defer validity to a runtime moved-state check.

After successful RHS production, satisfaction of the post-RHS structural admission, and a second satisfaction of the canonical Exclusive safe-authority compatibility requirement at the actual replacement point, `function-execution.md` owns source-first replacement ordering:

1. select and end only the target's then-current `frontier(p)` through `structural-ownership.md`;
2. preserve already consumed descendants as already ended rather than destroying them again;
3. transfer the successfully produced complete replacement value into the exact selected target `p`; and
4. apply the canonical successful non-empty subpath-installation transition, removing from `C` exactly `p` and consumed descendants of `p` while preserving every structurally disjoint consumed path.

The resulting selected target `p` is fully available. The complete binding root may still be partially available because a structurally disjoint path remains consumed.

A defined fault or divergence during RHS evaluation performs no target frontier cleanup, value installation, or consumed-path reset merely because field assignment was intended. Structural, reference, external-referent, and pointer-origin transitions that completed while evaluating the RHS remain in force under their existing owners.

This relation introduces no qualified assignment root, arbitrary receiver, capture-binding assignment, general place/lvalue, assignment expression, compound/destructuring assignment, reference-relative field assignment, projected safe-reference replacement spelling, raw field/path replacement, interior-mutability rule, or structural state splitting beneath a consumed ancestor.

## Binding cleanup and discard boundary

When represented execution ends a binding's ownership, its remaining owned source subvalues are exactly the complete-root remaining ownership frontier selected by `structural-ownership.md` from the binding's then-current state. For an abstract generic binding this frontier is either its one complete opaque root while still owned or empty after that root has been consumed. For a dedicated opaque closure binding it is likewise the complete opaque root or empty; ending that complete closure value then triggers the capture-environment cleanup consequence owned by `closures.md` rather than exposing source capture paths here.

`function-execution.md` owns ordinary lexical/activation cleanup and ordering between source-function bindings/scopes/parameters, assignment replacement, safe-reference referent replacement, raw replacement, normal return, and defined-fault cleanup. `closures.md` owns closure-specific composition: explicit parameters are transferred normally, capture bindings are established in capture-list order before body execution, closure root/body locals and remaining captures clean under the ordinary lexical relation, and explicit parameters then clean in the existing reverse slot order.

When a binding's remaining owned value is a safe reference, ending that value additionally removes its source reference carrier at that existing cleanup point under `references.md`. Removing the final carrier may end its authority or may leave a carrierless ancestor authority alive while a descendant remains. The reference-specific consequence adds no custom cleanup body and does not access the referent.

When a binding's remaining owned value is a raw pointer, ending that value has no pointer-specific pointee effect under `raw-pointers-unsafe.md`. It neither changes target structural ownership nor ends any safe authority.

The existing reverse local/declaration and activation cleanup ordering is part of represented lexical lifetime proof in `references.md` and lexical pointer-target validity in `raw-pointers-unsafe.md`: reference/pointer locals and child reference carriers end before earlier/ancestor target extents whose validity their initialization/derivation consumed. Closure capture bindings are root-scope bindings established after explicit parameter transfer and therefore end before explicit parameters on closure activation cleanup as specified by `closures.md`.

A binding is not source-invalid solely because one or more remaining owned subvalues are non-duplicable, because an abstract generic complete value remains owned, or because an opaque closure/capture value remains owned when its scope or activation terminates. This revision defines no source `drop` ability, must-consume classification, custom destructor, or unused-value prohibition.

Zero-field and recursively zero-leaf frontier members remain source-owned values even when faithful Core refinement emits no scalar destruction operation.

## Function, closure, call, assignment, pattern, control-flow, reference, raw-pointer, constant, and fault boundary

This document defines body-local binding identity, scope, local-first lookup, assignment mutability, binding lifecycle around structural ownership, safe-reference/raw-pointer/function-value/closure binding integration, ordinary whole-binding use, whole-binding assignment legality/reset, and bounded binding-root field assignment legality/reset. It consumes rather than owns closure-site/capture/invocation semantics from `closures.md`, module constant semantics from `constants.md`, module persistent-static semantics from `statics.md`, and generic type-parameter identity/capability/substitution from `generics.md`.

It does not redefine constant declaration/value/use semantics from `constants.md`, generic application/exact substitution/activation substitution from `generics.md`, or closure type/capture/call semantics from `closures.md`.

It does not redefine the ordinary execution relation owned by `function-execution.md`, including:

- source-function body execution and normal-continuation presence;
- shared source-call argument validation/evaluation and explicit parameter transfer;
- safe-reference produced-carrier transfer/caller suspension/external-referent consequences;
- whole-binding and bounded ordinary-local field assignment RHS evaluation, old-value cleanup, and replacement transfer;
- safe-reference referent replacement ordering;
- raw replacement source-first execution ordering;
- constant-value producer integration;
- result production and return transfer;
- source-function activation identity or recursion;
- lexical-scope/caller/callee cleanup sequencing; or
- defined-fault propagation across activations.

`closures.md` separately owns distinct closure activation identity, held-environment snapshot/transfer, capture-binding establishment, dedicated closure target selection, closure-call one-shot/reusable behavior, and closure-specific cleanup composition while consuming the existing execution rules.

It does not redefine safe-reference root formation, authority/carrier identity, dereference, reborrow, referent replacement, lexical lifetime validity, external-referent state, result provenance, or source-to-Core reference refinement from `references.md`.

It does not redefine raw address formation, raw pointee move/replacement, unsafe admission, pointer-origin validity, or source-to-Core raw refinement from `raw-pointers-unsafe.md`.

It does not redefine represented conditional or bounded-`while` condition/body selection, normal-successor composition, active binding/external-referent structural-state equality, raw-pointer-origin equality, or loop backedge/transfer admission from `control-flow.md`.

It likewise does not redefine field identity/path resolution or accessibility from `field-access.md`, field-value production from that owner, pattern structure/ownership from `patterns.md`, structural ownership mathematics from `structural-ownership.md`, generic parameter/substitution semantics from `generics.md`, or constant semantic values from `constants.md`.

Plain-Exclusive source references, reference pass modes, lifetime names, unsafe callable contracts, broader panic/catch forms, static semantics beyond the bounded owner in `statics.md`, and other future execution relations remain outside this owner.

## Implementation boundary

This revision does not add or require a particular parser, lossless-syntax, HIR, Core MIR production, runtime, or backend representation.

A faithful implementation MAY retain structural ownership for bindings/external referents using resolved field indices or another implementation identity after source field resolution, but those representations are not source semantic identity. It MAY separately retain safe-reference authority/provenance, generic type-parameter/substitution facts, closure-site/capture facts, exact raw-pointer origin facts, and resolved constant identities/values as required by their owners. Core path state, scalar liveness, Core reference-authority IDs, Core external regions, Core pointer-target metadata, generated closure-environment fields, physical specialized-function identity, constant storage choices, or runtime storage identities MUST NOT become the source binding/external-referent ownership, generic binder, closure binding/capture identity, reference-origin, pointer-origin, or constant semantic authority, including when represented control flow establishes a successor/backedge.

## Further boundaries

Beyond the represented concrete subset, accepted first function-only generic type-parameter relation, bounded closure relation, and module-level intrinsic-scalar constant relation, this revision does not define general type inference, assignment expressions, uninitialized locals, precedence/general expressions, arbitrary member/method lookup beyond the bounded assignment-target field path and existing field-value/reference selectors, additional refutable/shorthand pattern forms, unequal-state/path-dependent ownership after a two-normal-outcome conditional join, additional loop forms or general loop fixed-point inference, catch/recovery joins, plain-Exclusive source references, reference targets beyond the bounded Shared and replacement-capable forms plus accepted closure capture-root Shared form, reference-containing aggregates/results beyond the existing bounded Shared result, lifetime names/parameters/non-lexical shortening, raw-pointer call transfer or pointer-containing aggregates, unsafe callable contracts, nested/generic/escaping/reference-capturing closures beyond `closures.md`, generic bounds/inference/records/reference constructors, traits/coherence, methods/overloads, explicit clone/copy operators, custom destructors, must-consume/drop abilities, structural state splitting beneath a consumed ancestor, constant-expression evaluation beyond `constants.md`, static storage semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.

Activation-local raw pointers and lexical unsafe admission are represented by `raw-pointers-unsafe.md`; source constants are represented by `constants.md`; bounded closure bindings/captures are represented by `closures.md`. Their existence does not create the excluded broader pointer/call/unsafe, constant-expression/static-storage, or closure relations here.
