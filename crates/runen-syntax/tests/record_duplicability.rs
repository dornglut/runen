use runen_syntax::{SyntaxErrorKind, SyntaxKind, parse_source, user_identifier_key};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn token_kinds(source: &str) -> Vec<SyntaxKind> {
    parse(source)
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.kind())
        .collect()
}

#[test]
fn appends_copy_without_renumbering_existing_syntax_kinds() {
    assert_eq!(SyntaxKind::IfStatement as u16, 70);
    assert_eq!(SyntaxKind::KwCopy as u16, 71);
}

#[test]
fn reserves_copy_but_preserves_maximal_identifier_extent() {
    assert_eq!(user_identifier_key("copy"), None);
    assert_eq!(user_identifier_key("copycat").as_deref(), Some("copycat"));

    let parsed = parse("record copycat {} fn copycat_fn(copycat_arg: copycat) {}");
    let copycat_tokens = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.text().starts_with("copycat"))
        .collect::<Vec<_>>();
    assert!(!copycat_tokens.is_empty());
    assert!(
        copycat_tokens
            .iter()
            .all(|token| token.kind() == SyntaxKind::Ident)
    );
}

#[test]
fn parses_plain_selected_and_exported_selected_records_losslessly() {
    for source in [
        "record Plain {}",
        "record copy Point { x: I32, y: I32 }",
        "export record /* selection */ copy PublicPoint { export x: I32 }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    }

    let selected = parse("record /* before */ copy /* after */ Point {}");
    let record = selected
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::RecordDefinition)
        .expect("record definition");
    let direct_tokens = record
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert!(direct_tokens.contains(&SyntaxKind::KwCopy));
    assert_eq!(
        selected.text(),
        "record /* before */ copy /* after */ Point {}"
    );
}

#[test]
fn rejects_copy_outside_the_record_selection_position() {
    for source in [
        "copy record Wrong {}",
        "export copy record Wrong {}",
        "record Wrong copy {}",
        "copy fn wrong() {}",
        "fn copy() {}",
        "record copy {}",
        "record Holder { copy: I8 }",
        "fn f(copy: I8) {}",
        "fn f() { let copy: I8 = 1; }",
        "import copy;",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
    }
}

#[test]
fn malformed_selected_record_recovers_at_the_next_top_level_item() {
    let source = "record copy Broken { value: I8 fn ok() {}";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|error| matches!(error.kind(), SyntaxErrorKind::Expected(_)))
    );
    assert_eq!(
        parsed
            .syntax()
            .children()
            .filter(|node| node.kind() == SyntaxKind::FunctionDefinition)
            .count(),
        1
    );
}

#[test]
fn complete_copy_key_is_tokenized_as_kw_copy() {
    let kinds = token_kinds("record copy Point {}");
    assert_eq!(
        kinds,
        [
            SyntaxKind::KwRecord,
            SyntaxKind::KwCopy,
            SyntaxKind::Ident,
            SyntaxKind::LBrace,
            SyntaxKind::RBrace,
        ]
    );
}
