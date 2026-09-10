from pathlib import Path

path = Path('.github/scripts/p0d_678_hir_contexts.py')
text = path.read_text()
old = "base = base.replace('            unit,\\n            range:', '            unit: *unit,\\n            range:')\n"
new = old + "base = base.replace('                unit,\\n                range:', '                unit: *unit,\\n                range:')\n"
assert text.count(old) == 1, f'unit rewrite anchor matches: {text.count(old)}'
path.write_text(text.replace(old, new, 1))
print('extended context cleanup to dereference nested SourceLocation unit')
