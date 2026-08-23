use runen_syntax::{
    ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source, user_identifier_key,
};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn reserves_loop_keys_without_splitting_longer_identifiers() {
    assert_eq!(user_identifier_key("while"), None);
    assert_eq!(user_identifier_key("break"), None);
    assert_eq!(user_identifier_key("continue"), None);
    assert_eq!(user_identifier_key("whiled").as_deref(), Some("whiled"));
    assert_eq!(
        user_identifier_key("breakable").as_deref(),
        Some("breakable")
    );
    assert_eq!(
        user_identifier_key("continued").as_deref(),
        Some("continued")
    );
    assert_eq!(
        user_identifier_key("continue_").as_deref(),
        Some("continue_")
    );

    let source = "fn breakable(continued: Bool) { while continued { continue; break; } }";
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
    assert!(tokens.contains(&(SyntaxKind::KwBreak, "break".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::KwContinue, "continue".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::Ident, "breakable".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::Ident, "continued".to_owned())));
}

#[test]
fn break_and_continue_are_ordinary_body_statements_and_do_not_stop_parsing() {
    let source = "fn sink() {} fn f(flag: Bool) { while flag { break; sink(); } while flag { continue; sink(); } }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::BreakStatement)
            .count(),
        1
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::ContinueStatement)
            .count(),
        1
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::CallStatement)
            .count(),
        2
    );
}

#[test]
fn break_and_continue_require_exact_semicolon_forms() {
    for source in [
        "fn f(flag: Bool) { while flag { break } }",
        "fn f(flag: Bool) { while flag { continue } }",
        "fn f(flag: Bool) { while flag { break value; } }",
        "fn f(flag: Bool) { while flag { continue value; } }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(
            parsed.errors().iter().any(|error| {
                error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Semicolon)
            }),
            "expected semicolon diagnostic for {source:?}: {:?}",
            parsed.errors()
        );
    }
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
