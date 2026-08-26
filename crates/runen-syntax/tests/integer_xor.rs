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
fn xor_syntax_kinds_are_append_only() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BooleanAndValue).0, 97);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Caret).0, 98);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerXorValue).0, 99);
}

#[test]
fn caret_is_lossless_and_composite_spellings_stay_separate_tokens() {
    let ordinary = parse("fn f(a: I64, b: I64) -> I64 { return a /* left */ ^ /* right */ b; }");
    assert_eq!(
        ordinary.text(),
        "fn f(a: I64, b: I64) -> I64 { return a /* left */ ^ /* right */ b; }"
    );
    assert!(ordinary.errors().is_empty(), "{:?}", ordinary.errors());
    assert_eq!(count(&ordinary, SyntaxKind::IntegerXorValue), 1);
    assert_eq!(
        nontrivia_kinds(&ordinary)
            .into_iter()
            .filter(|kind| *kind == SyntaxKind::Caret)
            .count(),
        1
    );

    let assign = parse("fn f(a: I64, b: I64) { a ^= b; }");
    assert_eq!(assign.text(), "fn f(a: I64, b: I64) { a ^= b; }");
    assert!(!assign.errors().is_empty());
    assert!(
        nontrivia_kinds(&assign)
            .windows(2)
            .any(|window| window == [SyntaxKind::Caret, SyntaxKind::Eq])
    );
    assert_eq!(count(&assign, SyntaxKind::IntegerXorValue), 0);

    let double = parse("fn f(a: I64, b: I64) -> I64 { return a ^^ b; }");
    assert_eq!(
        double.text(),
        "fn f(a: I64, b: I64) -> I64 { return a ^^ b; }"
    );
    assert!(!double.errors().is_empty());
    assert!(
        nontrivia_kinds(&double)
            .windows(2)
            .any(|window| window == [SyntaxKind::Caret, SyntaxKind::Caret])
    );
}

#[test]
fn xor_is_bounded_and_grouping_is_the_only_way_to_repeat_it() {
    for source in [
        "fn f(a: I64, b: I64, c: I64) -> I64 { return a ^ b ^ c; }",
        "fn f(a: I64, b: I64, c: I64, d: I64) -> I64 { return a ^ b ^ c ^ d; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::IntegerXorValue), 1);
    }

    for source in [
        "fn f(a: I64, b: I64, c: I64) -> I64 { return (a ^ b) ^ c; }",
        "fn f(a: I64, b: I64, c: I64) -> I64 { return a ^ (b ^ c); }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::IntegerXorValue), 2);
        assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
    }
}

#[test]
fn additive_binds_tighter_than_xor() {
    let left = parse("fn f(a: I64, b: I64, c: I64) -> I64 { return a + b ^ c; }");
    assert!(left.errors().is_empty(), "{:?}", left.errors());
    let xor = left
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerXorValue)
        .expect("xor");
    assert_eq!(
        xor.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::AddValue, SyntaxKind::IdentifierUse]
    );

    let right = parse("fn f(a: I64, b: I64, c: I64) -> I64 { return a ^ b - c; }");
    assert!(right.errors().is_empty(), "{:?}", right.errors());
    let xor = right
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerXorValue)
        .expect("xor");
    assert_eq!(
        xor.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::IntegerSubValue]
    );
}

#[test]
fn xor_binds_tighter_than_equality_and_equality_binds_tighter_than_conjunction() {
    let left = parse("fn f(a: I64, b: I64, c: I64) -> Bool { return a ^ b == c; }");
    assert!(left.errors().is_empty(), "{:?}", left.errors());
    let equality = left
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("equality");
    assert_eq!(
        equality
            .children()
            .map(|node| node.kind())
            .collect::<Vec<_>>(),
        [SyntaxKind::IntegerXorValue, SyntaxKind::IdentifierUse]
    );

    let right = parse("fn f(a: I64, b: I64, c: I64) -> Bool { return a == b ^ c; }");
    assert!(right.errors().is_empty(), "{:?}", right.errors());
    let equality = right
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("equality");
    assert_eq!(
        equality
            .children()
            .map(|node| node.kind())
            .collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::IntegerXorValue]
    );

    let conjunction =
        parse("fn f(a: I64, b: I64, c: I64, d: I64) -> Bool { return a ^ b == c && d == d; }");
    assert!(
        conjunction.errors().is_empty(),
        "{:?}",
        conjunction.errors()
    );
    let and = conjunction
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanAndValue)
        .expect("conjunction");
    assert_eq!(
        and.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [
            SyntaxKind::BooleanEqualityValue,
            SyntaxKind::BooleanEqualityValue,
        ]
    );
}

#[test]
fn xor_preserves_signed_literal_and_prefix_boundaries() {
    let literal = parse("fn f(a: I64) -> I64 { return a ^ -2; }");
    assert!(literal.errors().is_empty(), "{:?}", literal.errors());
    let xor = literal
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerXorValue)
        .expect("xor");
    assert_eq!(
        xor.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::DecimalIntegerLiteral]
    );

    let prefix = parse("fn f(a: I64, b: I64) -> I64 { return ~a ^ -b; }");
    assert!(prefix.errors().is_empty(), "{:?}", prefix.errors());
    let xor = prefix
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerXorValue)
        .expect("xor");
    assert_eq!(
        xor.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [
            SyntaxKind::IntegerComplementValue,
            SyntaxKind::IntegerNegValue,
        ]
    );

    let prefix_start = parse("fn f(a: I64) -> I64 { return ^a; }");
    assert!(!prefix_start.errors().is_empty());
    assert_eq!(count(&prefix_start, SyntaxKind::IntegerXorValue), 0);
}

#[test]
fn conditional_xor_keeps_standalone_record_construction_excluded() {
    let valid =
        parse("record Box { value: I64 } fn f(a: I64) { if Box { value: 2 }.value ^ a {} }");
    assert_eq!(
        valid.text(),
        "record Box { value: I64 } fn f(a: I64) { if Box { value: 2 }.value ^ a {} }"
    );
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&valid, SyntaxKind::IntegerXorValue), 1);

    for source in [
        "record Box { value: I64 } fn f(a: I64) { if Box { value: 2 } ^ a {} }",
        "record Box { value: I64 } fn f(a: I64) { if a ^ Box { value: 2 } {} }",
        "record Box { value: I64 } fn f(a: I64) { if (Box { value: 2 }) ^ a {} }",
        "record Box { value: I64 } fn f(a: I64) { while a ^ (Box { value: 2 }) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
    }
}

#[test]
fn xor_does_not_widen_pattern_scrutinee_or_statement_categories() {
    let pattern = parse(
        "record Pair { value: I64 } fn f(a: Pair, b: Pair) { let Pair { value: x } = a ^ b; }",
    );
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::IntegerXorValue), 0);

    let statement = parse("fn f(a: I64, b: I64) { a ^ b; }");
    assert!(!statement.errors().is_empty());
    assert_eq!(count(&statement, SyntaxKind::IntegerXorValue), 0);
}
