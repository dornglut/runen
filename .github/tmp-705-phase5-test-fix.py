from pathlib import Path

p = Path('crates/runen-core-lowering/tests/external_callables.rs')
text = p.read_text()
old = '''         fn root() -> I64 { \\
             let call = fn[](value: I64) -> I64 { return ext(value); }; \\
             let first: I64 = generic[I64](1); \\
             return call(first); \\
         }'''
new = '''         fn root() -> I64 { \\
             let seed: I64 = 0; \\
             let call = fn[seed](value: I64) -> I64 { return ext(value) + seed; }; \\
             let first: I64 = generic[I64](1); \\
             return call(first); \\
         }'''
if text.count(old) != 1:
    raise SystemExit(f'closure lowering fixture anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('corrected #705 closure fixture to accepted nonempty capture syntax')
