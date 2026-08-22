use runen_syntax::{SyntaxKind, parse_source};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn field_value_uses_parse_losslessly_with_nested_paths_and_trivia() {
    let source = "record Inner { value: I8 } record Outer { inner: Inner } fn f(root: Outer) -> I8 { let a: I8 = root.value; let b: I8 = root /*a*/ . inner . /*b*/ value; return root.inner.value; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let uses = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::FieldValueUse)
        .map(|node| node.text().to_string())
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 3);
    assert_eq!(uses[0].trim_start(), "root.value");
    assert_eq!(uses[1].trim_start(), "root /*a*/ . inner . /*b*/ value");
    assert_eq!(uses[2].trim_start(), "root.inner.value");
    assert!(parsed.syntax().descendants_with_tokens().any(|element| {
        element
            .into_token()
            .is_some_and(|token| token.kind() == SyntaxKind::Dot)
    }));
}

#[test]
fn producer_receivers_wrap_complete_call_or_construction_nodes() {
    let source = "import ext; record Inner { value: I8 } record Outer { inner: Inner } fn make() -> Outer { return Outer { inner: Inner { value: 1 } }; } fn f() -> I8 { let a: I8 = make() /*a*/ . inner . value; let b: I8 = ext::make().inner.value; return Outer { inner: Inner { value: 2 } }.inner.value; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let uses = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::FieldValueUse)
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 3);

    let direct_call_receivers = uses
        .iter()
        .filter_map(|field_use| {
            field_use
                .children()
                .find(|node| node.kind() == SyntaxKind::DirectCall)
        })
        .collect::<Vec<_>>();
    assert_eq!(direct_call_receivers.len(), 2);
    assert_eq!(
        direct_call_receivers
            .iter()
            .filter(|call| {
                call.children()
                    .any(|node| node.kind() == SyntaxKind::QualifiedModuleMember)
            })
            .count(),
        1
    );
    assert_eq!(
        uses.iter()
            .filter(|field_use| {
                field_use
                    .children()
                    .any(|node| node.kind() == SyntaxKind::RecordConstruction)
            })
            .count(),
        1
    );
    assert!(uses.iter().all(|node| {
        node.children()
            .filter(|child| {
                matches!(
                    child.kind(),
                    SyntaxKind::DirectCall | SyntaxKind::RecordConstruction
                )
            })
            .count()
            == 1
    }));
}

#[test]
fn producer_receiver_requires_selector_and_bare_producers_stay_bare() {
    let source = "record Box { value: I8 } fn make() -> Box { return Box { value: 1 }; } fn f() -> Box { let a: Box = make(); return Box { value: 2 }; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::FieldValueUse)
            .count(),
        0
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::DirectCall)
            .count(),
        1
    );
}

#[test]
fn field_value_use_is_distinct_from_bare_identifier_use() {
    let parsed = parse("record Box { value: I8 } fn f(root: Box) -> I8 { return root.value; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::FieldValueUse)
            .count(),
        1
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::IdentifierUse)
            .count(),
        0
    );
}

#[test]
fn malformed_or_unrepresented_dot_forms_remain_syntax_invalid() {
    for source in [
        "record Box { value: I8 } fn f(root: Box) -> I8 { return root.; }",
        "record Box { value: I8 } fn f(root: Box) -> I8 { return root.value(); }",
        "record Box { value: I8 } fn make() -> Box { return Box { value: 1 }; } fn f() -> I8 { return make().value(); }",
        "record Box { value: I8 } fn f(root: Box) { root.value = 1; }",
        "fn f() -> I8 { return 1.2; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty(), "must reject: {source}");
    }
}
