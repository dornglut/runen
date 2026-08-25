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
fn integer_negation_syntax_kind_is_append_only() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Minus).0, 57);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerMulValue).0, 92);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerNegValue).0, 93);
}

#[test]
fn signed_integer_and_floating_literals_keep_priority_across_trivia() {
    let source = "fn f() { let a: I8 = -1; let b: I8 = - 1; let c: F32 = -1.0; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 2);
    assert_eq!(count(&parsed, SyntaxKind::DecimalFloatingLiteral), 1);
    assert_eq!(count(&parsed, SyntaxKind::IntegerNegValue), 0);
}

#[test]
fn parenthesized_binding_and_double_minus_form_integer_negation() {
    let source = "fn f(value: I8) { let grouped: I8 = -(1); let binding: I8 = -value; let nested: I8 = --1; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerNegValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 2);

    let nested = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::IntegerNegValue)
        .last()
        .expect("outer negation for --1");
    assert!(
        nested
            .children()
            .any(|child| child.kind() == SyntaxKind::DecimalIntegerLiteral),
        "--1 must contain the existing signed literal -1"
    );
}

#[test]
fn binary_subtraction_with_negative_literal_is_not_reinterpreted() {
    for source in [
        "fn f(a: I8) -> I8 { return a - -1; }",
        "fn f(a: I8) -> I8 { return a--1; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::IntegerSubValue), 1);
        assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 1);
        assert_eq!(count(&parsed, SyntaxKind::IntegerNegValue), 0);
    }
}

#[test]
fn boolean_and_integer_prefixes_recurse_rightward_without_absorbing_looser_tiers() {
    let source = "fn f(flag: Bool, value: I8, other: I8) { let a: I8 = -!flag; let b: Bool = !-value; let c: I8 = -value * other; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerNegValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::IntegerMulValue), 1);

    let multiplication = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerMulValue)
        .expect("multiplication");
    assert_eq!(
        multiplication
            .children()
            .map(|child| child.kind())
            .collect::<Vec<_>>(),
        [SyntaxKind::IntegerNegValue, SyntaxKind::IdentifierUse]
    );
}

#[test]
fn conditional_integer_negation_keeps_standalone_record_construction_excluded() {
    let valid = parse("record Box { value: I8 } fn f() { if -Box { value: 1 }.value {} }");
    assert_eq!(
        valid.text(),
        "record Box { value: I8 } fn f() { if -Box { value: 1 }.value {} }"
    );
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::IntegerNegValue), 1);
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 1);

    for source in [
        "record Box { value: I8 } fn f() { if -Box { value: 1 } {} }",
        "record Box { value: I8 } fn f() { while -(Box { value: 1 }) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
        assert!(count(&parsed, SyntaxKind::IntegerNegValue) >= 1);
    }
}

#[test]
fn integer_negation_does_not_widen_pattern_statement_or_postfix_categories() {
    let pattern = parse(
        "record Box { value: I8 } fn make() -> Box { return Box { value: 1 }; } fn f() { let Box { value: x } = -make(); }",
    );
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::IntegerNegValue), 0);

    let statement = parse("fn f(value: I8) { -value; }");
    assert!(!statement.errors().is_empty());
    assert_eq!(count(&statement, SyntaxKind::IntegerNegValue), 0);

    let postfix = parse("record Box { value: I8 } fn f(root: Box) { let x: I8 = (-root).value; }");
    assert!(!postfix.errors().is_empty());
    assert_eq!(count(&postfix, SyntaxKind::FieldValueUse), 0);
}
