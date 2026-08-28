use runen_syntax::{SyntaxKind, parse_source};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn node_count(parsed: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

#[test]
fn parses_lossless_operation_local_fast_selector_with_trivia() {
    for (source, root) in [
        (
            "fn f(a: F32, b: F32) -> F32 { return @ /* selector */ fast\n( a + b ); }",
            SyntaxKind::AddValue,
        ),
        (
            "fn f(a: F32, b: F32) -> F32 { return @ /* selector */ fast\n( a - b ); }",
            SyntaxKind::SubValue,
        ),
        (
            "fn f(a: F32, b: F32) -> F32 { return @ /* selector */ fast\n( a * b ); }",
            SyntaxKind::MulValue,
        ),
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(
            node_count(&parsed, SyntaxKind::NumericContractSelectedValue),
            1
        );
        assert_eq!(node_count(&parsed, root), 1);

        let tokens = parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .collect::<Vec<_>>();
        assert!(tokens.iter().any(|token| token.kind() == SyntaxKind::At));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind() == SyntaxKind::Ident && token.text() == "fast")
        );
    }
}

#[test]
fn fast_remains_an_ordinary_identifier_away_from_selector_position() {
    let source = "fn fast(value: F32) -> F32 { let fast: F32 = value; return fast; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(
        node_count(&parsed, SyntaxKind::NumericContractSelectedValue),
        0
    );
    assert!(
        parsed
            .syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.text() == "fast")
            .all(|token| token.kind() == SyntaxKind::Ident)
    );
}

#[test]
fn stacked_fast_selectors_are_represented_for_typed_rejection() {
    for operator in ["+", "-", "*"] {
        let source =
            format!("fn f(a: F32, b: F32) -> F32 {{ return @fast(@fast(a {operator} b)); }}");
        let parsed = parse(&source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(
            node_count(&parsed, SyntaxKind::NumericContractSelectedValue),
            2
        );
    }
}

#[test]
fn standard_and_reproducible_are_not_source_selectors() {
    for selector in ["standard", "reproducible"] {
        for operator in ["+", "-", "*"] {
            let source =
                format!("fn f(a: F32, b: F32) -> F32 {{ return @{selector}(a {operator} b); }}");
            let parsed = parse(&source);
            assert_eq!(parsed.text(), source);
            assert!(!parsed.errors().is_empty());
            assert_eq!(
                node_count(&parsed, SyntaxKind::NumericContractSelectedValue),
                0
            );
        }
    }
}

#[test]
fn direct_conditional_value_does_not_admit_numeric_contract_selection() {
    for operator in ["+", "-", "*"] {
        let source = format!("fn f(a: F32, b: F32) {{ if @fast(a {operator} b) {{ fault; }} }}");
        let parsed = parse(&source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(
            node_count(&parsed, SyntaxKind::NumericContractSelectedValue),
            0
        );
    }
}
