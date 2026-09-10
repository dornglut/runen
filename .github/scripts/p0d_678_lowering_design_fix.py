from pathlib import Path

path = Path('.github/scripts/p0d_678_lowering_patch.py')
text = path.read_text()

def once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    assert count == 1, f'{label}: expected one match, found {count}'
    text = text.replace(old, new, 1)

# Do not grow the public HIR API merely to enumerate lowering-internal type roots.
start = text.index('# Preserve opaque FunctionTypeId construction while allowing lowering to enumerate the canonical table.\n')
end = text.index('# Specialization discovery only follows direct calls.', start)
text = text[:start] + text[end:]

# Root callable mapping in actual source/HIR type uses, not every eager canonical declaration entry.
marker = '# Type-map capacity and canonical callable mapping.\n'
insert = marker + "s = once(s,\n'''        let reference_types = collect_used_safe_reference_types(compilation);\n        let raw_pointer_types = collect_used_raw_pointer_types(compilation);\n''',\n'''        let function_types = collect_used_function_types(compilation);\n        let reference_types = collect_used_safe_reference_types(compilation, &function_types);\n        let raw_pointer_types = collect_used_raw_pointer_types(compilation);\n''', 'used callable type roots')\n\n"
once(marker, insert, 'type-map root collection insertion')
once('.and_then(|count| count.checked_add(compilation.function_types.len()))',
     '.and_then(|count| count.checked_add(function_types.len()))',
     'callable capacity uses selected roots')
once('for (id, _) in compilation.function_type_entries() {',
     'for id in function_types {',
     'callable mapping uses selected roots')

# Make the helper insertion anchor specific to TypeMap rather than FunctionTypeMap.
old = "s = once(s, '    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\\n', insert + '    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\\n', 'function type lowering helpers')"
new = "get_anchor = '''    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\n        self.mapped\n'''\ns = once(s, get_anchor, insert + get_anchor, 'function type lowering helpers')"
once(old, new, 'TypeMap helper anchor')

# Replace the earlier all-table callable-reference scan with usage-root discovery plus nested closure.
ref_start = text.index('# Function-type safe references may only occur nested inside the canonical table; collect them up front.\n')
ref_end = text.index('# Statement call lowering uses the shared target relation.', ref_start)
new_block = r"""# Discover only function-value types actually present in source/HIR type positions,
# then close transitively over nested canonical callable components. This preserves the
# existing direct-call Core type table when the feature is unused.
collector_anchor = '''fn collect_used_safe_reference_types(
    compilation: &hir::TypedCompilation,
) -> BTreeSet<(hir::ReferenceReferent, hir::ReferencePermission)> {
    let mut references = BTreeSet::new();
'''
collector = '''fn collect_used_function_types(
    compilation: &hir::TypedCompilation,
) -> BTreeSet<hir::FunctionTypeId> {
    let mut function_types = BTreeSet::new();
    for function in &compilation.functions {
        for parameter in &function.parameters {
            if let hir::Type::Function(id) = parameter.ty {
                function_types.insert(id);
            }
        }
        if let Some(hir::Type::Function(id)) = function.result {
            function_types.insert(id);
        }
        collect_statement_function_types(&function.body.statements, &mut function_types);
    }

    let mut pending = function_types.iter().copied().collect::<Vec<_>>();
    let mut cursor = 0;
    while cursor < pending.len() {
        let id = pending[cursor];
        let interface = compilation.function_type(id);
        for ty in interface.parameters.iter().chain(interface.result.iter()) {
            if let hir::Type::Function(nested) = ty
                && function_types.insert(*nested)
            {
                pending.push(*nested);
            }
        }
        cursor += 1;
    }
    function_types
}

fn collect_statement_function_types(
    statements: &[hir::Statement],
    function_types: &mut BTreeSet<hir::FunctionTypeId>,
) {
    for statement in statements {
        match statement {
            hir::Statement::Local { ty, .. } => {
                if let hir::Type::Function(id) = ty {
                    function_types.insert(*id);
                }
            }
            hir::Statement::Block(block) => {
                collect_statement_function_types(&block.statements, function_types);
            }
            hir::Statement::If {
                then_block,
                else_block,
                ..
            } => {
                collect_statement_function_types(&then_block.statements, function_types);
                if let Some(else_block) = else_block {
                    collect_statement_function_types(&else_block.statements, function_types);
                }
            }
            hir::Statement::RefutableRecordSelection {
                success_block,
                mismatch_block,
                ..
            } => {
                collect_statement_function_types(&success_block.statements, function_types);
                if let Some(mismatch_block) = mismatch_block {
                    collect_statement_function_types(&mismatch_block.statements, function_types);
                }
            }
            hir::Statement::While { body, .. } => {
                collect_statement_function_types(&body.statements, function_types);
            }
            hir::Statement::RecordDestructure { .. }
            | hir::Statement::Assignment { .. }
            | hir::Statement::ReferenceAssign { .. }
            | hir::Statement::RawAssign { .. }
            | hir::Statement::Call { .. }
            | hir::Statement::Fault { .. }
            | hir::Statement::Break { .. }
            | hir::Statement::Continue { .. } => {}
        }
    }
}

fn collect_used_safe_reference_types(
    compilation: &hir::TypedCompilation,
    function_types: &BTreeSet<hir::FunctionTypeId>,
) -> BTreeSet<(hir::ReferenceReferent, hir::ReferencePermission)> {
    let mut references = BTreeSet::new();
    for id in function_types {
        let function_type = compilation.function_type(*id);
        for ty in function_type.parameters.iter().chain(function_type.result.iter()) {
            if let hir::Type::SafeReference {
                referent,
                permission,
            } = ty
            {
                references.insert((*referent, *permission));
            }
        }
    }
'''
s = once(s, collector_anchor, collector, 'used callable roots and callable safe-reference components')

"""
text = text[:ref_start] + new_block + text[ref_end:]

path.write_text(text)
print('rewrote #678 lowering patch to usage-driven callable mapping with no new HIR enumeration API')
