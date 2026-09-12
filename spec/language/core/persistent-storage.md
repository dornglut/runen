# Core Execution-Persistent Storage

Status: **provisional normative; incomplete**

This document owns the represented Core semantics for storage whose extent belongs to one represented execution rather than to one Core function activation: persistent-storage declaration identity, one fresh dynamic persistent storage instance per declaration per represented execution, initial stored value, persistent scalar read, Shared root-reference formation over persistent storage, terminal cleanup, and storage-extent ending.

It consumes represented Core scalar types/semantic values, stored-value lifetime, Live state, destruction, and semantic `StorageRegion` identity from [Core value and storage semantics](value-storage.md); the shared/exclusive overlap and alias-compatibility law from [Core borrowing](borrowing.md); safe-reference authority/carrier semantics from [Core references](references.md); outermost normal activation completion from [Core functions and calls](functions.md); outermost defined-fault completion from [Core faults](faults.md); and the abstract Execution phase from [Language lifecycle](../lifecycle.md). It does not redefine those owners.

This relation is semantic and representation-neutral. It introduces no physical global address, allocation class, data section, linker symbol, process-global identity, stable layout, ABI, FFI, module initializer, entry-point rule, or backend storage strategy.

## Persistent declaration identity

A represented Core program MAY contain a finite sequence of **execution-persistent storage declarations**.

Each represented persistent declaration has exactly:

- one stable declaration identity within the represented Core program;
- one exact admitted Core scalar type identity; and
- one exact already-materialized semantic initial value compatible with that type.

The declaration identity is distinct from every dynamic storage-instance identity created for execution. Two declarations remain distinct even when their types and initial values are equal.

The first represented persistent type domain is the ordinary non-pointer, non-reference scalar kinds corresponding to source `Bool`, fixed-width signed/unsigned integers, and binary floating values. Raw pointers, safe references, callable scalar values, structural aggregates, and another future storage-bearing type are not admitted by this revision.

A persistent declaration is not a `LocalId`, function entity, constant operand, callable entity, physical symbol, address, ABI identity, source module binding, or process-global object.

## Per-execution persistent storage instance

At the beginning of each represented Core **execution**, before body execution of the outermost represented Core function activation, exactly one fresh dynamic persistent storage instance is created for each persistent declaration.

Each such instance has one fresh semantic storage-instance identity. Separate represented executions create distinct dynamic instances for the same persistent declaration. Distinct declarations in one execution create distinct instances.

The complete root region of one persistent instance is one semantic `StorageRegion`. Its identity is stable for the complete persistent storage extent. This uses the same representation-neutral storage-region concept consumed by `references.md`; it does not require a physical address or byte layout.

The persistent storage extent begins when the execution's persistent instances are established and ends only at the execution-terminal boundary defined below. It therefore continues while function activations are created, suspended, resumed, recursively nested, or terminated.

## Initial Live state

Every persistent instance begins its storage extent fully **Live** with exactly the semantic initial value attached to its declaration.

This is an establishment fact of represented execution state, not execution of a Core initializer operation. There is no initializer function, activation, source-order evaluation, dependency graph, initialization cycle, fault, divergence, partial initialization, or visibility-before-initialization state in this slice.

Because the admitted first-slice value is one scalar leaf, its initial stored-value lifetime begins with creation of the persistent instance.

## Persistent scalar read

A **persistent scalar read** non-consumingly produces the semantic scalar value currently stored in one selected persistent instance.

A valid persistent scalar read requires:

1. the selected persistent declaration/instance exists in the current represented execution;
2. its complete root storage extent is active;
3. its scalar stored value is Live; and
4. Shared target-access compatibility succeeds against every overlapping active explicit-loan/reference-backed authority under `borrowing.md`.

Successful read:

- produces exactly the stored scalar semantic value with the declaration's exact Core type identity;
- leaves the stored value Live;
- does not move, destroy, replace, initialize, or mutate persistent storage;
- changes no storage-instance identity; and
- creates no reference authority or carrier.

No persistent ownership-moving read, Drop, Assign, InteriorAssign, or mutation operation is represented by this owner.

## Shared persistent root

`references.md` admits one additional root-reference source owned jointly with this persistent relation: the complete root region of a live persistent scalar instance.

Only **Shared** permission is admitted for a persistent root.

Before Shared root formation:

1. the selected persistent instance and its storage extent exist;
2. its exact scalar type equals the safe-reference referent type;
3. its stored value is Live; and
4. Shared alias admission succeeds under the common overlap/conflict relation.

Successful formation creates the ordinary fresh Shared root authority/carrier owned by `references.md` and targets exactly the persistent instance's complete semantic root region. Formation itself does not read, copy, move, destroy, replace, or mutate the persistent value.

Persistent storage cannot form an `Exclusive` or `ExclusiveReplace` root in this revision. It is not a raw-pointer `AddressOf` source. The persistent target category does not add a callable safe-reference result-origin contract.

After a Shared persistent root exists, ordinary safe-reference transport, Shared Copy, reborrow, call transfer, reference-relative Read/Copy, authority delegation, carrier cleanup, and storage-extent validity are exactly the existing relations in `references.md` and `functions.md`.

## Terminal execution cleanup

Persistent storage outlives every represented function activation in the execution, including the outermost activation's local cleanup.

On a **normal** outermost completion:

1. the outermost activation completes its existing return/result preservation and activation-local cleanup under `functions.md`;
2. before the optional normal result is delivered to the outer consumer, persistent terminal cleanup occurs as defined here; and
3. the represented execution then completes normally with the already-preserved optional result.

On a **defined fault** reaching the outermost activation:

1. that activation completes its existing defined-fault activation-local cleanup under `faults.md`;
2. persistent terminal cleanup occurs as defined here; and
3. the represented execution then terminates with the same defined-fault reason.

Before persistent terminal cleanup may begin, no live safe-reference carrier or active descendant authority may remain whose target lies in a persistent instance whose extent is about to end. This includes any carrier preserved in the outermost activation's optional normal result. The requirement is the existing non-dangling storage-extent validity law from `references.md`; violation is a Core language-validation failure. Persistent cleanup does not destroy, end, detach, re-root, or rewrite a preserved result carrier merely to satisfy this precondition.

For each live first-slice scalar persistent value, terminal cleanup ends its stored-value lifetime through the existing destruction/lifecycle relation and then ends the persistent storage extent.

The first persistent domain has no reference/raw/callable carrier, custom destructor body, source-visible drop effect, or other user-observable destruction action. Therefore no semantic inter-declaration cleanup ordering is established by this revision. Source declaration order, module order, and implementation collection order do not acquire cleanup meaning.

## Divergence and undefined behavior

A diverging represented execution retains all persistent instances and their continuing storage extents for that diverging execution. No terminal persistent cleanup is synthesized merely because execution does not complete.

Undefined behavior has no defined post-state or cleanup guarantee. This owner does not convert undefined behavior into normal completion or defined fault.

## Cross-activation reference consequence

A Shared reference whose target is execution-persistent storage may be transferred into and used by nested represented function activations. Its target does not belong to a suspended caller activation; validity follows the persistent storage extent owned here.

This does not widen raw-pointer call transfer, replacement-capable reference transfer policy, safe-reference result-origin contracts, or parameter referent type admission. `functions.md` and `references.md` continue to own those relations.

## Representation and ABI boundary

Persistent dynamic storage-instance identity and `StorageRegion` identity are semantic only.

This revision establishes no:

- stable physical address;
- byte representation, size, alignment, or section placement;
- relocation or pinning guarantee;
- process/global-variable object model;
- symbol name or visibility;
- source or binary linkage;
- calling convention;
- FFI identity;
- dynamic-library behavior;
- entry-point syntax or selection;
- package/module initialization;
- heap allocation; or
- backend/runtime implementation form.

[Core layout and ABI](layout-abi.md) remains unchanged: layout and ABI are not implicitly stable.

## Implementation boundary

This document defines semantic obligations only. It does not authorize or require a Core IR node, reference-machine field, compiler lowering representation, linker object, runtime allocation, backend global, or other implementation mechanism.

Implementation is separately selected only after the source/Core specification slice that consumes this owner is accepted and repository authority is re-established.
