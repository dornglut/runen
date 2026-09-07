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
fn strict_order_token_is_append_only_and_lossless() {
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::RefutableRecordPatternField).0,
        119
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Less).0, 120);

    let source = "fn f(left: I32, right: I32) -> Bool { return left < right; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert!(nontrivia_kinds(&parsed).contains(&SyntaxKind::Less));
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);
}

#[test]
fn strict_order_occupies_the_existing_bounded_comparison_position() {
    let source = "fn f(a: I32, b: I32, c: I32, flag: Bool) { let first: Bool = a | b < c && flag; let nested: Bool = (a < b) == flag; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let conjunction = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanAndValue)
        .expect("Boolean conjunction");
    let comparison = conjunction
        .children()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("comparison is tighter than conjunction");
    assert!(
        comparison
            .children()
            .any(|node| node.kind() == SyntaxKind::IntegerOrValue),
        "bitwise OR must remain tighter than comparison"
    );

    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
}

#[test]
fn strict_order_is_available_in_ordinary_and_conditional_values() {
    let source = "fn f(left: I32, right: I32) -> Bool { let ordered: Bool = left < right; if left < right {} while left < right { break; } return ordered; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 3);
    assert_eq!(count(&parsed, SyntaxKind::IfStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::WhileStatement), 1);
}

#[test]
fn mixed_and_repeated_ungrouped_comparison_chains_remain_invalid() {
    for source in [
        "fn bad(a: I32, b: I32, c: I32) -> Bool { return a < b < c; }",
        "fn bad(a: I32, b: I32, c: I32) -> Bool { return a < b == c; }",
        "fn bad(a: I32, b: I32, c: I32) -> Bool { return a == b < c; }",
        "fn bad(a: I32, b: I32, c: I32) -> Bool { return a != b < c; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);
    }
}

#[test]
fn grouping_explicitly_nests_comparison_results() {
    for source in [
        "fn f(a: I32, b: I32, flag: Bool) -> Bool { return (a < b) == flag; }",
        "fn f(a: I32, b: I32, flag: Bool) -> Bool { return flag != (a < b); }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 2);
        assert_eq!(count(&parsed, SyntaxKind::GroupedValue), 1);
    }
}

#[test]
fn unrepresented_ordering_spellings_remain_invalid_and_lossless() {
    let non_strict = "fn bad(a: I32, b: I32) -> Bool { return a <= b; }";
    let parsed = parse(non_strict);
    assert_eq!(parsed.text(), non_strict);
    assert!(!parsed.errors().is_empty());
    assert!(
        nontrivia_kinds(&parsed)
            .windows(2)
            .any(|window| window == [SyntaxKind::Less, SyntaxKind::Eq])
    );

    let greater = "fn bad(a: I32, b: I32) -> Bool { return a > b; }";
    let parsed = parse(greater);
    assert_eq!(parsed.text(), greater);
    assert!(!parsed.errors().is_empty());
    assert!(nontrivia_kinds(&parsed).contains(&SyntaxKind::ErrorToken));
}
