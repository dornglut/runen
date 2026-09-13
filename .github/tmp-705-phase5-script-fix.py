from pathlib import Path

p = Path('.github/tmp-705-phase5.py')
text = p.read_text()
old = '''# Shared body-lowering context gains the external identity map.
old = \'\'\'    persistent: &\'a BTreeMap<hir::StaticId, core::PersistentId>,
    functions: &\'a BTreeMap<SpecializationKey, core::FunctionId>,
\'\'\'
new = \'\'\'    persistent: &\'a BTreeMap<hir::StaticId, core::PersistentId>,
    external_callables: &\'a BTreeMap<hir::FunctionId, core::ExternalCallableId>,
    functions: &\'a BTreeMap<SpecializationKey, core::FunctionId>,
\'\'\'
if text.count(old) != 1:
    raise SystemExit(\'FunctionLoweringContext external map anchor mismatch\')
text = text.replace(old, new, 1)
'''
new = '''# Shared body-lowering context gains the external identity map.
old = \'\'\'struct FunctionLoweringContext<\'a> {
    compilation: &\'a hir::TypedCompilation,
    types: &\'a TypeMap,
    persistent: &\'a BTreeMap<hir::StaticId, core::PersistentId>,
    functions: &\'a BTreeMap<SpecializationKey, core::FunctionId>,
\'\'\'
new = \'\'\'struct FunctionLoweringContext<\'a> {
    compilation: &\'a hir::TypedCompilation,
    types: &\'a TypeMap,
    persistent: &\'a BTreeMap<hir::StaticId, core::PersistentId>,
    external_callables: &\'a BTreeMap<hir::FunctionId, core::ExternalCallableId>,
    functions: &\'a BTreeMap<SpecializationKey, core::FunctionId>,
\'\'\'
if text.count(old) != 1:
    raise SystemExit(\'FunctionLoweringContext external map anchor mismatch\')
text = text.replace(old, new, 1)
'''
if text.count(old) != 1:
    raise SystemExit(f'phase5 context-transform script anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('narrowed phase5 FunctionLoweringContext transform to its struct declaration')
