# Source Type Foundation

Status: **provisional normative; incomplete**

This document owns the represented intrinsic scalar source type identities, represented source type equality, nominal record declaration/type identity, record field structure, direct record-containment rule, and represented owned-value duplicability classification.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module/binding/lookup relations from [Source names and modules](names-modules.md), numeric semantics from [Core integer semantics](../core/numerics/integers.md) and [Core floating-point semantics](../core/numerics/floating-point.md), and applicable structural value/storage behavior from [Core value and storage semantics](../core/value-storage.md). It does not redefine those owners.

The represented concrete intrinsic type and record-definition spellings are owned by [Source concrete syntax](concrete-syntax.md). Structural source paths, structural ownership state, availability, and remaining frontiers are owned by [Source structural ownership](structural-ownership.md). Function-local binding identity, mutability, lifecycle, lookup, ordinary whole-binding owned use, and assignment are owned by [Source function-local bindings](local-bindings.md). [Source literal semantics](literals.md) consumes the scalar identities/value domains defined here. [Source field-value access](field-access.md) consumes nominal record/field identity, field source types, source type equality, and owned-value duplicability. [Source patterns](patterns.md) consumes nominal record/field identity, exact source type equality, structural field order, and duplicability for recursive exhaustive record-pattern validation and binding-leaf production. This document does not define literal materialization, structural ownership state, field-access execution/accessibility, pattern lookup/ownership, conversions, general member lookup, or an implementation representation.

## Intrinsic scalar source types

The following labels identify intrinsic source type identities. [Source concrete syntax](concrete-syntax.md) uses these exact labels as the represented intrinsic type spellings; semantic type identities and value domains remain owned here.

- `Bool`;
- signed fixed-width integer types `I8`, `I16`, `I32`, and `I64`;
- unsigned fixed-width integer types `U8`, `U16`, `U32`, and `U64`;
- binary floating types `F16`, `F32`, and `F64`.

These intrinsic scalar identities are language-level source types. Their existence/value domains MUST NOT vary with physical target, backend, optimization level, host ABI, or direct hardware support.

`Bool` has exactly two semantic values: true and false. The represented source literal producer for those values is owned by `literals.md`; this section does not define concrete spelling, representation, layout, integer conversion, ordering, or bit pattern.

Each represented integer type consumes the fixed-width integer semantics owned by Core:

- `I8`, `I16`, `I32`, `I64` are signed widths 8, 16, 32, and 64;
- `U8`, `U16`, `U32`, `U64` are unsigned widths 8, 16, 32, and 64.

The corresponding mathematical value domains and applicable overflow-operation contracts are those defined by the Core integer owner. Source type identity does not imply two's-complement storage, byte order, alignment, ABI representation, or another physical encoding.

Each represented floating source type consumes Core binary floating semantics with these semantic format parameters:

- `F16`: `p = 11`, `emin = -14`, `emax = 15`;
- `F32`: `p = 24`, `emin = -126`, `emax = 127`;
- `F64`: `p = 53`, `emin = -1022`, `emax = 1023`.

These establish the applicable semantic binary floating value format, including the special-value domain governed by the Core floating owner. They do not prescribe physical IEEE storage bits or ABI layout.

A floating source type identity does not itself select `standard`, `reproducible`, or `fast`. The existing numeric-contract authority, defaulting, refinement, and operation-specific rules remain controlling.

A realization lacking direct native support for one represented intrinsic type MUST preserve every applicable accepted semantic contract through an otherwise legal realization, including emulation or applicable environment admission/rejection where required. Lack of hardware support is not permission to change the source type's value domain or make source validity target-defined.

## Deliberately absent intrinsic types

This revision does not define intrinsic source type identities for:

- target-sized pointer/address-width integers;
- 128-bit or arbitrary-bit-width integers;
- extended, 128-bit, bfloat, decimal, or other floating formats;
- complex numbers, vectors, or matrices;
- character, string, or byte-sequence types;
- raw pointers, safe references, slices, arrays, tuples, enums, unions, function types, or other composite/indirection forms.

Their absence from this source foundation does not narrow parameterized semantic relations already defined by applicable non-source owners. A later source revision may add a type identity when an accepted consumer requires it.

## Represented source type equality

For the represented source type set:

- two intrinsic scalar source types are equal exactly when they are the same intrinsic identity;
- two nominal record source types are equal exactly when they originate from the same record declaration identity;
- an intrinsic scalar source type and a nominal record source type are never equal.

Two distinct record declarations do not define the same source type merely because their fields have equal keys/types/order.

This relation does not define subtyping, coercion, conversion, layout compatibility, ABI compatibility, trait conformance, or representation equivalence.

Recursive record patterns consume this relation directly:

- a direct-root binding or producer result must have exactly the nominal record type selected by the top pattern head; and
- every nested record-pattern head must have exactly the nominal record type of the field in which that nested pattern occurs.

Structural similarity is never pattern compatibility.

## Nominal record type declarations

A **record type declaration** is a module-level source declaration that introduces exactly one module binding under `names-modules.md`.

That binding denotes one nominal record source type. Type identity is the identity of the record declaration/binding itself. Distinct record declarations therefore denote distinct source types.

The binding's module-private/exported accessibility is determined only by `names-modules.md`.

The represented `record` form in `concrete-syntax.md` establishes one such declaration and maps its concrete field sequence to the structure below. Absence/presence of concrete `export` establishes module-private/exported accessibility respectively. Other future declaration forms may establish the same semantic declaration category only through their accepted mapping.

A record declaration contains one finite ordered field sequence, which MAY be empty.

Each field has exactly:

- one lexical identifier key governed by `lexical.md`; and
- one represented source value type.

Field lexical keys MUST be unique within one record declaration.

Field identity is scoped by the containing record type. Fields with the same lexical key in distinct record types are distinct fields.

For this revision, a record field type is either:

- one intrinsic scalar source type defined here; or
- one nominal record source type whose record binding is legally resolvable for the declaring source unit under same-module or qualified cross-module lookup from `names-modules.md`.

The ordered field sequence is semantic structural order for the source record value shape. It MAY be consumed where another accepted owner needs structural order. It does not define physical field order, byte offsets, padding, alignment, ABI layout, stable representation, or address arithmetic.

A value of a represented record type contains exactly one field value of the declared type for every field in the declaration. This source value shape creates no physical layout contract.

`structural-ownership.md` consumes record field identity/type/order to define structurally valid source paths and recursive remaining-ownership frontiers. `local-bindings.md` instantiates that relation for bindings. `field-access.md` consumes field lexical key, containing nominal record identity, declared type, and duplicability to select/produce a field value. `patterns.md` consumes nominal field identities/types to validate recursive exhaustive coverage and retain binding-leaf structural paths. Pattern presentation order never changes this declaration's structural field order.

## Direct record containment

For represented record types, define a **direct-containment edge** `A -> B` exactly when record type `A` has a field whose source type is record type `B`.

The finite graph consisting of represented record types and all such edges MUST be acyclic.

This requirement applies because every represented field type is scalar or direct structural record containment; no accepted source pointer/reference/other indirection type exists in this source type set.

The rule does not prohibit a later recursive nominal type when every cycle passes through an accepted indirection type whose canonical semantics establish indirection rather than structural containment. That later owner must define the applicable well-formedness relation explicitly.

## Owned-value duplicability

Every represented source value type has one source-semantic **owned-value duplicability** classification: **duplicable** or **non-duplicable**.

Duplicability means only that another accepted source operation may, when its own semantics explicitly use this capability, produce another owned value preserving the source semantic value without consuming the source value.

Duplicability does not define or require a source equality or comparison relation. It does not mean bitwise copying and does not imply shared storage identity, shared stored-value lifetime, aliasing, physical representation equality, ABI passing, or a particular realization strategy.

The represented intrinsic scalar source types are duplicable: `Bool`, every represented signed/unsigned fixed-width integer type, and `F16`, `F32`, and `F64`.

For floating values, duplication preserves the semantic floating value under applicable floating contracts. It does not define floating comparison equality and adds no NaN representation, payload, sign, or canonicalization guarantees beyond existing authority.

Each nominal record declaration has one abstract source-semantic **duplicable selection**. A record may select duplicability only when every field source type is duplicable. A record that does not select duplicability is non-duplicable even if every field type is duplicable.

The concrete record form in `concrete-syntax.md` makes no positive duplicable selection, so a record introduced by that form is non-duplicable under the no-selection rule.

This revision defines no concrete positive duplicability-selection syntax or trait mechanism.

Distinct nominal record declarations make the selection independently. Equal field shape does not transfer the selection, and selection does not alter nominal identity or field structure.

Duplicating a value of a duplicable nominal record type produces another owned record value by preserving every field's semantic value through that field type's duplicability capability. The original record value is not consumed.

The nominal selection is a conservative source ownership-policy choice. Structural shape alone does not silently grant duplication. This revision does not claim represented records already model unique resources, capabilities, handles, or custom destruction.

**Non-duplicable** means only that this non-consuming owned duplication capability is unavailable. It does not prohibit ownership transfer/consumption or a future explicit clone, copy-construction, conversion, factory, deserialization, or other operation from independently producing another value.

Ordinary whole-binding use uses this capability through `local-bindings.md`. Binding-rooted field-value use uses it through `field-access.md` for the final selected field path. Recursive record patterns use it through `patterns.md` independently for every binding leaf: a duplicable leaf produces a non-consuming duplicate from its complete structural path, while a non-duplicable leaf transfers/consumes exactly that complete path. Structural path availability and resulting ancestor/disjoint consequences are owned by `structural-ownership.md`, not by the duplicability classification.

This section does not define other expression contexts, field assignment or partial reinitialization, parameter passing, result transfer, calls, pattern syntax, or any explicit cloning/copy-construction operation.

Duplicability is source semantics independent of any future `Copy`-like trait spelling. A later trait/generic mechanism may expose/derive/constrain this capability only if its canonical semantics preserve this classification; this revision introduces no trait membership.

No custom destructor semantics are defined. A later custom-destruction owner must explicitly define compatibility with duplicability and partial structural ownership rather than silently changing either property.

This capability reflects the conceptual distinction between ownership transfer and non-consuming duplication already present in Core semantics, but Core copyability representation is not source-language authority. This revision defines no direct source-to-MIR lowering rule.

## Literal and conversion boundary

[Source literal semantics](literals.md) owns represented boolean literal values and required-type materialization of signed decimal integer literals into the fixed-width domains defined here. This type foundation supplies value domains; it does not redefine literal spelling, contextual materialization, representability diagnostics, or owned-value production.

No source floating literal form/materialization relation is defined by this revision.

This document defines no abstract/unbounded integer/float literal type, default literal type, literal suffix semantics, or compile-time-only numeric type. Context-typed integer literal materialization adds no source type.

This document grants no implicit conversion/coercion/promotion/widening/narrowing/subtyping/numeric defaulting relation between represented source types. Literal materialization is not conversion because the literal datum has no prior concrete source type.

Those omissions do not prohibit a later accepted operation from defining an explicit or implicit conversion. They prevent the type foundation from creating conversion behavior before an operation owns it.

## Callable and declaration boundary

This document defines the represented nominal record-type declaration/binding only. Source function entities/callable signatures are owned by [Source callables](callables.md).

This document does not define constants, statics, variables, type aliases, opaque types, traits, or another module-level declaration category beyond records.

Those declarations require independently owned source semantics rather than inference from current proving MIR.

## Structural ownership, bindings, field access, and patterns

`structural-ownership.md` is the sole source owner for structural source paths, structural ownership state, path availability/consumption requirements, and recursive remaining frontiers. Those facts are not type properties.

`local-bindings.md` owns binding identity, assignment mutability, lexical lookup/scope, binding lifecycle around structural ownership, ordinary whole-binding duplicate-or-consume use, and assignment legality/reset.

`field-access.md` owns binding-rooted field selection, direct field accessibility, final-path availability requirement, and duplicate-or-consume field-value production.

`patterns.md` owns recursive exhaustive record-pattern selection, binding-leaf introduction/order, direct-root leaf production, and producer-transient leaf ownership/cleanup selection.

This type owner supplies nominal record/field identity, field types, source type equality, structural field order, and owned-value duplicability only.

The represented type identity, record shape, and duplicability classification do not by themselves determine:

- structural path availability or ownership state;
- field assignment/partial-field reinitialization;
- interior mutability;
- broader cross-module field visibility;
- pattern scope/shadowing/matching/control flow;
- method/associated-item/trait/extension/overload lookup;
- custom destruction/destructor bodies.

Proving-kernel copyability, path state, scalar liveness, or interior-mutability metadata is not source-language authority for those concerns.

## Further boundaries

The concrete intrinsic/record forms do not themselves define literal semantics, additional refutable/rest/shorthand/wildcard/literal/guard pattern categories, record construction, field-value access, closures/captures, generics, traits/coherence, const/static semantics, source `unsafe`, pointer/reference/lifetime syntax, ABI/layout/FFI/linkage, package/filesystem mapping, parser/lossless syntax/HIR, Core MIR lowering, or backend representation.

Represented boolean/integer literal semantics are owned by `literals.md`; structural ownership by `structural-ownership.md`; binding-rooted field-value access by `field-access.md`; and recursive exhaustive record destructuring by `patterns.md`.

Additional type/declaration spellings require an accepted concrete-syntax owner and must preserve the type identities/relations defined here.
