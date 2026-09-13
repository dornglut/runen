from pathlib import Path


def replace_exact(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))

# Terminator-kind regression helper remains exhaustive over represented Core control.
replace_exact(
    'crates/runen-core-lowering/tests/grouping.rs',
    '''            Terminator::Call { .. } => "call",
            Terminator::IndirectCall { .. } => "indirect-call",
            Terminator::Fault(_) => "fault",
''',
    '''            Terminator::Call { .. } => "call",
            Terminator::ExternalCall { .. } => "external-call",
            Terminator::IndirectCall { .. } => "indirect-call",
            Terminator::Fault(_) => "fault",
''',
)

# Compiler-named legacy Core fixture has no external requirements.
replace_exact(
    'crates/runen-reference/tests/integer_or.rs',
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

# Field-access HIR tests use execution-origin-aware read/mutation access.
p = Path('crates/runen-core-lowering/tests/field_access.rs')
text = p.read_text()
old = '''use runen_hir::{
    FieldValueReceiver, IntrinsicType, LiteralValue, ModuleId, OwnedUse, SourceUnit, Type,
    ValueKind, build_typed_hir,
};
'''
new = '''use runen_hir::{
    FieldValueReceiver, FunctionExecution, IntrinsicType, LiteralValue, ModuleId, OwnedUse,
    SourceUnit, Type, ValueKind, build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('field_access import anchor mismatch')
text = text.replace(old, new, 1)
helper_anchor = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted HIR must lower through canonical Core validation")
}

'''
helper = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted HIR must lower through canonical Core validation")
}

fn runen_body_mut(function: &mut runen_hir::Function) -> &mut runen_hir::Body {
    let FunctionExecution::Runen { body, .. } = &mut function.execution else {
        panic!("test mutation requires Runen execution origin");
    };
    body
}

'''
if text.count(helper_anchor) != 1:
    raise SystemExit('field_access helper anchor mismatch')
text = text.replace(helper_anchor, helper, 1)
old = '''    let returned = f_hir
        .body
        .terminal_return
'''
new = '''    let returned = f_hir
        .runen_body()
        .expect("f has Runen execution origin")
        .terminal_return
'''
if text.count(old) != 1:
    raise SystemExit('field_access read-only body anchor mismatch')
text = text.replace(old, new, 1)
replacements = [
    ('compilation', 0, 'value'),
    ('wrong_ownership', 1, 'wrong_ownership_value'),
    ('wrong_category', 1, 'wrong_category_value'),
    ('wrong_type', 1, 'wrong_type_value'),
    ('overlap', 1, 'overlap_value'),
    ('incomplete_consume', 1, 'consume_value'),
    ('incomplete_duplicate', 1, 'duplicate_value'),
]
for compilation, index, variable in replacements:
    old = f'''    let {variable} = {compilation}.functions[{index}]\n        .body\n        .terminal_return\n'''
    new = f'''    let {variable} = runen_body_mut(&mut {compilation}.functions[{index}])\n        .terminal_return\n'''
    if text.count(old) != 1:
        raise SystemExit(f'field_access mutable body anchor mismatch for {compilation}: {text.count(old)}')
    text = text.replace(old, new, 1)
p.write_text(text)

print('staged compiler-proven #705 workspace integration batch 6')
