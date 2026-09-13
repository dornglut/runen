from pathlib import Path


def replace_exact(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))

# Compiler-named historical Core fixture: no external requirements in this test.
replace_exact(
    'crates/runen-reference/tests/interprocedural_entry_result.rs',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        types,
        functions: vec![entry],
''',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        external_callables: vec![],
        types,
        functions: vec![entry],
''',
)

# Test-only malformed-HIR mutation keeps execution-origin discrimination explicit.
p = Path('crates/runen-core-lowering/tests/operators.rs')
text = p.read_text()
old = 'use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};\n'
new = 'use runen_hir::{\n    FunctionExecution, IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir,\n};\n'
if text.count(old) != 1:
    raise SystemExit('operators import anchor mismatch')
text = text.replace(old, new, 1)
helper_anchor = '''fn lower_source(source: &str) -> ValidatedProgram {
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
if text.count(helper_anchor) != 1:
    raise SystemExit('operators helper anchor mismatch')
text = text.replace(helper_anchor, helper, 1)
old_chain = '''    let value = f
        .body
        .terminal_return
'''
new_chain = '''    let value = runen_body_mut(f)
        .terminal_return
'''
if text.count(old_chain) != 5:
    raise SystemExit(f'operators mutable body anchors mismatch: {text.count(old_chain)}')
text = text.replace(old_chain, new_chain, 5)
p.write_text(text)

p = Path('crates/runen-core-lowering/tests/floating_division.rs')
text = p.read_text()
old = '''use runen_hir::{
    ModuleId, NumericContract as HirNumericContract, SourceUnit, ValueKind, build_typed_hir,
};
'''
new = '''use runen_hir::{
    FunctionExecution, ModuleId, NumericContract as HirNumericContract, SourceUnit, ValueKind,
    build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('floating_division import anchor mismatch')
text = text.replace(old, new, 1)
helper_anchor = '''fn lower_source(source: &str) -> ValidatedProgram {
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
if text.count(helper_anchor) != 1:
    raise SystemExit('floating_division helper anchor mismatch')
text = text.replace(helper_anchor, helper, 1)
old_chain = '''    let value = reproducible.functions[0]
        .body
        .terminal_return
'''
new_chain = '''    let value = runen_body_mut(&mut reproducible.functions[0])
        .terminal_return
'''
if text.count(old_chain) != 1:
    raise SystemExit('floating_division mutable body anchor mismatch')
text = text.replace(old_chain, new_chain, 1)
p.write_text(text)

print('staged compiler-proven #705 workspace integration batch 4')
