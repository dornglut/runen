# Source Closures and Explicit By-Value Capture

Status: **provisional normative; incomplete**

This document owns the first represented source closure relation: closure declaration-site and body identity, opaque closure type identity, explicit ordered by-value capture formation, closure value/environment contents, closure duplicability, dedicated closure-binding and closure-activation capture-binding establishment, bounded closure call-target selection, held-environment invocation consequences, closure-activation consequences, and source-to-Core closure conversion for this slice.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md); module and source-unit lookup context from [Source names and modules](names-modules.md); represented source type identity and owned-value duplicability from [Source type foundation](types.md); callable parameter/result admission and bounded safe-reference result-contract derivation from [Source callable signatures](callables.md); captureless structural function-value semantics and bounded indirect-call classification from [Source function values and indirect calls](function-values.md); function-local binding scope, lookup, ordinary whole-binding owned-value use, assignment mutability, and lexical lifecycle from [Source function-local bindings](local-bindings.md); structural owned-value roots, path availability, consumption, duplication, and remaining ownership from [Source structural ownership](structural-ownership.md); source-call validation transactions, result receiving, ordinary argument evaluation, source-function activation/body execution, cleanup, return, defined fault, and divergence from [Source function execution](function-execution.md); safe-reference authority compatibility, parameter/result transfer, and result-origin validity from [Source safe references](references.md); activation-local raw-pointer boundaries from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md); the first represented generic-function boundary from [Source generics](generics.md); module constant use from [Source constants](constants.md); record field-value access from [Source field-value access](field-access.md); record pattern selection from [Source patterns](patterns.md); and conditional/loop state composition from [Source control flow](control-flow.md). Concrete spelling is owned by [Source concrete syntax](concrete-syntax.md).

The lower refinement target is the existing structural value/storage and function-call relation in [Core value and storage semantics](../core/value-storage.md) and [Core functions and calls](../core/functions.md). Captureless Core callable values remain separately owned by [Core callable values and indirect calls](../core/callable-values.md). This closure slice requires no new Core closure scalar, environment pointer, reference contract, heap object, ABI mechanism, target-set relation, or Core activation primitive.

This document does not redefine ordinary source function entities, structural captureless `fn(...)` function-value types, generic functions, reference authority, raw-pointer provenance, nominal record structure, field access, patterns, control-flow joins, module lookup, ABI/layout, or an implementation representation.

## Closure declaration sites, body identity, and opaque closure types

Each represented closure declaration site establishes exactly one stable **closure site identity** and exactly one fresh **opaque closure type identity**.

The closure site identity identifies the closure body and its ordered capture specification. Repeated dynamic executions of the same declaration site, including repeated entries through represented control flow, retain the same static site/type identities while producing distinct dynamic closure values as ordinary execution requires.

Two distinct closure declaration sites always establish distinct opaque closure type identities, even when all of the following are equal:

- their explicit callable parameter sequences;
- their result specifications;
- their safe-reference result contracts; and
- their ordered capture type sequences.

An opaque closure type is not structurally equal, nominally equal, convertible, coercible, or implicitly adaptable to any captureless structural function-value type from `function-values.md`. The existing `fn(...)` type remains exactly the environment-free structural callable type owned by that document and remains always source-duplicable under its accepted relation.

An opaque closure type likewise has no equality relation with another closure type merely because lower Core environment shapes happen to be equal. The declaration-site identity is the closure type-identity dimension.

The first closure type has no general source `Type` spelling. Its source-semantic existence is established only by the dedicated closure declaration relation below and may be retained by the bounded closure operations in this document. Absence of a general spelling is not type inference: the declaration itself canonically establishes one exact type from one exact closure site.

An opaque closure type is not a module declaration, module binding, record declaration, function declaration, marker trait, generic type parameter, runtime type object, reflection value, physical layout descriptor, ABI identity, or linker symbol.

## Closure values and environments

One dynamic value of opaque closure type contains exactly:

1. the static closure site identity; and
2. one owned captured source value for every capture slot, in exact capture-list order.

The ordered captured-value sequence is the closure's **source environment**.

The environment is semantic value structure required for ownership, duplicability, invocation, and cleanup. It is not a nominal source record and introduces no source field identities, field names, field selector syntax, structural field paths, layout offsets, physical allocation, address, or source-observable environment identity.

A closure value has no additional dynamic identity beyond its semantic site identity plus its captured values. In particular, duplicating a closure does not create or share a source-visible environment object identity. This first slice defines no shared mutable environment.

Capture containment is value containment for closure semantics, even though capture slots are opaque to source field/path operations. A later closure declared in the same ordinary source-function body may capture an earlier dedicated closure binding by value. The containment relation remains finite because a closure may capture only an already established outer binding and cannot capture itself or a later binding under the declaration-order rules below.

## Dedicated closure declarations

A represented closure is introduced only by one dedicated **closure declaration in an ordinary non-generic source-function body**. A `ClosureDeclaration` occurring syntactically inside a closure body is source-invalid in this slice even though `concrete-syntax.md` reuses the ordinary `Body` grammar for closure bodies.

The concrete grammar is owned by `concrete-syntax.md`; semantically the declaration contains exactly:

- one new dedicated closure-binding lexical identifier key;
- one non-empty finite ordered capture list;
- one finite ordered explicit parameter list, which may be empty;
- one optional result type;
- one derived safe-reference result contract;
- one closure body; and
- the fresh closure site/type identities defined above.

The dedicated closure binding is immutable for ordinary assignment. This slice defines no mutable/rebindable closure declaration.

A closure declaration is not an ordinary explicitly typed local declaration and does not introduce general local type inference. The closure binding's exact opaque type comes only from its declaration site.

The closure declaration is a body statement, not an ordinary `Value` producer. Consequently the concrete closure initializer cannot independently appear as a call argument, return value, record initializer, grouped value, field receiver, pattern scrutinee, or immediate callee in this slice.

Closure-body validation is latent with respect to creator execution. A `return`, explicit `fault;`, divergence, or absence of a local normal continuation in the closure body has no declaration-time execution consequence. Successful declaration execution consists only of validated capture formation followed by establishment of the dedicated closure binding and then continues normally in the creator when the enclosing source-function control flow otherwise continues.

## Explicit capture list

The first closure slice uses an explicit **non-empty ordered capture list**. No free-variable analysis implicitly adds a capture.

Each capture occurrence contains exactly one unqualified user-identifier key. Resolution is performed against the active outer source-function binding environment at the closure declaration site.

A capture occurrence MUST resolve to one active outer binding established before the closure declaration. Admitted targets are:

- an ordinary source-function parameter binding;
- an ordinary local binding;
- a pattern-introduced local binding where its existing local-binding relation makes it an active whole-value binding; or
- an earlier dedicated closure binding declared in that same ordinary source-function body.

A capture occurrence does not select a closure-activation capture binding because closure declarations are not admitted inside closure bodies in this slice. It also does not select a module constant, source static, source function entity, record/type declaration, marker trait, import alias, generic type parameter, field path, safe-reference target, raw-pointer target, arbitrary producer, dereference result, or other non-binding value. Module constants, source statics, and source functions may still be resolved directly from the closure body through their ordinary declaration-site module/source-unit relations where applicable; they do not require capture storage. A static therefore never occupies a closure environment slot.

If two capture occurrences resolve to the same outer binding identity, the closure declaration is invalid. Capture uniqueness is by selected binding identity rather than by implementation collection order.

The exact source capture-list order is semantic. It fixes capture production order, the environment slot sequence, capture-binding establishment order, and the reverse order used when an intact environment's still-owned values are cleaned as one closure value.

## Capture type admission

Every first-slice capture type MUST be a concrete owned source value type whose closure environment contains neither a safe-reference value nor a raw-pointer value/origin.

The admitted capture categories are exactly:

- one represented intrinsic scalar source type;
- one represented nominal record source type;
- one represented captureless function-value type from `function-values.md`; or
- one opaque closure type from an earlier dedicated closure declaration whose own recursively contained capture types satisfy this same first-slice boundary.

Current nominal record fields cannot contain safe references, raw pointers, captureless function values, or closure values, so an admitted nominal record capture contributes no hidden reference/pointer/callable/closure edge beyond its accepted nominal record value shape.

`SharedRef(T)`, `ExclusiveReplaceRef(T)`, and `RawPtr(T)` are not capture-admissible. This exclusion is semantic even when one of those value types is duplicable.

An abstract generic type parameter is not capture-admissible because closure declarations are not admitted inside generic source functions in this slice. There is therefore no abstract capture environment, generic closure type, generic environment specialization, or capture capability reconstructed from a later concrete generic application.

The first slice defines no reference capture, raw-pointer capture, field capture, capture by producer result, capture alias, explicit capture mode, capture pack, or implicit environment discovery.

## Capture formation

Before a closure declaration establishes a normal continuation, source validation MUST establish all of the following without committing a partial outer ownership continuation:

1. the declaration occurs in an ordinary non-generic source-function body rather than a generic source-function body or a closure body;
2. the dedicated closure-binding key is admissible under the enclosing local-binding scope rules;
3. every capture resolves to one exact active outer binding and capture identities are unique;
4. every capture type is capture-admissible;
5. every selected complete outer binding root is fully available for its required whole-value production;
6. every capture's ordinary whole-binding use is compatible with the current direct safe-reference authorities before any capture is produced: Shared compatibility for Duplicate and Exclusive compatibility for Consume/Move, exactly as required by `local-bindings.md` and `references.md`;
7. the explicit callable parameter/result interface and safe-reference result contract are valid under `callables.md` and `references.md`;
8. capture-binding and explicit parameter keys satisfy the closure-body uniqueness rules below; and
9. the closure body validates under the fresh closure-body context established below.

A frontend may inspect those facts in another order when diagnostics differ, but it MUST NOT establish a source-valid continuation whose outer ownership state reflects only a proper prefix of an otherwise rejected closure capture transaction. In particular, every capture's availability and mode-specific authority compatibility are prevalidated before the first dynamic capture consequence commits.

For a source-valid closure declaration, dynamic capture production follows exact capture-list order. Each capture uses the existing ordinary complete-binding owned-value production mode selected from the capture type's source duplicability:

- when the capture type is source-duplicable, capture formation duplicates the complete value and leaves the outer binding's structural ownership state unchanged;
- when the capture type is source-non-duplicable, capture formation consumes/moves the complete root and the outer binding becomes unavailable under its existing structural ownership state until an independently valid later reinitialization, when that binding category permits one.

Capture formation does not execute the closure body, create a source-function or closure activation, call another function merely because a closure exists, form a safe reference, form a raw pointer, allocate storage outside the receiving closure binding, fault, or diverge after source validation. The produced captured values initialize the new closure value owned by the dedicated closure binding.

The new closure binding is established only after successful capture formation. It is not in the outer lookup environment used to resolve its own captures.

## Closure binding ownership and duplicability

The dedicated closure binding owns one complete opaque structural owned-value root of the closure's exact opaque type.

Capture slots are not source structural paths. For the closure binding root, the empty path is the only source-visible structural path. Whole-root availability, consumption, duplication, cleanup, and control-flow state therefore use the existing complete-root ownership relation without exposing capture components as field-selectable source subpaths.

A closure type is source-duplicable exactly when **every** capture type in its non-empty ordered capture sequence is source-duplicable under that type's canonical source owner.

When all capture types are duplicable, duplicating the closure value duplicates every captured semantic value while preserving the same closure site identity. It leaves the stored closure binding value available and creates no shared mutable source environment.

If any capture type is source-non-duplicable, the closure type is source-non-duplicable. No callable-interface property upgrades that closure to duplicable.

Core structural copyability is not source authority for this classification. A faithful lowering selects source closure Copy/Move behavior from the source duplicability fact, exactly as source nominal-record operation selection remains authoritative even when a lower structural type could express a broader operation.

The dedicated closure binding cannot be the target of ordinary whole-binding assignment or binding-root field assignment in this first slice. The closure value may nevertheless be consumed by closure invocation or by a later explicit closure capture as defined here.

## Closure activation and lexical environment

A bounded closure invocation creates one distinct **closure activation**. A closure activation is not a source-function activation and does not make the closure site a source function entity. Existing return, fault, result, argument-transfer, lexical cleanup, and divergence relations apply to the current activation where `function-execution.md` integrates the two concrete cases; this document does not introduce a third generic callable-activation semantic object.

Each closure activation has its own fresh root lexical-scope tree. It retains the closure declaration site's exact **source module and source unit** for ordinary same-module lookup and source-unit-local qualified alias lookup. It does not inherit the creator source-function's body-local bindings except through explicit captured values, and it does not inherit creator loop targets, creator unsafe-admission state, or creator activation identity.

At static closure-body validation, the fresh closure root is seeded by the explicit parameter bindings supplied by the closure callable interface and by the capture-binding identities defined here. At dynamic activation entry, explicit parameters are established through the ordinary call-transfer relation first; then one **capture binding** is established for each capture slot in exact capture-list order before body statement execution begins.

Each capture binding has:

- the lexical identifier key written by its capture occurrence;
- one closure-body-local binding identity distinct from the outer binding identity that supplied its value;
- the exact capture source type; and
- immutable ordinary-assignment status.

Capture-binding keys and explicit parameter-binding keys MUST be pairwise distinct. Existing no-overlapping-shadow rules apply to ordinary locals/pattern bindings introduced later in the closure body relative to active capture and parameter bindings.

Outer creator bindings that are not explicitly captured do **not** participate in closure-body local lookup and are not lexical ancestors of the closure root. A closure explicit parameter or body local may therefore reuse an uncaptured creator key when the ordinary no-overlap relation inside the fresh closure root permits it. If no capture/parameter/body-local binding resolves a key, ordinary same-module fallback may occur where the consuming source form permits it; a same-spelled uncaptured creator local does not suppress that fallback.

The closure declaration's own outer dedicated binding is absent from its closure-body local environment. This first slice therefore defines no closure self-recursion.

A closure-body local key may equal a declaration-site source-unit module-alias key without affecting explicit `alias::member` lookup because the qualified alias domain remains separately owned by `names-modules.md`.

After activation establishment, a capture binding participates in existing immutable binding operations wherever its exact concrete type independently satisfies the consuming owner's requirements. This includes ordinary whole-binding use, binding-root field-value/pattern use for record captures, Shared root-reference formation where otherwise valid, and activation-local raw-address formation where otherwise valid. Capture-binding establishment itself remains owned only by this document.

Because capture bindings are immutable, they are not ordinary whole-binding assignment targets and are not replacement-capable safe-reference root targets. This does not prevent the existing unsafe raw-replacement relation from applying to a valid activation-local raw pointer targeting capture storage when that owner independently admits the operation; ordinary assignment mutability and raw replacement remain distinct properties under `raw-pointers-unsafe.md`.

Creator unsafe admission does not cross the fresh closure root. A raw move or raw assignment executed in a closure body requires an independently represented closure-body unsafe block under `raw-pointers-unsafe.md`.

A `return` or explicit `fault;` reached in the closure body terminates the closure activation, not the creator source-function activation. `break;` and `continue;` may target only a represented `while` in this closure activation's lexical-scope tree and cannot cross the closure-call boundary.

On normal or faulting closure-body completion, remaining root/body locals and capture bindings are cleaned by the ordinary lexical/activation cleanup relation, with capture bindings consequently ending in reverse establishment order where still owned; explicit parameters then use the existing reverse parameter-slot cleanup order. This ordering does not create a closure-specific destructor relation.

## Closure callable interface

Each closure site has one non-generic callable interface containing exactly:

1. the finite ordered sequence of **explicit** closure parameter types;
2. either no result or one explicit result type; and
3. the exact bounded safe-reference result contract derived by `callables.md` from those explicit parameter/result types.

Capture slots are not callable parameter slots. They do not participate in callable parameter order, generic arity, result-origin selection, safe-reference argument transfer, exported-signature accessibility, or captureless function-value structural type equality.

The explicit parameter/result type admission rules are exactly the existing concrete non-generic callable-interface rules. In particular:

- intrinsic, nominal record, captureless function-value, and represented safe-reference parameter forms remain admitted where `callables.md` already admits them;
- a valid scalar Shared-reference result continues to require the existing deterministic `SharedIdentity(i)` or `SharedDirectChild(i)` contract over one explicit parameter slot;
- replacement-capable and raw-pointer results remain invalid; and
- raw-pointer parameters remain invalid.

An opaque closure type itself is not an explicit callable parameter or result type in this slice.

A closure initializer has no generic type-parameter list. The closure body is validated once from its concrete capture/interface facts rather than once per call target or dynamic environment.

## Closure call target classification

The neutral source `Call` syntax remains the only represented invocation token shape for this slice.

For an unqualified call target in either a source-function body or a closure body, the applicable local-first lookup remains category-final:

1. if lookup selects one active **dedicated closure binding**, the call is one **bounded closure call** under this document;
2. if lookup selects one active binding whose exact type is the existing captureless function-value type, including a closure-activation capture binding of that exact type, the call remains the existing bounded indirect call under `function-values.md`;
3. if lookup selects an opaque-closure-valued **capture binding**, the selected binding is non-callable in this slice and the call is source-invalid without module fallback;
4. if lookup selects another active non-callable binding, the selected binding is final wrong-category failure; and
5. only when no active local binding resolves the key may ordinary same-module source-function lookup produce a direct call.

A qualified `alias::member(...)` call remains a direct source-function call. No closure value is a module member in this slice.

A generic type-argument list on a call whose selected target is a dedicated closure binding is invalid before any closure-callee or ordinary argument producer effect commits. There is no generic closure application.

The closure callee is only one unqualified active dedicated closure binding. This relation does not admit grouped closure values, opaque-closure-valued capture bindings, closure-producing calls, fields, record members, returned closures, constructed closures, arbitrary expression callees, or another postfix invocation grammar.

## Static call validation transaction

A bounded closure call is a third target classification of the existing source `Call` family, not a new producer family. Before source effects commit, `function-execution.md` uses the selected closure site's explicit callable interface as the third static interface source alongside direct source-function entities and captureless function-value types.

The complete bounded closure call, whether result-bearing or a no-result call statement, is one speculative source-validation transaction over the existing complete caller-side producer-validation state:

1. validate the selected dedicated closure target, exact closure interface, result presence/type, and surrounding result-receiving facts before callee or argument effects;
2. on the speculative state, apply the source-selected closure snapshot consequence defined below: Duplicate or Consume/Move;
3. validate explicit ordinary arguments left-to-right on that same speculative transaction;
4. validate the existing final safe-reference call-entry obligations on the resulting speculative state; and
5. commit the resulting caller-side state only when the complete call is source-valid.

If any later static argument/type/availability/authority/call-entry requirement rejects the call, none of the speculative closure snapshot or argument-producer consequences commits. In particular, a statically rejected call cannot leave a non-duplicable dedicated closure binding consumed. The same rollback requirement applies when a locally valid bounded closure call is nested inside a larger producer transaction that is ultimately rejected.

This static rollback law is distinct from runtime defined fault or divergence during a source-valid call, where already executed effects remain real as specified below.

## Held closure value and invocation ordering

For a source-valid bounded closure call, dynamic execution evaluates the selected dedicated closure binding exactly once **before** ordinary arguments:

- for a source-duplicable closure type, ordinary whole-binding use duplicates the closure value and leaves its stored binding value available;
- for a source-non-duplicable closure type, ordinary whole-binding use consumes/moves the complete closure value and leaves the stored binding unavailable.

The produced closure value becomes one operation-owned **held closure environment**. It is not a source binding, field path, reference authority, pointer provenance, allocation, or separately addressable source storage identity.

Only after that held value exists are explicit ordinary arguments evaluated left-to-right under the existing source-call rules. All argument ownership/reference effects remain those of their canonical producers.

After all argument production succeeds, the existing final safe-reference call-entry checks apply to the explicit safe-reference argument values. Capture environment values do not add hidden safe-reference arguments because first-slice captures are reference-free.

After call-entry obligations succeed, the closure activation receives every explicit argument through the existing parameter-transfer relation and then receives every environment value into its corresponding capture binding in exact capture-list order. The closure body begins only after those transfers establish complete initial ownership of every explicit parameter and capture binding.

A duplicable stored closure can therefore be invoked repeatedly. A stored closure containing any non-duplicable capture is one-shot through that value because its first valid invocation consumes the dedicated closure binding before ordinary argument runtime evaluation.

## Capture transfer into one activation

The held closure environment transfers each capture value exactly once into its corresponding activation-local capture binding in capture-list order. This is ownership transfer of already held values, not a second capture from the creator's outer bindings.

After transfer, the held environment retains no separately owned source capture values; ownership resides in the capture bindings. Capture-binding initialization adds no source-visible parameter slot and does not modify the closure callable interface.

A source implementation may represent the held environment and capture-binding storage differently provided it preserves the exact ownership, cleanup, lookup, and call ordering defined here.

## Safe-reference interaction

Safe-reference values cannot be captured into the first closure environment.

Safe-reference explicit closure parameters and bounded Shared-reference results remain governed by `callables.md`, `references.md`, and `function-execution.md`. Their result-origin parameter indices refer only to the explicit source parameter sequence.

A capture binding of admissible intrinsic/record type may be used as an immutable activation-local root for Shared safe-reference formation under `references.md` where that type is Shared-referent-admissible and all ordinary availability/authority requirements succeed. It is not a replacement-capable root because capture bindings are immutable.

A fresh Shared reference rooted in capture storage has the ordinary `RootOrigin(captureBinding)` provenance established by `references.md`. It does not become `ParameterOrigin(i)` merely because the captured value originally came from an outer binding. Returning such a fresh capture-root reference cannot satisfy `SharedIdentity(i)` or `SharedDirectChild(i)` merely because the closure environment owns that capture; no hidden capture slot is a result-contract origin.

This slice introduces no reference capture, reference-containing closure environment, closure outlives relation, environment-root result contract, detached reference authority, named lifetime, or non-lexical lifetime inference.

## Raw-pointer interaction

Raw-pointer values cannot be captured into the first closure environment.

A capture binding whose exact concrete type is otherwise a first-slice raw pointee may be selected by activation-local raw-address formation under `raw-pointers-unsafe.md`. Such a raw pointer originates from storage in the **closure activation**, not from the creator's outer binding and not from a captured raw-pointer value.

The existing lexical target-validity rule requires any receiving raw-pointer local extent to be contained by that capture-binding target extent. Raw-pointer parameters/results remain absent, so no raw pointer to capture storage leaves the closure activation through the represented call boundary.

No raw-pointer capture, cross-activation raw environment pointer, environment address, pinning rule, address stability, or ABI representation is introduced.

## Fault, divergence, and cleanup

Capture formation itself is non-faulting and non-diverging after source validation because every first-slice capture is an ordinary whole-binding owned-value production.

Once source-valid closure invocation has produced a held closure environment, later explicit argument evaluation may fault or diverge through an existing producer.

If a later argument produces runtime defined fault `F`:

1. the pre-argument closure snapshot has already occurred and is not rolled back;
2. no closure activation begins;
3. no later argument is evaluated;
4. every already produced argument transient is cleaned under the existing call-argument rule;
5. the held closure environment is cleaned by ending every still-owned capture value in reverse capture-list order; and
6. the same defined fault `F` propagates through the existing source-call relation.

If the callee was a non-duplicable closure, its source binding remains consumed; cleanup does not reconstruct that moved value. If the callee was duplicable, the original stored closure remains unchanged because only its duplicated held environment is cleaned.

If later argument evaluation diverges at runtime, no closure activation begins and the held environment plus already produced argument transients remain owned by the suspended call operation. Divergence does not synthesize cleanup merely because execution has not completed.

After activation entry, normal return, defined fault, loop transfer, and lexical block exit use the existing integrated cleanup relations. Capture bindings participate as immutable root-scope bindings. Their remaining ownership is ended in the applicable lexical/binding order, and any partially consumed nominal record capture uses `structural-ownership.md`'s existing remaining frontier. A capture binding already wholly consumed contributes no remaining owned value.

A diverging closure body retains its live closure activation and capture/local state for as long as execution remains divergent.

This document defines no custom closure destructor, finalizer, cancellation hook, panic unwinding, exception object, or environment drop callback.

## Recursion and nesting boundary

The dedicated closure binding does not exist in its own capture environment and is absent from its body's local environment. A closure therefore cannot directly capture or invoke itself through that binding in this slice.

The ordered source-function declaration relation likewise has no simultaneous closure-binding group, so mutually recursive closure initialization is not represented.

A later closure declared in the same ordinary non-generic source-function body MAY capture an earlier dedicated closure binding by value. This establishes an acyclic closure-environment containment edge from the later site to the earlier site. The ordinary capture duplicability/consumption rules apply to the earlier closure value exactly as to every other admitted capture type.

That by-value transport does **not** make the resulting opaque-closure-valued capture binding callable. Bounded closure invocation in this slice remains restricted to a dedicated closure binding selected by local-first lookup.

A closure declaration inside a closure body is source-invalid in this slice. No enclosing-closure capture lookup, recursive environment, self pointer, fixpoint closure identity, target set, or heap indirection is synthesized to bypass this boundary.

## Generic boundary

A closure declaration is invalid inside a generic source-function body in this first slice, regardless of whether one particular capture list happens to mention only concrete values.

This rule keeps closure site/type/body validity independent of generic activation substitution and avoids introducing an abstract environment type, generic closure specialization identity, generic capture capability, or closure-specific body revalidation relation.

A closure body has no generic binder. Inside a non-generic closure body, an ordinary explicit direct call to an existing generic module function may still supply concrete type arguments and use the existing generic-call relation where independently valid.

Opaque closure types are not generic type arguments, marker implementation targets, marker traits, runtime witnesses, or generic function-value forms.

This slice defines no generic/polymorphic closure value, higher-ranked callable, specialization-as-closure, closure template, abstract capture, or marker-derived closure capability.

## Source contextual admission and escape boundary

An opaque closure type/value is admitted directly only as:

- the type/value owned by its dedicated closure binding;
- the type/value of one activation-local capture binding produced when an earlier dedicated closure is captured by a later same-source-function closure; and
- an operation-owned held closure environment during bounded invocation of a dedicated closure binding.

It is not admitted as:

- an ordinary explicitly typed local declaration type or initializer result;
- an explicit source function parameter or result type;
- an explicit closure parameter or result type;
- a nominal record field type;
- a constant/static/module value type;
- a generic type argument;
- a safe-reference referent;
- a raw-pointer pointee;
- a pattern type/head;
- an operator/comparison operand merely by virtue of being a closure;
- a captureless function-value type/value; or
- an exported callable signature component.

Consequently a closure value cannot be passed as an ordinary source-call argument, returned to a caller, stored in a record, published from a module, or otherwise transported into a wider source extent under this first slice. Capture by a later lexical closure declared in the same ordinary source-function body is the only closure-to-closure value transport specifically added here.

This source restriction is not evidence that lower reference-free environment aggregates cannot cross a Core parameter/result boundary. Future first-class or escaping closure work must define an explicit source type/adaptation/opaque-transport relation rather than silently widening `fn(...)` or inferred-local semantics.

## Module static boundary

A source static from `statics.md` is a module entity, not an active outer value binding, and therefore is never an explicit closure capture target. The closure body's retained declaration-site source module/source-unit context permits ordinary static reads and Shared static-root formation directly inside the closure body without an environment slot. This adds no captured storage, creator activation dependency, or hidden closure parameter.

## Source-to-Core refinement

No new Core normative operation or type constructor is required for this closure slice.

A faithful source-to-Core refinement MAY closure-convert each closure site to ordinary existing Core structure as follows.

### Environment type

For each closure site, construct one Core structural environment type with one field per source capture slot in exact capture-list order. Each field type is the ordinary lower Core type corresponding to that capture type; an earlier opaque closure capture recursively maps to that earlier closure's generated environment type.

The generated Core structure is a refinement representation only. Core field order supports proving/cleanup of the lowered value but does not create source capture-field syntax, source nominal record identity, stable layout, ABI field order, byte offset, physical environment address, serialization format, or reflection.

Because every first-slice capture is recursively safe-reference-free and raw-pointer-free, the generated environment type is parameter-transfer-safe under current Core `functions.md`. No lower reference effect contract is needed merely to transfer the environment into the generated wrapper activation.

### Generated wrapper

For each closure site, construct one ordinary Core function implementing the closure body.

The wrapper's parameter sequence contains:

1. the lowered source-visible explicit closure parameters, in their source order; followed by
2. one hidden final environment parameter of the generated environment type.

Placing the hidden environment parameter last preserves the source-visible parameter indices used by any `SharedIdentity(i)` or `SharedDirectChild(i)` result contract. The wrapper advertises the same result type and same contract variant/origin index as the source closure interface.

The hidden environment parameter is not a source parameter and is never a safe-reference result origin. The environment type is reference-free, so adding that final parameter does not introduce an alternative scalar reference candidate.

At wrapper entry, lower the environment's fields into generated capture-local storage in capture-list order by ownership transfer after ordinary explicit parameter transfer. Those Core locals refine the source capture bindings. The source closure body is then lowered using the ordinary existing source-to-Core relations for its operations.

The generated wrapper function identity is not source-observable and does not make the closure a source function entity.

### Closure formation

Lower each source capture production according to its source-selected Duplicate/Consume mode and initialize the corresponding generated environment field. Store the resulting complete environment value in the Core local that refines the dedicated source closure binding.

The lowerer MUST NOT infer source closure duplicability from lower structural copyability when source type policy is stricter. It must preserve the source-selected operation mode.

### Closure invocation

Before lowering/evaluating any ordinary source argument producer, snapshot the Core local representing the selected dedicated closure binding into one operation-owned environment temporary:

- Core `Copy` exactly when the source closure type is duplicable; or
- Core `Move` when the source closure type is non-duplicable.

This snapshot is the lower refinement of source closure-callee evaluation and therefore must precede ordinary source argument producer effects.

Then lower ordinary source arguments left-to-right using existing call lowering. Once those values are ready, emit an ordinary Core call to the statically known generated wrapper. Supply the lowered source-visible arguments in source order and supply `Move(environment_temp)` as the final hidden argument.

The final internal Move of the already-held environment temporary is not source callee evaluation and does not reorder source-visible argument production. It merely transfers the previously produced held environment into the wrapper's final parameter slot.

A direct Core call is sufficient because one opaque closure type/site has exactly one statically known closure body. Adding a callable field/function value, vtable, target set, environment pointer, or second indirect dispatch solely for implementation uniformity is unnecessary for this refinement.

Existing Core call destination admission, parameter transfer, activation, result, safe-reference result contract, fault propagation, and cleanup apply unchanged.

### Cleanup refinement

Source closure cleanup maps to ordinary Core cleanup of the generated environment/capture local structure. Reverse capture-list cleanup corresponds to reverse generated environment field order for a still-intact environment. Partial capture-binding consumption after wrapper entry maps to ordinary Core local/field liveness and cleanup.

A source zero-leaf captured value may have a source ownership cleanup event with no lower scalar destruction effect, consistent with the existing source/Core ownership refinement boundary.

## ABI, layout, address, and realization neutrality

Closure site/type identity, capture order, environment value contents, and invocation semantics establish no physical representation contract.

This document defines no:

- stable environment size, alignment, padding, field offset, or field order in memory;
- stack allocation, heap allocation, arena allocation, reference counting, tracing, or garbage collection requirement;
- physical code pointer or environment pointer;
- address stability, relocation, or pinning guarantee;
- calling convention, ABI, linkage, symbol export, FFI, plugin, or dynamic-library relation;
- backend target capability or hardware closure object.

A legal realization may erase, scalarize, inline, specialize, or otherwise transform the refinement representation when all source/Core semantics remain preserved.

## Deliberate first-slice boundaries

This revision does not define:

- closure declarations inside closure bodies;
- implicit/free-variable capture;
- empty-capture anonymous functions or closure literals;
- capture aliases or explicit move/copy/reference capture modes;
- safe-reference or raw-pointer capture;
- persistent mutable capture state;
- generally passable, returnable, or escaping closure values;
- a nameable closure type spelling;
- general local type inference;
- closure-valued record fields, constants, statics, parameters, or results;
- safe references/raw pointers to closure values;
- generic/polymorphic closures, abstract captures, or closure generic arguments;
- self-recursive or mutually recursive closures;
- invoking an opaque-closure-valued capture binding;
- conversion/coercion/adaptation to captureless `fn(...)` values;
- callable traits, methods, trait objects, vtables, witnesses, or dynamic target sets;
- arbitrary postfix/value invocation;
- closure equality, ordering, hashing, pattern tests, reflection, or stringification;
- async closures, coroutines, generators, or task capture semantics;
- custom destructors/finalizers;
- physical environment representation, ABI, FFI, linkage, or stable layout.

A later accepted consumer may extend one of these boundaries without reinterpreting the first-slice closure-site identity, explicit by-value environment ownership, dedicated-binding invocation boundary, or captureless function-value semantics defined by the current owners.
