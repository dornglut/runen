use runen_syntax::{ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn constructions_parse_losslessly_in_every_represented_value_position() {
    let source = r#"
record Empty {}
record Inner { value: I8 }
record Pair { left: I8, right: I8, nested: Inner }
fn sink(value: Pair) {}
fn build() -> Pair {
    let empty: Empty = Empty {};
    let mut value: Pair = Pair { right: 2, /* keep field trivia */ left: 1, nested: Inner { value: 3, }, };
    value = Pair { left: 4, right: 5, nested: Inner { value: 6 } };
    sink(Pair { left: 7, right: 8, nested: Inner { value: 9 } });
    return Pair { left: 10, right: 11, nested: Inner { value: 12 } };
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordConstruction)
            .count(),
        9
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordInitializer)
            .count(),
        16
    );
}

#[test]
fn reordered_initializers_retain_source_order_as_explicit_nodes() {
    let parsed = parse(
        "record Pair { left: I8, right: I8 } fn f() -> Pair { return Pair { right: 2, left: 1, }; }",
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let construction = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::RecordConstruction)
        .expect("record construction node");
    let initializers = construction
        .children()
        .filter(|node| node.kind() == SyntaxKind::RecordInitializer)
        .map(|node| node.text().to_string())
        .collect::<Vec<_>>();

    assert_eq!(initializers.len(), 2);
    assert!(initializers[0].trim_start().starts_with("right"));
    assert!(initializers[1].trim_start().starts_with("left"));
}

#[test]
fn qualified_construction_reuses_qualified_member_and_stays_distinct_from_call() {
    let source = r#"
import dep;
record Pair { left: I8 }
fn f() -> Pair { return dep::Pair { left: 1 }; }
fn g() -> Pair { return dep::make(); }
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    let construction = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::RecordConstruction)
        .expect("qualified record construction");
    assert_eq!(
        construction
            .children()
            .filter(|node| node.kind() == SyntaxKind::QualifiedModuleMember)
            .count(),
        1
    );
    assert_eq!(
        construction
            .children()
            .filter(|node| node.kind() == SyntaxKind::DirectCall)
            .count(),
        0
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::DirectCall)
            .count(),
        1
    );
}

#[test]
fn qualified_construction_composes_in_value_and_field_receiver_positions() {
    let source = r#"
import dep;
record Pair { left: I8 }
fn sink(value: Pair) {}
fn build() -> Pair {
    let mut value: Pair = dep::Pair { left: 1 };
    value = dep::Pair { left: 2 };
    sink(dep::Pair { left: 3 });
    let selected: I8 = dep::Pair { left: 4 }.left;
    return dep::Pair { left: dep::Pair { left: 5 }.left };
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordConstruction)
            .count(),
        6
    );
    let field_use = root
        .descendants()
        .find(|node| {
            node.kind() == SyntaxKind::FieldValueUse
                && node
                    .text()
                    .to_string()
                    .contains("dep::Pair { left: 4 }.left")
        })
        .expect("qualified-construction-backed field use");
    assert!(
        field_use
            .children()
            .any(|node| node.kind() == SyntaxKind::RecordConstruction)
    );
}

#[test]
fn qualified_construction_backed_field_use_is_a_condition_but_bare_construction_is_not() {
    let accepted = parse(
        "import dep; fn f() { if dep::Flag { ready: true }.ready { let value: Bool = true; } }",
    );
    assert_eq!(
        accepted.text(),
        "import dep; fn f() { if dep::Flag { ready: true }.ready { let value: Bool = true; } }"
    );
    assert!(accepted.errors().is_empty(), "{:?}", accepted.errors());
    assert!(accepted.syntax().descendants().any(|node| {
        node.kind() == SyntaxKind::FieldValueUse
            && node
                .children()
                .any(|child| child.kind() == SyntaxKind::RecordConstruction)
    }));

    let rejected = parse("import dep; fn f() { if dep::Flag {} { let value: Bool = true; } }");
    assert!(!rejected.errors().is_empty());
    assert!(
        !rejected
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::RecordConstruction)
    );
}

#[test]
fn qualified_construction_is_syntactically_available_as_pattern_scrutinee_only() {
    let source = "import dep; record Pair { left: I8 } fn f() { let Pair { left: value } = dep::Pair { left: 1 }; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let declaration = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::RecordDestructuringDeclaration)
        .expect("record destructuring declaration");
    assert!(
        declaration
            .children()
            .any(|node| node.kind() == SyntaxKind::RecordConstruction)
    );
    let pattern = declaration
        .children()
        .find(|node| node.kind() == SyntaxKind::RecordPattern)
        .expect("record pattern");
    assert!(
        !pattern
            .descendants()
            .any(|node| node.kind() == SyntaxKind::QualifiedModuleMember),
        "qualified construction must not broaden pattern-head syntax"
    );
}

#[test]
fn standalone_construction_is_not_a_body_statement() {
    let standalone = parse("record Pair {} fn f() { Pair {} }");
    assert!(
        standalone
            .errors()
            .iter()
            .any(|error| { error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Statement) })
    );
}

#[test]
fn malformed_initializers_preserve_following_initializer_boundaries() {
    for source in [
        "record Pair { left: I8, right: I8 } fn f() -> Pair { return Pair { left 1, right: 2 }; }",
        "record Pair { left: I8, right: I8 } fn f() -> Pair { return Pair { left: , right: 2 }; }",
        "record Pair { left: I8, right: I8 } fn f() -> Pair { return Pair { left: 1 right: 2 }; }",
        "import dep; fn f() -> dep::Pair { return dep::Pair { left 1, right: 2 }; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty(), "{source}");
        assert!(
            parsed
                .syntax()
                .descendants()
                .filter(|node| node.kind() == SyntaxKind::RecordInitializer)
                .count()
                >= 2,
            "later initializer must remain structurally recoverable: {source}"
        );
    }
}

#[test]
fn missing_constructor_close_preserves_later_body_and_top_level_constructs() {
    for source in [
        "record Pair { left: I8 } fn f() { let value: Pair = Pair { left: 1; let later: I8 = 2; } record Next {}",
        "import dep; fn f() { let value: dep::Pair = dep::Pair { left: 1; let later: I8 = 2; } record Next {}",
    ] {
        let parsed = parse(source);

        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().iter().any(|error| {
            error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace)
        }));

        let root = parsed.syntax();
        assert!(root.descendants().any(|node| {
            node.kind() == SyntaxKind::LocalDeclaration && node.text().to_string().contains("later")
        }));
        assert!(root.descendants().any(|node| {
            node.kind() == SyntaxKind::RecordDefinition && node.text().to_string().contains("Next")
        }));
    }
}
