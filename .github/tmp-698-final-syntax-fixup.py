#!/usr/bin/env python3
from pathlib import Path
p = Path('/tmp/runen-698-final-syntax-consistency.py')
text = p.read_text()

fixes = [
    ('a qualified bare module member does not become a field receiver',
     'A qualified bare module member does not become a field receiver', 2),
    ('neither wrapper, a constant reference',
     'either wrapper, a constant reference', 2),
    ('named constants are likewise not admitted as pattern test leaves or bounds merely because their values are scalar.',
     'Named constants are likewise not admitted as pattern test leaves or bounds merely because their values are scalar.', 1),
]

for old, new, expected in fixes:
    count = text.count(old)
    if count != expected:
        raise SystemExit(f'expected exactly {expected} transform occurrence(s), found {count}: {old!r}')
    text = text.replace(old, new)

p.write_text(text)
