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
print('removed unsafe broad fixture migration and aligned focused syntax harness')
