use runen_syntax::{ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

fn nontrivia_tokens(parsed: &runen_syntax::Parse) -> Vec<(SyntaxKind, String)> {
    parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| (token.kind(), token.text().to_owned()))
        .collect()
}

#[test]
fn literals_parse_losslessly_in_every_represented_value_position() {
    let source = r#"
fn sink(number: I8, flag: Bool) {}
fn entry() -> I8 {
    let mut value: I8 = - /* sign trivia */ 128;
    let flag: Bool = true;
    value = 001;
    sink(- 1, false);
    return 0;
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::DecimalIntegerLiteral)
            .count(),
        4
    );
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::BooleanLiteral)
            .count(),
        2
    );

    let signed_minimum = root
        .descendants()
        .find(|node| {
            node.kind() == SyntaxKind::DecimalIntegerLiteral
                && node.text().to_string().contains("128")
        })
        .expect("negative literal node");
    assert_eq!(signed_minimum.text().to_string(), "- /* sign trivia */ 128");
}

#[test]
fn true_and_false_are_reserved_only_after_maximal_identifier_formation() {
    let source = "fn trueish(falsehood: I8) -> I8 { return falsehood; }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let tokens = nontrivia_tokens(&parsed);
    assert!(tokens.contains(&(SyntaxKind::Ident, "trueish".into())));
    assert!(tokens.contains(&(SyntaxKind::Ident, "falsehood".into())));

    let literals = parse("fn flags() -> Bool { return true; }");
    assert!(literals.errors().is_empty(), "{:?}", literals.errors());
    assert!(nontrivia_tokens(&literals).contains(&(SyntaxKind::KwTrue, "true".into())));
}

#[test]
fn decimal_magnitudes_are_maximal_ascii_digit_tokens_without_suffixes() {
    let parsed = parse("fn bad() { let value: I8 = 123abc; }");
    let tokens = nontrivia_tokens(&parsed);
    let pair = tokens
        .windows(2)
        .find(|pair| pair[0].1 == "123" && pair[1].1 == "abc")
        .expect("digits and following identifier are distinct tokens");
    assert_eq!(pair[0].0, SyntaxKind::DecimalMagnitude);
    assert_eq!(pair[1].0, SyntaxKind::Ident);
    assert!(
        !parsed.errors().is_empty(),
        "adjacent identifier text must not become a numeric suffix"
    );
}

#[test]
fn arrow_precedes_standalone_minus_tokenization() {
    let parsed = parse("fn value() -> I8 { return -1; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let tokens = nontrivia_tokens(&parsed);

    assert_eq!(
        tokens
            .iter()
            .filter(|(kind, _)| *kind == SyntaxKind::Arrow)
            .count(),
        1
    );
    assert_eq!(
        tokens
            .iter()
            .filter(|(kind, _)| *kind == SyntaxKind::Minus)
            .count(),
        1
    );
}

#[test]
fn non_ascii_decimal_lookalikes_are_not_decimal_magnitudes() {
    let parsed = parse("fn bad() -> I8 { return ١; }");
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| { error.kind() == SyntaxErrorKind::UnrecognizedToken })
    );
    assert!(
        !nontrivia_tokens(&parsed)
            .iter()
            .any(|(kind, text)| *kind == SyntaxKind::DecimalMagnitude && text == "١")
    );
}

#[test]
fn unsupported_numeric_spellings_are_not_silently_reinterpreted() {
    for source in [
        "fn bad() -> I8 { return +1; }",
        "fn bad() -> I8 { return 1_0; }",
        "fn bad() -> I8 { return 0x10; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty(), "{source}");
    }
}

#[test]
fn minus_without_a_decimal_magnitude_has_a_structured_syntax_error() {
    let parsed = parse("fn bad() -> I8 { return -; }");
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::DecimalMagnitude)
    }));
}
