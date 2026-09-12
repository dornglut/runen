#!/usr/bin/env python3
from pathlib import Path

P = Path("spec/language/source/concrete-syntax.md")
text = P.read_text()

def repl(old: str, new: str, expected: int = 1) -> None:
    global text
    count = text.count(old)
    if count != expected:
        raise SystemExit(f"expected {expected} occurrence(s), found {count}: {old!r}")
    text = text.replace(old, new)

repl("`ConstReference`", "`ModuleScalarReference`", 3)
repl("bounded constant references", "bounded constant/static scalar references")
repl(
    "A selected source constant may produce its exact intrinsic scalar value under `constants.md`; when the surrounding receiving position instead supplies one exact function-value type, a selected non-generic source function may form the exact function value under `function-values.md`.",
    "A selected source constant may produce its exact intrinsic scalar value under `constants.md`; a selected source static may perform its immutable persistent scalar read under `statics.md`; when the surrounding receiving position instead supplies one exact function-value type, a selected non-generic source function may form the exact function value under `function-values.md`.",
)
repl(
    "For the module-constant branch, no structural root, carrier, pointer origin, availability state, hidden storage, or Copy-from-storage operation exists, and repeated uses independently produce the same declared semantic scalar value. Context-typed module-function formation likewise creates no binding root or storage state and is effect-free after successful validation under `function-values.md`.",
    "For the module-constant branch, no structural root, carrier, pointer origin, availability state, hidden storage, or Copy-from-storage operation exists, and repeated uses independently produce the same declared semantic scalar value. For the module-static branch, the persistent instance remains Live and unchanged; one immutable read produces an independent owned duplicable scalar under `statics.md` without creating activation-local structural ownership, consuming the persistent value, or exposing a physical address. Context-typed module-function formation likewise creates no binding root or storage state and is effect-free after successful validation under `function-values.md`.",
)
repl(
    "The constant relation requires an exported source constant and produces its exact intrinsic scalar value; the function-value relation requires an exported non-generic source function whose exact callable interface equals the required function-value type. A private, wrong-category, generic-function, or type-mismatched target is final and invalid. In a direct conditional atom the surrounding exact type is `Bool`, so only the constant relation can succeed.",
    "The module-scalar relation requires an exported source constant or static and respectively produces the constant's exact intrinsic scalar value under `constants.md` or performs the static's immutable scalar read under `statics.md`; the function-value relation requires an exported non-generic source function whose exact callable interface equals the required function-value type. A private, wrong-category, generic-function, or type-mismatched target is final and invalid. In a direct conditional atom the surrounding exact type is `Bool`, so only an exported constant or static of exact type `Bool` can succeed through the module-scalar relation.",
)
repl(
    "The bounded `SafeReferenceValue` forms and `SafeDereferenceValue`, plus `RawAddressOfValue` and `RawMoveValue`, use ordinary unqualified function-local value lookup and require the selected entity to satisfy the exact parameter/local category/type restrictions owned by `references.md` or `raw-pointers-unsafe.md`. For a Shared or replacement-capable binding-field root, only the first root identifier participates in that binding lookup; for a Shared or replacement-capable field-relative reborrow, only the parent safe-reference identifier participates. Later `FieldSelector`s in those bounded branches resolve nominal field identities under `references.md`/`field-access.md` rather than performing another function-local or module lookup. `ReferenceReplaceStatement` and `RawAssignStatement` use the ordinary binding precedence for their operands as well. When local lookup fails and same-module lookup selects a constant in one of these binding-only forms, category validation rejects it rather than treating the constant as source-addressable storage.",
    "The activation-local `SafeReferenceValue` branches, `SafeDereferenceValue`, `RawAddressOfValue`, and `RawMoveValue` use ordinary unqualified function-local value lookup and require the selected entity to satisfy the exact parameter/local category/type restrictions owned by `references.md` or `raw-pointers-unsafe.md`. The one represented exception is zero-selector Shared `&x`: when no active local binding resolves `x`, same-module lookup may select one source static; qualified `&alias::x` may independently select one exported source static. Those static-root cases delegate to `statics.md`/`references.md` and are not activation-local binding roots. For a Shared or replacement-capable activation-local binding-field root, only the first root identifier participates in binding lookup; for a Shared or replacement-capable field-relative reborrow, only the parent safe-reference identifier participates. Later `FieldSelector`s in those bounded branches resolve nominal field identities under `references.md`/`field-access.md` rather than performing another function-local or module lookup. `ReferenceReplaceStatement` and `RawAssignStatement` use the ordinary binding precedence for their operands as well. When local lookup fails and same-module lookup selects a constant or static in a binding-only replacement, dereference, reborrow, or raw form, category validation rejects it rather than treating that module binding as an activation-local operand.",
)
repl(
    "`function-execution.md` defines required-type propagation, complete producer-state handling, fault/divergence, constant/safe-reference/raw-pointer value preservation, and typed/lowering erasure boundaries.",
    "`function-execution.md` defines required-type propagation, complete producer-state handling, fault/divergence, constant/static-scalar/safe-reference/raw-pointer value preservation, and typed/lowering erasure boundaries.",
)
repl(
    "Constant, safe-reference, and raw-pointer producers are not governed roots and fail selector applicability before any producer-state effects may commit.",
    "Constant, static-scalar, safe-reference, and raw-pointer producers are not governed roots and fail selector applicability before any producer-state effects may commit.",
)
repl(
    "in a direct conditional position only the constant relation can satisfy the required `Bool`.",
    "in a direct conditional position only the module-scalar relation for an exact-`Bool` constant or static can satisfy the required `Bool`.",
)
repl(
    "no general unary or binary expression hierarchy beyond the bounded constant-reference, safe-reference dereference/root/reborrow forms, raw address/move atoms, operator prefix/multiplicative/additive/exclusive-or/bitwise-OR/comparison/logical-conjunction tiers;",
    "no general unary or binary expression hierarchy beyond the bounded constant/static module-scalar reference, safe-reference dereference/root/reborrow forms, raw address/move atoms, operator prefix/multiplicative/additive/exclusive-or/bitwise-OR/comparison/logical-conjunction tiers;",
)
repl(
    "A resolved constant reference supplies `Exact(T)` from its declared intrinsic type.",
    "A resolved constant reference or static scalar read supplies `Exact(T)` from its declaration's exact intrinsic type.",
)
repl(
    "including exact constant references,",
    "including exact constant references and static scalar reads,",
)
repl(
    "a `GroupedValue`, a `NumericContractSelectedValue`, a constant reference, a bounded safe-reference/raw atom, or the bounded `SafeDereferenceValue`.",
    "a `GroupedValue`, a `NumericContractSelectedValue`, a constant reference, a static scalar read, a bounded safe-reference/raw atom, or the bounded `SafeDereferenceValue`.",
)
repl(
    "`constants.md` owns constant lookup/value production, `references.md` separately owns safe-reference producer typing",
    "`constants.md` owns constant lookup/value production, `statics.md` owns immutable static lookup/read production, `references.md` separately owns safe-reference producer typing",
)
repl(
    "including constant references, represented operators, bounded safe-reference producers, bounded raw-pointer producers, bounded contextual grouping, operation-local numeric-contract selection, and bounded producer-backed field-value use.",
    "including constant references, static scalar reads, represented operators, bounded safe-reference producers, bounded raw-pointer producers, bounded contextual grouping, operation-local numeric-contract selection, and bounded producer-backed field-value use.",
)
repl(
    "A source-valid constant reference may fill one initializer exactly when its intrinsic constant type equals that field type; the constructor does not reinterpret the constant as a literal or hidden load.",
    "A source-valid constant reference or static scalar read may fill one initializer exactly when its declaration's intrinsic scalar type equals that field type; the constructor does not reinterpret either producer as a literal or change the static read's persistent-storage semantics.",
)
repl(
    "Boolean, decimal integer, decimal floating, and named constant scalar values and represented operator/safe-reference/raw-pointer values are not producer-backed record-pattern scrutinees.",
    "Boolean, decimal integer, decimal floating, and named constant/static scalar values and represented operator/safe-reference/raw-pointer values are not producer-backed record-pattern scrutinees.",
)
repl(
    "may now resolve to a constant after local lookup fails.",
    "may now resolve to a constant or static after local lookup fails.",
)
repl(
    "A qualified constant reference is likewise not admitted.",
    "A qualified constant or static module-scalar reference is likewise not admitted.",
)
repl(
    "Boolean, decimal integer, decimal floating, or named constant scalar values, represented operator values,",
    "Boolean, decimal integer, decimal floating, or named constant/static scalar values, represented operator values,",
)
repl(
    "A bare qualified constant-or-function-value candidate is not a field receiver.",
    "A bare qualified constant/static/function-value candidate is not a field receiver.",
)
repl(
    "a qualified bare module member does not become a field receiver merely by being a constant reference or a context-typed function-value candidate.",
    "a qualified bare module member does not become a field receiver merely by being a constant/static module-scalar reference or a context-typed function-value candidate.",
)
repl(
    "neither wrapper, a constant reference, a safe-reference producer,",
    "neither wrapper, a constant reference, a static scalar read, a safe-reference producer,",
)
repl(
    "standalone literal, named constant reference, operator, safe-reference/raw-pointer producer, or other scrutinee category.",
    "standalone literal, named constant/static module-scalar reference, operator, safe-reference/raw-pointer producer, or other scrutinee category.",
)
repl(
    "floating literal/ordering test, named constant pattern test, scalar top pattern,",
    "floating literal/ordering test, named constant/static pattern test, scalar top pattern,",
)
repl(
    "A module constant selected only after local lookup fails is never a valid assignment target because constants are not parameter/local bindings and have no assignment mutability or storage state.",
    "A module constant or static selected only after local lookup fails is never a valid assignment target because neither is an activation-local parameter/local binding with assignment mutability; static storage remains immutable under `statics.md`.",
)
repl(
    "raw pointer target, constant binding, arbitrary postfix chain,",
    "raw pointer target, constant/static module binding, arbitrary postfix chain,",
)
repl(
    "a Shared-reference, raw-pointer binding, or module constant does not become a replacement target",
    "a Shared-reference, raw-pointer binding, module constant, or module static does not become a replacement target",
)
repl(
    "A same-module constant reached after local lookup fails is a wrong category and does not become a raw-pointer operand.",
    "A same-module constant or static reached after local lookup fails is a wrong category and does not become a raw-pointer operand.",
)
repl(
    "A `GroupedValue`, `NumericContractSelectedValue`, constant reference, safe-reference producer, `RawAddressOfValue`, or `RawMoveValue` does not become a `CallStatement`",
    "A `GroupedValue`, `NumericContractSelectedValue`, constant reference, static scalar read, safe-reference producer, `RawAddressOfValue`, or `RawMoveValue` does not become a `CallStatement`",
)
repl(
    "named constants are likewise not admitted as pattern test leaves or bounds merely because their values are scalar.",
    "named constants or statics are likewise not admitted as pattern test leaves or bounds merely because their values are scalar.",
)
repl(
    "grouping changes neither constant lookup/category nor reference/raw target syntax/authority/unsafe admission",
    "grouping changes neither constant/static module-scalar lookup/category nor reference/raw target syntax/authority/unsafe admission",
)

P.write_text(text)
