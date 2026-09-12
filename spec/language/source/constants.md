# Source Constants

Status: **provisional normative; incomplete**

This document owns the represented source constant declaration, constant-value, constant-use, and constant-to-Core refinement relations. It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module binding identity/accessibility and lookup from [Source names and modules](names-modules.md), intrinsic source type identity and duplicability from [Source type foundation](types.md), and exact scalar literal materialization from [Source literal semantics](literals.md). It does not redefine those owners.

Concrete spelling is owned by [Source concrete syntax](concrete-syntax.md). Local-first lookup precedence in source-function and closure bodies is owned by [Source function-local bindings](local-bindings.md). Closure bodies retain declaration-site source module/source-unit context under [Source closures and explicit by-value capture](closures.md). Evaluation/producer sequencing is owned by [Source function execution](function-execution.md).

## Constant declarations

A **source constant declaration** is a module-level declaration that introduces exactly one module binding under `names-modules.md`.

Each represented constant declaration has:

- one lexical user-identifier key;
- one stable declaration/binding identity;
- one source accessibility class, either module-private or exported;
- one exact declared source type from the admitted constant type domain below; and
- exactly one fully materialized compile-time semantic scalar value of that declared type.

Distinct constant declarations are distinct module bindings even when their declared types and semantic values are equal.

Constant declaration identity is name-resolution identity only. It is not runtime value identity, storage identity, address identity, linkage identity, ABI identity, reflection identity, or Core global identity.

Constant bindings occupy the ordinary module declaration namespace from `names-modules.md`. A constant therefore participates in the same duplicate-key prohibition as represented record, function, and marker-trait declarations. This document introduces no separate module value namespace or overload set.

## Accessibility and lookup

Constant accessibility is exactly the ordinary source accessibility relation from `names-modules.md`.

- A module-private constant may be selected by same-module lookup.
- An exported constant may be selected by same-module lookup and by qualified cross-module lookup through an existing source-unit module alias.

Accessibility is source lookup only. Exporting a constant does not establish ABI export, linkage, physical symbol publication, runtime storage, or another realization guarantee.

A qualified constant lookup follows exactly one existing `alias::member` module-qualification relation. Imported modules are not searched implicitly. This document introduces no direct imported constant binding, re-export, nested module path, wildcard/selective import, prelude, package relation, or alternative lookup domain.

The consuming value context requires the selected module binding to be a constant. A selected inaccessible or wrong-category binding is final; lookup does not skip it in search of a constant.

## Admitted constant types

The represented constant type domain is exactly the intrinsic scalar source types:

- `Bool`;
- `I8`, `I16`, `I32`, `I64`;
- `U8`, `U16`, `U32`, `U64`;
- `F16`, `F32`, `F64`.

Every represented constant declaration states one exact declared type from this domain. This relation provides no type inference, default type, suffix selection, conversion, coercion, promotion, widening, narrowing, or subtyping.

The following are not represented constant types under this revision:

- nominal record types;
- safe-reference types;
- raw-pointer types;
- captureless function-value types;
- opaque closure-site types;
- abstract generic type-parameter expressions; and
- any other future source type category.

The intrinsic-only boundary is semantic rather than a representation accident. Every admitted type is source-duplicable under `types.md`, and every value admitted by the initializer relation below has an existing representation-neutral Core constant value.

## Literal initializer

Each represented constant declaration has exactly one initializer. The initializer is exactly one represented source literal, not a general source `Value` producer.

The declaration's exact type is the literal's exact required source type. Initialization succeeds only through the literal materialization relation owned by `literals.md`:

- `Bool` requires one represented Boolean literal;
- a represented fixed-width integer type requires one represented decimal integer literal whose mathematical datum is representable under that exact required type; and
- `F16`, `F32`, or `F64` requires one represented decimal floating literal materialized under that exact required binary-floating type.

A literal that cannot materialize under the exact declared type makes the constant declaration source-invalid and establishes no constant value.

Successful materialization establishes the declaration's exact compile-time semantic scalar value. It performs no runtime initialization, creates no function/closure activation or transient source storage, and has no initialization side effect.

This initializer relation introduces no:

- operator evaluation;
- constant reference;
- record construction;
- direct, bounded-indirect, or bounded-closure call;
- closure capture/formation;
- safe-reference or raw-pointer formation;
- defined-fault or divergence-producing computation;
- general constant-expression category; or
- compile-time execution model.

## Declaration ordering and dependency boundary

A represented constant initializer is self-contained and cannot name another declaration. Consequently this revision creates no constant-initializer dependency edge and requires no constant-evaluation ordering, cycle, or fixed-point relation.

Constant declarations remain ordinary order-independent module declarations. Reordering source units or textual module declarations does not change constant binding identity, lookup, type, or value.

Existing source module-import cycles remain governed by `names-modules.md`. A future constant-expression or static-initialization owner may introduce an independently justified dependency/cycle relation without changing the constant declaration/value relation defined here.

## Unqualified constant use

In a bare unqualified value position within a represented **source-function body or closure body**, local-first lookup retains the precedence owned by `local-bindings.md` for that body's own lexical tree.

1. If an active binding in the current body resolves the lexical identifier key, that selection is final and the consuming operation applies the selected binding/category semantics.
2. Only when no active binding in the current body resolves the key does same-module lookup under `names-modules.md` apply, using that body's source-module context.
3. If same-module lookup selects a constant binding, the constant-value production relation below applies.
4. If same-module lookup selects another category, the consuming value context rejects it; lookup does not continue by desired category.

For a closure body, the same-module context is the closure declaration site's source module retained by `closures.md`. Creator source-function bindings that were **not explicitly captured** are absent from the fresh closure-body local domain. Consequently an uncaptured creator local with the same key does not suppress same-module constant fallback inside the closure body. Explicit capture bindings, closure explicit parameters, and active closure-body locals remain category-final and do suppress fallback exactly as ordinary local-first lookup requires.

A constant is not a body-local binding and acquires no local binding identity, assignment mutability, structural availability, lexical storage extent, closure capture slot, or cleanup state merely because it is used in a source-function or closure body.

## Qualified constant use

A qualified constant use consumes the existing two-part module-qualified lookup relation from `names-modules.md`.

The first key selects a source-unit module alias. The second key must select an exported constant binding in that target module. Body-local value bindings do not participate in this explicitly qualified lookup.

A closure body uses the **declaration-site source unit** retained by `closures.md`, so it sees exactly that source unit's module-alias environment. A same-spelled closure/body-local key does not affect explicit `alias::member` lookup.

Qualification creates no nested path, arbitrary member access, method/associated-item lookup, indirect call, or runtime module value.

## Constant value production

A successful constant use produces exactly one owned source scalar value:

- its source type is exactly the constant declaration's intrinsic source type; and
- its semantic value is exactly the declaration's fully materialized constant value.

Constant use is effect-free, non-faulting, and non-diverging. It does not consume, move, mutate, initialize, or otherwise change the constant declaration. It creates no source-visible storage instance, address, alias, lifetime, closure capture, or shared storage relation between uses.

A represented constant may therefore be used repeatedly. Repeated use is not modeled as copying from hidden runtime storage. Each source use directly produces one owned value equal to the declaration's semantic constant value.

Because every admitted constant type is duplicable, this repeated-production relation does not grant a new duplicability capability to any type.

A resolved constant use supplies one exact static result source type to receiving/operation-selection relations that consume exact producer result facts. `function-execution.md` owns integration of this producer with ordinary source evaluation and receiving contexts in either activation kind.

This document does not make a named constant interchangeable with a pattern literal, range bound, type-level value, enum discriminant, array size, or another future constant-consuming syntax. Those consumers require their own accepted relations.

## Generic and closure boundary

A represented constant declaration is not generic and has no type, value, const, lifetime, pack, effect, or other generic parameter.

A constant use inside a generic source-function body remains one ordinary concrete module-value producer. Its exact intrinsic type and semantic value are independent of the generic function's type-parameter substitution.

A constant use inside a closure body likewise remains one ordinary module-value producer resolved from the retained declaration-site module/source-unit context. Constants are not captured into closure environments; direct module lookup remains available when local-first lookup does not select a body binding.

The generic type-parameter lookup domain remains type-position-only and does not shadow constant lookup in value positions. Body-local value bindings retain their existing local-first precedence.

This relation introduces no const/value generic parameter, type-level constant, closure capture of module constants, specialization-key dimension, associated constant, marker requirement, trait capability, or runtime witness.

## Core refinement

No Core semantic extension is required by the represented constant relation.

Every admitted source constant value maps to an existing representation-neutral Core constant value of the corresponding scalar type:

- `Bool` and fixed-width integer values map to their exact corresponding Core scalar values; and
- represented `F16`, `F32`, and `F64` literal values map to the existing representation-neutral Core binary-floating values produced by accepted decimal-literal materialization, including represented signed zero, subnormal, normal, and infinity values.

The represented initializer domain cannot produce a NaN member, safe-reference value, raw-pointer value, function value, closure value, or another value absent from the current Core constant-value relation.

A source constant use may therefore refine directly to the corresponding existing Core constant operand/value at its consuming Core operation after source validation. The constant declaration itself need not become a Core program entity.

Source module identity, constant binding identity, name, accessibility, source location, declaration presentation order, and closure declaration-site context erase after source resolution once a use retains the exact typed semantic value required for lowering.

This refinement does not prescribe textual substitution, constant folding, compiler storage, object-file representation, or another realization strategy. It establishes only that the represented source semantics require no runtime global storage, initializer function, load, symbol, reference, pointer, ABI, or linkage object.

## Static-storage boundary

A source constant is not static storage.

A represented constant declaration has no:

- persistent runtime storage identity or extent;
- source addressability;
- safe-reference root-target status;
- raw-pointer target/provenance status;
- closure capture/environment identity;
- assignment or replacement mutability;
- runtime initialization phase or ordering;
- program-start or program-end lifecycle;
- cleanup/destructor ordering; or
- physical address, layout, ABI, FFI, or linkage guarantee.

Persistent source storage, if later accepted, requires its own canonical semantics for storage identity, initialization, access, reference/raw-pointer origins, lifecycle, and applicable physical guarantees. Nothing in this document predefines that relation.

## Deliberate boundaries

This revision does not define:

- source static storage;
- constant references in constant initializers;
- constant-expression operators;
- constant-evaluable functions or general compile-time execution;
- const/value/lifetime generic parameters;
- nominal-record, function-value, or closure constants;
- safe-reference or raw-pointer constants;
- NaN constant source fabrication;
- pattern/range/type-level constants;
- macros, reflection, metaprogramming, or build execution;
- runtime global initialization or cleanup;
- mutable, thread-local, or atomic globals;
- source addressability, physical address stability, relocation, or pinning; or
- ABI, layout, FFI, linkage, or physical symbol semantics.
