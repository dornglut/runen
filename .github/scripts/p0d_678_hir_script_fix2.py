from pathlib import Path

path = Path('.github/scripts/p0d_678_hir_patch.py')
text = path.read_text()
old = '''old_local = \"\"\"    let declared = resolve_type(\\n        header.module,\\n        header.unit,\\n        &type_node,\\n        context.modules,\\n        context.imports,\\n        &header.type_parameters,\\n        diagnostics,\\n    )?;\\n\"\"\"\nnew_local = \"\"\"    let declared = resolve_full_type(\\n        header.module,\\n        header.unit,\\n        &type_node,\\n        context.modules,\\n        context.imports,\\n        context.records,\\n        context.function_types,\\n        &header.type_parameters,\\n        true,\\n        diagnostics,\\n    )?;\\n\"\"\"\n'''
new = '''old_local = \"\"\"    let declared = resolve_type(\\n        header.module,\\n        header.unit,\\n        &type_node,\\n        context.modules,\\n        context.imports,\\n        &header.type_parameters,\\n        diagnostics,\\n    );\\n\"\"\"\nnew_local = \"\"\"    let declared = resolve_full_type(\\n        header.module,\\n        header.unit,\\n        &type_node,\\n        context.modules,\\n        context.imports,\\n        context.records,\\n        context.function_types,\\n        &header.type_parameters,\\n        true,\\n        diagnostics,\\n    );\\n\"\"\"\n'''
assert text.count(old) == 1
path.write_text(text.replace(old, new, 1))
print('preserved non-short-circuiting local type/initializer diagnostic flow')
