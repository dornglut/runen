use runen_syntax::{SyntaxKind, parse_source, user_identifier_key};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn kinds(source: &str) -> Vec<SyntaxKind> {
    parse(source)
        .syntax()
        .descendants()
        .map(|node| node.kind())
        .collect()
}

#[test]
fn raw_and_unsafe_are_reserved_while_move_and_assign_remain_contextual() {
    assert_eq!(user_identifier_key("raw"), None);
    assert_eq!(user_identifier_key("unsafe"), None);
    assert_eq!(user_identifier_key("move").as_deref(), Some("move"));
    assert_eq!(user_identifier_key("assign").as_deref(), Some("assign"));

    let parsed = parse("fn f(move: I64, assign: I64) { let rawish: I64 = move; assign = rawish; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
}

#[test]
fn parses_exact_raw_type_values_assignment_and_unsafe_wrapper_nodes() {
    let source = "fn f(x: I64) {\
        let mut p: raw I64 = raw &x;\
        unsafe {\
            let moved: I64 = raw move p;\
            raw assign p = moved;\
            unsafe { raw assign p = 7; }\
        }\
    }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);

    let root = parsed.syntax();
    let node_kinds = root
        .descendants()
        .map(|node| node.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        node_kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::RawAddressOfValue)
            .count(),
        1
    );
    assert_eq!(
        node_kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::RawMoveValue)
            .count(),
        1
    );
    assert_eq!(
        node_kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::RawAssignStatement)
            .count(),
        2
    );
    assert_eq!(
        node_kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::UnsafeBlockStatement)
            .count(),
        2
    );

    let tokens = root
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert!(tokens.contains(&SyntaxKind::KwRaw));
    assert!(tokens.contains(&SyntaxKind::KwUnsafe));
}

#[test]
fn parses_qualified_raw_pointee_and_contextual_keys_without_reserving_them() {
    let parsed = parse(
        "fn f(x: I64) {\
            let p: raw pkg::Thing = raw &x;\
            unsafe { let y: I64 = raw move p; raw assign p = y; }\
        }",
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let root = parsed.syntax();
    assert!(
        root.descendants()
            .any(|node| node.kind() == SyntaxKind::QualifiedModuleMember)
    );

    let contextual = root
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.text() == "move" || token.text() == "assign")
        .collect::<Vec<_>>();
    assert_eq!(contextual.len(), 2);
    assert!(
        contextual
            .iter()
            .all(|token| token.kind() == SyntaxKind::Ident)
    );
}

#[test]
fn raw_values_are_not_conditional_atoms_but_are_allowed_inside_call_arguments() {
    let direct = parse("fn f(p: raw Bool) { if raw move p { } }");
    assert!(!direct.errors().is_empty());

    let nested = parse(
        "fn pred(x: Bool) -> Bool { return x; } fn f(p: raw Bool) { if pred(raw move p) { } }",
    );
    assert!(nested.errors().is_empty(), "{:?}", nested.errors());
    assert!(
        nested
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::RawMoveValue)
    );
}

#[test]
fn rejects_recursive_type_constructors_and_wrong_contextual_raw_keys() {
    for source in [
        "fn f() { let p: raw raw I64 = bad; }",
        "fn f() { let p: raw &I64 = bad; }",
        "fn f() { let p: &raw I64 = bad; }",
        "fn f() { let p: &&I64 = bad; }",
        "fn f(p: raw I64) { let x: I64 = raw copy p; }",
        "fn f(p: raw I64) { raw replace p = 1; }",
        "fn f() { unsafe; }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly parsed cleanly: {source}"
        );
    }
}

#[test]
fn malformed_constructs_recover_to_following_raw_unsafe_and_items() {
    let source = "fn broken(x: I64) {\
        nope ???\
        raw assign p = 1;\
        unsafe { raw assign p = 2; }\
    }\
    fn next() {}";
    let parsed = parse(source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(parsed.text(), source);

    let node_kinds = kinds(source);
    assert!(node_kinds.contains(&SyntaxKind::RawAssignStatement));
    assert!(node_kinds.contains(&SyntaxKind::UnsafeBlockStatement));
    assert_eq!(
        node_kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::FunctionDefinition)
            .count(),
        2
    );
}
