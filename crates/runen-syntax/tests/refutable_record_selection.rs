use runen_syntax::{SyntaxKind, parse_source};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

fn count(parsed: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

#[test]
fn parses_parenthesized_direct_root_and_record_construction_scrutinees() {
    let source = "record R { tag: Bool } fn f(root: R) { if let R { tag: true } = (root) {} else {} if let R { tag: false } = (R { tag: true }) {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordSelectionStatement), 2);
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordPattern), 2);
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordPatternField), 2);
    assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 0);
    assert_eq!(count(&parsed, SyntaxKind::IfStatement), 0);
}

#[test]
fn parses_nested_refutable_patterns_rest_and_signed_integer_tests() {
    let source = "record Inner { code: I8, keep: Bool } record Outer { inner: Inner, flag: Bool } fn f(root: Outer) { if let Outer { inner: Inner { code: - 1, .. }, flag: true } = (root) {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordPattern), 2);
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordPatternField), 3);
    assert_eq!(count(&parsed, SyntaxKind::RecordPatternRest), 1);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanLiteral), 1);
}

#[test]
fn reuses_only_the_existing_bounded_scrutinee_categories() {
    let source = "record R { tag: Bool } record Box { value: R } fn make() -> R { return R { tag: true }; } fn f(boxed: Box) { if let R { tag: true } = (make()) {} if let R { tag: false } = (boxed.value) {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 0);
}

#[test]
fn zero_literal_refutable_pattern_is_syntax_valid_for_hir_rejection() {
    let source = "record R { tag: Bool } fn f(root: R) { if let R { tag: bound } = (root) {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordSelectionStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanLiteral), 0);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 0);
}

#[test]
fn ordinary_if_and_irrefutable_record_patterns_remain_distinct() {
    let ordinary = parse("fn f(flag: Bool) { if flag {} }");
    assert!(ordinary.errors().is_empty(), "{:?}", ordinary.errors());
    assert_eq!(count(&ordinary, SyntaxKind::IfStatement), 1);
    assert_eq!(
        count(&ordinary, SyntaxKind::RefutableRecordSelectionStatement),
        0
    );

    let irrefutable_literal =
        parse("record R { tag: Bool } fn f(root: R) { let R { tag: true } = root; }");
    assert!(!irrefutable_literal.errors().is_empty());
    assert_eq!(
        count(&irrefutable_literal, SyntaxKind::RefutableRecordPattern),
        0
    );
}

#[test]
fn rejects_floating_literal_test_targets() {
    let source = "record R { value: F32 } fn f(root: R) { if let R { value: 1.0 } = (root) {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::DecimalFloatingLiteral), 0);
}

#[test]
fn statement_local_parentheses_do_not_widen_scrutinees_to_general_values() {
    let source = "record R { tag: Bool } fn f(root: R, other: R) { if let R { tag: true } = (root + other) {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 0);
    assert_eq!(count(&parsed, SyntaxKind::AddValue), 0);
}

#[test]
fn requires_the_statement_local_scrutinee_parentheses() {
    let source = "record R { tag: Bool } fn f(root: R) { if let R { tag: true } = root {} }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
}
