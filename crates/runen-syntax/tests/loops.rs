use runen_syntax::{
    ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source, user_identifier_key,
};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn reserves_while_without_splitting_longer_identifiers() {
    assert_eq!(user_identifier_key("while"), None);
    assert_eq!(user_identifier_key("whiled").as_deref(), Some("whiled"));

    let source = "fn whiled(flag: Bool) { while flag {} }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let tokens = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| (token.kind(), token.text().to_owned()))
        .collect::<Vec<_>>();
    assert!(tokens.contains(&(SyntaxKind::KwWhile, "while".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::Ident, "whiled".to_owned())));
}

#[test]
fn parses_all_represented_conditional_value_shapes_for_while() {
    let source = r#"
record Flag { ready: Bool }
fn check() -> Bool { return true; }
fn loops(flag: Bool, state: Flag) {
    while true {}
    while false {}
    while 1 {}
    while - 1 {}
    while flag {}
    while check() {}
    while state.ready {}
    while Flag { ready: true }.ready {}
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::WhileStatement)
            .count(),
        8
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordConstruction)
            .count(),
        1
    );
}

#[test]
fn while_uses_one_block_and_nests_as_an_ordinary_body_statement() {
    let source = "fn loops(a: Bool, b: Bool) { while a { if b { while a {} } else {} } }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::WhileStatement)
            .count(),
        2
    );
    let outer = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::WhileStatement)
        .expect("outer while");
    assert_eq!(
        outer
            .children()
            .filter(|node| node.kind() == SyntaxKind::BlockStatement)
            .count(),
        1
    );
}

#[test]
fn standalone_record_construction_is_not_a_while_condition() {
    let source = "fn bad() { while Flag { ready: true } {} kept(); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());

    let while_node = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::WhileStatement)
        .expect("while node survives recovery");
    assert_eq!(
        while_node
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordConstruction)
            .count(),
        0
    );
}

#[test]
fn malformed_condition_stops_at_body_and_preserves_following_statement() {
    let source = "fn broken() { while { sink(); } kept(); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Value))
    );
    assert!(
        parsed
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::CallStatement && node.to_string() == "kept();")
    );
}

#[test]
fn while_has_no_else_form() {
    let parsed = parse("fn bad(flag: Bool) { while flag {} else {} kept(); }");
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::WhileStatement)
            .count(),
        1
    );
}
