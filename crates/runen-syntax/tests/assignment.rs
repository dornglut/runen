use runen_syntax::{SyntaxKind, parse_source, user_identifier_key};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn round_trips_mutable_locals_and_assignment_with_trivia() {
    let source = "fn f(input: I64) {\n    let /*a*/ mut /*b*/ value: I64 = input;\n    value /*c*/ = /*d*/ input;\n}\n";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(
        parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::KwMut)
            .count(),
        1
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::AssignmentStatement)
            .count(),
        1
    );
}

#[test]
fn round_trips_one_and_nested_field_assignment_targets() {
    let source = "record Inner { value: I64 } record Outer { inner: Inner } fn f(input: I64, root: Outer) { root /*a*/ . /*b*/ inner /*c*/ . /*d*/ value /*e*/ = /*f*/ input; root.inner.value = input; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let assignments = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::AssignmentStatement)
        .collect::<Vec<_>>();
    assert_eq!(assignments.len(), 2);
    for assignment in assignments {
        assert_eq!(
            assignment
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .filter(|token| token.kind() == SyntaxKind::Dot)
                .count(),
            2
        );
    }
}

#[test]
fn mut_is_reserved_only_after_complete_identifier_formation() {
    assert!(user_identifier_key("mut").is_none());
    assert_eq!(user_identifier_key("mutable").as_deref(), Some("mutable"));
    assert_eq!(user_identifier_key("mutation").as_deref(), Some("mutation"));

    let source = "fn f(mutable: I64) { let mutation: I64 = mutable; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert!(
        parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .all(|token| token.kind() != SyntaxKind::KwMut)
    );
}

#[test]
fn identifier_started_statements_distinguish_assignment_and_calls() {
    let source = "record Box { value: I64 } fn id(v: I64) -> I64 { return v; } fn f(x: I64, root: Box) { x = id(x); root.value = id(x); id(x); }";
    let parsed = parse(source);

    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::AssignmentStatement)
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
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::DirectCall)
            .count(),
        3,
        "two result calls in assignments and the no-result call statement"
    );
}

#[test]
fn malformed_assignment_forms_preserve_following_constructs() {
    for source in [
        "fn f(x: I64) { x = ; let kept: I64 = x; } fn ok() {}",
        "fn f(x: I64) { x = x let kept: I64 = x; } fn ok() {}",
        "fn f(x: I64) { x x; let kept: I64 = x; } fn ok() {}",
        "record Box { value: I64 } fn f(x: I64, root: Box) { root. = x; let kept: I64 = x; } fn ok() {}",
        "record Box { value: I64 } fn f(x: I64, root: Box) { root.value x; let kept: I64 = x; } fn ok() {}",
        "record Box { value: I64 } fn f(x: I64, root: Box) { root.value = ; let kept: I64 = x; } fn ok() {}",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(
            !parsed.errors().is_empty(),
            "malformed source must diagnose"
        );
        assert_eq!(
            parsed
                .syntax()
                .descendants()
                .filter(|node| node.kind() == SyntaxKind::LocalDeclaration)
                .count(),
            1,
            "later legal local must remain represented for {source}"
        );
        assert_eq!(
            parsed
                .syntax()
                .children()
                .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
                .count(),
            2,
            "later top-level function must remain represented for {source}"
        );
    }
}
