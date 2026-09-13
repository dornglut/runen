from pathlib import Path


def replace_exact(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))

# Compiler-named legacy Core fixtures have no external requirements.
replace_exact(
    'crates/runen-reference/tests/integer_ordering.rs',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        types,
''',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        external_callables: vec![],
        types,
''',
    4,
)
replace_exact(
    'crates/runen-reference/tests/integer_addition.rs',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        types,
''',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        external_callables: vec![],
        types,
''',
)

# Integer-OR malformed-HIR tests mutate only Runen-origin bodies.
p = Path('crates/runen-core-lowering/tests/integer_or.rs')
text = p.read_text()
old = 'use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};\n'
new = '''use runen_hir::{
    FunctionExecution, IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('integer_or import anchor mismatch')
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
    raise SystemExit('integer_or helper anchor mismatch')
text = text.replace(anchor, helper, 1)
old = '''    let value = non_integer.functions[0]
        .body
        .terminal_return
'''
new = '''    let value = runen_body_mut(&mut non_integer.functions[0])
        .terminal_return
'''
if text.count(old) != 1:
    raise SystemExit('integer_or non_integer body anchor mismatch')
text = text.replace(old, new, 1)
old = '''    let value = left_mismatch.functions[0]
        .body
        .terminal_return
'''
new = '''    let value = runen_body_mut(&mut left_mismatch.functions[0])
        .terminal_return
'''
if text.count(old) != 1:
    raise SystemExit('integer_or left body anchor mismatch')
text = text.replace(old, new, 1)
old = '''    let value = right_mismatch.functions[0]
        .body
        .terminal_return
'''
new = '''    let value = runen_body_mut(&mut right_mismatch.functions[0])
        .terminal_return
'''
if text.count(old) != 1:
    raise SystemExit('integer_or right body anchor mismatch')
text = text.replace(old, new, 1)
p.write_text(text)

print('staged compiler-proven #705 workspace integration batch 8')
