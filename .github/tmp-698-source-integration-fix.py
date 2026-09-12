#!/usr/bin/env python3
from pathlib import Path

def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one occurrence, found {count}: {old!r}")
    p.write_text(text.replace(old, new, 1))

# source/references.md
replace_once(
    "spec/language/source/references.md",
    "- broader generics/traits/coherence, const/static storage, async/tasks, ABI/layout/FFI/linkage, or package behavior.",
    "- broader generics/traits/coherence, constant-expression or static-storage semantics beyond the represented constant and immutable execution-static relations, async/tasks, ABI/layout/FFI/linkage, or package behavior.",
)

# source/concrete-syntax.md
replace_once(
    "spec/language/source/concrete-syntax.md",
    "Import declarations and module-level items MAY be interspersed. Their textual order does not change the order-independent module binding, source-unit alias, qualified lookup, marker implementation coherence, marker-obligation, or represented literal-initialized constant-value relations owned by `names-modules.md`, `traits.md`, and `constants.md`.",
    "Import declarations and module-level items MAY be interspersed. Their textual order does not change the order-independent module binding, source-unit alias, qualified lookup, marker implementation coherence, marker-obligation, represented literal-initialized constant-value, or represented literal-initialized static initial-value relations owned by `names-modules.md`, `traits.md`, `constants.md`, and `statics.md`.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "At source-unit item position, `export` modifies only one represented `ExportableDefinition`: a record definition, function definition, marker trait declaration, or constant declaration. The same reserved key has the separate bounded record-field position defined below. `export import` and `export impl` are not represented forms, and this reuse does not establish a general declaration-modifier system.",
    "At source-unit item position, `export` modifies only one represented `ExportableDefinition`: a record definition, function definition, marker trait declaration, constant declaration, or static declaration. The same reserved key has the separate bounded record-field position defined below. `export import` and `export impl` are not represented forms, and this reuse does not establish a general declaration-modifier system.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "## Constant declarations\n",
    "## Constant and static declarations\n",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "The spelling creates immutable execution-persistent storage only through `statics.md`; syntax does not expose a physical address. No mutable/thread-local/atomic modifier, aggregate static, storage class, linkage modifier, runtime initializer, `&mut` static, or raw-static form is introduced.",
    "The spelling creates immutable execution-persistent storage only through `statics.md`; syntax does not expose a physical address. No mutable/thread-local/atomic modifier, aggregate static, storage class, linkage modifier, runtime initializer, `&mut` static, or raw-static form is introduced.\n\n`static` is contextual, not reserved. `StaticDeclaration` is recognized only at the represented top-level exportable-definition position. Outside that position an identifier-form token whose lexical key is `static` remains an ordinary `UserIdentifier` when the surrounding grammar admits one. In particular, the second identifier in `static static: I64 = 1;` is syntactically an ordinary `UserIdentifier`.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "Each declaration maps to exactly one source constant declaration under `constants.md`. Its name contributes one ordinary module binding through `names-modules.md`, sharing the same declaration namespace and duplicate-key prohibition as records, functions, and marker traits.",
    "Each constant declaration maps to exactly one source constant declaration under `constants.md`. Its name contributes one ordinary module binding through `names-modules.md`, sharing the same declaration namespace and duplicate-key prohibition as records, functions, marker traits, and source statics.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "ValueAtom            = Literal\n                     | IdentifierUse\n                     | QualifiedModuleMember\n                     | Call",
    "ValueAtom            = Literal\n                     | ModuleScalarReference\n                     | Call",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "ModuleScalarReference         = UserIdentifier | QualifiedModuleMember",
    "ModuleScalarReference         = IdentifierUse | QualifiedModuleMember",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "`ModuleScalarReference` names the already shared bare/qualified scalar module-value token shapes. Its unqualified `UserIdentifier` branch shares `IdentifierUse`: semantic lookup first selects an active body binding when present and otherwise may select a same-module constant or static.",
    "`ModuleScalarReference` names the already shared bare/qualified scalar module-value token shapes. Its unqualified `IdentifierUse` branch applies semantic local-first lookup: an active body binding is selected when present, and otherwise same-module lookup may select a constant or static.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "Bare and qualified constant references reuse the existing identifier/module-qualification tokens without creating a general path or member grammar.",
    "Bare and qualified constant/static scalar references reuse the existing identifier/module-qualification tokens without creating a general path or member grammar.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A source-valid constant reference is different from a contextual literal: it has the exact intrinsic type retained by its declaration. A constant of type `Bool` may therefore directly satisfy the condition requirement, while an integer or floating constant used directly as a condition is source-invalid because its exact type is not `Bool`.",
    "A source-valid constant reference or static scalar read is different from a contextual literal: each has the exact intrinsic type retained by its declaration. A constant or static of type `Bool` may therefore directly satisfy the condition requirement, while an integer or floating constant/static used directly as a condition is source-invalid because its exact type is not `Bool`.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A same-module or qualified integer constant may supply the same exact comparison anchor under its constant declaration type.",
    "A same-module or qualified integer constant or static may supply the same exact comparison anchor under its declaration type.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "An unqualified `IdentifierUse` may denote an active local or, only after local lookup fails, a same-module constant. A bare `QualifiedModuleMember` conditional atom may denote only an exported constant under the constant-reference category. All lookup, function-value, constant, generic application, receiver-transient, operator-operand, grouping-transparency, and producer rules remain owned by their existing semantic owners.",
    "An unqualified `IdentifierUse` may denote an active local or, only after local lookup fails, a same-module constant or static. A bare `QualifiedModuleMember` conditional atom may denote an exported constant or static under the applicable module-scalar category. All lookup, function-value, constant, static, generic application, receiver-transient, operator-operand, grouping-transparency, and producer rules remain owned by their existing semantic owners.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A source-valid constant reference initializer is one ordinary scalar producer under `constants.md` and does not create local storage or availability state for the constant declaration itself.",
    "A source-valid constant reference or static scalar read initializer is one ordinary scalar producer under `constants.md` or `statics.md` respectively. Neither creates local storage or availability state for the module declaration, and a static read leaves persistent storage unchanged.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A constant reference, safe-reference root/reborrow/dereference, raw address formation, or raw ownership move discovered as the selected wrapper's root is not a governed floating operation and therefore fails numeric-contract selector applicability before its producer-state effects commit.",
    "A constant reference, static scalar read, safe-reference root/reborrow/dereference, raw address formation, or raw ownership move discovered as the selected wrapper's root is not a governed floating operation and therefore fails numeric-contract selector applicability before its producer-state effects commit.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A same-module constant is not source-addressable and cannot satisfy the root category.",
    "A same-module constant is not source-addressable, and a source static is not replacement-capable; neither can satisfy the replacement-root category.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A same-module constant reached after local lookup fails is wrong-category and cannot be dereferenced.",
    "A same-module constant or static reached after local lookup fails is wrong-category and cannot be dereferenced.",
)
replace_once(
    "spec/language/source/concrete-syntax.md",
    "A module constant is not a pointer binding and cannot satisfy this form.",
    "A module constant or static is not a pointer binding and cannot satisfy this form.",
)

# source/local-bindings.md
replace_once(
    "spec/language/source/local-bindings.md",
    "defined-fault propagation, raw-operation execution ordering, and constant-value producer integration are owned by [Source function execution](function-execution.md).",
    "defined-fault propagation, raw-operation execution ordering, constant-value producer integration, and static-read/Shared-static-root producer integration are owned by [Source function execution](function-execution.md).",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "Concrete parameter/local/pattern/closure/value/call/field-value/assignment/block/conditional/while/return/reference/raw-pointer/unsafe/generic/constant spellings are owned by [Source concrete syntax](concrete-syntax.md).",
    "Concrete parameter/local/pattern/closure/value/call/field-value/assignment/block/conditional/while/return/reference/raw-pointer/unsafe/generic/constant/static spellings are owned by [Source concrete syntax](concrete-syntax.md).",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "This document does not define structural ownership mathematics, closure-site/type/capture semantics, constant declaration/value production, generic parameter identity/substitution/capability semantics, safe-reference formation/dereference/reborrow/replacement/authority semantics, raw-pointer formation/pointee access/unsafe admission semantics, normal-continuation presence, conditional or loop selection/successor composition, field lookup/accessibility, pattern structure, general expression evaluation, traits, ABI, Core liveness, or an implementation representation.",
    "This document does not define structural ownership mathematics, closure-site/type/capture semantics, constant declaration/value production, static declaration/persistent-read/Shared-root semantics, generic parameter identity/substitution/capability semantics, safe-reference formation/dereference/reborrow/replacement/authority semantics, raw-pointer formation/pointee access/unsafe admission semantics, normal-continuation presence, conditional or loop selection/successor composition, field lookup/accessibility, pattern structure, general expression evaluation, traits, ABI, Core liveness, or an implementation representation.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "A body-local binding key MAY equal a module-level declaration key, including a module constant/function key, and MAY equal a source-unit module-alias key.",
    "A body-local binding key MAY equal a module-level declaration key, including a module constant/static/function key, and MAY equal a source-unit module-alias key.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "Only when no active binding resolves the key may same-module lookup select a module declaration: `constants.md` owns constant value production, while `function-values.md` owns context-typed function-value formation when an exact required function-value type selects a non-generic function entity.",
    "Only when no active binding resolves the key may same-module lookup select a module declaration: `constants.md` owns constant value production, `statics.md` owns immutable static scalar-read production, and `function-values.md` owns context-typed function-value formation when an exact required function-value type selects a non-generic function entity.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "For Shared root `&x` or `&x.field...`, `references.md` may select an active parameter, ordinary local, or independently admissible closure capture binding and owns selection of the complete root or bounded structural field path plus the exact Shared referent/accessibility/availability requirements. Dedicated opaque closure bindings remain inadmissible because closure types are not Shared referents.",
    "For Shared root `&x` or `&x.field...`, `references.md` may select an active parameter, ordinary local, or independently admissible closure capture binding and owns selection of the complete root or bounded structural field path plus the exact Shared referent/accessibility/availability requirements. For the zero-selector unqualified form `&x` only, when no active binding resolves `x`, same-module lookup may instead select one source static and delegate complete-root Shared formation to `statics.md`/`references.md`; an active wrong-category local remains final, and `&x.field...` never falls through to a static. Dedicated opaque closure bindings remain inadmissible because closure types are not Shared referents.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "Source-unit module aliases remain the distinct qualified-lookup mechanism owned by `names-modules.md`. The concrete `alias::member` qualified call target and qualified constant/function-value candidate resolve through that mechanism rather than this unqualified lookup. Qualified calls remain direct module-function calls; an exact required function-value type may instead select qualified formation from an exported non-generic function. Body-local bindings do not block either explicitly qualified value relation.",
    "Source-unit module aliases remain the distinct qualified-lookup mechanism owned by `names-modules.md`. The concrete `alias::member` qualified call target and qualified constant/static/function-value candidates, plus the bounded `&alias::member` Shared static-root form, resolve through that mechanism rather than this unqualified lookup. Qualified calls remain direct module-function calls; an exact required function-value type may instead select qualified formation from an exported non-generic function. Body-local bindings do not block these explicitly qualified relations.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "A generic type-parameter key therefore does not block an otherwise valid module constant lookup in a value position.",
    "A generic type-parameter key therefore does not block an otherwise valid module constant or static lookup in a value position.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "bounded closure capture/target lookup, constant-value lookup consumed by `constants.md`, and the distinct generic type-position lookup above",
    "bounded closure capture/target lookup, constant-value lookup consumed by `constants.md`, static scalar-value/Shared-root lookup consumed by `statics.md`, and the distinct generic type-position lookup above",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "## Function, closure, call, assignment, pattern, control-flow, reference, raw-pointer, constant, and fault boundary",
    "## Function, closure, call, assignment, pattern, control-flow, reference, raw-pointer, constant, static, and fault boundary",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "It does not redefine constant declaration/value/use semantics from `constants.md`, generic application/exact substitution/activation substitution from `generics.md`, or closure type/capture/call semantics from `closures.md`.",
    "It does not redefine constant declaration/value/use semantics from `constants.md`, static declaration/storage/read/Shared-root semantics from `statics.md`, generic application/exact substitution/activation substitution from `generics.md`, or closure type/capture/call semantics from `closures.md`.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "- constant-value producer integration;\n",
    "- constant-value producer integration;\n- static-read and Shared-static-root producer integration;\n",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "or constant semantic values from `constants.md`.",
    "or constant semantic values from `constants.md` or static declaration/storage/read/root semantics from `statics.md`.",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "Beyond the represented concrete subset, accepted first function-only generic type-parameter relation, bounded closure relation, and module-level intrinsic-scalar constant relation, this revision does not define",
    "Beyond the represented concrete subset, accepted first function-only generic type-parameter relation, bounded closure relation, module-level intrinsic-scalar constant relation, and bounded immutable execution-static relation, this revision does not define",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "constant-expression evaluation beyond `constants.md`, static storage semantics, ABI/FFI/linkage",
    "constant-expression evaluation beyond `constants.md`, static storage semantics beyond `statics.md`, ABI/FFI/linkage",
)
replace_once(
    "spec/language/source/local-bindings.md",
    "Activation-local raw pointers and lexical unsafe admission are represented by `raw-pointers-unsafe.md`; source constants are represented by `constants.md`; bounded closure bindings/captures are represented by `closures.md`.",
    "Activation-local raw pointers and lexical unsafe admission are represented by `raw-pointers-unsafe.md`; source constants are represented by `constants.md`; immutable execution statics are represented by `statics.md`; bounded closure bindings/captures are represented by `closures.md`.",
)

# source/function-execution.md
replace_once(
    "spec/language/source/function-execution.md",
    "The represented concrete function/closure-declaration/body/block/value/constant-reference/grouping/numeric-contract-selection/operator/call/record-construction/field-value/record-destructuring/assignment/reference-replacement/conditional/while/break/continue/return/explicit-fault/safe-reference/raw-pointer/unsafe grammar is owned by [Source concrete syntax](concrete-syntax.md).",
    "The represented concrete function/closure-declaration/body/block/value/constant-reference/static-read/grouping/numeric-contract-selection/operator/call/record-construction/field-value/record-destructuring/assignment/reference-replacement/conditional/while/break/continue/return/explicit-fault/safe-reference/raw-pointer/unsafe grammar is owned by [Source concrete syntax](concrete-syntax.md).",
)
replace_once(
    "spec/language/source/function-execution.md",
    "Literal evaluation has no source-visible side effect under `literals.md`, and constant-reference production has none under `constants.md`; admitting either to represented ordinary value positions therefore adds no competing effect-order relation.",
    "Literal evaluation has no source-visible side effect under `literals.md`, constant-reference production has none under `constants.md`, and immutable static scalar reads have none under `statics.md`; admitting any of these to represented ordinary value positions therefore adds no competing effect-order relation.",
)
replace_once(
    "spec/language/source/function-execution.md",
    "`concrete-syntax.md` owns represented concrete grammar, including module-level contextual `const` declarations and bounded unqualified/qualified constant references; bounded Shared `&T`, replacement-capable `&mut T`, Shared root `&x`/`&x.field...`, replacement-capable root `&mut x`/`&mut x.field...`,",
    "`concrete-syntax.md` owns represented concrete grammar, including module-level contextual `const` and `static` declarations and bounded unqualified/qualified constant/static scalar references; bounded Shared `&T`, replacement-capable `&mut T`, Shared root `&x`/`&x.field...` plus bounded qualified Shared static root `&alias::x`, replacement-capable root `&mut x`/`&mut x.field...`,",
)
replace_once(
    "spec/language/source/function-execution.md",
    "`constants.md` owns source constant declaration/value/use semantics, exact static result-type evidence, and direct source-to-Core constant refinement.",
    "`constants.md` owns source constant declaration/value/use semantics, exact static result-type evidence, and direct source-to-Core constant refinement. `statics.md` owns immutable execution-static declaration/storage/read/Shared-root semantics and source-to-Core persistent-storage refinement.",
)
replace_once(
    "spec/language/source/function-execution.md",
    "After source validation, a represented constant-reference producer refines only through the exact source-to-Core constant mapping owned by `constants.md`. This execution owner adds no Core operation, temporary storage, global, load, initializer, symbol, ABI/linkage object, or runtime state merely to consume that value.",
    "After source validation, a represented constant-reference producer refines only through the exact source-to-Core constant mapping owned by `constants.md`. This execution owner adds no Core operation, temporary storage, global, load, initializer, symbol, ABI/linkage object, or runtime state merely to consume that value.\n\nA represented immutable static scalar-read producer refines through `statics.md` to the persistent scalar-read relation in Core `persistent-storage.md`. A source Shared static root `&S` or `&alias::S` refines jointly through `statics.md`/`references.md` to the Core Shared persistent-root relation over the resolved execution-persistent storage instance. These mappings retain semantic persistent declaration/instance identity and reference target/authority as required for validation but establish no physical global address, load instruction shape, linker symbol, ABI/linkage identity, or backend storage class.",
)
replace_once(
    "spec/language/source/function-execution.md",
    "Shared root `&x`/`&x.field...` to the existing Core `ReferenceRoot` relation with Shared permission over direct storage representing `x` followed by the retained exact resolved field-projection sequence;",
    "activation-local Shared root `&x`/`&x.field...` to the existing Core `ReferenceRoot` relation with Shared permission over direct storage representing `x` followed by the retained exact resolved field-projection sequence;",
)
