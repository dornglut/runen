from pathlib import Path


def migrate_body_test(path: str, operation_name: str, labels: list[str]) -> None:
    p = Path(path)
    text = p.read_text()
    old = 'use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};\n'
    new = '''use runen_hir::{
    FunctionExecution, IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir,
};
'''
    if text.count(old) != 1:
        raise SystemExit(f'{operation_name} import anchor mismatch')
    text = text.replace(old, new, 1)
    anchor = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted HIR must lower to validated Core")
}

'''
    helper = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted HIR must lower to validated Core")
}

fn runen_body_mut(function: &mut runen_hir::Function) -> &mut runen_hir::Body {
    let FunctionExecution::Runen { body, .. } = &mut function.execution else {
        panic!("test mutation requires Runen execution origin");
    };
    body
}

'''
    if text.count(anchor) != 1:
        raise SystemExit(f'{operation_name} helper anchor mismatch')
    text = text.replace(anchor, helper, 1)
    for label in labels:
        old = f'''    let value = {label}.functions[0]\n        .body\n        .terminal_return\n'''
        new = f'''    let value = runen_body_mut(&mut {label}.functions[0])\n        .terminal_return\n'''
        if text.count(old) != 1:
            raise SystemExit(f'{operation_name} {label} body anchor mismatch: {text.count(old)}')
        text = text.replace(old, new, 1)
    p.write_text(text)

migrate_body_test(
    'crates/runen-core-lowering/tests/integer_addition.rs',
    'integer_addition',
    ['compilation', 'left_mismatch', 'right_mismatch'],
)
migrate_body_test(
    'crates/runen-core-lowering/tests/boolean_conjunction.rs',
    'boolean_conjunction',
    ['result_mismatch', 'left_mismatch', 'right_mismatch'],
)

print('staged compiler-proven #705 workspace integration batch 9')
