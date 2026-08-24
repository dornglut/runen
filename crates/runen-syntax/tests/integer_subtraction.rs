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

fn nontrivia_kinds(parsed: &Parse) -> Vec<SyntaxKind> {
    parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.kind())
        .collect()
}

#[test]
fn subtraction_syntax_kind_is_append_only_and_minus_is_reused_losslessly() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Minus).0, 57);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerAddValue).0, 89);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerSubValue).0, 90);

    let source = "fn sub(a: I8, b: I8) -> I8 { return a /* left */ - /* right */ b; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerSubValue), 1);
    assert!(nontrivia_kinds(&parsed).contains(&SyntaxKind::Minus));
}

#[test]
fn arrow_remains_longest_match_and_no_new_minus_token_exists() {
    let source = "fn f() -> I8 { return -1; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let kinds = nontrivia_kinds(&parsed);
    assert!(kinds.contains(&SyntaxKind::Arrow));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::Minus)
            .count(),
        1
    );
}

#[test]
fn subtraction_preserves_signed_literal_right_operand_with_or_without_whitespace() {
    for source in [
        "fn f(a: I8) -> I8 { return a - -1; }",
        "fn f(a: I8) -> I8 { return a--1; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::IntegerSubValue), 1);
        assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 1);
        assert_eq!(
            nontrivia_kinds(&parsed)
                .iter()
                .filter(|kind| **kind == SyntaxKind::Minus)
                .count(),
            2
        );
    }
}

#[test]
fn minus_does_not_create_unary_negation_decrement_or_compound_assignment_forms() {
    for source in [
        "fn bad() -> I8 { return -(1); }",
        "fn bad(value: I8) -> I8 { return -value; }",
        "fn bad() -> I8 { return --1; }",
        "fn bad(a: I8) { let value: I8 = a -= 1; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
    }

    let compound = parse("fn bad(a: I8) { let value: I8 = a -= 1; }");
    assert!(
        nontrivia_kinds(&compound)
            .windows(2)
            .any(|window| window == [SyntaxKind::Minus, SyntaxKind::Eq])
    );
}

#[test]
fn additive_tier_selects_operation_specific_nodes_between_prefix_and_equality() {
    let source = "fn f(a: I8, b: I8, flag: Bool) { let sub: I8 = !a - b; let compared: Bool = a - b == flag; let add: I8 = a + b; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerSubValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::IntegerAddValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);

    let equality = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("equality node");
    assert!(
        equality
            .children()
            .any(|node| node.kind() == SyntaxKind::IntegerSubValue)
    );

    let first_sub = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerSubValue)
        .expect("subtraction node");
    assert!(
        first_sub
            .children()
            .any(|node| node.kind() == SyntaxKind::BooleanNotValue)
    );
}

#[test]
fn ungrouped_repeated_and_mixed_additive_chains_remain_invalid() {
    for source in [
        "fn bad(a: I8, b: I8, c: I8) -> I8 { return a - b - c; }",
        "fn bad(a: I8, b: I8, c: I8) -> I8 { return a + b - c; }",
        "fn bad(a: I8, b: I8, c: I8) -> I8 { return a - b + c; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(
            count(&parsed, SyntaxKind::IntegerAddValue)
                + count(&parsed, SyntaxKind::IntegerSubValue),
            1
        );
    }
}

#[test]
fn grouping_represents_explicit_nested_addition_and_subtraction_trees() {
    let source = "fn f(a: I8, b: I8, c: I8) { let x: I8 = (a + b) - c; let y: I8 = (a - b) + c; let z: I8 = a - (b - c); let w: I8 = a + (b - c); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerAddValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::IntegerSubValue), 5);
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 4);
}

#[test]
fn conditional_subtraction_preserves_existing_construction_exclusion() {
    let valid_source = r#"
record Flag { ready: Bool }
fn choose(flag: Bool) {
    if Flag { ready: true }.ready - flag {}
    while flag - Flag { ready: false }.ready {}
}
"#;
    let valid = parse(valid_source);
    assert_eq!(valid.text(), valid_source);
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::IntegerSubValue), 2);
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 2);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 2);

    for source in [
        "record Flag { ready: Bool } fn bad(flag: Bool) { if Flag { ready: true } - flag {} }",
        "record Flag { ready: Bool } fn bad(flag: Bool) { while flag - Flag { ready: true } {} }",
        "record Flag { ready: Bool } fn bad(flag: Bool) { if (Flag { ready: true } - flag) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
    }
}

#[test]
fn subtraction_does_not_widen_field_receivers_or_pattern_scrutinees() {
    let field_source = "fn bad(a: I8, b: I8) { let value: I8 = (a - b).ready; }";
    let field = parse(field_source);
    assert_eq!(field.text(), field_source);
    assert!(!field.errors().is_empty());
    assert_eq!(count(&field, SyntaxKind::FieldValueUse), 0);
    assert_eq!(count(&field, SyntaxKind::IntegerSubValue), 1);

    let pattern_source =
        "record Pair { left: I8 } fn bad(a: Pair, b: Pair) { let Pair { left: x } = a - b; }";
    let pattern = parse(pattern_source);
    assert_eq!(pattern.text(), pattern_source);
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::IntegerSubValue), 0);
}

#[test]
fn malformed_subtraction_keeps_following_delimiters_and_statements_lossless() {
    let call_source = "fn sink(a: I8, b: I8) {} fn f(a: I8, b: I8) { sink(a -, b); }";
    let call = parse(call_source);
    assert_eq!(call.text(), call_source);
    assert!(!call.errors().is_empty());
    assert_eq!(count(&call, SyntaxKind::ArgumentList), 1);
    assert!(nontrivia_kinds(&call).contains(&SyntaxKind::Comma));

    let statement_source = "fn f(a: I8) { let first: I8 = a - ; let second: I8 = a; }";
    let statement = parse(statement_source);
    assert_eq!(statement.text(), statement_source);
    assert!(!statement.errors().is_empty());
    assert_eq!(count(&statement, SyntaxKind::LocalDeclaration), 2);

    let condition_source = "fn f(flag: Bool) { if flag - {} let after: Bool = flag; }";
    let condition = parse(condition_source);
    assert_eq!(condition.text(), condition_source);
    assert!(!condition.errors().is_empty());
    assert_eq!(count(&condition, SyntaxKind::IfStatement), 1);
    assert_eq!(count(&condition, SyntaxKind::BlockStatement), 1);
    assert_eq!(count(&condition, SyntaxKind::LocalDeclaration), 1);
}
