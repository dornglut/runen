use runen_syntax::{ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn parses_nested_direct_call_values_with_trailing_commas() {
    let source = "fn forward(value: I64) -> I64 { return outer(inner(value,),); }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::DirectCall)
            .count(),
        2
    );
}

#[test]
fn missing_initializer_value_preserves_the_following_statement() {
    let source = "fn broken(value: I64) { let missing: I64 = let kept: I64 = value; sink(kept); }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::LocalDeclaration)
            .count(),
        2
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::CallStatement)
            .count(),
        1
    );
}

#[test]
fn bare_return_with_missing_semicolon_does_not_invent_a_required_value() {
    let source = "fn broken() { return }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| { error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Semicolon) })
    );
    assert!(
        !parsed
            .errors()
            .iter()
            .any(|error| { error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Value) })
    );
}

#[test]
fn missing_return_terminator_and_body_close_preserve_the_next_item() {
    let source = "fn broken() { return fn ok() {}";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .children()
            .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
            .count(),
        2
    );
}
