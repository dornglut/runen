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
    '''    Program {\n        types,\n        persistent: Vec::new(),\n        functions,\n    }\n''',
    '''    Program {\n        types,\n        persistent: Vec::new(),\n        external_callables: Vec::new(),\n        functions,\n    }\n''',
)


def migrate_read_only_hir_test(path: str) -> None:
    p = Path(path)
    text = p.read_text()

    # Calls through the common test helper need wrapping before the simple identifier cases.
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
    text, body_count = re.subn(
        r'\b([A-Za-z_][A-Za-z0-9_]*)\.body\b',
        r'runen_body(\1)',
        text,
    )
    text, parameter_count = re.subn(
        r'\b([A-Za-z_][A-Za-z0-9_]*)\.parameters\b',
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

    # This batch is intentionally exhaustive for direct legacy Function fields in each named file.
    legacy_body = re.findall(r'\b[A-Za-z_][A-Za-z0-9_]*\.body\b', text)
    legacy_parameters = re.findall(r'\b[A-Za-z_][A-Za-z0-9_]*\.parameters\b', text)
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
]:
    migrate_read_only_hir_test(path)

print('staged compiler-proven #705 read-only HIR integration batch 10')
