use runen_syntax::{Parse, SyntaxKind, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count(parsed: &Parse, kind: SyntaxKind) -> usize {
    parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

#[test]
fn grouping_syntax_kind_is_append_only_and_nested_groups_are_lossless() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::ContinueStatement).0, 81);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Bang).0, 82);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BooleanNotValue).0, 83);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::EqEq).0, 84);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BangEq).0, 85);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::BooleanEqualityValue).0,
        86
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::GroupedValue).0, 87);

    let source = "fn f(flag: Bool) -> Bool { return (((flag))); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::IdentifierUse), 1);
}

#[test]
fn grouping_explicitly_nests_the_complete_equality_tier() {
    let source = r#"
fn not_equal(a: Bool, b: Bool) -> Bool { return !(a == b); }
fn left(a: Bool, b: Bool, c: Bool) -> Bool { return (a == b) == c; }
fn right(a: Bool, b: Bool, c: Bool) -> Bool { return a == (b != c); }
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 5);
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 1);

    let not = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanNotValue)
        .expect("Boolean-not node");
    let grouped = not
        .children()
        .find(|node| node.kind() == SyntaxKind::GroupedValue)
        .expect("grouped equality operand");
    assert!(
        grouped
            .children()
            .any(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
    );

    for source in [
        "fn bad(a: Bool, b: Bool, c: Bool) -> Bool { return a == b == c; }",
        "fn bad(a: Bool, b: Bool, c: Bool) -> Bool { return a != b == c; }",
        "fn bad(a: Bool, b: Bool, c: Bool) -> Bool { return a == b != c; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);
    }
}

#[test]
fn ordinary_grouping_composes_with_existing_values_and_signed_literals() {
    let source = r#"
record Flag { ready: Bool }
fn make() -> Bool { return true; }
fn f(root: Flag, flag: Bool) -> Bool {
    let literal: Bool = (true);
    let binding: Bool = (flag);
    let call: Bool = (make());
    let field: Bool = (root.ready);
    let construction: Flag = (Flag { ready: true });
    let integer: I64 = (-1);
    return (field);
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 7);
    assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 1);

    let negated = parse("fn f() { let value: I64 = -(1); }");
    assert_eq!(negated.text(), "fn f() { let value: I64 = -(1); }");
    assert!(negated.errors().is_empty(), "{:?}", negated.errors());
    assert_eq!(count(&negated, SyntaxKind::IntegerNegValue), 1);
    assert_eq!(count(&negated, SyntaxKind::GroupedValue), 1);
}

#[test]
fn conditional_grouping_preserves_conditional_context_at_every_depth() {
    let valid_source = r#"
record Flag { ready: Bool }
fn f(flag: Bool) {
    if ((flag)) {}
    while !(flag == false) { break; }
    if (Flag { ready: true }.ready) {}
}
"#;
    let valid = parse(valid_source);
    assert_eq!(valid.text(), valid_source);
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::GroupedValue), 4);
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 1);

    for source in [
        "record Flag { ready: Bool } fn bad() { if (Flag { ready: true }) {} }",
        "record Flag { ready: Bool } fn bad() { if ((Flag { ready: true })) {} }",
        "record Flag { ready: Bool } fn bad() { if !(Flag { ready: true }) {} }",
        "record Flag { ready: Bool } fn bad(flag: Bool) { if flag == (Flag { ready: true }) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(
            count(&parsed, SyntaxKind::RecordConstruction),
            0,
            "standalone conditional construction must not be represented"
        );
    }
}

#[test]
fn empty_tuple_like_and_category_widening_forms_remain_invalid() {
    for source in [
        "fn bad() { let value: Bool = (); }",
        "fn bad(a: Bool, b: Bool) { let value: Bool = (a, b); }",
        "fn bad(f: Bool, x: Bool) { let value: Bool = (f)(x); }",
        "record Flag { ready: Bool } fn bad(root: Flag) { let value: Bool = (root).ready; }",
        "record Flag { ready: Bool } fn make() -> Flag { return Flag { ready: true }; } fn bad() { let Flag { ready: value } = (make()); }",
        "fn bad(flag: Bool) { (flag) = true; }",
        "fn bad(flag: Bool) { (flag); }",
        "fn bad() { let value: (Bool) = true; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly valid: {source}"
        );
    }

    let field =
        parse("record Flag { ready: Bool } fn bad(root: Flag) { let value: Bool = (root).ready; }");
    assert_eq!(count(&field, SyntaxKind::FieldValueUse), 0);

    let indirect = parse("fn bad(f: Bool, x: Bool) { let value: Bool = (f)(x); }");
    assert_eq!(count(&indirect, SyntaxKind::DirectCall), 0);
}

#[test]
fn empty_group_reports_a_value_error_without_swallowing_its_close() {
    let source = "fn f() { let bad: Bool = (); let good: Bool = true; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 2);

    let grouped = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::GroupedValue)
        .expect("empty grouped node");
    let tokens = grouped
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert_eq!(tokens, [SyntaxKind::LParen, SyntaxKind::RParen]);
}

#[test]
fn missing_group_close_preserves_call_argument_comma_and_following_argument() {
    let source = "fn sink(a: Bool, b: Bool) {} fn f(flag: Bool) { sink((flag, true); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::ArgumentList), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanLiteral), 1);
}

#[test]
fn missing_group_close_preserves_statement_semicolons() {
    let declaration = "fn f(flag: Bool) { let first: Bool = (flag; let second: Bool = true; }";
    let parsed = parse(declaration);
    assert_eq!(parsed.text(), declaration);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 2);

    let assignment =
        "fn f(flag: Bool) { let mut value: Bool = true; value = (flag; let after: Bool = true; }";
    let parsed = parse(assignment);
    assert_eq!(parsed.text(), assignment);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::AssignmentStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 2);

    let returned = "fn f(flag: Bool) -> Bool { return (flag; } fn g() -> Bool { return true; }";
    let parsed = parse(returned);
    assert_eq!(parsed.text(), returned);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 2);
    assert_eq!(count(&parsed, SyntaxKind::ReturnStatement), 2);
}

#[test]
fn missing_conditional_group_close_preserves_arm_and_later_statement() {
    let source = "fn f(flag: Bool) { if (flag {} let after: Bool = true; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::IfStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::BlockStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 1);
}

#[test]
fn malformed_nested_groups_preserve_right_brace_and_next_top_level_item() {
    let source = "fn f(flag: Bool) { let value: Bool = ((flag; } fn g() -> Bool { return true; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 2);
    assert_eq!(count(&parsed, SyntaxKind::ReturnStatement), 1);
}
