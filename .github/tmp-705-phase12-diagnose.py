from pathlib import Path

for path in [
    'crates/runen-core-lowering/tests/integer_addition.rs',
    'crates/runen-core-lowering/tests/boolean_conjunction.rs',
]:
    text = Path(path).read_text()
    import_anchor = 'use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};\n'
    helper_anchor = '''fn lower_source(source: &str) -> ValidatedProgram {\n    lower(&hir(source)).expect("accepted HIR must lower to validated Core")\n}\n\n'''
    counts = [
        f'import={text.count(import_anchor)}',
        f'helper={text.count(helper_anchor)}',
    ]
    for label in ['compilation', 'left_mismatch', 'right_mismatch', 'result_mismatch']:
        body_anchor = f'''    let value = {label}.functions[0]\n        .body\n        .terminal_return\n'''
        count = text.count(body_anchor)
        if count:
            counts.append(f'{label}={count}')
    direct_body_lines = sum('.body' in line for line in text.splitlines())
    direct_parameter_lines = sum('.parameters' in line for line in text.splitlines())
    counts.append(f'body_lines={direct_body_lines}')
    counts.append(f'parameter_lines={direct_parameter_lines}')
    print(f'::notice file={path},title=#705 phase11 anchors::{" ".join(counts)}')
