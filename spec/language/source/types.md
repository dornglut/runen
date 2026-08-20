# Source Type Foundation

Status: **provisional normative; incomplete**

This document owns the represented intrinsic scalar source type identities, represented source type equality, nominal record declaration/type identity, record field structure, direct record-containment rule, and represented owned-value duplicability classification.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module/binding/lookup relations from [Source names and modules](names-modules.md), numeric semantics from [Core integer semantics](../core/numerics/integers.md) and [Core floating-point semantics](../core/numerics/floating-point.md), and applicable structural value/storage behavior from [Core value and storage semantics](../core/value-storage.md). It does not redefine those owners.

The represented concrete intrinsic type and record-definition spellings are owned by [Source concrete syntax](concrete-syntax.md). Function-local binding mutability, availability, and ordinary owned-value use are owned by [Source function-local bindings](local-bindings.md). This document does not define literal typing, conversions, member lookup, or an implementation representation.

## Intrinsic scalar source types

The following labels identify intrinsic source type identities. [Source concrete syntax](concrete-syntax.md) uses these exact labels as the concrete spellings for its represented intrinsic type forms; the semantic type identities and value domains remain owned here.

- `Bool`;
- signed fixed-width integer types `I8`, `I16`, `I32`, and `I64`;
- unsigned fixed-width integer types `U8`, `U16`, `U32`, and `U64`;
- binary floating types `F16`, `F32`, and `F64`.

These intrinsic scalar type identities are language-level source types. Their existence and semantic value domains MUST NOT vary with physical target, backend, optimization level, host ABI, or direct hardware support.

`Bool` has exactly two semantic values: true and false. This section does not define their source literal spellings, representation, layout, integer conversion, ordering, or bit pattern.

Each represented integer type consumes the fixed-width integer semantics owned by Core with the following width and signedness:

- `I8`, `I16`, `I32`, `I64` are signed types of width 8, 16, 32, and 64 respectively;
- `U8`, `U16`, `U32`, `U64` are unsigned types of width 8, 16, 32, and 64 respectively.

The corresponding mathematical value domains and applicable overflow-operation contracts are those defined by the Core integer owner. Source type identity does not imply two's-complement storage, byte order, alignment, ABI representation, or another physical encoding.

Each represented floating source type consumes the binary floating semantics owned by Core with the following semantic format parameters:

- `F16`: `p = 11`, `emin = -14`, `emax = 15`;
- `F32`: `p = 24`, `emin = -126`, `emax = 127`;
- `F64`: `p = 53`, `emin = -1022`, `emax = 1023`.

These parameters establish the applicable semantic binary floating value format, including the special-value domain governed by the Core floating owner. They do not prescribe physical IEEE storage bits or ABI layout.

A floating source type identity does not itself select `standard`, `reproducible`, or `fast`. The existing numeric-contract authority, defaulting, refinement, and operation-specific rules remain controlling.

A realization that lacks direct native support for one of these intrinsic types MUST preserve every applicable accepted semantic contract through an otherwise legal realization, including emulation or applicable environment admission/rejection where required. Lack of direct support is not permission to change the source type's value domain or make source-language validity target-defined.

## Deliberately absent intrinsic types

This revision does not define intrinsic source type identities for:

- target-sized pointer- or address-width integers;
- 128-bit or arbitrary-bit-width integers;
- extended, 128-bit, bfloat, decimal, or other floating formats;
- complex numbers, vectors, or matrices;
- character, string, or byte-sequence types;
- raw pointers, safe references, slices, arrays, tuples, enums, unions, function types, or other composite/indirection forms.

Their absence from this source foundation does not narrow the parameterized semantic relations already defined by their applicable non-source owners. A later source-language revision may add a source type identity when an accepted consumer requires it.

## Represented source type equality

For the represented source type set:

- two intrinsic scalar source types are the same type exactly when they are the same intrinsic type identity;
- two nominal record source types are the same type exactly when they originate from the same record declaration identity;
- an intrinsic scalar source type and a nominal record source type are never the same type.

Two distinct record declarations do not define the same source type merely because their fields have equal keys, equal field types, or equal structural order.

This type-equality relation does not define subtyping, coercion, conversion, layout compatibility, ABI compatibility, trait conformance, or representation equivalence.

## Nominal record type declarations

A **record type declaration** is a module-level source declaration that introduces one module binding under `names-modules.md`.

That binding denotes one nominal record source type. The source type identity is the identity of the record declaration/binding itself. Distinct record declarations therefore denote distinct source types.

The binding's module-private or exported accessibility is determined only by `names-modules.md`; this document does not redefine accessibility.

The represented `record` form in `concrete-syntax.md` establishes one such declaration with module-private accessibility and maps its concrete field sequence to the structure defined below. Other future declaration forms may establish the same semantic declaration category only through their accepted mapping.

A record declaration contains one finite ordered sequence of fields. The sequence MAY be empty.

Each field has exactly:

- one lexical identifier key governed by `lexical.md`; and
- one represented source value type.

Field lexical identifier keys MUST be unique within one record declaration.

Field identity is scoped by the containing record type. Fields with the same lexical identifier key in distinct record types are distinct fields.

For this revision, a record field type is either:

- one intrinsic scalar source type defined by this document; or
- one nominal record source type whose record binding is legally resolvable for the declaring source unit under the same-module or qualified cross-module relations in `names-modules.md`.

The ordered field sequence is semantic structural order for the source record value shape. It MAY be consumed where another accepted semantic owner requires structural field order. It does not define physical field order, byte offsets, padding, alignment, ABI layout, stable representation, or address arithmetic.

A value of a represented record source type contains exactly one field value of the declared field type for every field in the declaration. This source value shape does not create a separate physical layout contract.

## Direct record containment

For represented record types, define a **direct-containment edge** `A -> B` exactly when record type `A` has a field whose source type is record type `B`.

The finite directed graph consisting of represented record types and all such direct-containment edges MUST be acyclic.

This requirement applies because every represented field type in this revision is either scalar or direct structural record containment; no accepted source pointer, reference, or other indirection type exists in this source type set.

The rule does not prohibit a later recursive nominal type when every recursive cycle passes through an accepted source type whose canonical semantics establish indirection rather than direct structural containment. That later owner must define the applicable well-formedness relation explicitly.

## Owned-value duplicability

Every source value type represented by this revision has one source-semantic **owned-value duplicability** classification: **duplicable** or **non-duplicable**.

Duplicability means only that another accepted source operation may, when its own semantics explicitly use this capability, produce another owned value that preserves the source semantic value under the accepted value semantics of that type without consuming the source value.

Duplicability does not define or require a source equality or comparison relation. It does not mean bitwise copying and does not imply shared storage identity, shared stored-value lifetime, aliasing, physical representation equality, ABI passing, or a particular realization strategy.

The represented intrinsic scalar source types are duplicable: `Bool`, every represented signed and unsigned fixed-width integer type, and `F16`, `F32`, and `F64`.

For floating values, duplication preserves the source semantic floating value governed by the applicable floating contracts. It does not define floating comparison equality and does not create additional NaN representation, payload, sign, or canonicalization guarantees beyond existing authority.

Each nominal record declaration has one abstract source-semantic **duplicable selection**. A nominal record declaration may select duplicability only when every field source type is duplicable. A record declaration that does not select duplicability is non-duplicable even when every field source type is duplicable.

The concrete record form in `concrete-syntax.md` makes no positive duplicable selection. A nominal record declaration introduced by that form is therefore non-duplicable under the no-selection rule above.

This revision defines no concrete positive duplicability-selection syntax or trait mechanism.

Distinct nominal record declarations make the selection independently. Equal field keys, field types, or structural order do not transfer the selection between nominal types, and the selection does not alter nominal type identity or field structure.

Duplicating a value of a duplicable nominal record type produces another owned record value by preserving each field's source semantic value through that field type's accepted duplicability capability. The original record value is not consumed.

The nominal selection is a conservative source ownership-policy choice. Structural field shape alone does not silently grant non-consuming duplication to a nominal type. This revision does not claim that represented records already model unique resources, capabilities, handles, or custom destruction.

**Non-duplicable** means only that the non-consuming owned-value duplication capability defined here is unavailable. It does not prohibit a future explicit cloning, copy-construction, conversion, factory, deserialization, or other operation from producing another value under that operation's independently accepted semantics.

Ordinary whole-binding owned use of this capability, including when a local use duplicates or consumes its binding, is owned by `local-bindings.md`. This section does not define other expression contexts, partial field moves, member access, parameter passing, result transfer, calls, or any explicit cloning/copy-construction operation.

Duplicability is source semantics independent of any future `Copy`-like trait spelling. A later trait or generic mechanism may expose, derive, or constrain this capability only if its canonical semantics preserve the classification defined here; this revision introduces no trait membership.

No custom destructor semantics are defined. A later custom-destruction owner must explicitly define compatibility with duplicability rather than silently changing this property.

This capability consumes the conceptual distinction between consuming ownership transfer and non-consuming duplication already present in Core semantics, but the current proving-MIR copyability representation is not source-language authority. This revision defines no direct source-to-MIR lowering rule.

## Literal and conversion boundary

This document defines no source literal form or literal type.

In particular, it does not define abstract or unbounded integer/float literal types, default literal types, suffixes, contextual literal typing, or compile-time-only numeric types.

This document also grants no implicit conversion, coercion, promotion, widening, narrowing, subtyping, or numeric defaulting relation between represented source types.

Those omissions do not prohibit a later accepted source operation from defining an explicit or implicit conversion. They prevent the bare type foundation from creating conversion behavior before an expression, literal, call, or other consumer owns it.

## Callable and declaration boundary

This document defines the represented nominal record-type declaration and binding only. Represented source function entities and callable signatures are owned by [Source callables](callables.md); they are not redefined here.

This document does not define constants, statics, variables, type aliases, opaque types, traits, or another module-level declaration category beyond its record-type concern.

Those declarations require independently owned source semantics rather than being inferred from the current proving MIR.

## Local-binding, mutability, and member boundary

Function-local binding identity, assignment mutability, availability, lexical lookup, and ordinary whole-binding duplicate-or-consume behavior are owned by `local-bindings.md`; they are not source type properties.

The represented source type identity, record shape, and owned-value duplicability classification do not determine:

- interior mutability;
- partial field move/member availability;
- field accessibility or access syntax;
- method, member, associated-item, trait, extension, or overload lookup;
- custom destruction or destructor bodies.

Proving-kernel copyability or interior-mutability metadata is not source-language authority for those concerns.

## Further boundaries

The concrete intrinsic and record forms represented by `concrete-syntax.md` do not define literals, patterns, record construction, member access, closures/captures, generics, traits/coherence, const/static semantics, source `unsafe`, pointer/reference/lifetime syntax, ABI/layout/FFI/linkage, package/filesystem mapping, parser/lossless syntax/HIR, Core MIR lowering, or backend representation.

Additional type/declaration spellings require an accepted concrete-syntax owner and must preserve the type identities and relations defined here.