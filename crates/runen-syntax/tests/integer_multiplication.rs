use runen_syntax::{Parse, SyntaxErrorKind, SyntaxKind, parse_source};

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
fn multiplication_syntax_kinds_are_append_only() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Minus).0, 57);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Plus).0, 88);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::AddValue).0, 89);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerSubValue).0, 90);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Star).0, 91);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerMulValue).0, 92);
}

#[test]
fn standalone_star_is_lossless_without_disturbing_comment_delimiters() {
    let source = "fn f(a: I64, b: I64) -> I64 { return a /* left */ * /* right */ b; } /* outer /* nested * */ done */";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::IntegerMulValue), 1);
    assert_eq!(
        nontrivia_kinds(&parsed)
            .into_iter()
            .filter(|kind| *kind == SyntaxKind::Star)
            .count(),
        1
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::BlockComment)
            .count(),
        3
    );
}

#[test]
fn slash_remains_unsupported_and_double_star_and_star_equals_are_not_operators() {
    let slash = parse("fn f(a: I64, b: I64) -> I64 { return a / b; }");
    assert_eq!(
        slash.text(),
        "fn f(a: I64, b: I64) -> I64 { return a / b; }"
    );
    assert!(
        slash
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken)
    );

    for source in [
        "fn f(a: I64, b: I64) -> I64 { return a ** b; }",
        "fn f(a: I64, b: I64) -> I64 { return a *= b; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::IntegerMulValue), 0);
    }

    let double = parse("fn f(a: I64, b: I64) -> I64 { return a ** b; }");
    assert!(
        nontrivia_kinds(&double)
            .windows(2)
            .any(|window| window == [SyntaxKind::Star, SyntaxKind::Star])
    );

    let assign = parse("fn f(a: I64, b: I64) -> I64 { return a *= b; }");
    assert!(
        nontrivia_kinds(&assign)
            .windows(2)
            .any(|window| window == [SyntaxKind::Star, SyntaxKind::Eq])
    );
}

#[test]
fn multiplicative_tier_is_bounded_and_tighter_than_additive() {
    for source in [
        "fn f(a: I64, b: I64, c: I64) -> I64 { return a * b * c; }",
        "fn f(a: I64, b: I64, c: I64, d: I64) -> I64 { return a * b * c * d; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::IntegerMulValue), 1);
    }

    let right = parse("fn f(a: I64, b: I64, c: I64) -> I64 { return a + b * c; }");
    assert!(right.errors().is_empty(), "{:?}", right.errors());
    let add = right
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::AddValue)
        .expect("addition");
    let children = add.children().map(|node| node.kind()).collect::<Vec<_>>();
    assert_eq!(
        children,
        [SyntaxKind::IdentifierUse, SyntaxKind::IntegerMulValue]
    );

    let left = parse("fn f(a: I64, b: I64, c: I64) -> I64 { return a * b + c; }");
    assert!(left.errors().is_empty(), "{:?}", left.errors());
    let add = left
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::AddValue)
        .expect("addition");
    let children = add.children().map(|node| node.kind()).collect::<Vec<_>>();
    assert_eq!(
        children,
        [SyntaxKind::IntegerMulValue, SyntaxKind::IdentifierUse]
    );
}

#[test]
fn grouping_explicitly_repeats_or_overrides_multiplicative_nesting() {
    for (source, multiplications, additions) in [
        (
            "fn f(a: I64, b: I64, c: I64) -> I64 { return (a * b) * c; }",
            2,
            0,
        ),
        (
            "fn f(a: I64, b: I64, c: I64) -> I64 { return a * (b * c); }",
            2,
            0,
        ),
        (
            "fn f(a: I64, b: I64, c: I64) -> I64 { return (a + b) * c; }",
            1,
            1,
        ),
        (
            "fn f(a: I64, b: I64, c: I64) -> I64 { return a * (b - c); }",
            1,
            0,
        ),
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::IntegerMulValue), multiplications);
        assert_eq!(count(&parsed, SyntaxKind::AddValue), additions);
        if source.contains("b - c") {
            assert_eq!(count(&parsed, SyntaxKind::IntegerSubValue), 1);
        }
    }
}

#[test]
fn multiplication_preserves_signed_literal_and_prefix_boundaries() {
    let source = "fn f(a: I64) -> I64 { return a * -2; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let mul = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerMulValue)
        .expect("multiplication");
    assert_eq!(
        mul.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::DecimalIntegerLiteral,]
    );

    let prefix = parse("fn f(a: I64, b: I64) -> Bool { return !a * b; }");
    assert_eq!(
        prefix.text(),
        "fn f(a: I64, b: I64) -> Bool { return !a * b; }"
    );
    assert!(prefix.errors().is_empty(), "{:?}", prefix.errors());
    let mul = prefix
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerMulValue)
        .expect("multiplication");
    assert_eq!(
        mul.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::BooleanNotValue, SyntaxKind::IdentifierUse]
    );
}

#[test]
fn conditional_multiplication_keeps_standalone_record_construction_excluded() {
    let valid =
        parse("record Box { value: I64 } fn f(a: I64) { if Box { value: 2 }.value * a {} }");
    assert_eq!(
        valid.text(),
        "record Box { value: I64 } fn f(a: I64) { if Box { value: 2 }.value * a {} }"
    );
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&valid, SyntaxKind::IntegerMulValue), 1);

    for source in [
        "record Box { value: I64 } fn f(a: I64) { if Box { value: 2 } * a {} }",
        "record Box { value: I64 } fn f(a: I64) { if a * Box { value: 2 } {} }",
        "record Box { value: I64 } fn f(a: I64) { while a * (Box { value: 2 }) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
    }
}

#[test]
fn multiplication_does_not_widen_pattern_scrutinee_or_statement_categories() {
    let pattern = parse(
        "record Pair { value: I64 } fn f(a: Pair, b: Pair) { let Pair { value: x } = a * b; }",
    );
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::IntegerMulValue), 0);

    let statement = parse("fn f(a: I64, b: I64) { a * b; }");
    assert!(!statement.errors().is_empty());
    assert_eq!(count(&statement, SyntaxKind::IntegerMulValue), 0);
}
