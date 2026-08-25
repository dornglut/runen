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
fn or_syntax_kinds_append_after_the_accepted_xor_identities() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BooleanAndValue).0, 97);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Caret).0, 98);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerXorValue).0, 99);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Pipe).0, 100);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerOrValue).0, 101);
}

#[test]
fn pipe_is_lossless_and_composite_spellings_remain_repeated_existing_tokens() {
    let ordinary = parse("fn f(a: I64, b: I64) -> I64 { return a /* left */ | /* right */ b; }");
    assert_eq!(
        ordinary.text(),
        "fn f(a: I64, b: I64) -> I64 { return a /* left */ | /* right */ b; }"
    );
    assert!(ordinary.errors().is_empty(), "{:?}", ordinary.errors());
    assert_eq!(count(&ordinary, SyntaxKind::IntegerOrValue), 1);
    assert_eq!(
        nontrivia_kinds(&ordinary)
            .into_iter()
            .filter(|kind| *kind == SyntaxKind::Pipe)
            .count(),
        1
    );

    let assign = parse("fn f(a: I64, b: I64) { a |= b; }");
    assert_eq!(assign.text(), "fn f(a: I64, b: I64) { a |= b; }");
    assert!(!assign.errors().is_empty());
    assert!(
        nontrivia_kinds(&assign)
            .windows(2)
            .any(|window| window == [SyntaxKind::Pipe, SyntaxKind::Eq])
    );

    let double = parse("fn f(a: I64, b: I64) -> I64 { return a || b; }");
    assert_eq!(
        double.text(),
        "fn f(a: I64, b: I64) -> I64 { return a || b; }"
    );
    assert!(!double.errors().is_empty());
    assert!(
        nontrivia_kinds(&double)
            .windows(2)
            .any(|window| window == [SyntaxKind::Pipe, SyntaxKind::Pipe])
    );

    let triple = parse("fn f(a: I64, b: I64) -> I64 { return a ||| b; }");
    assert_eq!(
        triple.text(),
        "fn f(a: I64, b: I64) -> I64 { return a ||| b; }"
    );
    assert!(!triple.errors().is_empty());
    assert!(
        nontrivia_kinds(&triple)
            .windows(3)
            .any(|window| window == [SyntaxKind::Pipe, SyntaxKind::Pipe, SyntaxKind::Pipe])
    );
}

#[test]
fn or_is_bounded_and_grouping_is_the_only_way_to_repeat_it() {
    for source in [
        "fn f(a: I64, b: I64, c: I64) -> I64 { return a | b | c; }",
        "fn f(a: I64, b: I64, c: I64, d: I64) -> I64 { return a | b | c | d; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::IntegerOrValue), 1);
    }

    for source in [
        "fn f(a: I64, b: I64, c: I64) -> I64 { return (a | b) | c; }",
        "fn f(a: I64, b: I64, c: I64) -> I64 { return a | (b | c); }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::IntegerOrValue), 2);
        assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
    }
}

#[test]
fn xor_binds_tighter_than_or_and_or_binds_tighter_than_equality() {
    let left_xor = parse("fn f(a: I64, b: I64, c: I64) -> I64 { return a ^ b | c; }");
    assert!(left_xor.errors().is_empty(), "{:?}", left_xor.errors());
    let or = left_xor
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerOrValue)
        .expect("or");
    assert_eq!(
        or.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IntegerXorValue, SyntaxKind::IdentifierUse]
    );

    let right_xor = parse("fn f(a: I64, b: I64, c: I64) -> I64 { return a | b ^ c; }");
    assert!(right_xor.errors().is_empty(), "{:?}", right_xor.errors());
    let or = right_xor
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerOrValue)
        .expect("or");
    assert_eq!(
        or.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::IntegerXorValue]
    );

    let left_equality = parse("fn f(a: I64, b: I64, c: I64) -> Bool { return a | b == c; }");
    assert!(
        left_equality.errors().is_empty(),
        "{:?}",
        left_equality.errors()
    );
    let equality = left_equality
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("equality");
    assert_eq!(
        equality
            .children()
            .map(|node| node.kind())
            .collect::<Vec<_>>(),
        [SyntaxKind::IntegerOrValue, SyntaxKind::IdentifierUse]
    );

    let right_equality = parse("fn f(a: I64, b: I64, c: I64) -> Bool { return a == b | c; }");
    assert!(
        right_equality.errors().is_empty(),
        "{:?}",
        right_equality.errors()
    );
    let equality = right_equality
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("equality");
    assert_eq!(
        equality
            .children()
            .map(|node| node.kind())
            .collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::IntegerOrValue]
    );

    let conjunction =
        parse("fn f(a: I64, b: I64, c: I64, d: I64) -> Bool { return a | b == c && d == d; }");
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
fn or_preserves_tighter_signed_literal_prefix_additive_and_xor_boundaries() {
    let parsed = parse("fn f(a: I64, b: I64) -> I64 { return ~a + 2 ^ -b | -3; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let or = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerOrValue)
        .expect("or");
    assert_eq!(
        or.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [
            SyntaxKind::IntegerXorValue,
            SyntaxKind::DecimalIntegerLiteral
        ]
    );
    let xor = or
        .children()
        .find(|node| node.kind() == SyntaxKind::IntegerXorValue)
        .expect("xor");
    assert_eq!(
        xor.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IntegerAddValue, SyntaxKind::IntegerNegValue]
    );

    let prefix_start = parse("fn f(a: I64) -> I64 { return |a; }");
    assert!(!prefix_start.errors().is_empty());
    assert_eq!(count(&prefix_start, SyntaxKind::IntegerOrValue), 0);
}

#[test]
fn conditional_or_keeps_standalone_record_construction_excluded_everywhere() {
    let valid =
        parse("record Box { value: I64 } fn f(a: I64) { if Box { value: 2 }.value | a {} }");
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&valid, SyntaxKind::IntegerOrValue), 1);

    for source in [
        "record Box { value: I64 } fn f(a: I64) { if Box { value: 2 } | a {} }",
        "record Box { value: I64 } fn f(a: I64) { if a | Box { value: 2 } {} }",
        "record Box { value: I64 } fn f(a: I64) { if (Box { value: 2 }) | a {} }",
        "record Box { value: I64 } fn f(a: I64) { if a | (Box { value: 2 }) {} }",
        "record Box { value: I64 } fn f(a: I64) { while a | (Box { value: 2 }) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
    }
}

#[test]
fn or_does_not_widen_other_categories_and_no_pipe_source_keeps_existing_tree_shape() {
    let pattern = parse(
        "record Pair { value: I64 } fn f(a: Pair, b: Pair) { let Pair { value: x } = a | b; }",
    );
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::IntegerOrValue), 0);

    let statement = parse("fn f(a: I64, b: I64) { a | b; }");
    assert!(!statement.errors().is_empty());
    assert_eq!(count(&statement, SyntaxKind::IntegerOrValue), 0);

    let unchanged = parse("fn f(a: I64, b: I64) -> I64 { return ~a + 2 ^ -b; }");
    assert!(unchanged.errors().is_empty(), "{:?}", unchanged.errors());
    assert_eq!(count(&unchanged, SyntaxKind::IntegerOrValue), 0);
    assert_eq!(count(&unchanged, SyntaxKind::IntegerXorValue), 1);
    let xor = unchanged
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::IntegerXorValue)
        .expect("existing XOR tree");
    assert_eq!(
        xor.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IntegerAddValue, SyntaxKind::IntegerNegValue]
    );
}
