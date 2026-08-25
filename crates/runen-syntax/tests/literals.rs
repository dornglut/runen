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
    assert_eq!(
        signed_minimum.text().to_string().trim_start(),
        "- /* sign trivia */ 128"
    );
}

#[test]
fn floating_syntax_kinds_are_appended_after_accepted_kinds() {
    assert_eq!(SyntaxKind::WhileStatement as u16, 75);
    assert_eq!(SyntaxKind::DecimalFloatingMagnitude as u16, 76);
    assert_eq!(SyntaxKind::DecimalFloatingLiteral as u16, 77);
}

#[test]
fn decimal_floating_literals_parse_losslessly_in_represented_value_positions() {
    let source = r#"
record Sample { value: F32 }
fn sink(a: F64) {}
fn entry() -> F16 {
    let mut local: F32 = 1.25;
    local = 000.5000;
    sink(- /* sign trivia */ 2.0);
    let sample: Sample = Sample { value: 3.5 };
    return -0.0;
}
"#;
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::DecimalFloatingLiteral)
            .count(),
        5
    );
    assert!(
        nontrivia_tokens(&parsed)
            .contains(&(SyntaxKind::DecimalFloatingMagnitude, "000.5000".into()))
    );

    let negative = root
        .descendants()
        .find(|node| {
            node.kind() == SyntaxKind::DecimalFloatingLiteral
                && node.text().to_string().contains("2.0")
        })
        .expect("negative floating literal node");
    assert_eq!(
        negative.text().to_string().trim_start(),
        "- /* sign trivia */ 2.0"
    );
}

#[test]
fn decimal_floating_magnitude_is_one_contiguous_digit_dot_digit_token() {
    for spelling in ["0.0", "1.25", "000.5000"] {
        let source = format!("fn value() -> F32 {{ return {spelling}; }}");
        let parsed = parse(&source);
        assert!(
            parsed.errors().is_empty(),
            "{spelling}: {:?}",
            parsed.errors()
        );
        let floating = nontrivia_tokens(&parsed)
            .into_iter()
            .filter(|(kind, _)| *kind == SyntaxKind::DecimalFloatingMagnitude)
            .collect::<Vec<_>>();
        assert_eq!(
            floating,
            vec![(SyntaxKind::DecimalFloatingMagnitude, spelling.into())]
        );
    }
}

#[test]
fn decimal_point_boundaries_do_not_widen_the_floating_token() {
    let cases = [
        (
            ".5",
            vec![(SyntaxKind::Dot, "."), (SyntaxKind::DecimalMagnitude, "5")],
        ),
        (
            "1.",
            vec![(SyntaxKind::DecimalMagnitude, "1"), (SyntaxKind::Dot, ".")],
        ),
        (
            "1 . 0",
            vec![
                (SyntaxKind::DecimalMagnitude, "1"),
                (SyntaxKind::Dot, "."),
                (SyntaxKind::DecimalMagnitude, "0"),
            ],
        ),
    ];

    for (spelling, expected) in cases {
        let source = format!("fn bad() -> F32 {{ return {spelling}; }}");
        let parsed = parse(&source);
        let tokens = nontrivia_tokens(&parsed);
        for (kind, text) in expected {
            assert!(
                tokens.contains(&(kind, text.into())),
                "{spelling}: {tokens:?}"
            );
        }
        assert!(
            !tokens
                .iter()
                .any(|(kind, _)| *kind == SyntaxKind::DecimalFloatingMagnitude),
            "{spelling} must not form one floating magnitude"
        );
        assert!(!parsed.errors().is_empty(), "{spelling}");
    }

    for spelling in ["1/*x*/.0", "1./*x*/0"] {
        let source = format!("fn bad() -> F32 {{ return {spelling}; }}");
        let parsed = parse(&source);
        assert!(
            !nontrivia_tokens(&parsed)
                .iter()
                .any(|(kind, _)| *kind == SyntaxKind::DecimalFloatingMagnitude),
            "{spelling} must not form one floating magnitude"
        );
        assert!(!parsed.errors().is_empty(), "{spelling}");
    }
}

#[test]
fn unsupported_floating_suffixes_and_literal_field_receivers_are_rejected() {
    for spelling in ["1.0e3", "1.0.foo"] {
        let source = format!("fn bad() -> F32 {{ return {spelling}; }}");
        let parsed = parse(&source);
        let tokens = nontrivia_tokens(&parsed);
        assert!(tokens.contains(&(SyntaxKind::DecimalFloatingMagnitude, "1.0".into())));
        assert!(!parsed.errors().is_empty(), "{spelling}");
        assert!(
            !parsed
                .syntax()
                .descendants()
                .any(|node| node.kind() == SyntaxKind::FieldValueUse),
            "floating literal must not become a field receiver"
        );
    }
}

#[test]
fn floating_literals_are_conditional_values_but_not_pattern_scrutinees() {
    let conditional = parse("fn test() { if 1.0 {} while - 2.0 {} }");
    assert!(
        conditional.errors().is_empty(),
        "{:?}",
        conditional.errors()
    );
    assert_eq!(
        conditional
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::DecimalFloatingLiteral)
            .count(),
        2
    );

    let pattern = parse("record R { value: F32 } fn bad() { let R { value: v } = 1.0; }");
    assert!(!pattern.errors().is_empty());
    assert!(
        !pattern
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::DecimalFloatingLiteral),
        "record-pattern scrutinee grammar must remain narrower than Value"
    );
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
    let parsed = parse("fn value() -> F32 { return -1.0; }");
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
    assert!(!nontrivia_tokens(&parsed).iter().any(|(kind, text)| {
        matches!(
            kind,
            SyntaxKind::DecimalMagnitude | SyntaxKind::DecimalFloatingMagnitude
        ) && text == "١"
    }));
}

#[test]
fn unsupported_numeric_spellings_are_not_silently_reinterpreted() {
    for source in [
        "fn bad() -> I8 { return +1; }",
        "fn bad() -> I8 { return 1_0; }",
        "fn bad() -> I8 { return 0x10; }",
        "fn bad() -> F32 { return +1.0; }",
        "fn bad() -> F32 { return 1_0.0; }",
        "fn bad() -> F32 { return 0x1.0; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty(), "{source}");
    }
}

#[test]
fn minus_without_an_operand_has_a_structured_value_error() {
    let source = "fn bad() -> I8 { return -; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| { error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Value) })
    );
    assert_eq!(
        parsed
            .syntax()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::IntegerNegValue)
            .count(),
        1
    );
}
