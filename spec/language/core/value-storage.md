# Core Value and Storage Semantics

Status: **provisional normative; incomplete**

This document owns the currently defined Core semantics for values, local storage places, storage extent, dynamic local storage-instance identity, stored-value lifetime, initialization state, ownership transfer, assignment mutability, interior-mutability regions, non-replacing result storage for represented Core integer addition, subtraction, multiplication, exclusive-or, bitwise OR, and standard binary floating addition, assignment, destruction domains, and cleanup.

The shared/exclusive access authority required to reach storage while loans are active is owned by [Core borrowing](borrowing.md). Raw-pointer values and provenance formed from storage are owned by [Core pointers and provenance](pointers.md). Dynamic function-activation creation, caller suspension, and value transfer across direct calls are owned by [Core functions and direct calls](functions.md). Intra-activation basic-block transfer and cyclic control-flow divergence are owned by [Core control flow](control-flow.md). The exact mathematical value relations consumed by represented fixed-width integer addition, subtraction, multiplication, exclusive-or, and bitwise OR are owned by [Core integer semantics](numerics/integers.md). The numerical result relation and numeric-contract authority consumed by represented standard binary floating addition are owned by [Core floating-point semantics](numerics/floating-point.md).

## Terms

### Type

A semantic classification for values and places. This revision defines scalar or leaf types and closed structural aggregate types for the operations below.

Every represented scalar type has one semantic **scalar kind** classifying its scalar value family. Scalar kind and the per-program type identity/type definition used to refer to one represented type are distinct facts; distinct represented type definitions MAY have the same scalar kind.

This revision uses that distinction below for the `Bool` scalar kind. It does not by that fact enumerate, redefine, or unify the separately governed integer, floating, raw-pointer, or verification-fixture scalar semantics.

A type may carry an **interior-mutable** semantic marker. The marker belongs to the proving-kernel type model; this revision does not define source syntax for declaring such a type.

### Local

A typed storage declaration belonging to one function body. A local is immutable or mutable for ordinary assignment purposes.

`LocalId` is a stable identifier for that MIR declaration within one body. It is not the dynamic identity of one execution's storage extent.

The local's assignment-mutability flag does not determine alias exclusivity and does not determine whether storage inside the local is interior-mutable.

### Place

A static proving-MIR storage designation consisting of a local declaration plus zero or more structural field projections.

A place denotes which structural storage is selected within the current execution's corresponding local storage instance; it is not itself a value, dynamic storage-instance identity, pointer, address, or provenance token.

### Sub-place

A place reached by projecting a field from an aggregate place.

### Value

An initialized semantic datum whose structure is compatible with the type required by its use. The currently defined constant-value representation does not carry independent nominal type identity or dynamic raw-pointer provenance.

A represented semantic value need not be directly fabricable by the current constant-value representation merely because an accepted runtime operation can produce and store that value. Constant fabrication is one operand-production capability, not the definition of the complete semantic runtime value domain. An operation that produces a semantic value outside the current constant carrier therefore does not by itself introduce a corresponding constant form.

Every represented Core scalar type whose scalar kind is `Bool` has exactly two semantic values: **`true`** and **`false`**.

This two-value domain belongs to the Bool scalar kind; it does not designate or require one globally distinguished Bool type identity. Distinct represented type definitions may each have scalar kind `Bool` while using the same two semantic scalar values.

The Bool values are not required physical bits, integers, source spellings, ABI representations, or layout encodings. This value definition introduces no truthiness, conversion, comparison, logical operation, ordering, or other Bool operation. An accepted consumer such as [Core control flow](control-flow.md) may select behavior from these two values without redefining their value domain.

### Assignment mutability

Assignment mutability is permission for the ordinary `Assign` operation to replace or reinitialize storage rooted in a local.

In the current Core proving MIR, ordinary assignment mutability is declared by the containing `LocalDecl.mutable` flag.

Assignment mutability is independent of:

- whether current alias authority is shared or exclusive;
- whether a target lies inside an interior-mutable region.

Therefore an exclusive loan does not make an immutable local ordinarily assignable, and an interior-mutable type does not make ordinary `Assign` legal on an immutable local.

### Interior-mutable region

A place lies **within an interior-mutable region** exactly when, while following the structural path from the containing local's root type to the target place type, the target type or at least one structural ancestor type on that path is marked interior-mutable.

Consequences:

- when a local's root type is marked, the whole local storage region and all structural descendants lie within that interior-mutable region;
- when only a nested field type is marked, that field and its structural descendants lie within an interior-mutable region;
- an unmarked containing aggregate does not become wholly interior-mutable merely because one descendant type is marked;
- a disjoint sibling outside the marked descendant region does not inherit the marker.

Interior mutability is storage/type capability, not alias authority. It does not create or upgrade a loan, permit ownership-consuming access through a shared loan, or imply ordinary local assignment mutability.

### Storage extent

The storage extent of a place is the interval of execution during which that storage exists and may potentially hold a value.

For represented Core functions, each dynamic local storage root exists from creation of its containing function activation through that local's termination cleanup. Structural sub-place storage exists within the storage extent of its containing local. Activation creation itself is owned by [Core functions and direct calls](functions.md).

Storage extent is independent of initialization state. Never-initialized, Live, and Dead storage all continue to exist until their storage extent ends.

Ending, moving, destroying, or replacing a stored value does not by itself end the containing storage extent.

Storage extent does not imply a physical address, allocation identity, relocation rule, or pointer provenance.

### Dynamic storage-instance identity

Every dynamic local storage extent has one semantic **storage-instance identity** for the complete duration of that extent.

Distinct simultaneously existing local storage extents have distinct storage-instance identities.

Whenever a Core function activation is created, one fresh dynamic local storage instance is created for every local declaration in that activation's body. Repeated or recursive activations of the same function therefore create new storage instances for the same static local declarations. The static `LocalId` identifies a declaration within one function body; the dynamic storage-instance identity identifies one particular activation's storage extent. They are different semantic concepts even when a reference implementation allocates deterministic verification tokens in local-declaration order.

The storage-instance identity remains stable while the storage extent continues. In particular, none of the following creates a new storage instance:

- initialization, including initialization after an earlier stored-value lifetime ended;
- moving a stored value out;
- explicit destruction;
- ordinary assignment or reinitialization;
- interior assignment or reinitialization.

Those operations affect stored-value lifetimes and initialization state, not storage-instance identity.

The current reference oracle may represent storage-instance identity using an opaque deterministic integer so tests can distinguish and compare instances. That numeric representation is verification-only. It is not Runen-observable, not a physical address, not an ABI promise, and not permission to access storage.

When a local's storage extent ends after cleanup, that dynamic storage instance ends. A later function activation creates a fresh dynamic storage instance rather than reviving the ended instance, even when it executes the same static local declaration. Storage owners outside represented Core function activations must likewise define their own dynamic identities rather than treating `LocalId` as globally unique storage identity.

A static place resolved during execution therefore denotes a **structural storage region** within the current dynamic local storage instance: the root storage-instance identity plus the place's structural projection path. The projection path is structural semantics, not a byte offset or layout guarantee.

### Stored-value lifetime

A stored-value lifetime is the interval during which one scalar storage leaf is Live with one stored semantic value.

A stored-value lifetime begins when a semantic write to that scalar leaf completes and changes it to Live.

A stored-value lifetime ends when the stored value is consumed by move, destroyed, or destroyed as part of replacement. A later write into the same storage begins a new stored-value lifetime without creating a new storage extent or storage-instance identity.

`Read` and `Copy` do not end the source stored-value lifetime.

`Init` and represented `IntegerAdd`/`IntegerSub`/`IntegerMul`/`IntegerXor`/`IntegerOr`/`FloatAdd` result initialization may begin either the first stored-value lifetime or a later stored-value lifetime in vacant storage. Ordinary `Assign` and `InteriorAssign` may end old stored-value lifetimes and begin replacement lifetimes in the same storage extent and the same dynamic storage instance.

The current revision defines stored-value lifetime at scalar storage leaves. Aggregate initialization and liveness are derived recursively from the states of those leaves; an aggregate does not acquire a separate hidden lifetime identity.

Transient values produced while evaluating constants, moves, copies, pointer formation, or represented integer- or floating-operation operands are semantic values. This revision does not give ordinary transient operand results independently addressable storage or a separately specified storage extent.

### Live

A scalar storage leaf currently contains an initialized value and therefore has an active stored-value lifetime.

### Never-initialized

A scalar storage leaf has not yet begun any stored-value lifetime during its current storage extent.

### Dead

A scalar storage leaf previously had a stored-value lifetime that ended by move, destruction, or replacement, and it has not subsequently been written again.

Aggregate initialization state is derived recursively from its leaves; it is not a separate boolean flag.

### Vacant

A scalar storage leaf is **vacant** exactly when it is not Live: its state is Never-initialized or Dead.

A structural aggregate place is **wholly vacant** exactly when every recursively contained scalar leaf is vacant. A mixed Never-initialized/Dead aggregate with no Live scalar leaf is therefore wholly vacant. A recursively zero-leaf structural value is vacuously wholly vacant; no hidden aggregate lifetime state is introduced merely to distinguish first from later initialization.

Vacancy is determined for the selected destination place or sub-place. Live storage outside that selected structural region does not make the selected destination non-vacant.

A wholly vacant place has an empty destruction domain.

### Destruction domain

The destruction domain of a place at a semantic step is the ordered sequence of currently Live scalar leaf places recursively contained by that place.

For a scalar place, the destruction domain is that place itself when Live and is empty when Never-initialized or Dead.

For an aggregate place, the destruction domain is formed by recursively concatenating field destruction domains in reverse field declaration order.

A destruction domain is determined from semantic storage state at the point where destruction is to occur. Never-initialized, moved, and already-destroyed leaves are not members of the domain.

The destruction domain specifies which stored-value lifetimes are ended by destruction and in what order. It does not define a custom destructor body.

## Structural initialization

A scalar place is fully initialized exactly when its leaf is Live.

An aggregate place is fully initialized exactly when all recursively contained scalar leaves are Live.

An aggregate may be partially initialized when only a strict subset of its leaves are Live.

A move from one field affects only that field. A partially initialized aggregate cannot be read, moved, or copied as a whole until every required leaf is Live again.

Partial initialization does not change storage extent or storage-instance identity. It changes only which scalar leaves currently have stored-value lifetimes.

## Non-replacing initialization

`Init(dst, value)` writes a complete value into a place only if `dst` is wholly vacant.

Destination vacancy is established at the `Init` operation point before evaluation of the source operand. Source evaluation therefore cannot make an initially Live destination admissible to that same `Init`. In particular, `Init(dst, Move(dst))` is invalid when `dst` is Live at admission rather than becoming an assignment-like replacement after the move.

The value MUST structurally match the type of `dst`.

Initialization does not require the containing local to be mutable for ordinary assignment.

After successful source evaluation, `Init` writes the complete value without destroying destination contents because a wholly vacant destination has no Live destruction domain. Each scalar leaf written by the operation becomes Live and begins a new stored-value lifetime, whether or not an earlier stored-value lifetime existed in that storage.

`Init` remains an exclusive-access operation under the borrowing rules. Interior mutability does not weaken initialization access requirements.

Initialization does not create or replace the local's storage-instance identity; that identity already exists for the continuing storage extent before initialization occurs.

## Plain fixed-width integer addition result initialization

The represented Core proving relation contains one plain fixed-width integer-add operation, normatively written here as:

```text
IntegerAdd {
    dst: Place,
    left: Operand,
    right: Operand,
}
```

This semantic spelling identifies the represented Core operation. It does not require one particular implementation enum, instruction encoding, backend opcode, or physical machine instruction.

Let `D` be the exact Core type identity of `dst`. A valid `IntegerAdd` requires all of the following static type facts before execution:

- `D` denotes a scalar type whose scalar kind is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`;
- every place-derived left or right operand produces a value whose selected Core type identity is exactly `D`;
- a constant operand is admitted only when its semantic value matches `D` under the existing type-table value-matching relation; and
- no equality of scalar kind alone makes two distinct Core type identities interchangeable for an operand.

An `AddressOf` operand therefore cannot satisfy this operation's fixed-width integer operand requirement. Any applicable `Move`, `Copy`, `RawMove`, or other represented operand form retains its own existing access, liveness, copyability, provenance, and ownership-transfer requirements while also producing the exact required type `D`.

`IntegerAdd` reuses the non-replacing destination lifecycle of `Init` rather than defining a second initialization model. Before either operand is evaluated:

1. resolve `dst` as direct storage under the existing place/type rules;
2. require the exclusive direct-storage authority that ordinary `Init` requires under `borrowing.md`; and
3. require `dst` to be wholly vacant.

These destination preconditions are fixed at operation admission before left-operand evaluation. Operand evaluation cannot make an initially Live or insufficiently authorized destination retrospectively admissible to that same operation. Ordinary assignment mutability is not required, exactly as for `Init`.

After destination admission, execution is exactly:

1. evaluate `left` completely under its existing `Operand` semantics with required Core type identity `D`;
2. preserve all state consequences of that operand evaluation;
3. evaluate `right` completely under its existing `Operand` semantics with required Core type identity `D` in the resulting state;
4. preserve all state consequences of that operand evaluation;
5. consume the two produced semantic fixed-width integer operand values to compute the exact plain addition result owned by `numerics/integers.md` for the scalar kind of `D`;
6. write exactly that one result into `dst`; and
7. mark the written scalar destination Live, beginning a new stored-value lifetime there.

The left operand is therefore evaluated before the right operand. `Move`, `Copy`, raw-pointer operand access, and all other operand-local effects remain exactly those of the existing operand relation; `IntegerAdd` does not duplicate or weaken them. A source place moved by the left operand is already Dead when the right operand is evaluated. A right operand cannot read, copy, or move such a place unless an independent represented operation has legally restored its value beforehand.

Because `dst` was wholly vacant at admission, the result write has an empty destruction domain and performs no replacement destruction. The write has the same stored-value-lifetime and storage-instance consequences as successful non-replacing `Init`: it begins the destination stored-value lifetime without changing the destination storage extent or storage-instance identity.

After both operand values have been produced, the arithmetic/result-write portion is finite, deterministic, non-faulting, and non-diverging under the represented Core semantics. The operation introduces no new borrow interval, reference value, pointer provenance, cleanup category, storage identity, assignment-mutability rule, interior-mutability rule, fault reason, control-flow edge, layout/ABI promise, runtime flag, or backend-visible semantic fact.

The operation consumes only the plain fixed-width integer-add value relation from `numerics/integers.md`. It does not represent checked, saturating, or explicitly wrapping addition, another arithmetic or comparison operation, a conversion, a source numeric-contract selection, or a constant-folding requirement.

## Plain fixed-width integer subtraction result initialization

The represented Core proving relation contains one distinct plain fixed-width integer-subtract operation, normatively written here as:

```text
IntegerSub {
    dst: Place,
    left: Operand,
    right: Operand,
}
```

This semantic spelling identifies the represented Core operation. It does not require one particular implementation enum, generic arithmetic opcode, instruction encoding, backend opcode, or physical machine instruction.

Let `D` be the exact Core type identity of `dst`. A valid `IntegerSub` requires all of the following static type facts before execution:

- `D` denotes a scalar type whose scalar kind is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`;
- every place-derived left or right operand produces a value whose selected Core type identity is exactly `D`;
- a constant operand is admitted only when its semantic value matches `D` under the existing type-table value-matching relation; and
- no equality of scalar kind alone makes two distinct Core type identities interchangeable for an operand.

An `AddressOf` operand therefore cannot satisfy this operation's fixed-width integer operand requirement. Any applicable `Move`, `Copy`, `RawMove`, or other represented operand form retains its own existing access, liveness, copyability, provenance, and ownership-transfer requirements while also producing the exact required type `D`.

`IntegerSub` reuses the non-replacing destination lifecycle of `Init` and `IntegerAdd` rather than defining a second storage model. Before either operand is evaluated:

1. resolve `dst` as direct storage under the existing place/type rules;
2. require the exclusive direct-storage authority that ordinary `Init` and `IntegerAdd` require under `borrowing.md`; and
3. require `dst` to be wholly vacant.

These destination preconditions are fixed at operation admission before left-operand evaluation. Operand evaluation cannot make an initially Live or insufficiently authorized destination retrospectively admissible to that same operation. Ordinary assignment mutability is not required, exactly as for `Init` and `IntegerAdd`.

After destination admission, execution is exactly:

1. evaluate `left` completely under its existing `Operand` semantics with required Core type identity `D`;
2. preserve all state consequences of that operand evaluation;
3. evaluate `right` completely under its existing `Operand` semantics with required Core type identity `D` in the resulting state;
4. preserve all state consequences of that operand evaluation;
5. consume the two produced semantic fixed-width integer operand values to compute the exact plain subtraction result owned by `numerics/integers.md` for the scalar kind of `D`;
6. write exactly that one result into `dst`; and
7. mark the written scalar destination Live, beginning a new stored-value lifetime there.

The left operand is therefore evaluated before the right operand. `Move`, `Copy`, raw-pointer operand access, and all other operand-local effects remain exactly those of the existing operand relation; `IntegerSub` does not duplicate or weaken them. A source place moved by the left operand is already Dead when the right operand is evaluated. A right operand cannot read, copy, or move such a place unless an independent represented operation has legally restored its value beforehand.

Because `dst` was wholly vacant at admission, the result write has an empty destruction domain and performs no replacement destruction. The write has the same stored-value-lifetime and storage-instance consequences as successful non-replacing `Init` and `IntegerAdd`: it begins the destination stored-value lifetime without changing the destination storage extent or storage-instance identity.

After both operand values have been produced, the subtraction/result-write portion is finite, deterministic, non-faulting, and non-diverging under the represented Core semantics. The operation introduces no new borrow interval, reference value, pointer provenance, cleanup category, storage identity, assignment-mutability rule, interior-mutability rule, fault reason, control-flow edge, layout/ABI promise, numeric-contract fact, runtime flag, or backend-visible semantic fact.

The operation consumes only the plain fixed-width integer-subtract value relation from `numerics/integers.md`. It does not represent checked, saturating, or explicitly wrapping subtraction, another arithmetic or comparison operation, a conversion, a source numeric-contract selection, a constant-folding requirement, or a generic arithmetic instruction family.

## Plain fixed-width integer multiplication result initialization

The represented Core proving relation contains one distinct plain fixed-width integer-multiply operation, normatively written here as:

```text
IntegerMul {
    dst: Place,
    left: Operand,
    right: Operand,
}
```

This semantic spelling identifies the represented Core operation. It does not require one particular implementation enum, generic arithmetic opcode, instruction encoding, backend opcode, or physical machine instruction.

Let `D` be the exact Core type identity of `dst`. A valid `IntegerMul` requires all of the following static type facts before execution:

- `D` denotes a scalar type whose scalar kind is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`;
- every place-derived left or right operand produces a value whose selected Core type identity is exactly `D`;
- a constant operand is admitted only when its semantic value matches `D` under the existing type-table value-matching relation; and
- no equality of scalar kind alone makes two distinct Core type identities interchangeable for an operand.

An `AddressOf` operand therefore cannot satisfy this operation's fixed-width integer operand requirement. Any applicable `Move`, `Copy`, `RawMove`, or other represented operand form retains its own existing access, liveness, copyability, provenance, and ownership-transfer requirements while also producing the exact required type `D`.

`IntegerMul` reuses the non-replacing destination lifecycle of `Init`, `IntegerAdd`, and `IntegerSub` rather than defining another storage model. Before either operand is evaluated:

1. resolve `dst` as direct storage under the existing place/type rules;
2. require the exclusive direct-storage authority that `Init`, `IntegerAdd`, and `IntegerSub` require under `borrowing.md`; and
3. require `dst` to be wholly vacant.

These destination preconditions are fixed at operation admission before left-operand evaluation. Operand evaluation cannot make an initially Live or insufficiently authorized destination retrospectively admissible to that same operation. Ordinary assignment mutability is not required, exactly as for the existing non-replacing initialization/arithmetic destinations.

After destination admission, execution is exactly:

1. evaluate `left` completely under its existing `Operand` semantics with required Core type identity `D`;
2. preserve all state consequences of that operand evaluation;
3. evaluate `right` completely under its existing `Operand` semantics with required Core type identity `D` in the resulting state;
4. preserve all state consequences of that operand evaluation;
5. consume the two produced semantic fixed-width integer operand values to compute the exact plain multiplication result owned by `numerics/integers.md` for the scalar kind of `D`;
6. write exactly that one result into `dst`; and
7. mark the written scalar destination Live, beginning a new stored-value lifetime there.

The left operand is therefore evaluated before the right operand. `Move`, `Copy`, raw-pointer operand access, and all other operand-local effects remain exactly those of the existing operand relation; `IntegerMul` does not duplicate or weaken them. A source place moved by the left operand is already Dead when the right operand is evaluated. A right operand cannot read, copy, or move such a place unless an independent represented operation has legally restored its value beforehand.

Because `dst` was wholly vacant at admission, the result write has an empty destruction domain and performs no replacement destruction. The write has the same stored-value-lifetime and storage-instance consequences as successful `Init`, `IntegerAdd`, and `IntegerSub`: it begins the destination stored-value lifetime without changing the destination storage extent or storage-instance identity.

After both operand values have been produced, the multiplication/result-write portion is finite, deterministic, non-faulting, and non-diverging under the represented Core semantics. The operation introduces no new borrow interval, reference value, pointer provenance, cleanup category, storage identity, assignment-mutability rule, interior-mutability rule, fault reason, control-flow edge, layout/ABI promise, numeric-contract fact, runtime flag, or backend-visible semantic fact.

The operation consumes only the plain fixed-width integer-multiply value relation from `numerics/integers.md`. It does not represent checked, saturating, or explicitly wrapping multiplication, another arithmetic or comparison operation, a conversion, a source numeric-contract selection, a constant-folding requirement, or a generic arithmetic instruction family.

## Plain fixed-width integer exclusive-or result initialization

The represented Core proving relation contains one distinct plain fixed-width integer-exclusive-or operation, normatively written here as:

```text
IntegerXor {
    dst: Place,
    left: Operand,
    right: Operand,
}
```

This semantic spelling identifies the represented Core operation. It does not require one particular implementation enum, generic arithmetic or bitwise opcode, instruction encoding, backend opcode, or physical machine instruction.

Let `D` be the exact Core type identity of `dst`. A valid `IntegerXor` requires all of the following static type facts before execution:

- `D` denotes a scalar type whose scalar kind is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`;
- every place-derived left or right operand produces a value whose selected Core type identity is exactly `D`;
- a constant operand is admitted only when its semantic value matches `D` under the existing type-table value-matching relation; and
- no equality of scalar kind alone makes two distinct Core type identities interchangeable for an operand.

An `AddressOf` operand therefore cannot satisfy this operation's fixed-width integer operand requirement. Any applicable `Move`, `Copy`, `RawMove`, or other represented operand form retains its own existing access, liveness, copyability, provenance, and ownership-transfer requirements while also producing the exact required type `D`.

`IntegerXor` reuses the non-replacing destination lifecycle of `Init`, `IntegerAdd`, `IntegerSub`, and `IntegerMul` rather than defining another storage model. Before either operand is evaluated:

1. resolve `dst` as direct storage under the existing place/type rules;
2. require the exclusive direct-storage authority that the existing non-replacing initialization/integer-result operations require under `borrowing.md`; and
3. require `dst` to be wholly vacant.

These destination preconditions are fixed at operation admission before left-operand evaluation. Operand evaluation cannot make an initially Live or insufficiently authorized destination retrospectively admissible to that same operation. Ordinary assignment mutability is not required, exactly as for the existing non-replacing initialization/integer-result destinations.

After destination admission, execution is exactly:

1. evaluate `left` completely under its existing `Operand` semantics with required Core type identity `D`;
2. preserve all state consequences of that operand evaluation;
3. evaluate `right` completely under its existing `Operand` semantics with required Core type identity `D` in the resulting state;
4. preserve all state consequences of that operand evaluation;
5. consume the two produced semantic fixed-width integer operand values to compute the exact plain exclusive-or result owned by `numerics/integers.md` for the scalar kind of `D`;
6. write exactly that one result into `dst`; and
7. mark the written scalar destination Live, beginning a new stored-value lifetime there.

The left operand is therefore evaluated before the right operand. `Move`, `Copy`, raw-pointer operand access, and all other operand-local effects remain exactly those of the existing operand relation; `IntegerXor` does not duplicate or weaken them. A source place moved by the left operand is already Dead when the right operand is evaluated. A right operand cannot read, copy, or move such a place unless an independent represented operation has legally restored its value beforehand.

Because `dst` was wholly vacant at admission, the result write has an empty destruction domain and performs no replacement destruction. The write has the same stored-value-lifetime and storage-instance consequences as successful non-replacing `Init`, `IntegerAdd`, `IntegerSub`, and `IntegerMul`: it begins the destination stored-value lifetime without changing the destination storage extent or storage-instance identity.

After both operand values have been produced, the exclusive-or/result-write portion is finite, deterministic, non-faulting, and non-diverging under the represented Core semantics. The operation introduces no new borrow interval, reference value, pointer provenance, cleanup category, storage identity, assignment-mutability rule, interior-mutability rule, fault reason, control-flow edge, layout/ABI promise, numeric-contract fact, runtime flag, or backend-visible semantic fact.

The operation consumes only the plain fixed-width integer-exclusive-or value relation from `numerics/integers.md`. It does not represent binary AND or OR, complement, shift, comparison, conversion, explicit overflow mode, source numeric-contract selection, constant-folding requirement, or a generic arithmetic/bitwise instruction family.

## Plain fixed-width integer bitwise-OR result initialization

The represented Core proving relation contains one distinct plain fixed-width integer-bitwise-OR operation, normatively written here as:

```text
IntegerOr {
    dst: Place,
    left: Operand,
    right: Operand,
}
```

This semantic spelling identifies the represented Core operation. It does not require one particular implementation enum, generic arithmetic or bitwise opcode, instruction encoding, backend opcode, or physical machine instruction.

Let `D` be the exact Core type identity of `dst`. A valid `IntegerOr` requires all of the following static type facts before execution:

- `D` denotes a scalar type whose scalar kind is exactly one of `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`;
- every place-derived left or right operand produces a value whose selected Core type identity is exactly `D`;
- a constant operand is admitted only when its semantic value matches `D` under the existing type-table value-matching relation; and
- no equality of scalar kind alone makes two distinct Core type identities interchangeable for an operand.

An `AddressOf` operand therefore cannot satisfy this operation's fixed-width integer operand requirement. Any applicable `Move`, `Copy`, `RawMove`, or other represented operand form retains its own existing access, liveness, copyability, provenance, and ownership-transfer requirements while also producing the exact required type `D`.

`IntegerOr` reuses the non-replacing destination lifecycle of `Init` and the existing represented integer-result operations rather than defining another storage model. Before either operand is evaluated:

1. resolve `dst` as direct storage under the existing place/type rules;
2. require the exclusive direct-storage authority that the existing non-replacing initialization/integer-result operations require under `borrowing.md`; and
3. require `dst` to be wholly vacant.

These destination preconditions are fixed at operation admission before left-operand evaluation. Operand evaluation cannot make an initially Live or insufficiently authorized destination retrospectively admissible to that same operation. Ordinary assignment mutability is not required, exactly as for the existing non-replacing initialization/integer-result destinations.

After destination admission, execution is exactly:

1. evaluate `left` completely under its existing `Operand` semantics with required Core type identity `D`;
2. preserve all state consequences of that operand evaluation;
3. evaluate `right` completely under its existing `Operand` semantics with required Core type identity `D` in the resulting state;
4. preserve all state consequences of that operand evaluation;
5. consume the two produced semantic fixed-width integer operand values to compute the exact plain bitwise-OR result owned by `numerics/integers.md` for the scalar kind of `D`;
6. write exactly that one result into `dst`; and
7. mark the written scalar destination Live, beginning a new stored-value lifetime there.

The left operand is therefore evaluated before the right operand. `Move`, `Copy`, raw-pointer operand access, and all other operand-local effects remain exactly those of the existing operand relation; `IntegerOr` does not duplicate or weaken them. A source place moved by the left operand is already Dead when the right operand is evaluated. A right operand cannot read, copy, or move such a place unless an independent represented operation has legally restored its value beforehand.

Because `dst` was wholly vacant at admission, the result write has an empty destruction domain and performs no replacement destruction. The write has the same stored-value-lifetime and storage-instance consequences as successful non-replacing `Init` and the existing represented integer-result operations: it begins the destination stored-value lifetime without changing the destination storage extent or storage-instance identity.

After both operand values have been produced, the bitwise-OR/result-write portion is finite, deterministic, non-faulting, and non-diverging under the represented Core semantics. The operation introduces no new borrow interval, reference value, pointer provenance, cleanup category, storage identity, assignment-mutability rule, interior-mutability rule, fault reason, control-flow edge, layout/ABI promise, numeric-contract fact, runtime flag, or backend-visible semantic fact.

The operation consumes only the plain fixed-width integer bitwise-OR value relation from `numerics/integers.md`. It does not represent binary AND, an XOR/complement/arithmetic rewrite, shift, comparison, conversion, explicit overflow mode, source numeric-contract selection, constant-folding requirement, or a generic arithmetic/bitwise instruction family.

## Standard binary floating addition result initialization

The represented Core proving relation contains one binary floating-add operation, normatively written here as:

```text
FloatAdd {
    dst: Place,
    left: Operand,
    right: Operand,
}
```

This semantic spelling identifies the represented Core operation. It does not require one particular implementation enum, generic numeric opcode, instruction encoding, backend opcode, or physical machine instruction. This revision attaches no explicit numeric-contract field or ambient numeric mode to the operation.

Let `D` be the exact Core type identity of `dst`. A valid `FloatAdd` requires all of the following static type facts before execution:

- `D` denotes a scalar type whose scalar kind is exactly one of `F16`, `F32`, or `F64`;
- every place-derived left or right operand produces a value whose selected Core type identity is exactly `D`;
- a constant operand is admitted only when its semantic value matches `D` under the existing type-table value-matching relation; and
- no equality of scalar kind alone makes two distinct Core type identities interchangeable for an operand.

An `AddressOf` operand therefore cannot satisfy this operation's binary-floating operand requirement. Any applicable `Move`, `Copy`, `RawMove`, or other represented operand form retains its own existing access, liveness, copyability, provenance, and ownership-transfer requirements while also producing the exact required type `D`. This operation introduces no cross-format conversion, promotion, widening, narrowing, coercion, defaulting, overload relation, generic numeric type, or operand-derived result-type inference.

`FloatAdd` is governed by the numeric-contract authority in [Core floating-point semantics](numerics/floating-point.md). This operation relation makes no explicit semantic contract selection, so the accepted floating owner establishes `standard` for each represented `FloatAdd` by its existing default rule. `reproducible` and `fast` remain accepted floating contracts but this Core operation has no represented non-default selection path in this revision. A later semantic consumer that explicitly selects another contract must establish the minimum required retention/refinement relation at that time rather than treating a backend mode or pre-created hidden state as authority.

`FloatAdd` reuses the non-replacing destination lifecycle of `Init` and the represented integer-result operations rather than defining another storage model. Before either operand is evaluated:

1. resolve `dst` as direct storage under the existing place/type rules;
2. require the exclusive direct-storage authority that the existing non-replacing initialization/result operations require under `borrowing.md`; and
3. require `dst` to be wholly vacant.

These destination preconditions are fixed at operation admission before left-operand evaluation. Operand evaluation cannot make an initially Live or insufficiently authorized destination retrospectively admissible to that same operation. Ordinary assignment mutability is not required, exactly as for the existing non-replacing initialization/result destinations.

After destination admission, execution is exactly:

1. evaluate `left` completely under its existing `Operand` semantics with required Core type identity `D`;
2. preserve all state consequences of that operand evaluation;
3. evaluate `right` completely under its existing `Operand` semantics with required Core type identity `D` in the resulting state;
4. preserve all state consequences of that operand evaluation;
5. consume the two produced semantic binary-floating operand values and select exactly one result permitted by the accepted `standard` same-format floating-addition relation in `numerics/floating-point.md` for the scalar kind of `D`;
6. write that one semantic result into `dst`; and
7. mark the written scalar destination Live, beginning a new stored-value lifetime there.

The left operand is therefore evaluated before the right operand. `Move`, `Copy`, raw-pointer operand access, and all other operand-local effects remain exactly those of the existing operand relation; `FloatAdd` does not duplicate or weaken them. A place moved by the left operand is already Dead when the right operand is evaluated. A right operand cannot read, copy, or move such a place unless an independent represented operation has legally restored its value beforehand.

The floating owner remains the sole numerical authority for finite normal/subnormal results, signed-zero selection, binary rounding, finite/infinity boundaries, signed infinities, and NaN-class outcomes. When that owner requires only that a result belong to `D`'s NaN value class, `FloatAdd` may store any semantic NaN member permitted by that relation. This storage relation does not select a canonical member, payload, sign, quiet/signaling identity, physical encoding, equality identity, or another member-sensitive property.

A semantic NaN value produced at runtime is an ordinary storable value of `D`. Its existence does not create a NaN form in the current constant-value representation and does not make `Operand::Constant` capable of fabricating a NaN. A reference or verification implementation may represent the accepted class-only result with a class-level observation when that abstraction preserves every represented semantic operation; such an assurance carrier is not a semantic NaN member identity.

Because `dst` was wholly vacant at admission, the result write has an empty destruction domain and performs no replacement destruction. The write has the same stored-value-lifetime and storage-instance consequences as successful non-replacing `Init` and the existing represented integer-result operations: it begins the destination stored-value lifetime without changing the destination storage extent or storage-instance identity.

After both operand values have been produced, the floating-addition/result-write step is finite, non-faulting, and non-diverging under the represented Core semantics. NaN, signed infinity, signed zero, subnormal, and normal outcomes selected by the floating owner are ordinary numerical results rather than a Core `Fault`, undefined behavior, panic, exception, or alternate control-flow outcome. The operation introduces no new borrow interval, reference value, pointer provenance, cleanup category, storage identity, assignment-mutability rule, interior-mutability rule, fault reason, control-flow edge, floating exception/status state, layout/ABI promise, runtime numeric mode, or backend-visible semantic fact.

The operation consumes only the already accepted `standard` same-format floating-addition result relation and existing default numeric-contract authority from `numerics/floating-point.md`. It does not represent floating subtraction, multiplication, division, negation, comparison, conversion, source numeric-contract selection, a NaN literal, a generic floating operation family, constant folding, or a physical floating representation.

## Read

`Read(src)` requires `src` to be fully initialized.

`Read` does not transfer ownership, change initialization state, end any stored-value lifetime, or change storage-instance identity.

Reading a partially initialized or Dead place is invalid in safe Core.

## Move

`Move(src)` requires `src` to be fully initialized.

It produces the complete value previously stored at `src` and changes every leaf in `src` from Live to Dead.

Each affected source stored-value lifetime ends at the move. Move does not destroy the transferred value.

A later read, copy, or second move of that place is invalid unless the affected storage is legally reinitialized.

Moving a sub-place affects only that sub-place. Disjoint initialized sibling places remain Live and their stored-value lifetimes continue.

The semantic value produced by the move may subsequently be written into another place; such a write begins stored-value lifetimes at the destination rather than extending the ended source storage lifetimes.

Move does not end the source storage extent or change its storage-instance identity.

Interior mutability does not make `Move` a shared-authority operation. The borrowing rules continue to require exclusive alias authority for ownership transfer.

## Copy

`Copy(src)` requires that `src` is fully initialized and its type is copyable.

It produces an equal owned value while leaving `src` Live. The source stored-value lifetimes therefore continue unchanged.

When the produced copy is written into destination storage, that write begins distinct stored-value lifetimes at the destination leaves.

For the structural types defined by this revision, an aggregate is copyable exactly when all of its fields are copyable. Raw-pointer leaf types are copyable; their pointer-specific target/provenance preservation is owned by [Core pointers and provenance](pointers.md).

The general language mechanism that determines copyability is not defined by this revision.

## Ordinary assignment

`Assign(dst, value)` requires the local containing `dst` to be mutable for ordinary assignment.

It also requires the exclusive alias authority specified by [Core borrowing](borrowing.md). Interior-mutability markers do not weaken either ordinary-assignment requirement.

Unlike `Init`, `Assign` may target storage containing Live leaves and therefore may perform replacement. Its `dst` may be wholly Never-initialized, partially initialized, fully Live, or contain Dead subobjects.

Assignment evaluates conceptually as:

1. evaluate the source operand completely;
2. determine the destruction domain of `dst` from the resulting storage state;
3. destroy exactly that domain in its defined order, ending those old stored-value lifetimes;
4. write the new value into `dst`;
5. mark all written leaves Live, beginning new stored-value lifetimes there.

The source-first rule is semantically significant. If source evaluation moves from storage related to `dst`, those moved leaves are already Dead when the destination destruction domain is determined and therefore MUST NOT be destroyed as part of replacement.

Assignment may therefore perform a mutable first write, replace a Live value, replace partial storage, or reinitialize storage after move or destruction.

Never-initialized and Dead subobjects have nothing to destroy before the write.

The source value MUST structurally match the type of `dst`.

Assignment changes stored-value lifetimes but does not by itself end the destination storage extent or change its storage-instance identity.

## Interior assignment

The proving MIR has a distinct interior-replacement operation:

```text
InteriorAssign { dst: PlaceAccess, src: Operand }
```

`InteriorAssign` is legal only when the resolved concrete destination place lies within an interior-mutable region.

It does **not** require the containing local to be mutable for ordinary assignment. Instead, its alias requirement is independently defined by [Core borrowing](borrowing.md): shared alias authority is sufficient at an interior-mutable target.

`InteriorAssign` uses exactly the same replacement lifecycle and source-first ordering as ordinary `Assign`:

1. authorize and resolve the destination access under the borrowing rules and require the resulting place to lie within an interior-mutable region;
2. evaluate the source operand completely;
3. determine the destruction domain of `dst` from the resulting storage state;
4. destroy exactly that domain in its defined order;
5. write the new value into `dst`;
6. mark all written leaves Live, beginning new stored-value lifetimes there.

Like ordinary assignment, interior assignment is path-state tolerant. The destination may be Never-initialized, partially initialized, Live, or contain Dead subobjects. Only then-Live contents belong to the replacement destruction domain.

Interior assignment does not grant any other operation a weaker access requirement:

- `Move` still requires exclusive alias authority;
- `Drop` still requires exclusive alias authority;
- exclusive reborrow still requires exclusive parent authority;
- ordinary `Assign` still requires both exclusive alias authority and mutable-local permission.

An exclusive loan may perform `InteriorAssign` only because exclusive authority includes shared authority; the interior-mutability marker remains independently required.

A shared loan may remain active across a legal interior replacement. The loan governs access to a structural storage region, while the replacement ends old stored-value lifetimes and begins new stored-value lifetimes within that continuing storage extent and storage instance. Borrowing owns the detailed access and delegation rules.

This revision does not define a `RefCell`-style runtime borrow guard, synchronization, atomics, or a source-level interior-mutability API.

## Destruction

Destruction consumes only currently Live stored values.

Destroying a scalar Live place ends its stored-value lifetime and changes the leaf to Dead. Never-initialized and Dead storage has nothing to destroy during automatic cleanup.

Destroying an aggregate destroys exactly its destruction domain. The recursive definition of that domain gives reverse declaration order for struct fields while skipping leaves that are not Live.

`Drop(place)` requires a non-empty destruction domain. It destroys exactly that domain once. Destroyed leaves become Dead; Never-initialized leaves remain Never-initialized.

A moved or already-destroyed subobject MUST NOT be destroyed a second time.

Destruction does not by itself end the containing storage extent or change its storage-instance identity.

Interior mutability does not weaken the exclusive alias authority required by explicit `Drop`.

The current revision has no custom destructor body. A later custom-destructor specification may refine actions that occur during destruction, but it must preserve the selected destruction domain and ordering unless the canonical owner of those rules explicitly changes them.

## Function termination cleanup

On both defined `Return` and defined `Fault`, function locals are cleaned in reverse local declaration order.

When a local is reached for cleanup, its then-current destruction domain is computed and destroyed. Partial initialization is therefore respected and Never-initialized, Dead, moved, or already-destroyed leaves are skipped.

A local's storage extent and storage-instance identity continue through its cleanup and end after that cleanup completes. Structural sub-place storage ends with the containing local storage instance.

Defined `Fault` uses the same stored-value lifetime and destruction-domain rules as defined `Return`. `Fault` is a defined terminal state, not undefined behavior.

When an applicable non-Core semantic contract defines termination of a represented Core function execution by a distinct cancellation outcome, that termination MUST use the same reverse-local cleanup order, then-current destruction-domain rules, and storage-extent ending rule above. This paragraph defines only the Core storage consequence once cancellation termination has already been selected by another canonical owner. It does not define cancellation request or observation, propagation, catch or unwind policy, custom destructor bodies, or source cancellation syntax, and it does not reclassify cancellation as a Core `Fault`.

When [Core control flow](control-flow.md) selects a cyclic execution that diverges, no termination cleanup occurs merely because execution has run for a long time; there is no implicit step budget that ends storage extents.

## Determinism

For a fixed validated Core program using only the semantics defined here, operation admission, operand sequencing, dynamic local storage-instance creation, stored-value lifetime transitions, interior-mutability capability, non-replacing result-write occurrence, destruction domains, and destruction order are deterministic.

Represented integer-operation result values are determined by their separately owned integer relations. A represented `FloatAdd` result is determined by the separately owned standard floating-addition relation, including that relation's explicit permitted variation when a NaN-class outcome allows any NaN member of the result type. Such permitted NaN-member variation is numerical semantic latitude owned by `numerics/floating-point.md`; it is not hidden storage or loan state and does not make host NaN propagation, backend behavior, physical encoding, scheduling, or container iteration order an additional semantic input.

The actual verification token chosen to represent a storage-instance identity is not program-observable. Semantics depend on instance distinction and stability, not on a particular integer assignment.

The interior-mutability marker is static semantic type metadata. `InteriorAssign` introduces no hidden runtime borrow state and no new path-state component beyond the storage transitions it already performs. `IntegerAdd`, `IntegerSub`, `IntegerMul`, `IntegerXor`, `IntegerOr`, and `FloatAdd` likewise introduce no hidden storage-state component beyond their operand consequences and one non-replacing result initialization.

The semantics defined here do not depend on physical addresses, host arithmetic/bitwise/floating behavior, destruction behavior, container iteration order, physical scheduling, or backend behavior.

## Separate semantic owners

This document does not define heap or raw allocation, deallocation, borrowing duration or loan delegation, first-class references, raw-pointer dereference/access, numeric pointer addresses, pointer arithmetic, numeric operation value relations beyond consuming the separately owned integer-add, integer-subtract, integer-multiply, integer-exclusive-or, integer-bitwise-OR, and standard floating-addition relations, pinning, atomics or concurrency, custom destructor bodies, panic catching, cancellation request or observation, cancellation propagation, asynchronous preemption beyond the cleanup consequence above, ABI/layout guarantees, or source grammar.

Raw-pointer type/value formation and provenance derived from the storage-instance identity defined here are owned by [Core pointers and provenance](pointers.md). That pointer specification does not change the storage extent or stored-value lifetime rules in this document. Fixed-width integer numerical value relations are owned by [Core integer semantics](numerics/integers.md); standard binary floating-addition numerical results and numeric-contract defaulting are owned by [Core floating-point semantics](numerics/floating-point.md). This document owns only the represented operations' operand/storage/lifetime consequences.

This revision defines only proving-kernel interior-mutability capability and replacement semantics; it does not define source spelling, library abstractions, dynamic borrow guards, synchronization mechanisms, or which future public types expose that capability.

Where this revision defines storage or lifetime facts that later borrowing, pointer access, validity, control-flow, or concurrency concerns may depend on, their canonical owners govern the additional policy. In particular, a shared loan remaining active across an interior replacement implies stable structural storage identity for that continuing extent, but does not imply physical address stability, legal raw-pointer dereference, data-race freedom, or a first-class reference guarantee.
