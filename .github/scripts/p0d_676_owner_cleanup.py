from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    n = text.count(old)
    if n != 1:
        raise SystemExit(f"{path}: expected one occurrence, found {n}: {old!r}")
    p.write_text(text.replace(old, new))

replace(
    "spec/language/source/generics.md",
    "an abstract type parameter is unequal to every concrete intrinsic, nominal-record, safe-reference, and raw-pointer source type.",
    "an abstract type parameter is unequal to every concrete intrinsic, nominal-record, safe-reference, raw-pointer, and function-value source type.",
)

fa = "spec/language/source/field-access.md"
replace(
    fa,
    "It does not redefine the validation transaction of a direct call, record construction, bounded field assignment, or another producer when used in another receiving position.",
    "It does not redefine the validation transaction of a source call, record construction, bounded field assignment, or another producer when used in another receiving position.",
)
replace(
    fa,
    "may have exactly the fault/divergence/transient behavior already associated with that direct call or record construction and its nested producers.",
    "may have exactly the fault/divergence/transient behavior already associated with that source call or record construction and its nested producers.",
)

replace(
    "spec/language/source/names-modules.md",
    "The consuming concrete type, direct-call, record-construction-target, record-pattern-head, marker-reference, implementation-target, or constant-value context validates the category of the resolved binding after this lookup; qualified lookup does not skip an inaccessible or wrong-category binding.",
    "The consuming concrete type, `Call`, record-construction-target, record-pattern-head, marker-reference, implementation-target, constant-value, or context-typed function-value-formation relation validates the category of the resolved binding after this lookup; a qualified `Call` remains direct, and qualified lookup does not skip an inaccessible or wrong-category binding.",
)

checks = {
    "spec/language/source/generics.md": ["nominal-record, safe-reference, and raw-pointer source type."],
    fa: ["validation transaction of a direct call", "associated with that direct call or record construction"],
    "spec/language/source/names-modules.md": ["concrete type, direct-call, record-construction-target"],
}
for path, forbidden in checks.items():
    text = Path(path).read_text()
    for phrase in forbidden:
        if phrase in text:
            raise SystemExit(f"{path}: residual remains: {phrase}")
