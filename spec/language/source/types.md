# Source Type Foundation

Status: **provisional normative; incomplete**

This document owns the represented intrinsic scalar source type identities, represented source type equality, nominal record declaration/type identity, record field structure, and direct record-containment rule.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module/binding/lookup relations from [Source names and modules](names-modules.md), numeric semantics from [Core integer semantics](../core/numerics/integers.md) and [Core floating-point semantics](../core/numerics/floating-point.md), and applicable structural value/storage behavior from [Core value and storage semantics](../core/value-storage.md). It does not redefine those owners.

This document does not define concrete source spellings, literal typing, conversions, callable signatures, local bindings, member lookup, or an implementation representation.

## Intrinsic scalar source types

The following labels are specification notation for intrinsic source type identities. They do not define source spellings, keywords, or ordinary module bindings.

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

## Literal and conversion boundary

This document defines no source literal form or literal type.

In particular, it does not define abstract or unbounded integer/float literal types, default literal types, suffixes, contextual literal typing, or compile-time-only numeric types.

This document also grants no implicit conversion, coercion, promotion, widening, narrowing, subtyping, or numeric defaulting relation between represented source types.

Those omissions do not prohibit a later accepted source operation from defining an explicit or implicit conversion. They prevent the bare type foundation from creating conversion behavior before an expression, literal, call, or other consumer owns it.

## Callable and declaration boundary

This revision introduces exactly one concrete module-binding entity category: a record-type binding.

It does not define function or callable declarations, parameter/result types, function values, calls, unit/no-result typing, constants, statics, variables, type aliases, opaque types, traits, or another module-level declaration category.

Those declarations require independently owned source semantics rather than being inferred from the current proving MIR.

## Copyability, mutability, and member boundary

The represented source type identity and record shape do not determine:

- source copyability or a `Copy`-like contract;
- move-only classification;
- assignment mutability;
- interior mutability;
- field accessibility or access syntax;
- method, member, associated-item, trait, extension, or overload lookup;
- custom destruction or destructor bodies.

Proving-kernel copyability or interior-mutability metadata is not source-language authority for those concerns.

## Further boundaries

This revision does not define concrete type/declaration syntax, keywords, punctuation, comments, literals, local bindings or scopes, patterns, closures/captures, generics, traits/coherence, const/static semantics, source `unsafe`, pointer/reference/lifetime syntax, ABI/layout/FFI/linkage, package/filesystem mapping, parser/lossless syntax/HIR, Core MIR lowering, or backend representation.
