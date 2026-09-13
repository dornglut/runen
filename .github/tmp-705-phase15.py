from pathlib import Path

# The closure migration briefly needed this helper, but Closure itself owns `parameters`.
path = Path('crates/runen-hir/tests/closures.rs')
text = path.read_text()
helper = '''fn runen_parameters(function: &runen_hir::Function) -> &[runen_hir::Parameter] {\n    function\n        .runen_parameters()\n        .expect("test function has Runen execution origin")\n}\n\n'''
if text.count(helper) != 1:
    raise SystemExit(f'closures.rs: expected one stale runen_parameters helper, found {text.count(helper)}')
if 'runen_parameters(site)' in text:
    raise SystemExit('closures.rs: Closure.parameters false-positive was not restored')
path.write_text(text.replace(helper, '', 1))

# The first I64 type in this rejection test is shadowed before it is needed.
path = Path('crates/runen-core-ir/tests/external_callables.rs')
text = path.read_text()
anchor = '''fn external_declarations_reject_non_scalar_and_reference_contract_interfaces() {\n    let mut types = TypeTable::new();\n    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));\n    let record_ty = types.push(TypeDef::structure("Record", vec![]));\n'''
replacement = '''fn external_declarations_reject_non_scalar_and_reference_contract_interfaces() {\n    let mut types = TypeTable::new();\n    let record_ty = types.push(TypeDef::structure("Record", vec![]));\n'''
if text.count(anchor) != 1:
    raise SystemExit('Core external-callable unused fixture anchor mismatch')
path.write_text(text.replace(anchor, replacement, 1))

print('staged #705 warning-free test hygiene')
