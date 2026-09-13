from pathlib import Path
import re


# #705 explicitly authorizes mechanical migration of historical Program test fixtures.
# Work test-only and skip every literal that already declares external requirements,
# including Rust shorthand fields such as `external_callables,`.
def migrate_program_test_fixtures() -> int:
    migrated = 0
    for p in sorted(Path('crates').glob('*/tests/**/*.rs')):
        text = p.read_text()
        positions = [match.start() for match in re.finditer(r'\bProgram\s*\{', text)]
        if not positions:
            continue
        for start in reversed(positions):
            open_end = text.find('{', start) + 1
            suffix = text[open_end:]
            functions = re.search(r'\n(?P<indent>[ \t]*)functions\s*:', suffix)
            if functions is None:
                continue
            prefix_to_functions = suffix[:functions.start()]
            if re.search(r'\bexternal_callables\b', prefix_to_functions):
                continue
            first_field = re.match(r'\n(?P<indent>[ \t]+)', suffix)
            if first_field is None:
                raise SystemExit(f'{p}: Program literal does not use multiline test-fixture formatting')
            indent = first_field.group('indent')
            text = text[:open_end] + f'\n{indent}external_callables: vec![],' + text[open_end:]
            migrated += 1
        p.write_text(text)
    if migrated == 0:
        raise SystemExit('expected at least one historical Program test fixture to migrate')
    return migrated


migrated_programs = migrate_program_test_fixtures()


def migrate_read_only_hir_test(path: str) -> None:
    p = Path(path)
    text = p.read_text()

    # Indexed compilation function access is a Function place, so borrow it explicitly.
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
    if text.count(anchor) < 1:
        raise SystemExit(f'{path}: test insertion anchor missing')
    text = text.replace(anchor, helpers + anchor, 1)

    # Exhaust the selected legacy spellings in each explicitly compiler-selected inspection file.
    legacy_body = re.findall(r'\b[A-Za-z_][A-Za-z0-9_]*\s*\.body\b', text)
    legacy_parameters = re.findall(r'\b[A-Za-z_][A-Za-z0-9_]*\s*\.parameters\b', text)
    legacy_indexed_body = re.findall(
        r'\b[A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\]\s*\.body\b', text
    )
    legacy_indexed_parameters = re.findall(
        r'\b[A-Za-z_][A-Za-z0-9_]*\.functions\[[^\]\n]+\]\s*\.parameters\b', text
    )
    if legacy_body or legacy_parameters or legacy_indexed_body or legacy_indexed_parameters:
        raise SystemExit(
            f'{path}: legacy Function fields remain: '
            f'body={legacy_body + legacy_indexed_body} '
            f'parameters={legacy_parameters + legacy_indexed_parameters}'
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
    'crates/runen-hir/tests/assignment.rs',
    'crates/runen-hir/tests/loops.rs',
    'crates/runen-hir/tests/function_values.rs',
    'crates/runen-hir/tests/integer_addition.rs',
    'crates/runen-hir/tests/record_destructuring_regressions.rs',
    'crates/runen-hir/tests/integer_multiplication.rs',
    'crates/runen-hir/tests/floating_division.rs',
    'crates/runen-hir/tests/literals.rs',
    'crates/runen-hir/tests/statics.rs',
    'crates/runen-hir/tests/closures.rs',
    'crates/runen-hir/tests/floating_subtraction.rs',
    'crates/runen-hir/tests/integer_xor.rs',
    'crates/runen-hir/tests/marker_traits.rs',
    'crates/runen-hir/tests/faults.rs',
    'crates/runen-hir/tests/integer_equality.rs',
    'crates/runen-hir/tests/integer_negation.rs',
    'crates/runen-hir/tests/floating_multiplication.rs',
    'crates/runen-hir/tests/boolean_conjunction.rs',
    'crates/runen-hir/tests/conditionals.rs',
    'crates/runen-hir/tests/refutable_record_selection.rs',
]:
    migrate_read_only_hir_test(path)

print(f'staged #705 batch 10: {migrated_programs} historical Program fixtures plus compiler-selected read-only HIR tests')
