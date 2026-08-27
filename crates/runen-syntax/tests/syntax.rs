use runen_syntax::{SourceInputError, SyntaxErrorKind, SyntaxKind, identifier_key, parse_source};

fn parse(text: &str) -> runen_syntax::Parse {
    parse_source(text.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn unicode_dependencies_are_pinned_to_17_0_0() {
    assert_eq!(unicode_ident::UNICODE_VERSION, (17, 0, 0));
    assert_eq!(unicode_normalization::UNICODE_VERSION, (17, 0, 0));
}

#[test]
fn round_trips_empty_trivia_and_valid_source() {
    for source in [
        "",
        " \t// comment\r\n/* outer /* inner */ done */\n",
        "record Ticket { id: I64, }\nfn id(value: Ticket) -> Ticket { return value; }\n",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn preserves_only_the_initial_bom_as_bom_trivia() {
    let source = "\u{feff}fn id(value: I64) -> I64 { return value; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    let kinds = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert_eq!(kinds.first(), Some(&SyntaxKind::Bom));
    assert!(kinds.contains(&SyntaxKind::KwFn));

    let later_bom = parse("fn a() {}\u{feff}");
    assert!(
        later_bom
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken)
    );
}

#[test]
fn rejects_invalid_utf8_before_parsing() {
    let error = parse_source(&[0xff]).expect_err("invalid UTF-8 must fail");
    assert_eq!(error, SourceInputError::InvalidUtf8 { valid_up_to: 0 });
}

#[test]
fn maximal_identifier_extent_precedes_reserved_key_classification() {
    let parsed = parse("fnx(value: I64) {}");
    let tokens = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .collect::<Vec<_>>();
    assert_eq!(tokens[0].kind(), SyntaxKind::Ident);
    assert_eq!(tokens[0].text(), "fnx");
}

#[test]
fn derives_nfc_equivalent_identifier_keys() {
    let decomposed = "e\u{301}";
    let composed = "é";
    assert_eq!(identifier_key(decomposed), identifier_key(composed));
    assert_eq!(identifier_key(composed).as_deref(), Some(composed));
    assert_eq!(identifier_key("fn"), Some("fn".to_owned()));
    assert_eq!(identifier_key("1bad"), None);
}

#[test]
fn pattern_whitespace_matches_the_pinned_profile() {
    for character in [
        '\u{0009}', '\u{000a}', '\u{000b}', '\u{000c}', '\u{000d}', '\u{0020}', '\u{0085}',
        '\u{200e}', '\u{200f}', '\u{2028}', '\u{2029}',
    ] {
        let source = format!("fn{character}a() {{}}");
        let parsed = parse(&source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    }

    for character in ['\u{00a0}', '\u{1680}', '\u{2000}', '\u{3000}'] {
        let source = format!("fn{character}a() {{}}");
        let parsed = parse(&source);
        assert_eq!(parsed.text(), source);
        assert!(
            parsed
                .errors()
                .iter()
                .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken)
        );
    }
}

#[test]
fn handles_crlf_and_other_logical_line_comment_boundaries() {
    let source = "fn a() {//x\r\n//y\u{2028}return;\n}";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(
        !parsed
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken)
    );
}

#[test]
fn handles_nested_and_unterminated_block_comments() {
    let nested = parse("/* outer /* inner */ done */ fn a() {}");
    assert_eq!(nested.text(), "/* outer /* inner */ done */ fn a() {}");
    assert!(
        !nested
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnterminatedBlockComment)
    );

    let source = "fn a() { /* never closes";
    let unterminated = parse(source);
    assert_eq!(unterminated.text(), source);
    assert!(
        unterminated
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnterminatedBlockComment)
    );
}

#[test]
fn unsupported_concrete_text_is_retained_as_error_tokens() {
    let source = "fn a() { let x: I64 = 42; $ . }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| error.kind() == SyntaxErrorKind::UnrecognizedToken)
    );
}

#[test]
fn parses_representative_records_functions_locals_calls_and_returns() {
    let source = r#"
record Ticket { id: I64, }
fn identity(value: Ticket) -> Ticket {
    return value;
}
fn forward(value: Ticket) -> Ticket {
    let moved: Ticket = identity(value,);
    return identity(moved);
}
fn sink(value: Ticket) {
    consume(value);
}
fn consume(value: Ticket,) {}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let root = parsed.syntax();
    assert_eq!(
        root.children()
            .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
            .count(),
        4
    );
}

#[test]
fn malformed_input_recovers_without_losing_text() {
    let source =
        "record A { x I64, y: I64 } fn broken( { let x: I64 = ; return; trailing } fn ok() {}";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().len() >= 3);
    assert_eq!(
        parsed
            .syntax()
            .children()
            .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
            .count(),
        2
    );
}

#[test]
fn missing_parameter_close_resynchronizes_at_the_body_and_next_item() {
    let source = "fn broken(value: I64 { return value; } fn ok() {}";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .children()
            .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
            .count(),
        2
    );
}

#[test]
fn missing_record_close_resynchronizes_at_the_next_item() {
    let source = "record A { value: I64 fn ok() {}";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(
        parsed
            .syntax()
            .children()
            .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
            .count(),
        1
    );
}
