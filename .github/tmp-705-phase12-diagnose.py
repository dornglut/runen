from pathlib import Path
import re


def replace_exact(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))


# Compiler-named Core test support has no external requirements.
replace_exact(
    'crates/runen-core-ir/tests/support/mod.rs',
    '''    Program {\n        persistent: vec![],\n        types,\n        functions: vec![Function {\n''',
    '''    Program {\n        persistent: vec![],\n        external_callables: vec![],\n        types,\n        functions: vec![Function {\n''',
)

# The issue explicitly authorizes mechanical empty external-declaration fixture migration.
for path, expected in [
    ('crates/runen-core-ir/tests/interprocedural.rs', 34),
    ('crates/runen-reference/tests/floating_multiplication.rs', 3),
    ('crates/runen-reference/tests/integer_multiplication.rs', 1),
    ('crates/runen-reference/tests/floating_constants.rs', 5),
]:
    p = Path(path)
    text = p.read_text()
    count = text.count('Program {')
    if count != expected:
        raise SystemExit(f'{path}: expected {expected} Program fixtures, found {count}')
    p.write_text(text.replace('Program {', 'Program {\n        external_callables: vec![],', expected))


def migrate_read_only_hir_test(path: str) -> None:
    p = Path(path)
    text = p.read_text()

    # Calls through the common test helper need wrapping before simple aliases.
    text, call_body_count = re.subn(
        r'function\(([^)\n]+)\)\s*\.body\b',
        r'runen_body(function(\1))',
        text,
    )
    text, call_parameter_count = re.subn(
        r'function\(([^)\n]+)\)\s*\.parameters\b',
        r'runen_parameters(function(\1))',
        text,
    )

    # Direct local aliases in these compiler-named HIR inspection tests are Function references.
    # Whitespace is admitted because rustfmt commonly places `.body` on the next line.
    text, body_count = re.subn(
        r'\b([A-Za-z_][A-Za-z0-9_]*)\s*\.body\b',
        r'runen_body(\1)',
        text,
    )
    text, parameter_count = re.subn(
        r'\b([A-Za-z_][A-Za-z0-9_]*)\s*\.parameters\b',
        r'runen_parameters(\1)',
        text,
    )

    needs_body = call_body_count + body_count > 0
    needs_parameters = call_parameter_count + parameter_count > 0
    if not needs_body and not needs_parameters:
        raise SystemExit(f'{path}: no compiler-named Function body/parameter accesses found')

    helpers = ''
    if needs_body:
        helpers += '''fn runen_body(function: &runen_hir::Function) -> &runen_hir::Body {\n    function\n        .runen_body()\n        .expect("test function has Runen execution origin")\n}\n\n'''
    if needs_parameters:
        helpers += '''fn runen_parameters(function: &runen_hir::Function) -> &[runen_hir::Parameter] {\n    function\n        .runen_parameters()\n        .expect("test function has Runen execution origin")\n}\n\n'''

    anchor = '#[test]\n'
    if text.count(anchor) < 1:
        raise SystemExit(f'{path}: test insertion anchor missing')
    text = text.replace(anchor, helpers + anchor, 1)

    # Exhaust the simple/direct legacy spellings in each explicitly selected inspection file.
    legacy_body = re.findall(r'\b[A-Za-z_][A-Za-z0-9_]*\s*\.body\b', text)
    legacy_parameters = re.findall(r'\b[A-Za-z_][A-Za-z0-9_]*\s*\.parameters\b', text)
    if legacy_body or legacy_parameters:
        raise SystemExit(
            f'{path}: legacy direct Function fields remain: body={legacy_body} parameters={legacy_parameters}'
        )
    p.write_text(text)


for path in [
    'crates/runen-hir/tests/references.rs',
    'crates/runen-hir/tests/field_access.rs',
    'crates/runen-hir/tests/integer_subtraction.rs',
    'crates/runen-hir/tests/integer_complement.rs',
    'crates/runen-hir/tests/record_destructuring.rs',
    'crates/runen-hir/tests/integer_or.rs',
    'crates/runen-hir/tests/operators.rs',
    'crates/runen-hir/tests/grouping.rs',
    'crates/runen-hir/tests/module_order.rs',
    'crates/runen-hir/tests/record_destructuring_zero_field.rs',
    'crates/runen-hir/tests/record_duplicability.rs',
    'crates/runen-hir/tests/exclusive_references.rs',
    'crates/runen-hir/tests/function_value_edges.rs',
    'crates/runen-hir/tests/generics.rs',
]:
    migrate_read_only_hir_test(path)

print('staged compiler-proven #705 fixture and read-only HIR integration batch 10')
