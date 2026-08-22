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
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordPattern)
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
fn recursive_record_patterns_parse_losslessly_and_classify_targets_syntactically() {
    let source = r#"
record Leaf { value: I8, other: U8 }
record Inner { leaf: Leaf, count: I8 }
record Outer { tail: U8, inner: Inner }
fn f(root: Outer) {
    let Outer {
        inner: Inner {
            count: count,
            leaf: Leaf {
                other: other,
                value: value,
            },
        },
        tail: tail,
    } = root;
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordPattern)
            .count(),
        3
    );
    let declaration = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::RecordDestructuringDeclaration)
        .expect("record destructuring declaration");
    let pattern = declaration
        .children()
        .find(|node| node.kind() == SyntaxKind::RecordPattern)
        .expect("top record pattern");
    assert!(pattern.text().to_string().trim_start().starts_with("Outer"));
    assert!(!declaration.children().any(|child| matches!(
        child.kind(),
        SyntaxKind::DirectCall | SyntaxKind::RecordConstruction | SyntaxKind::FieldValueUse
    )));
}

#[test]
fn producer_backed_scrutinees_reuse_existing_nodes_while_bare_root_stays_direct() {
    let source = r#"
import api;
record Token {}
record Pair { left: I8, right: Token }
record Outer { pair: Pair }
fn make() -> Pair { return Pair { left: 1, right: Token {} }; }
fn make_outer() -> Outer { return Outer { pair: Pair { left: 3, right: Token {} } }; }
fn f(root: Pair, outer: Outer) {
    let Pair { left: a, right: b } = root;
    let Pair { left: c, right: d } = make();
    let Pair { left: e, right: f } = Pair { left: 2, right: Token {} };
    let Pair { left: g, right: h } = outer.pair;
    let Pair { left: i, right: j } = api::make();
    let Pair { left: k, right: l } = make_outer().pair;
    let Pair { left: m, right: n } = Outer { pair: Pair { left: 4, right: Token {} } }.pair;
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let declarations = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::RecordDestructuringDeclaration)
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 7);

    assert!(!declarations[0].children().any(|child| matches!(
        child.kind(),
        SyntaxKind::DirectCall | SyntaxKind::RecordConstruction | SyntaxKind::FieldValueUse
    )));
    assert!(
        declarations[1]
            .children()
            .any(|child| child.kind() == SyntaxKind::DirectCall)
    );
    assert!(
        declarations[2]
            .children()
            .any(|child| child.kind() == SyntaxKind::RecordConstruction)
    );
    assert!(
        declarations[3]
            .children()
            .any(|child| child.kind() == SyntaxKind::FieldValueUse)
    );
    let qualified_call = declarations[4]
        .children()
        .find(|child| child.kind() == SyntaxKind::DirectCall)
        .expect("qualified producer call");
    assert!(
        qualified_call
            .children()
            .any(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    );

    let call_field = declarations[5]
        .children()
        .find(|child| child.kind() == SyntaxKind::FieldValueUse)
        .expect("call-backed field scrutinee");
    assert_eq!(
        call_field
            .children()
            .filter(|child| child.kind() == SyntaxKind::DirectCall)
            .count(),
        1
    );

    let construction_field = declarations[6]
        .children()
        .find(|child| child.kind() == SyntaxKind::FieldValueUse)
        .expect("construction-backed field scrutinee");
    assert_eq!(
        construction_field
            .children()
            .filter(|child| child.kind() == SyntaxKind::RecordConstruction)
            .count(),
        1
    );
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
fn excluded_pattern_extensions_and_scrutinees_are_not_silently_accepted() {
    for source in [
        "record Pair { left: I8 } fn f(root: Pair) { let Pair { left } = root; }",
        "record Pair { left: I8 } fn f(root: Pair) { let Pair { left: value, .. } = root; }",
        "record Pair { left: I8 } fn f(root: Pair) { let other::Pair { left: value } = root; }",
        "record Pair { left: I8 } fn f() { let Pair { left: value } = true; }",
        "record Pair { left: I8 } fn f() { let Pair { left: value } = 1; }",
        "record Pair { left: I8 } fn f() { let Pair { left: value } = other::root; }",
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
fn malformed_nested_pattern_preserves_following_statement_and_top_level_boundary() {
    let source = "record Leaf { value: I8 } record Outer { leaf: Leaf, count: I8 } fn f(root: Outer) { let Outer { leaf: Leaf { value: item = root; let later: I8 = 2; } record Next {}";
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
