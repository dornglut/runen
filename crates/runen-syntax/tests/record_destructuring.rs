use runen_syntax::{ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn record_destructuring_parses_losslessly_with_reordering_trailing_comma_and_trivia() {
    let source = r#"
record Empty {}
record Pair { left: I8, right: U8 }
fn f(empty: Empty, pair: Pair) {
    let Empty {} = empty;
    let Pair {
        right: renamed_right,
        /* retained */ left: renamed_left,
    } = pair;
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordDestructuringDeclaration)
            .count(),
        2
    );
    let fields = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::RecordPatternField)
        .map(|node| node.text().to_string())
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 2);
    assert!(fields[0].trim_start().starts_with("right"));
    assert!(fields[1].contains("left"));
}

#[test]
fn ordinary_local_forms_remain_distinct_and_unchanged() {
    let source = "fn f() { let value: I8 = 1; let mut other: U8 = 2; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::LocalDeclaration)
            .count(),
        2
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordDestructuringDeclaration)
            .count(),
        0
    );
}

#[test]
fn excluded_pattern_extensions_are_not_silently_accepted() {
    for source in [
        "record Pair { left: I8 } fn f(root: Pair) { let Pair { left } = root; }",
        "record Pair { left: I8 } fn f(root: Pair) { let Pair { left: value, .. } = root; }",
        "record Inner { value: I8 } record Outer { inner: Inner } fn f(root: Outer) { let Outer { inner: Inner { value: leaf } } = root; }",
        "record Pair { left: I8 } fn f(root: Pair) { let other::Pair { left: value } = root; }",
        "record Pair { left: I8 } fn make() -> Pair { return Pair { left: 1 }; } fn f(root: Pair) { let Pair { left: value } = make(); }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(
            !parsed.errors().is_empty(),
            "excluded form parsed cleanly: {source}"
        );
    }
}

#[test]
fn malformed_pattern_field_recovers_to_following_field_and_statement() {
    let source = "record Pair { left: I8, right: I8 } fn f(root: Pair) { let Pair { left: first right: second } = root; let later: I8 = 3; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightBrace)
    }));
    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordPatternField)
            .count(),
        2
    );
    assert!(root.descendants().any(|node| {
        node.kind() == SyntaxKind::LocalDeclaration && node.text().to_string().contains("later")
    }));
}

#[test]
fn missing_pattern_close_preserves_following_body_and_top_level_boundaries() {
    let source = "record Pair { left: I8 } fn f(root: Pair) { let Pair { left: value = root; let later: I8 = 2; } record Next {}";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| { error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace) })
    );
    let root = parsed.syntax();
    assert!(root.descendants().any(|node| {
        node.kind() == SyntaxKind::LocalDeclaration && node.text().to_string().contains("later")
    }));
    assert!(root.descendants().any(|node| {
        node.kind() == SyntaxKind::RecordDefinition && node.text().to_string().contains("Next")
    }));
}
