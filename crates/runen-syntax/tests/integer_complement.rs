use runen_syntax::{ExpectedSyntax, Parse, SyntaxErrorKind, SyntaxKind, parse_source};

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
fn integer_complement_syntax_kinds_are_append_only() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerNegValue).0, 93);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Tilde).0, 94);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::IntegerComplementValue).0,
        95
    );
}

#[test]
fn standalone_tilde_and_tilde_equals_follow_the_accepted_punctuation_boundary() {
    let source = "fn f(value: I8) { let complemented: I8 = ~value; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerComplementValue), 1);
    assert!(parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::Tilde && token.text() == "~"));

    let malformed = parse("fn f(value: I8) { let x: I8 = ~= value; }");
    assert_eq!(malformed.text(), "fn f(value: I8) { let x: I8 = ~= value; }");
    let kinds = malformed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&SyntaxKind::Tilde));
    assert!(kinds.contains(&SyntaxKind::Eq));
    assert!(!malformed.errors().is_empty(), "~= is not one operator");
}

#[test]
fn complement_prefixes_recurse_rightward_and_keep_signed_literals_intact() {
    let source = "fn f(flag: Bool, value: I8) { let a: I8 = ~~value; let b: I8 = ~-1; let c: I8 = -~value; let d: I8 = ~!flag; let e: Bool = !~value; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerComplementValue), 6);
    assert_eq!(count(&parsed, SyntaxKind::IntegerNegValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 1);

    let signed = parsed
        .syntax()
        .descendants()
        .find(|node| {
            node.kind() == SyntaxKind::IntegerComplementValue
                && node
                    .children()
                    .any(|child| child.kind() == SyntaxKind::DecimalIntegerLiteral)
        })
        .expect("~-1 must retain the signed literal as the complement operand");
    assert!(signed
        .children()
        .any(|child| child.kind() == SyntaxKind::DecimalIntegerLiteral));
}

#[test]
fn complement_prefix_stays_tighter_than_multiplicative_additive_and_equality_tiers() {
    let source = "fn f(value: I8, other: I8) { let a: I8 = ~value * other; let b: I8 = ~value + other; let c: Bool = ~value == other; let d: I8 = ~(value + other); }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

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
        [SyntaxKind::IntegerComplementValue, SyntaxKind::IdentifierUse]
    );
    assert_eq!(count(&parsed, SyntaxKind::IntegerAddValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
}

#[test]
fn conditional_complement_keeps_standalone_record_construction_excluded() {
    let valid = parse("record Box { value: I8 } fn f() { if ~Box { value: 1 }.value {} }");
    assert_eq!(
        valid.text(),
        "record Box { value: I8 } fn f() { if ~Box { value: 1 }.value {} }"
    );
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::IntegerComplementValue), 1);
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 1);

    for source in [
        "record Box { value: I8 } fn f() { if ~Box { value: 1 } {} }",
        "record Box { value: I8 } fn f() { while ~(Box { value: 1 }) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
        assert!(count(&parsed, SyntaxKind::IntegerComplementValue) >= 1);
    }
}

#[test]
fn bare_tilde_is_lossless_incomplete_complement_with_missing_value_recovery() {
    let source = "fn f() -> I8 { return ~; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::IntegerComplementValue), 1);
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Value)
    }));
}

#[test]
fn complement_does_not_widen_pattern_statement_or_postfix_categories() {
    let pattern = parse(
        "record Box { value: I8 } fn make() -> Box { return Box { value: 1 }; } fn f() { let Box { value: x } = ~make(); }",
    );
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::IntegerComplementValue), 0);

    let statement = parse("fn f(value: I8) { ~value; }");
    assert!(!statement.errors().is_empty());
    assert_eq!(count(&statement, SyntaxKind::IntegerComplementValue), 0);

    let postfix = parse("record Box { value: I8 } fn f(root: Box) { let x: I8 = (~root).value; }");
    assert!(!postfix.errors().is_empty());
    assert_eq!(count(&postfix, SyntaxKind::FieldValueUse), 0);
}