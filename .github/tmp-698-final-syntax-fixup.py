#!/usr/bin/env python3
from pathlib import Path
p = Path('/tmp/runen-698-final-syntax-consistency.py')
text = p.read_text()

old_case = 'a qualified bare module member does not become a field receiver'
new_case = 'A qualified bare module member does not become a field receiver'
count_case = text.count(old_case)
if count_case != 2:
    raise SystemExit(f'expected exactly two transform casing occurrences, found {count_case}')
text = text.replace(old_case, new_case)

old_wrapper = 'neither wrapper, a constant reference'
new_wrapper = 'either wrapper, a constant reference'
count_wrapper = text.count(old_wrapper)
if count_wrapper != 2:
    raise SystemExit(f'expected exactly two transform wrapper occurrences, found {count_wrapper}')
text = text.replace(old_wrapper, new_wrapper)

p.write_text(text)
