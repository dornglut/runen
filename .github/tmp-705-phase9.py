from pathlib import Path

# Generic malformed-HIR tests mutate only Runen-origin parameter/body state.
p = Path('crates/runen-core-lowering/tests/generics.rs')
text = p.read_text()
old = '''use runen_hir::{
    CallTarget, IntrinsicType, ModuleId, ReferencePermission, ReferenceReferent, SourceUnit, Type,
    ValueKind, build_typed_hir,
};
'''
new = '''use runen_hir::{
    CallTarget, FunctionExecution, IntrinsicType, ModuleId, ReferencePermission, ReferenceReferent,
    SourceUnit, Type, ValueKind, build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('generics import anchor mismatch')
text = text.replace(old, new, 1)
anchor = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted generic HIR must lower to validated concrete Core")
}

'''
helper = '''fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted generic HIR must lower to validated concrete Core")
}

fn runen_body_mut(function: &mut runen_hir::Function) -> &mut runen_hir::Body {
    let FunctionExecution::Runen { body, .. } = &mut function.execution else {
        panic!("test mutation requires Runen execution origin");
    };
    body
}

fn runen_parameters_mut(function: &mut runen_hir::Function) -> &mut [runen_hir::Parameter] {
    let FunctionExecution::Runen { parameters, .. } = &mut function.execution else {
        panic!("test mutation requires Runen execution origin");
    };
    parameters
}

'''
if text.count(anchor) != 1:
    raise SystemExit('generics helper anchor mismatch')
text = text.replace(anchor, helper, 1)
old = '''    let call = root
        .body
        .terminal_return
'''
new = '''    let call = runen_body_mut(root)
        .terminal_return
'''
if text.count(old) != 2:
    raise SystemExit(f'generics body mutation anchor mismatch: {text.count(old)}')
text = text.replace(old, new, 2)
old = '''    unresolved_abstract
        .functions
        .iter_mut()
        .find(|function| function.name == "root")
        .expect("root exists")
        .parameters[0]
        .ty = Type::Parameter(slot);
'''
new = '''    let root = unresolved_abstract
        .functions
        .iter_mut()
        .find(|function| function.name == "root")
        .expect("root exists");
    runen_parameters_mut(root)[0].ty = Type::Parameter(slot);
'''
if text.count(old) != 1:
    raise SystemExit('generics parameter mutation anchor mismatch')
text = text.replace(old, new, 1)
p.write_text(text)

# Refutable-selection helpers/mutations preserve explicit Runen execution origin.
p = Path('crates/runen-core-lowering/tests/refutable_record_selection.rs')
text = p.read_text()
old = '''use runen_hir::{
    IntrinsicType, LiteralValue, ModuleId, RecordPatternTestKind, RecordPatternTransientCleanup,
    SourceUnit, Statement, Type, build_typed_hir,
};
'''
new = '''use runen_hir::{
    FunctionExecution, IntrinsicType, LiteralValue, ModuleId, RecordPatternTestKind,
    RecordPatternTransientCleanup, SourceUnit, Statement, Type, build_typed_hir,
};
'''
if text.count(old) != 1:
    raise SystemExit('refutable selection import anchor mismatch')
text = text.replace(old, new, 1)
old = '''fn selection_mut<'a>(
    compilation: &'a mut runen_hir::TypedCompilation,
    function_name: &str,
) -> &'a mut Statement {
    compilation
        .functions
        .iter_mut()
        .find(|function| function.name == function_name)
        .and_then(|function| function.body.statements.first_mut())
        .unwrap_or_else(|| panic!("missing selection statement for {function_name}"))
}
'''
new = '''fn runen_body_mut(function: &mut runen_hir::Function) -> &mut runen_hir::Body {
    let FunctionExecution::Runen { body, .. } = &mut function.execution else {
        panic!("test mutation requires Runen execution origin");
    };
    body
}

fn selection_mut<'a>(
    compilation: &'a mut runen_hir::TypedCompilation,
    function_name: &str,
) -> &'a mut Statement {
    let function = compilation
        .functions
        .iter_mut()
        .find(|function| function.name == function_name)
        .unwrap_or_else(|| panic!("missing function {function_name}"));
    runen_body_mut(function)
        .statements
        .first_mut()
        .unwrap_or_else(|| panic!("missing selection statement for {function_name}"))
}
'''
if text.count(old) != 1:
    raise SystemExit('refutable selection helper anchor mismatch')
text = text.replace(old, new, 1)
old = '&mut compilation.functions[0].body.statements[0]'
new = '&mut runen_body_mut(&mut compilation.functions[0]).statements[0]'
if text.count(old) != 1:
    raise SystemExit(f'refutable destructuring mutation anchor mismatch: {text.count(old)}')
text = text.replace(old, new, 1)
p.write_text(text)

print('staged compiler-proven #705 workspace integration batch 7')
