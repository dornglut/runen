from pathlib import Path


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old!r}")
    p.write_text(text.replace(old, new, count))

replace_exact(
    "crates/runen-reference/tests/support/mod.rs",
    "    Program {\n        persistent: vec![],\n        types,\n        functions:",
    "    Program {\n        persistent: vec![],\n        external_callables: Vec::new(),\n        types,\n        functions:",
)

replace_exact(
    "crates/runen-reference/tests/floating_division.rs",
    "    let program = Program {\n        persistent: vec![],\n        types,\n        functions:",
    "    let program = Program {\n        persistent: vec![],\n        external_callables: Vec::new(),\n        types,\n        functions:",
    count=3,
)

replace_exact(
    "crates/runen-reference/tests/integer_equality.rs",
    "    let validated = validate_program(Program {\n        persistent: vec![],\n        types,\n        functions:",
    "    let validated = validate_program(Program {\n        persistent: vec![],\n        external_callables: Vec::new(),\n        types,\n        functions:",
    count=3,
)

replace_exact(
    "crates/runen-reference/tests/floating_addition.rs",
    "    let program = Program {\n        persistent: vec![],\n        types,\n        functions:",
    "    let program = Program {\n        persistent: vec![],\n        external_callables: Vec::new(),\n        types,\n        functions:",
    count=4,
)
replace_exact(
    "crates/runen-reference/tests/floating_addition.rs",
    "    let validated = validate_program(Program {\n        persistent: vec![],\n        types,\n        functions:",
    "    let validated = validate_program(Program {\n        persistent: vec![],\n        external_callables: Vec::new(),\n        types,\n        functions:",
)

print("staged compiler-proven #705 Program fixture migration batches 1-3")
