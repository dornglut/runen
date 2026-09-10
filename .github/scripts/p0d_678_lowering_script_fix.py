from pathlib import Path

# Temporary guarded rewrite used only to narrow ambiguous anchors in the lowering patch.
path = Path('.github/scripts/p0d_678_lowering_patch.py')
text = path.read_text()

old = "s = once(s, '    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\\n', insert + '    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\\n', 'function type lowering helpers')"
new = "get_anchor = '''    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\n        self.mapped\n'''\ns = once(s, get_anchor, insert + get_anchor, 'function type lowering helpers')"
assert text.count(old) == 1, f'get anchor script match: {text.count(old)}'
text = text.replace(old, new, 1)

old = "s = once(s,\n'''    for function in &compilation.functions {\n''',\n'''    for function_type in &compilation.function_types {"
new = "s = once(s,\n'''fn collect_used_safe_reference_types(\n    compilation: &hir::TypedCompilation,\n) -> BTreeSet<(hir::ReferenceReferent, hir::ReferencePermission)> {\n    let mut references = BTreeSet::new();\n    for function in &compilation.functions {\n''',\n'''fn collect_used_safe_reference_types(\n    compilation: &hir::TypedCompilation,\n) -> BTreeSet<(hir::ReferenceReferent, hir::ReferencePermission)> {\n    let mut references = BTreeSet::new();\n    for function_type in &compilation.function_types {"
assert text.count(old) == 1, f'reference collector script match: {text.count(old)}'
text = text.replace(old, new, 1)

path.write_text(text)
print('narrowed lowering patch anchors to TypeMap and the safe-reference collector')
