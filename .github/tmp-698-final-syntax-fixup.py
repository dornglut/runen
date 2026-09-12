#!/usr/bin/env python3
from pathlib import Path
p = Path('/tmp/runen-698-final-syntax-consistency.py')
text = p.read_text()
old = 'a qualified bare module member does not become a field receiver'
new = 'A qualified bare module member does not become a field receiver'
count = text.count(old)
if count != 2:
    raise SystemExit(f'expected exactly two transform casing occurrences, found {count}')
p.write_text(text.replace(old, new))
