# Source Immutable Execution Statics

Status: **provisional normative; incomplete**

This document owns the represented source semantics for the first persistent static-storage slice: module-level static declaration identity, admitted intrinsic scalar type and literal initializer, module accessibility/lookup, one per-represented-execution persistent storage instance, non-consuming scalar value use, Shared static-root formation, generic/closure lookup behavior, and source-to-Core persistent-storage refinement.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module binding identity/accessibility and same-module/qualified lookup from [Source names and modules](names-modules.md), intrinsic source type identity/duplicability from [Source type foundation](types.md), exact scalar literal materialization from [Source literal semantics](literals.md), local-first body lookup from [Source function-local bindings](local-bindings.md), safe-reference authority/carrier/result-provenance semantics from [Source safe references](references.md), source producer/call execution from [Source function execution](function-execution.md), closure declaration-site module/source-unit context from [Source closures and explicit by-value capture](closures.md), and concrete spelling from [Source concrete syntax](concrete-syntax.md). Its lower semantic owner is [Core execution-persistent storage](../core/persistent-storage.md).

A source static is distinct from a source constant. [Source constants](constants.md) owns compile-time values with no runtime storage identity or addressability. This document instead establishes one semantic persistent storage instance per represented execution and makes that storage observable only through the bounded Shared-reference relation below.

## Static declarations

A **source static declaration** is one module-level declaration that introduces exactly one ordinary module binding under `names-modules.md`.

Each represented static declaration has:

- one lexical user-identifier key;
- one stable source declaration/binding identity;
- one source accessibility class, module-private or exported;
- one exact admitted intrinsic scalar source type;
- one exact semantic scalar initial value materialized from its literal initializer; and
- one mapping to the corresponding Core persistent-storage declaration for refinement.

Distinct static declarations remain distinct module bindings and persistent declarations even when their types and initial values are equal.

Static declaration identity is not the dynamic persistent storage-instance identity created for an execution. It is also not a physical address, linker symbol, ABI identity, process-global identity, reflection identity, or implementation storage handle.

Static bindings occupy the same ordinary module declaration namespace as records, functions, marker traits, and constants. The ordinary duplicate-key prohibition applies across all participating declaration categories.

## Accessibility and module lookup

Static accessibility is exactly the source accessibility relation from `names-modules.md`.

- A module-private static is selectable by same-module lookup.
- An exported static is selectable by same-module lookup and by the existing one-hop qualified `alias::member` lookup from another source module.

`export` changes source lookup accessibility only. It does not establish physical symbol export, linkage, ABI/FFI visibility, stable address, runtime publication mechanism, or dynamic-library behavior.

The consuming static context requires the selected module binding to be a static. A selected inaccessible or wrong-category binding is final; lookup does not skip it in search of a static.

## Admitted static types

The first represented static type domain is exactly:

- `Bool`;
- `I8`, `I16`, `I32`, `I64`;
- `U8`, `U16`, `U32`, `U64`; and
- `F16`, `F32`, `F64`.

Every static states one exact type from this domain. No type inference, default type, conversion, coercion, promotion, widening, narrowing, or subtyping relation is introduced.

The following are not represented static types:

- nominal records;
- safe references;
- raw pointers;
- captureless function values;
- opaque closure-site values;
- abstract generic type parameters; or
- another future source type category.

Every admitted first-slice static type is source-duplicable and has no structural ownership subpaths requiring persistent ownership-state tracking.

## Literal initializer

Each static has exactly one initializer and it is exactly one existing represented source literal, not a general `Value`.

The static's declared type is the required type supplied to `literals.md`:

- `Bool` requires a represented Boolean literal;
- fixed-width integer types require a represented decimal integer literal materializable at that exact type; and
- `F16`/`F32`/`F64` require a represented decimal floating literal materializable at that exact type.

Successful source validation establishes the exact semantic scalar initial value attached to the static declaration and lower persistent declaration.

No source operation executes at runtime to initialize the static. This slice introduces no static initializer call, declaration-order evaluation, dependency edge, cycle relation, fault, divergence, partial initialization, lazy initialization, module initialization phase, or source entry-point phase.

## Per-execution persistent instance

For each represented execution, the lower persistent-storage relation creates exactly one fresh storage instance for each represented source static declaration.

The static instance is fully Live before body execution of the outermost represented activation and its extent continues across every source/Core function or closure activation until execution-terminal persistent cleanup.

Separate executions create fresh instances for the same static declaration. Two distinct static declarations create distinct instances in the same execution.

This source semantic fact does not expose the lower storage-instance token, Core `StorageRegion`, physical address, allocation identity, or backend realization.

## Unqualified static value use

In a bare unqualified body value position:

1. active binding lookup in the current source-function or closure body's own lexical tree occurs first under `local-bindings.md`;
2. any selected active binding is category-final for that occurrence;
3. only when no active binding resolves the key does same-module lookup under `names-modules.md` apply;
4. when same-module lookup selects a static, the static-read relation below applies; and
5. a selected wrong-category module binding is final and is not bypassed.

In a closure body the same-module context is the closure declaration site's source module. Uncaptured creator bindings are absent from the fresh closure root and therefore do not suppress same-module static lookup.

A static is not a body-local binding. Static use introduces no binding structural-ownership state, assignment-mutability state, lexical storage extent, capture slot, or activation cleanup entry.

## Qualified static value use

A qualified static value use consumes the existing one-hop `alias::member` lookup relation.

The alias comes from the current source unit—or the retained declaration-site source unit for a closure body—and the selected target binding must be exported and must be a static. Body-local bindings do not participate in explicit qualification.

Qualification adds no nested path, runtime module object, static member lookup, or general value/path grammar.

## Static scalar read

A successful static value use performs one non-consuming read of the selected execution-persistent scalar storage.

The read:

- requires the selected static instance to exist and remain Live;
- requires canonical Shared safe-authority compatibility for direct access to that persistent root;
- produces exactly one owned scalar value with the static's exact declared source type and current semantic value;
- leaves persistent storage Live and unchanged;
- changes no source structural ownership state;
- creates no reference authority/carrier; and
- is non-faulting and non-diverging after source validation.

Because the storage is immutable in this slice and the initial scalar value cannot be replaced, every successful ordinary read observes the same semantic scalar value. The operation is nevertheless a storage read, not constant-value production: the static's independently addressable Shared storage identity is semantically real.

Repeated static reads are valid. No read consumes persistent storage.

## Shared static root

A represented static may be the complete root target of **Shared** safe-reference formation.

The admitted concrete forms are:

```text
&Name
&alias::Name
```

For unqualified `&Name`, local-first lookup remains controlling: an active body binding with key `Name` is selected first and is category-final. Only when no active local resolves may same-module lookup select a static root.

For `&alias::Name`, explicit qualified lookup must select one exported static. This is one bounded static-specific qualified Shared-root form; it does not introduce general qualified address-of/place syntax.

A valid Shared static-root formation requires:

- exact surrounding required type `SharedRef(T)`;
- the selected static's exact declared type is `T`;
- `T` is Shared-referent-admissible under `references.md`;
- the persistent static instance exists and is Live; and
- canonical Shared authority compatibility succeeds at that root.

Successful formation creates one fresh Shared authority/carrier targeting exactly the persistent static root and records fresh **StaticRootOrigin(staticBinding)** validation provenance.

Static root formation performs no static value read, copy, move, replacement, or mutation.

A static cannot be the root of `&mut`, `raw &`, field-root addressability, ownership Move/Drop, or assignment in this slice.

## Reference transport and result boundary

A Shared static-root reference is an ordinary `SharedRef(T)` value after formation. It may be duplicated, stored in an immutable reference local, reborrowed over its complete scalar referent, transferred as a call argument, and used from another activation under the existing safe-reference relations.

Its target remains the same execution-persistent storage region rather than caller-activation storage.

Fresh static-root provenance is **not** an admitted source callable result origin. A function or closure cannot directly return a newly formed `&STATIC` merely because the target outlives the activation. Existing callable result contracts remain explicit-parameter-origin-only.

If a caller passes a static-root Shared reference into a parameter selected by `SharedIdentity(i)`, the callee may return that exact incoming authority under the ordinary identity-preserving contract. The caller-side provenance remains the caller's original static-root provenance.

## Immutability and excluded access

Persistent static storage is immutable under the represented source relation.

This revision defines no:

- static assignment or reinitialization;
- `&mut` static root;
- raw pointer/address of static storage;
- ownership-moving static value access;
- explicit Drop of static storage;
- interior mutation;
- mutable/thread-local/atomic global; or
- synchronization/data-race rule for statics.

The absence of mutation keeps Exec concurrency outside this first static slice.

## Generic and closure behavior

A static declaration is not generic and has no type/value/const/lifetime/effect parameter.

A static use inside a generic source-function body is one ordinary concrete module storage/value relation independent of generic substitution.

A closure body retains its declaration-site module and source-unit alias context and may therefore read or form a Shared reference to an applicable static directly through ordinary module lookup.

A static is **not** a closure capture target. A capture-list identifier must resolve to an admitted active outer binding under `closures.md`; module statics are not such bindings and occupy no closure environment slot. Direct static lookup remains available inside the closure body instead.

## Terminal lifecycle

The source static's persistent storage extent and terminal cleanup refine directly to `core/persistent-storage.md`.

On normal or defined-fault terminal execution, outermost activation cleanup occurs before persistent storage cleanup. Divergence retains persistent storage for the diverging execution.

The first source static domain has no custom destructor, source-visible cleanup body, or user-observable inter-static cleanup action. This document establishes no source declaration-order cleanup semantics.

## Core refinement

Each source static declaration refines to one Core execution-persistent storage declaration:

- the exact source intrinsic scalar type maps to its accepted corresponding Core scalar type;
- the source literal initializer's already-materialized semantic value maps to the corresponding Core semantic initial value;
- source static declaration identity maps to a distinct lower persistent declaration identity;
- each represented execution obtains the fresh dynamic instance owned by `core/persistent-storage.md`;
- ordinary source static reads refine to persistent scalar reads; and
- Shared source static roots refine to Shared persistent-root formation under Core `references.md`.

Source name, module identity, accessibility, source location, and original qualification need not survive after source lookup except as compiler evidence needed to select the resolved persistent declaration.

No source `export` fact becomes Core linkage or ABI metadata through this refinement.

## Deliberate boundaries

This revision does not define aggregate/non-duplicable statics, static fields/structural paths, runtime static initialization, static initializer dependencies/cycles, mutation, thread-local or atomic statics, static-origin callable result contracts, raw static pointers, pointer/address observation, stable layout/address, relocation/pinning, process-global lifetime, entry-point selection, ABI, FFI, linkage, symbol visibility/versioning, dynamic libraries, package initialization, heap allocation, or implementation representation.

Those require separately accepted consumers and canonical owners.
