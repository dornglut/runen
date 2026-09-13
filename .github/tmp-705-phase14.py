from pathlib import Path
import re


def remove_unused_closure_parameter_helper() -> None:
    path = Path('crates/runen-hir/tests/closures.rs')
    text = path.read_text()
    bad = 'runen_parameters(site)'
    if text.count(bad) != 1:
        raise SystemExit(f'closures.rs: expected one Closure.parameters false-positive, found {text.count(bad)}')
    text = text.replace(bad, 'site.parameters', 1)
    helper = '''fn runen_parameters(function: &runen_hir::Function) -> &[runen_hir::Parameter] {\n    function\n        .runen_parameters()\n        .expect("test function has Runen execution origin")\n}\n\n'''
    if text.count('runen_parameters(') == 1:
        if text.count(helper) != 1:
            raise SystemExit('closures.rs: unused Function parameter helper shape changed')
        text = text.replace(helper, '', 1)
    path.write_text(text)


remove_unused_closure_parameter_helper()


def migrate_read_only_hir_test(path: str) -> None:
    p = Path(path)
    text = p.read_text()

    text, indexed_body_count = re.subn(
        r'\b([A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\])\s*\.body\b',
        r'runen_body(&\1)',
        text,
    )
    text, indexed_parameter_count = re.subn(
        r'\b([A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\])\s*\.parameters\b',
        r'runen_parameters(&\1)',
        text,
    )
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

    needs_body = indexed_body_count + call_body_count + body_count > 0
    needs_parameters = indexed_parameter_count + call_parameter_count + parameter_count > 0
    if not needs_body and not needs_parameters:
        raise SystemExit(f'{path}: no compiler-named Function body/parameter accesses found')

    helpers = ''
    if needs_body:
        helpers += '''fn runen_body(function: &runen_hir::Function) -> &runen_hir::Body {\n    function\n        .runen_body()\n        .expect("test function has Runen execution origin")\n}\n\n'''
    if needs_parameters:
        helpers += '''fn runen_parameters(function: &runen_hir::Function) -> &[runen_hir::Parameter] {\n    function\n        .runen_parameters()\n        .expect("test function has Runen execution origin")\n}\n\n'''

    anchor = '#[test]\n'
    if anchor not in text:
        raise SystemExit(f'{path}: missing test helper insertion anchor')
    text = text.replace(anchor, helpers + anchor, 1)

    if re.search(r'\b[A-Za-z_][A-Za-z0-9_]*\s*\.body\b', text):
        raise SystemExit(f'{path}: direct legacy Function.body access remains')
    if re.search(r'\b[A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\]\s*\.body\b', text):
        raise SystemExit(f'{path}: indexed legacy Function.body access remains')
    if re.search(r'\b[A-Za-z_][A-Za-z0-9_]*\s*\.parameters\b', text):
        raise SystemExit(f'{path}: direct legacy Function.parameters access remains')
    if re.search(r'\b[A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\]\s*\.parameters\b', text):
        raise SystemExit(f'{path}: indexed legacy Function.parameters access remains')

    p.write_text(text)


for path in [
    'crates/runen-hir/tests/raw_pointers_unsafe.rs',
    'crates/runen-hir/tests/floating_addition.rs',
    'crates/runen-hir/tests/hir.rs',
    'crates/runen-hir/tests/record_construction.rs',
    'crates/runen-hir/tests/record_pattern_rest.rs',
    'crates/runen-hir/tests/constants.rs',
    'crates/runen-hir/tests/integer_ordering.rs',
    'crates/runen-hir/tests/blocks.rs',
]:
    migrate_read_only_hir_test(path)

print('staged compiler-selected #705 final read-only HIR integration batch')
