from pathlib import Path


def replace_exact(path: str, old: str, new: str, count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))

# Two compiler-named legacy Core fixtures have no external requirements.
replace_exact(
    'crates/runen-reference/tests/derived_reference_results.rs',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        types,
''',
    '''    let validated = validate_program(Program {
        persistent: vec![],
        external_callables: vec![],
        types,
''',
    2,
)

# Malformed-HIR interface test mutates only the Runen parameter category.
p = Path('crates/runen-core-lowering/tests/raw_pointers_unsafe.rs')
text = p.read_text()
old = '''use runen_hir::{
    IntrinsicType, ModuleId, RawPointerPointee, SourceUnit, Type, TypedCompilation, build_typed_hir,
};
'''
new = '''use runen_hir::{
    FunctionExecution, IntrinsicType, ModuleId, RawPointerPointee, SourceUnit, Type,
    TypedCompilation, build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('raw_pointers import anchor mismatch')
text = text.replace(old, new, 1)
helper_anchor = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted raw-pointer HIR must lower to validated Core")
}

'''
helper = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted raw-pointer HIR must lower to validated Core")
}

fn runen_parameters_mut(function: &mut runen_hir::Function) -> &mut [runen_hir::Parameter] {
    let FunctionExecution::Runen { parameters, .. } = &mut function.execution else {
        panic!("test mutation requires Runen execution origin");
    };
    parameters
}

'''
if text.count(helper_anchor) != 1:
    raise SystemExit('raw_pointers helper anchor mismatch')
text = text.replace(helper_anchor, helper, 1)
old = '    parameter.functions[0].parameters[0].ty = raw_i64;\n'
new = '    runen_parameters_mut(&mut parameter.functions[0])[0].ty = raw_i64;\n'
if text.count(old) != 1:
    raise SystemExit('raw_pointers parameter mutation anchor mismatch')
text = text.replace(old, new, 1)
p.write_text(text)

# Record malformed-HIR tests all mutate statements of a Runen-origin body.
p = Path('crates/runen-core-lowering/tests/record_destructuring.rs')
text = p.read_text()
old = '''use runen_hir::{
    ModuleId, RecordPatternScrutinee, RecordPatternTransientCleanup, SourceUnit, Statement, Type,
    build_typed_hir,
};
'''
new = '''use runen_hir::{
    FunctionExecution, ModuleId, RecordPatternScrutinee, RecordPatternTransientCleanup, SourceUnit,
    Statement, Type, build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('record_destructuring import anchor mismatch')
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
    raise SystemExit('record_destructuring helper anchor mismatch')
text = text.replace(helper_anchor, helper, 1)
old = '&mut compilation.functions[0].body.statements[0]'
new = '&mut runen_body_mut(&mut compilation.functions[0]).statements[0]'
if text.count(old) != 9:
    raise SystemExit(f'record_destructuring body mutation anchors mismatch: {text.count(old)}')
text = text.replace(old, new, 9)
p.write_text(text)

print('staged compiler-proven #705 workspace integration batch 5')
