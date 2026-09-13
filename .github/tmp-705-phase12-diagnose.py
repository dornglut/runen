from pathlib import Path

for path in [
    'crates/runen-core-lowering/tests/integer_addition.rs',
    'crates/runen-core-lowering/tests/boolean_conjunction.rs',
]:
    text = Path(path).read_text()
    print(f'=== {path} ===')
    import_anchor = 'use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};\n'
    helper_anchor = '''fn lower_source(source: &str) -> ValidatedProgram {\n    lower(&hir(source)).expect("accepted HIR must lower to validated Core")\n}\n\n'''
    print('import_anchor_count=', text.count(import_anchor))
    print('helper_anchor_count=', text.count(helper_anchor))
    for label in ['compilation', 'left_mismatch', 'right_mismatch', 'result_mismatch']:
        body_anchor = f'''    let value = {label}.functions[0]\n        .body\n        .terminal_return\n'''
        count = text.count(body_anchor)
        if count:
            print(f'{label}_body_anchor_count=', count)
    for index, line in enumerate(text.splitlines(), 1):
        if 'use runen_hir' in line or '.body' in line or '.parameters' in line:
            print(f'{index}: {line}')
