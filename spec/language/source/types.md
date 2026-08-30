# Source Type Foundation

Status: **provisional normative; incomplete**

This document owns the represented intrinsic scalar source type identities, represented source type equality, nominal record declaration/type identity, record field structure, direct record-containment rule, and represented owned-value duplicability classification. The first represented Shared-reference type constructor is owned canonically by [Source Shared references](references.md), and the first represented raw-pointer type constructor is owned canonically by [Source raw pointers and unsafe admission](raw-pointers-unsafe.md); this type foundation integrates those constructors into source type equality, duplicability, and contextual record-shape boundaries without redefining reference authority/lifetime or raw-pointer provenance/unsafe semantics.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module/binding/lookup relations from [Source names and modules](names-modules.md), numeric semantics from [Core integer semantics](../core/numerics/integers.md) and [Core floating-point semantics](../core/numerics/floating-point.md), applicable structural value/storage behavior from [Core value and storage semantics](../core/value-storage.md), first-slice Shared-reference type/referent admission from [Source Shared references](references.md), and first-slice raw-pointer type/pointee/contextual admission from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md). It does not redefine those owners.

The represented concrete intrinsic, Shared-reference, raw-pointer, and record-definition spellings are owned by [Source concrete syntax](concrete-syntax.md). Structural source paths, structural ownership state, availability, and remaining frontiers are owned by [Source structural ownership](structural-ownership.md). Function-local binding identity, mutability, lifecycle, lookup, ordinary whole-binding owned use, assignment, and first-slice reference/raw-pointer local integration are owned by [Source function-local bindings](local-bindings.md). [Source literal semantics](literals.md) consumes the scalar identities/value domains defined here. [Source field-value access](field-access.md) consumes nominal record/field identity, field source types, source type equality, and owned-value duplicability and separately owns direct record-field accessibility. [Source patterns](patterns.md) consumes nominal record/field identity, exact source type equality, structural field order, and duplicability for recursive record-pattern validation, including bounded node-local omission, and binding-leaf production. This document does not define literal materialization, reference formation/access/lifetimes, raw-pointer formation/access/provenance/unsafe admission, structural ownership state, field-access execution/accessibility, pattern lookup/ownership, conversions, general member lookup, or an implementation representation.

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

## Shared reference source types

The represented source type set additionally contains the bounded Shared-reference type constructor `SharedRef(T)` owned by `references.md`.

A `SharedRef(T)` exists in this revision only when `T` satisfies the first-slice Shared-referent-admission relation from `references.md`: `T` is a represented source value type, is source-duplicable, and contains no Shared-reference or raw-pointer type in its structural source value shape.

The concrete spelling `&T` maps to this constructor through `concrete-syntax.md`.

A Shared-reference referent edge is semantic indirection, not nominal-record direct containment. It therefore does not contribute a direct-containment edge under the record graph defined below.

This type foundation does not define reference targets, authorities, carriers, borrowing, dereference, lifetime validity, or source-to-Core reference refinement. Those relations are owned only by `references.md`.

## Raw-pointer source types

The represented source type set additionally contains the bounded raw-pointer type constructor `RawPtr(T)` owned by `raw-pointers-unsafe.md`.

A `RawPtr(T)` exists in this revision only when `T` satisfies that owner's first-slice raw-pointee-admission relation: `T` is exactly one represented intrinsic scalar or nominal record source type. Raw-pointer and Shared-reference types are not admitted as raw pointees in this slice.

The concrete `raw T` spelling maps to this constructor through `concrete-syntax.md`.

A raw-pointer pointee edge is semantic indirection, not nominal-record direct containment. It therefore does not contribute a direct-containment edge under the record graph defined below.

This type foundation does not define pointer targets, pointer-origin provenance, raw address formation, raw target access, unsafe admission, lexical pointer validity, or source-to-Core raw-pointer refinement. Those relations are owned only by `raw-pointers-unsafe.md`.

## Deliberately absent intrinsic and indirection types

This revision does not define intrinsic source type identities for:

- target-sized pointer/address-width integers;
- 128-bit or arbitrary-bit-width integers;
- extended, 128-bit, bfloat, decimal, or other floating formats;
- complex numbers, vectors, or matrices;
- character, string, or byte-sequence types;
- slices, arrays, tuples, enums, unions, function types, or other unrepresented composite/indirection forms.

Safe references and raw pointers are no longer wholly absent: the bounded Shared-only `SharedRef(T)` constructor is represented under `references.md`, and the bounded activation-local `RawPtr(T)` constructor is represented under `raw-pointers-unsafe.md`. This revision still defines no Exclusive/ExclusiveReplace source-reference type, mutable-reference form, nested Shared-reference referent, pointer-to-pointer/reference form, reference/raw-pointer-containing record field, or corresponding aggregate indirection form.

The absence of other source type forms does not narrow parameterized semantic relations already defined by applicable non-source owners. A later source revision may add a type identity when an accepted consumer requires it.

## Represented source type equality

For the represented source type set:

- two intrinsic scalar source types are equal exactly when they are the same intrinsic identity;
- two nominal record source types are equal exactly when they originate from the same record declaration identity;
- two Shared reference source types `SharedRef(A)` and `SharedRef(B)` are equal exactly when `A` and `B` are equal under this same source type-equality relation;
- two raw-pointer source types `RawPtr(A)` and `RawPtr(B)` are equal exactly when `A` and `B` are equal under this same source type-equality relation; and
- values from different represented type categories are never equal merely because lower representation or structure is similar.

Two distinct record declarations do not define the same source type merely because their fields have equal keys/types/order.

Lifetime, dynamic reference target/authority identity, raw-pointer target/origin provenance, and lower Core type identity are not source type-equality dimensions for `SharedRef(T)` or `RawPtr(T)` in this slice.

This relation does not define subtyping, coercion, conversion, layout compatibility, ABI compatibility, trait conformance, or representation equivalence.

Recursive record patterns consume this relation directly:

- a direct-root binding or producer result must have exactly the nominal record type selected by the top pattern head; and
- every nested record-pattern head must have exactly the nominal record type of the field in which that nested pattern occurs.

Structural similarity is never pattern compatibility.

## Nominal record type declarations

A **record type declaration** is a module-level source declaration that introduces exactly one module binding under `names-modules.md`.

That binding denotes one nominal record source type. Type identity is the identity of the record declaration/binding itself. Distinct record declarations therefore denote distinct source types.

The binding's module-private/exported accessibility is determined only by `names-modules.md`.

The represented `record` form in `concrete-syntax.md` establishes one such declaration and maps its concrete field sequence to the structure below. Absence/presence of concrete `export` on the record item establishes module-private/exported record-binding accessibility respectively. Other future declaration forms may establish the same semantic declaration category only through their accepted mapping.

A record declaration contains one finite ordered field sequence, which MAY be empty.

For record value shape and field identity, each field has exactly:

- one lexical identifier key governed by `lexical.md`; and
- one represented record-field-admissible source value type.

Each field additionally has one direct record-field accessibility fact owned by `field-access.md`. That accessibility fact is not part of field identity, nominal record identity, source type equality, structural field order, direct-containment shape, or physical layout.

Field lexical keys MUST be unique within one record declaration.

Field identity is scoped by the containing record type. Fields with the same lexical key in distinct record types are distinct fields.

For this revision, a **record-field-admissible source type** is exactly either:

- one intrinsic scalar source type defined here; or
- one nominal record source type whose record binding is legally resolvable for the declaring source unit under same-module or qualified cross-module lookup from `names-modules.md`.

`SharedRef(T)` and `RawPtr(T)` are represented source value types but are deliberately **not** record-field-admissible in these first indirection slices. Consequently a nominal record cannot yet store a Shared reference or raw-pointer value, and record construction/pattern/field-value owners require no reference-carrier or pointer-provenance aggregate semantics.

The ordered field sequence is semantic structural order for the source record value shape. It MAY be consumed where another accepted owner needs structural order. It does not define physical field order, byte offsets, padding, alignment, ABI layout, stable representation, or address arithmetic.

A value of a represented record type contains exactly one field value of the declared type for every field in the declaration. This source value shape creates no physical layout contract.

`structural-ownership.md` consumes record field identity/type/order to define structurally valid source paths and recursive remaining-ownership frontiers. `local-bindings.md` instantiates that relation for bindings. `field-access.md` consumes field lexical key, containing nominal record identity, declared type, and duplicability while owning the separate direct-accessibility rule used to select/produce a field value. `patterns.md` consumes nominal field identities/types and the separately owned direct-accessibility relation to validate explicit field selection, no-rest exhaustiveness or rest-authorized omission, and retain binding-leaf structural paths. Pattern presentation order never changes this declaration's structural field order.

## Direct record containment

For represented record types, define a **direct-containment edge** `A -> B` exactly when record type `A` has a field whose source type is record type `B`.

The finite graph consisting of represented record types and all such edges MUST be acyclic.

This requirement still applies because first-slice record fields remain restricted to intrinsic scalars or direct nominal-record containment. The existence of `SharedRef(T)` and `RawPtr(T)` as separate source value types does not alter that graph while indirection-containing record fields remain forbidden.

A Shared-reference referent edge and a raw-pointer pointee edge are semantic indirection and are not direct-containment edges. This fact does not itself authorize an indirection-containing record field or recursive nominal type in this revision.

The rule does not prohibit a later recursive nominal type when every cycle passes through an accepted indirection type whose canonical source semantics establish the field-admission and lifetime/provenance relation. That later owner must define the applicable well-formedness relation explicitly.

## Owned-value duplicability

Every represented source value type has one source-semantic **owned-value duplicability** classification: **duplicable** or **non-duplicable**.

Duplicability means only that another accepted source operation may, when its own semantics explicitly use this capability, produce another owned value preserving the source semantic value without consuming the source value.

Duplicability does not define or require a source equality or comparison relation. It does not mean bitwise copying and does not imply shared storage identity, shared stored-value lifetime, aliasing, physical representation equality, ABI passing, or a particular realization strategy.

The represented intrinsic scalar source types are duplicable: `Bool`, every represented signed/unsigned fixed-width integer type, and `F16`, `F32`, and `F64`.

For floating values, duplication preserves the semantic floating value under applicable floating contracts. It does not define floating comparison equality and adds no NaN representation, payload, sign, or canonicalization guarantees beyond existing authority.

Every represented `SharedRef(T)` is duplicable under the reference-carrier consequence owned by `references.md`. Duplicating one Shared reference preserves its target/authority identity and adds one source carrier; it does not duplicate the referent value or create a new root Shared authority.

Every represented `RawPtr(T)` is duplicable under the pointer-value/provenance consequence owned by `raw-pointers-unsafe.md`. Duplicating one raw pointer preserves its pointer value and exact pointer-origin provenance; it does not duplicate, borrow, move, or otherwise access the pointee value and creates no Shared authority.

Each nominal record declaration has one source-semantic **duplicable selection**. A record may select duplicability only when every field source type is duplicable. A record that does not select duplicability is non-duplicable even if every field type is duplicable.

The concrete record form in `concrete-syntax.md` provides one optional record-specific `copy` selection. Presence of that selection makes the nominal record perform the positive duplicable selection above. Absence makes no positive selection and therefore leaves the record non-duplicable under the no-selection rule.

Selection validity is determined from the resolved source types of all direct fields and is independent of declaration order. For a nominal-record field, that field type is duplicable only when the referenced nominal record itself has a valid positive duplicable selection. Because the represented direct-containment graph is finite and acyclic, this recursively determines eligibility for every selected record without introducing a cyclic capability definition.

A selected record with no fields is duplicable: every field source type is duplicable vacuously. An unselected zero-field record remains non-duplicable because structural eligibility does not itself make the positive selection.

Every nominal record selects independently. A record containing only intrinsic or positively selected duplicable record fields does not become duplicable unless it also makes its own positive selection. Likewise, a selected record containing any unselected nominal-record field is source-invalid even when that field's lower structural representation could otherwise be copied.

Record-binding accessibility and direct record-field accessibility are independent of duplicability. After a field source type has legally resolved under the existing name/type rules, the field contributes to duplicability eligibility only through that source type's duplicability classification; module-private/exported status does not grant or deny the capability.

Distinct nominal record declarations make the selection independently. Equal field shape does not transfer the selection, and selection does not alter nominal identity, field identity, field type, structural field order, direct-containment edges, source type equality, or accessibility.

Duplicating a value of a duplicable nominal record type produces another owned record value by preserving every field's semantic value through that field type's duplicability capability. The original record value is not consumed.

The nominal selection is a conservative source ownership-policy choice. Structural shape alone does not silently grant duplication. This revision does not claim represented records already model unique resources, capabilities, handles, or custom destruction.

**Non-duplicable** means only that this non-consuming owned duplication capability is unavailable. It does not prohibit ownership transfer/consumption or a future explicit clone, copy-construction, conversion, factory, deserialization, or other operation from independently producing another value.

Ordinary whole-binding use uses this capability through `local-bindings.md`. Binding-rooted field-value use uses it through `field-access.md` for the final selected field path. Recursive record patterns use it through `patterns.md` independently for every binding leaf: a duplicable leaf produces a non-consuming duplicate from its complete structural path, while a non-duplicable leaf transfers/consumes exactly that complete path. Structural path availability and resulting ancestor/disjoint consequences are owned by `structural-ownership.md`, not by the duplicability classification.

First-slice Shared references and raw pointers are not record fields, so nominal-record duplicability does not yet recurse through an indirection-containing aggregate.

This section does not define other expression contexts, field assignment or partial reinitialization, parameter passing, result transfer, calls, pattern syntax, or any explicit cloning/copy-construction operation.

Duplicability is source semantics independent of any future `Copy`-like trait spelling. A later trait/generic mechanism may expose/derive/constrain this capability only if its canonical semantics preserve this classification; this revision introduces no trait membership.

No custom destructor semantics are defined. A later custom-destruction owner must explicitly define compatibility with duplicability and partial structural ownership rather than silently changing either property.

This capability reflects the conceptual distinction between ownership transfer and non-consuming duplication already present in Core semantics, but Core copyability representation is not source-language authority. A lower representation may be structurally copyable even when a source record made no positive duplicable selection; that lower fact MUST NOT grant source duplicability. Shared-reference duplicability likewise comes from `references.md`, and raw-pointer duplicability from `raw-pointers-unsafe.md`, not from inspecting lower Core copyability. This revision defines no independent source-to-MIR lowering rule or Core semantic change.

## Literal and conversion boundary

[Source literal semantics](literals.md) owns represented boolean literal values, required-type materialization of signed decimal integer literals into the fixed-width integer domains defined here, and required-type materialization of represented decimal floating literals into the `F16`/`F32`/`F64` semantic formats defined here. This type foundation supplies source type identities, integer value domains, and floating format parameters; it does not redefine literal spelling, contextual materialization, integer representability diagnostics, decimal-rational interpretation, floating rounding, or owned-value production.

This document defines no abstract/unbounded integer/float literal type, default literal type, literal suffix semantics, or compile-time-only numeric type. Context-typed integer or floating literal materialization adds no source type.

This document grants no implicit conversion/coercion/promotion/widening/narrowing/subtyping/numeric defaulting relation between represented source types. Literal materialization is not conversion because the literal datum has no prior concrete source type.

Those omissions do not prohibit a later accepted operation from defining an explicit or implicit conversion. They prevent the type foundation from creating conversion behavior before an operation owns it.

## Callable and declaration boundary

This document defines represented source type identity and the nominal record-type declaration/binding. Source function entities/callable signatures and contextual admission of Shared-reference parameter/result types are owned by [Source callables](callables.md). Raw-pointer contextual exclusion from callable parameters/results is owned by `raw-pointers-unsafe.md` and consumed by `callables.md`. Shared-reference value/lifetime semantics remain owned by `references.md`, and raw-pointer provenance/unsafe semantics remain owned by `raw-pointers-unsafe.md`.

This document does not define constants, statics, variables, type aliases, opaque types, traits, or another module-level declaration category beyond records.

Those declarations require independently owned source semantics rather than inference from current proving MIR.

## Structural ownership, bindings, field access, patterns, references, and raw pointers

`structural-ownership.md` is the sole source owner for structural source paths, structural ownership state, path availability/consumption requirements, and recursive remaining frontiers. Those facts are not type properties.

`local-bindings.md` owns binding identity, assignment mutability, lexical lookup/scope, binding lifecycle around structural ownership, ordinary whole-binding duplicate-or-consume use, assignment legality/reset, and first-slice Shared-reference/raw-pointer local integration.

`field-access.md` owns binding-rooted and bounded producer-backed field selection, direct field accessibility, final-path availability requirement, producer-receiver transient ownership, and duplicate-or-consume field-value production.

`patterns.md` owns recursive record-pattern selection with bounded node-local omission, binding-leaf introduction/order, direct-root leaf production, and producer-transient leaf ownership/cleanup selection.

`references.md` owns Shared reference targets, authority/carriers, borrowing, dereference/copy, lifetime validity, target-assignment exclusion, call consequences, and source-to-Core reference refinement.

`raw-pointers-unsafe.md` owns raw-pointer target/origin provenance, lexical pointer validity, raw address formation, raw ownership move/replacement, unsafe admission, represented unsafe-precondition discharge, and source-to-Core raw refinement.

This type owner supplies intrinsic/nominal type identities plus integration of the canonically owned Shared-reference/raw-pointer constructors into represented type equality and duplicability, nominal record/field identity, field types, source type equality, structural field order, and owned-value duplicability only.

The represented type identity, record shape, and duplicability classification do not by themselves determine:

- structural path availability or ownership state;
- Shared reference authority or lifetime;
- raw-pointer target/origin validity or unsafe admission;
- direct record-field accessibility;
- field assignment/partial-field reinitialization;
- interior mutability;
- pattern scope/shadowing/matching/control flow;
- method/associated-item/trait/extension/overload lookup;
- custom destruction/destructor bodies.

Proving-kernel copyability, path state, scalar liveness, reference-authority identity, raw-pointer provenance representation, or interior-mutability metadata is not source-language authority for those concerns.

## Further boundaries

The concrete intrinsic/record/Shared-reference/raw-pointer forms do not themselves define literal semantics, additional refutable/shorthand/wildcard/literal/guard pattern categories, record construction, field-value access, closures/captures, generics, traits/coherence, const/static semantics, unsafe callable contracts, mutable/exclusive source references, named lifetime syntax, ABI/layout/FFI/linkage, package/filesystem mapping, parser/lossless syntax/HIR, Core MIR lowering, or backend representation.

Represented boolean, decimal integer, and decimal floating literal semantics are owned by `literals.md`; Shared reference semantics by `references.md`; raw-pointer and unsafe-admission semantics by `raw-pointers-unsafe.md`; structural ownership by `structural-ownership.md`; field-value access and direct record-field accessibility by `field-access.md`; and recursive record destructuring with bounded node-local omission by `patterns.md`.

Additional type/declaration spellings require an accepted concrete-syntax owner and must preserve the type identities/relations defined here.