from pathlib import Path

for path in Path('.').rglob('*.rs'):
    if path.as_posix() == 'crates/runen-core-ir/tests/external_callables.rs':
        continue
    text = path.read_text()
    if 'external_callables: Vec::new(),' in text:
        text = text.replace('    external_callables: Vec::new(),\n', '')
        text = text.replace('        external_callables: Vec::new(),\n', '')
        text = text.replace('            external_callables: Vec::new(),\n', '')
        path.write_text(text)
print('removed unsafe broad fixture migration for phase-1 compiler gate')
