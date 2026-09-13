from pathlib import Path

p = Path('crates/runen-core-lowering/src/lib.rs')
text = p.read_text()
old = '''        for ty in function_type
            .parameters
            .iter()
            .chain(function_type.result.iter())
'''
new = '''        for ty in function_type.parameters.iter().chain(function_type.result.iter())
'''
if text.count(old) != 1:
    raise SystemExit(f'function-type iterator staging anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('isolated FunctionLowerer parameter iterator anchor from function-type traversal')
