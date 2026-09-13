from pathlib import Path
import re


BODY_HELPER = '''fn runen_body_mut(function: &mut runen_hir::Function) -> &mut runen_hir::Body {\n    let runen_hir::FunctionExecution::Runen { body, .. } = &mut function.execution else {\n        panic!("test mutation requires Runen execution origin");\n    };\n    body\n}\n\n'''
PARAM_HELPER = '''fn runen_parameters_mut(\n    function: &mut runen_hir::Function,\n) -> &mut [runen_hir::Parameter] {\n    let runen_hir::FunctionExecution::Runen { parameters, .. } = &mut function.execution else {\n        panic!("test mutation requires Runen execution origin");\n    };\n    parameters\n}\n\n'''


def ensure_helpers(path: str, need_parameters: bool = False) -> None:
    p = Path(path)
    text = p.read_text()
    helpers = ''
    if 'fn runen_body_mut(' not in text:
        helpers += BODY_HELPER
    if need_parameters and 'fn runen_parameters_mut(' not in text:
        helpers += PARAM_HELPER
    if helpers:
        anchor = '#[test]\n'
        if anchor not in text:
            raise SystemExit(f'{path}: missing test helper insertion anchor')
        text = text.replace(anchor, helpers + anchor, 1)
        p.write_text(text)


def migrate_indexed_hir_mutations(path: str, need_parameters: bool = False) -> None:
    ensure_helpers(path, need_parameters)
    p = Path(path)
    text = p.read_text()

    # Preserve an existing mutable borrow of a selected HIR body while changing its owner shape.
    text = re.sub(
        r'&mut\s+([A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\])\s*\.body\b',
        r'&mut runen_body_mut(&mut \1)',
        text,
    )
    text = re.sub(
        r'\b([A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\])\s*\.body\b',
        r'runen_body_mut(&mut \1)',
        text,
    )
    if need_parameters:
        text = re.sub(
            r'\b([A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\])\s*\.parameters\b',
            r'runen_parameters_mut(&mut \1)',
            text,
        )

    if re.search(r'\b[A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\]\s*\.body\b', text):
        raise SystemExit(f'{path}: indexed legacy HIR body access remains')
    if need_parameters and re.search(
        r'\b[A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\]\s*\.parameters\b', text
    ):
        raise SystemExit(f'{path}: indexed legacy HIR parameter access remains')
    p.write_text(text)


for path in [
    'crates/runen-core-lowering/tests/integer_multiplication.rs',
    'crates/runen-core-lowering/tests/integer_xor.rs',
    'crates/runen-core-lowering/tests/references.rs',
    'crates/runen-core-lowering/tests/integer_subtraction.rs',
    'crates/runen-core-lowering/tests/floating_addition.rs',
    'crates/runen-core-lowering/tests/floating_subtraction.rs',
    'crates/runen-core-lowering/tests/floating_multiplication.rs',
]:
    migrate_indexed_hir_mutations(path)

for path in [
    'crates/runen-core-lowering/tests/integer_negation.rs',
    'crates/runen-core-lowering/tests/integer_complement.rs',
]:
    migrate_indexed_hir_mutations(path, need_parameters=True)

# Iterator-selected mutable HIR Functions need the same explicit execution-origin match.
for path in [
    'crates/runen-core-lowering/tests/integer_equality.rs',
    'crates/runen-core-lowering/tests/integer_ordering.rs',
]:
    ensure_helpers(path)
    p = Path(path)
    text = p.read_text()
    old = '|function| function.body.terminal_return.as_mut()'
    new = '|function| runen_body_mut(function).terminal_return.as_mut()'
    if old not in text:
        raise SystemExit(f'{path}: expected mutable returned-value HIR anchor')
    p.write_text(text.replace(old, new, 1))

# Conditional malformed-HIR defenses use a local &mut Function named `f`; Core `f.body`
# accesses elsewhere in the file are intentionally not touched.
path = 'crates/runen-core-lowering/tests/conditionals.rs'
ensure_helpers(path)
p = Path(path)
text = p.read_text()
for old, new, expected in [
    ('&mut f.body.statements[0]', '&mut runen_body_mut(f).statements[0]', 2),
    ('f.body.has_normal_continuation = true;', 'runen_body_mut(f).has_normal_continuation = true;', 1),
]:
    found = text.count(old)
    if found != expected:
        raise SystemExit(f'{path}: expected {expected} anchors for {old!r}, found {found}')
    text = text.replace(old, new, expected)
p.write_text(text)

# Loop malformed-HIR defenses likewise use a local &mut Function named `f` only in these forms.
path = 'crates/runen-core-lowering/tests/loops.rs'
ensure_helpers(path)
p = Path(path)
text = p.read_text()
for old, new, expected in [
    ('&f.body.statements[0]', '&runen_body_mut(f).statements[0]', 1),
    ('f.body.statements = vec![transfer];', 'runen_body_mut(f).statements = vec![transfer];', 1),
    ('f.body.terminal_return = None;', 'runen_body_mut(f).terminal_return = None;', 1),
    ('f.body.has_normal_continuation = false;', 'runen_body_mut(f).has_normal_continuation = false;', 1),
    ('&mut f.body.statements[0]', '&mut runen_body_mut(f).statements[0]', 1),
]:
    found = text.count(old)
    if found != expected:
        raise SystemExit(f'{path}: expected {expected} anchors for {old!r}, found {found}')
    text = text.replace(old, new, expected)
p.write_text(text)

print('staged compiler-selected #705 mutable HIR lowering-test migration batch 11')
