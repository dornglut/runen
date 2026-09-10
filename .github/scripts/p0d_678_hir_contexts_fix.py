from pathlib import Path

path = Path('.github/scripts/p0d_678_hir_contexts.py')
text = path.read_text()
old = '''needle = ''' + "'''" + '''    let declared = resolve_full_type(\n        header.module,\n        header.unit,\n        &type_node,\n        context.modules,\n        context.imports,\n        context.records,\n        context.function_types,\n        &header.type_parameters,\n        true,\n        diagnostics,\n    );\n''' + "'''" + '''
replacement = ''' + "'''" + '''    let type_context = FullTypeResolutionContext {\n        names: TypeNameResolutionContext {\n            module: header.module,\n            unit: header.unit,\n            modules: context.modules,\n            imports: context.imports,\n            type_parameters: &header.type_parameters,\n        },\n        records: context.records,\n        function_types: context.function_types,\n    };\n    let declared = resolve_full_type(&type_node, &type_context, true, diagnostics);\n''' + "'''" + '''
once(needle, replacement, 'local full type')
'''
new = '''needle = ''' + "'''" + '''    let declared = resolve_full_type(\n        header.module,\n        header.unit,\n        &type_node,\n        context.modules,\n        context.imports,\n        context.records,\n        context.function_types,\n        &header.type_parameters,\n        true,\n        diagnostics,\n    )\n''' + "'''" + '''
replacement = ''' + "'''" + '''    let type_context = FullTypeResolutionContext {\n        names: TypeNameResolutionContext {\n            module: header.module,\n            unit: header.unit,\n            modules: context.modules,\n            imports: context.imports,\n            type_parameters: &header.type_parameters,\n        },\n        records: context.records,\n        function_types: context.function_types,\n    };\n    let declared = resolve_full_type(&type_node, &type_context, true, diagnostics)\n''' + "'''" + '''
once(needle, replacement, 'local full type')
'''
assert text.count(old) == 1, f'local context-script block matches: {text.count(old)}'
path.write_text(text.replace(old, new, 1))
print('updated local full-type cleanup to preserve existing chained validation')
