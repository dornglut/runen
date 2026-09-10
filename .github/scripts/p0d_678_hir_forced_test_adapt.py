from pathlib import Path
import re

root = Path('crates/runen-hir/tests')
changed = []


def direct_value_pattern(match: re.Match[str]) -> str:
    inner = match.group(1)
    names = {part.strip() for part in inner.split(',') if part.strip() and part.strip() != '..'}
    unknown = names - {'function', 'type_arguments', 'arguments'}
    assert not unknown, f'unsupported DirectCall pattern fields: {unknown} in {match.group(0)!r}'
    target_fields = [name for name in ('function', 'type_arguments') if name in names]
    target = 'CallTarget::Direct { ' + ''.join(f'{name}, ' for name in target_fields) + '.. }'
    outer = [f'target: {target}']
    if 'arguments' in names:
        outer.append('arguments')
    return 'ValueKind::Call { ' + ', '.join(outer) + ', .. }'


def statement_call_pattern(match: re.Match[str]) -> str:
    inner = match.group(1)
    names = {part.strip() for part in inner.split(',') if part.strip() and part.strip() != '..'}
    if not ({'function', 'type_arguments'} & names):
        return match.group(0)
    unknown = names - {'function', 'type_arguments', 'arguments', 'location'}
    assert not unknown, f'unsupported Statement::Call pattern fields: {unknown} in {match.group(0)!r}'
    target_fields = [name for name in ('function', 'type_arguments') if name in names]
    target = 'CallTarget::Direct { ' + ''.join(f'{name}, ' for name in target_fields) + '.. }'
    outer = [f'target: {target}']
    outer.extend(name for name in ('arguments', 'location') if name in names)
    return 'Statement::Call { ' + ', '.join(outer) + ', .. }'

for path in sorted(root.glob('*.rs')):
    text = path.read_text()
    original = text
    text = re.sub(r'ValueKind::DirectCall\s*\{([^{}]*)\}', direct_value_pattern, text)
    text = re.sub(r'Statement::Call\s*\{([^{}]*)\}', statement_call_pattern, text)
    if 'ValueKind::DirectCall' in text:
        raise AssertionError(f'unadapted ValueKind::DirectCall remains in {path}')
    # Every newly introduced CallTarget reference needs the import. Insert just
    # after the opening brace so both one-line and multiline use forms are valid;
    # rustfmt restores the repository's preferred formatting afterward.
    if 'CallTarget::Direct' in text and not re.search(r'use\s+runen_hir::\{[^;]*\bCallTarget\b', text, re.S):
        marker = 'use runen_hir::{'
        assert marker in text, f'expected runen_hir grouped import in {path}'
        text = text.replace(marker, marker + ' CallTarget,', 1)
    if text != original:
        path.write_text(text)
        changed.append(str(path))

print('adapted direct-call assertions in:')
for path in changed:
    print(f'  {path}')
