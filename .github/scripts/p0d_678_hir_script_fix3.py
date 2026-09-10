from pathlib import Path

path = Path('.github/scripts/p0d_678_hir_patch.py')
text = path.read_text()
start = text.index('old_local = """')
end_marker = 'build = replace_once(build, old_local, new_local, "local full type resolution")\n'
end = text.index(end_marker, start) + len(end_marker)
replacement = r'''local_anchor = "    let declared = resolve_type(\n"
local_start = build.index(local_anchor)
local_end = build.index("    );\n", local_start) + len("    );\n")
old_local = build[local_start:local_end]
assert "        context.imports,\n" in old_local
assert "        &header.type_parameters,\n" in old_local
new_local = old_local.replace("resolve_type", "resolve_full_type", 1).replace(
    "        &header.type_parameters,\n",
    "        context.records,\n"
    "        context.function_types,\n"
    "        &header.type_parameters,\n"
    "        true,\n",
    1,
)
build = build[:local_start] + new_local + build[local_end:]
'''
text = text[:start] + replacement + text[end:]
path.write_text(text)
print('local resolver patch now keys from its owning declaration instead of full whitespace shape')
