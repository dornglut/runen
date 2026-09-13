from pathlib import Path

files = [
    'crates/runen-hir/tests/record_destructuring.rs',
    'crates/runen-core-lowering/tests/record_duplicability.rs',
    'crates/runen-hir/tests/grouping.rs',
    'crates/runen-hir/tests/execution_static_storage.rs',
    'crates/runen-core-lowering/tests/statics.rs',
    'crates/runen-hir/tests/function_values.rs',
]
for path in files:
    print(f'=== {path} ===')
    lines = Path(path).read_text().splitlines()
    for index, line in enumerate(lines, 1):
        if '.body' in line or '.parameters' in line:
            lo = max(0, index - 3)
            hi = min(len(lines), index + 2)
            print(f'-- around line {index} --')
            for current in range(lo, hi):
                print(f'{current + 1:4}: {lines[current]}')
