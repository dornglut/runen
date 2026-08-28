use runen_syntax::{Parse, SyntaxKind, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count_nodes(parsed: &Parse, kind: SyntaxKind) -> usize {
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

fn count_tokens(parsed: &Parse, kind: SyntaxKind) -> usize {
    parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == kind)
        .count()
}

#[test]
fn slash_is_append_only_without_moving_established_raw_kinds() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Star).0, 91);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::MulValue).0, 92);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::IntegerNegValue).0, 93);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::NumericContractSelectedValue).0,
        105
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Slash).0, 106);
}

#[test]
fn standalone_slash_is_lossless_and_comments_keep_priority() {
    let source =
        "fn f(a: F64, b: F64) -> F64 { // line / *\n return a /* left / */ / /* right */ b; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count_nodes(&parsed, SyntaxKind::MulValue), 1);
    assert_eq!(count_tokens(&parsed, SyntaxKind::Slash), 1);
    assert_eq!(count_tokens(&parsed, SyntaxKind::LineComment), 1);
    assert_eq!(count_tokens(&parsed, SyntaxKind::BlockComment), 2);
}

#[test]
fn slash_equals_is_not_a_compound_operator() {
    let source = "fn f(a: F64, b: F64) -> F64 { return a /= b; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count_nodes(&parsed, SyntaxKind::MulValue), 0);
    assert!(
        nontrivia_kinds(&parsed)
            .windows(2)
            .any(|window| window == [SyntaxKind::Slash, SyntaxKind::Eq])
    );
}

#[test]
fn slash_reuses_the_bounded_multiplicative_node_for_grouped_mixed_forms() {
    for source in [
        "fn f(a: F64, b: F64, c: F64) -> F64 { return (a / b) / c; }",
        "fn f(a: F64, b: F64, c: F64) -> F64 { return (a / b) * c; }",
        "fn f(a: F64, b: F64, c: F64) -> F64 { return a / (b * c); }",
        "fn f(a: F64, b: F64, c: F64) -> F64 { return a * (b / c); }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(
            parsed.errors().is_empty(),
            "{source}: {:?}",
            parsed.errors()
        );
        assert_eq!(count_nodes(&parsed, SyntaxKind::MulValue), 2);
    }

    for source in [
        "fn f(a: F64, b: F64, c: F64) -> F64 { return a / b / c; }",
        "fn f(a: F64, b: F64, c: F64) -> F64 { return a * b / c; }",
        "fn f(a: F64, b: F64, c: F64) -> F64 { return a / b * c; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty(), "{source}");
        assert_eq!(count_nodes(&parsed, SyntaxKind::MulValue), 1);
    }
}

#[test]
fn slash_keeps_multiplicative_precedence_and_prefix_boundaries() {
    let right = parse("fn f(a: F64, b: F64, c: F64) -> F64 { return a + b / c; }");
    assert!(right.errors().is_empty(), "{:?}", right.errors());
    let add = right
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::AddValue)
        .expect("addition");
    assert_eq!(
        add.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::IdentifierUse, SyntaxKind::MulValue]
    );

    let left = parse("fn f(a: F64, b: F64, c: F64) -> F64 { return a / b + c; }");
    assert!(left.errors().is_empty(), "{:?}", left.errors());
    let add = left
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::AddValue)
        .expect("addition");
    assert_eq!(
        add.children().map(|node| node.kind()).collect::<Vec<_>>(),
        [SyntaxKind::MulValue, SyntaxKind::IdentifierUse]
    );

    let signed = parse("fn f(a: F64) -> F64 { return a / -2.0; }");
    assert!(signed.errors().is_empty(), "{:?}", signed.errors());
    let division = signed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::MulValue)
        .expect("division-shaped multiplicative value");
    assert_eq!(
        division
            .children()
            .map(|node| node.kind())
            .collect::<Vec<_>>(),
        [
            SyntaxKind::IdentifierUse,
            SyntaxKind::DecimalFloatingLiteral
        ]
    );
}

#[test]
fn fast_selector_can_wrap_a_division_shaped_multiplicative_value() {
    let source = "fn f(a: F64, b: F64) -> F64 { return @fast(a / b); }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(
        count_nodes(&parsed, SyntaxKind::NumericContractSelectedValue),
        1
    );
    assert_eq!(count_nodes(&parsed, SyntaxKind::MulValue), 1);
    assert_eq!(count_tokens(&parsed, SyntaxKind::Slash), 1);
}
