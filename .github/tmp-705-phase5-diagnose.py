from pathlib import Path

p = Path('crates/runen-core-lowering/src/lib.rs')
text = p.read_text()
needle = '            .parameters\n            .iter()'
print(f'parameter-iterator-count={text.count(needle)}')
start = 0
for index in range(text.count(needle)):
    pos = text.find(needle, start)
    lo = max(0, pos - 140)
    hi = min(len(text), pos + len(needle) + 180)
    print(f'--- occurrence {index + 1} ---')
    print(text[lo:hi])
    start = pos + len(needle)
