use runen_syntax::{
    ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source, user_identifier_key,
};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn reserves_if_and_else_without_splitting_longer_identifiers() {
    assert_eq!(user_identifier_key("if"), None);
    assert_eq!(user_identifier_key("else"), None);
    assert_eq!(user_identifier_key("ifonly").as_deref(), Some("ifonly"));
    assert_eq!(user_identifier_key("elsewhere").as_deref(), Some("elsewhere"));

    let parsed = parse("fn ifonly(elsewhere: Bool) { if elsewhere {} }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let tokens = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| (token.kind(), token.text().to_owned()))
        .collect::<Vec<_>>();
    assert!(tokens.contains(&(SyntaxKind::KwIf, "if".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::Ident, "ifonly".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::Ident, "elsewhere".to_owned())));
}

#[test]
fn parses_all_represented_conditional_value_shapes_losslessly() {
    let source = r#"
fn literals(flag: Bool) {
    if true {}
    if false {} else {}
    if 1 {}
    if - 1 {}
    if flag {}
    if check() {}
    if ext::check() {}
    if state.ready {}
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::IfStatement)
            .count(),
        8
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::BooleanLiteral)
            .count(),
        2
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::DecimalIntegerLiteral)
            .count(),
        2
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::DirectCall)
            .count(),
        2
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::FieldValueUse)
            .count(),
        1
    );
}

#[test]
fn identifier_followed_by_arm_block_is_not_record_construction() {
    let source = "fn choose(flag: Bool) { if flag { sink(); } }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let conditional = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IfStatement)
        .expect("conditional node");
    assert_eq!(
        conditional
            .children()
            .filter(|node| node.kind() == SyntaxKind::IdentifierUse)
            .count(),
        1
    );
    assert_eq!(
        conditional
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordConstruction)
            .count(),
        0
    );
}

#[test]
fn record_construction_shaped_condition_is_not_admitted() {
    let source = "fn bad() { if Flag { value: true } {} sink(); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordConstruction)
            .count(),
        0
    );
}

#[test]
fn else_is_optional_and_explicit_else_retains_its_block() {
    let source = "fn choose(a: Bool, b: Bool) { if a {} if b {} else {} }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let conditionals = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::IfStatement)
        .collect::<Vec<_>>();
    assert_eq!(conditionals.len(), 2);
    assert_eq!(
        conditionals[0]
            .children()
            .filter(|node| node.kind() == SyntaxKind::BlockStatement)
            .count(),
        1
    );
    assert_eq!(
        conditionals[1]
            .children()
            .filter(|node| node.kind() == SyntaxKind::BlockStatement)
            .count(),
        2
    );
}

#[test]
fn direct_else_if_is_rejected_but_nested_if_in_else_block_is_valid() {
    let direct = parse("fn bad(a: Bool, b: Bool) { if a {} else if b {} }");
    assert!(!direct.errors().is_empty());

    let nested_source = "fn good(a: Bool, b: Bool) { if a {} else { if b {} } }";
    let nested = parse(nested_source);
    assert_eq!(nested.text(), nested_source);
    assert!(nested.errors().is_empty(), "{:?}", nested.errors());
    assert_eq!(
        nested
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::IfStatement)
            .count(),
        2
    );
}

#[test]
fn nested_return_remains_rejected_and_following_if_is_preserved() {
    let source = "fn bad(flag: Bool) { if flag { return; if flag {} } }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::IfStatement)
            .count(),
        2
    );
}

#[test]
fn missing_then_close_preserves_else_boundary() {
    let source = "fn broken(flag: Bool) { if flag { sink(); else { other(); } kept(); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(
        parsed.errors().iter().any(|error| {
            error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace)
        })
    );
    let conditional = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IfStatement)
        .expect("conditional survives recovery");
    assert_eq!(
        conditional
            .children()
            .filter(|node| node.kind() == SyntaxKind::BlockStatement)
            .count(),
        2
    );
    assert!(
        parsed
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::CallStatement && node.to_string() == "kept();")
    );
}

#[test]
fn trivia_round_trips_around_condition_and_arms() {
    let source = "fn choose(flag: Bool) { if /*a*/ flag /*b*/ { //c\n } /*d*/ else /*e*/ { } }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
}
