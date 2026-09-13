from pathlib import Path
import re

for path in Path('.').rglob('*.rs'):
    if path.as_posix() == 'crates/runen-core-ir/tests/external_callables.rs':
        continue
    text = path.read_text()
    if 'external_callables: Vec::new(),' in text:
        text = ''.join(
            line
            for line in text.splitlines(keepends=True)
            if line.strip() != 'external_callables: Vec::new(),'
        )
        # The temporary phase-1 insertion was placed before the constructor's original
        # newline. Restore only the whitespace-only line that removal can leave directly
        # after a Program opener; do not normalize unrelated source formatting.
        text = re.sub(r'((?:core::)?Program\s*\{)\n[ \t]*\n', r'\1\n', text)
        path.write_text(text)

syntax_test = Path('crates/runen-syntax/tests/external_callables.rs')
text = syntax_test.read_text()
text = text.replace(
    'use runen_syntax::{SyntaxKind, parse};\n\nfn count(parse: &runen_syntax::Parse, kind: SyntaxKind) -> usize {',
    'use runen_syntax::{Parse, SyntaxKind, parse_source, user_identifier_key};\n\nfn parse(source: &str) -> Parse {\n    parse_source(source.as_bytes()).expect("valid UTF-8 test source")\n}\n\nfn count(parse: &Parse, kind: SyntaxKind) -> usize {',
    1,
)
text = text.replace(
    'fn external_remains_contextual_identifier_outside_item_introducer() {\n    let parsed = parse(',
    'fn external_remains_contextual_identifier_outside_item_introducer() {\n    assert_eq!(user_identifier_key("external").as_deref(), Some("external"));\n    let parsed = parse(',
    1,
)
syntax_test.write_text(text)
print('removed unsafe broad fixture migration, restored constructor text, and aligned focused syntax harness')
