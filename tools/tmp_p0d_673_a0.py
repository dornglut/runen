from pathlib import Path

path = Path("docs/verification/a0.md")
text = path.read_text()


def once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one occurrence, found {count}: {old[:120]!r}")
    text = text.replace(old, new, 1)


once(
    "This document owns the test obligations for the executable A0 oracle. The normative semantics exercised by A0 are owned by `spec/language/core/value-storage.md`, `spec/language/core/borrowing.md`, `spec/language/core/pointers.md`, `spec/language/core/unsafe.md`, and `spec/language/core/functions.md`.",
    "This document owns the test obligations for the executable A0 oracle. The normative semantics exercised by A0 are owned by `spec/language/core/value-storage.md`, `spec/language/core/borrowing.md`, `spec/language/core/pointers.md`, `spec/language/core/unsafe.md`, `spec/language/core/functions.md`, and `spec/language/core/callable-values.md`.",
)

once(
    "Raw `runen-core-ir::Body` is a construction/data form. It is not executable by the reference machine until `validate_body` establishes the structural and language-validity rules represented by the current A0 MIR and returns `ValidatedBody`. This is language validation, not the environment-admission phase defined by `spec/language/lifecycle.md`.",
    "Raw `runen-core-ir::Program` is a construction/data form. It is not executable by the reference machine until `validate_program` establishes the structural and language-validity rules represented by the current A0 MIR and returns `ValidatedProgram`. This is language validation, not the environment-admission phase defined by `spec/language/lifecycle.md`.",
)

anchor = "Validation also proves reachable initialization and active-loan-forest state transitions for defined executions."
addition = """Program-level validation also proves the represented callable-value relation. Each function derives one canonical callable interface from its ordered parameter-local types, optional result type, and exact safe-reference result contract; callable scalar types consume that same interface relation rather than defining a second signature validator. Callable parameter/result `TypeId` edges are semantic interface edges, not by-value structural containment, so callable-signature cycles are compatible with the existing rejection of recursive by-value struct shapes. A function-value operand is admitted only under one exact expected callable `TypeId`, names an existing same-program `FunctionId`, and requires exact structural equality between the target function's derived callable interface and that callable type's interface. Distinct callable `TypeId`s remain distinct even when their interfaces are equal; ordinary storage, Copy, Move, aggregate transport, parameter transfer, and result transfer preserve the exact stored type rather than introducing structural callable conversion.\n\nAn indirect call carries one exact static callable `TypeId`. Validation admits its optional result destination before operand effects, evaluates the callee operand under that exact callable type, evaluates arguments left-to-right against the callable interface, performs the existing final safe-reference call-entry admission after argument effects, and then reuses the existing function-call state summary. Validator state does not add a target-set analysis: the same-program `FunctionId` selected dynamically is guaranteed to match the static interface by the function-value formation invariant. Direct calls remain direct and continue to use their statically named function entity.\n\n"""
if text.count(anchor) != 1:
    raise SystemExit("unexpected path-state anchor count")
text = text.replace(anchor, addition + anchor, 1)

once(
    "Result-bearing direct calls use the same wholly-vacant and Init-like exclusive destination admission before argument state transitions.",
    "Result-bearing direct and indirect calls use the same wholly-vacant and Init-like exclusive destination admission before call operand state transitions.",
)

once(
    "The reference machine accepts only `ValidatedBody`.",
    "The reference machine accepts only `ValidatedProgram`.",
)

section_anchor = "## Lifetime and destruction-domain coverage\n"
callable_section = """## Callable-value and indirect-call coverage\n\nThe reference oracle represents a callable runtime value only as the exact same-program `FunctionId` selected when the function value is formed. Formation creates no activation, storage identity, safe-reference authority, raw-pointer provenance, address, ABI token, or other dynamic callable identity. Copy, Move, storage, aggregate containment, parameter/result transfer, replacement, and destruction use the ordinary scalar-value machinery; callable destruction has no callable-specific cleanup effect. `ObservedValue::Function(FunctionId)` is verification-only harness evidence and does not add Core equality, ordering, hashing, conversion, address observation, or representation semantics.\n\nFor an indirect call, the oracle evaluates and holds the callee operand before evaluating arguments left-to-right. It then selects the held `FunctionId` and enters the same activation-construction, parameter-transfer, caller-suspension, normal-return, reference-result, fault-propagation, and cleanup machinery already used after direct-call target selection. There is no second indirect-call activation machine. The validator's exact callable-type/interface invariant makes null, invalid-target, signature-mismatch, and result-contract-mismatch runtime branches unnecessary for validated programs.\n\nTests must prove at least:\n\n- callable signature edges can cycle without making the represented value shape structurally recursive, while by-value struct cycles remain invalid;\n- callable-type admission reuses existing parameter/result transfer safety and safe-reference result-contract rules;\n- one same-program function may be formed under distinct callable `TypeId`s with equal interfaces, while stored values preserve their exact nominal `TypeId` and cannot be used as a different equal-interface callable type;\n- unknown function entities, non-callable formation contexts, and interface mismatches are rejected during validation rather than becoming runtime faults;\n- callable values survive ordinary scalar Copy/Move/storage, aggregate transport, parameter transfer, result transfer, and destruction without custom cleanup or extra dynamic identity;\n- indirect result destinations are admitted before callee effects, the callee operand is evaluated before the first argument, arguments are evaluated left-to-right, and callee Move commits before argument evaluation;\n- indirect dispatch preserves exact function-entity identity and reuses the existing function activation machine for normal return, recursive call graphs, defined fault propagation, divergence, and cleanup;\n- `SharedIdentity` and `SharedDirectChild` results through indirect calls preserve the same reference-authority consequences as the common call semantics;\n- direct calls remain executable through their existing statically named target path rather than being lowered through callable values.\n\n"""
if text.count(section_anchor) != 1:
    raise SystemExit("unexpected lifetime-section anchor count")
text = text.replace(section_anchor, callable_section + section_anchor, 1)

path.write_text(text)
