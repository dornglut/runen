use runen_syntax::{SyntaxKind, parse_source, user_identifier_key};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn round_trips_import_export_and_qualified_forms_losslessly() {
    let source = "/* head */ import dep; export record Holder { value: dep::Ticket } export fn use(value: dep::Ticket) -> dep::Ticket { dep::ping(); return value; }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);

    let root = parsed.syntax();
    let kinds = root.children().map(|node| node.kind()).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::ImportDeclaration,
            SyntaxKind::RecordDefinition,
            SyntaxKind::FunctionDefinition,
        ]
    );

    let qualified = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::QualifiedModuleMember)
        .collect::<Vec<_>>();
    assert_eq!(qualified.len(), 4);
    for node in qualified {
        let token_kinds = node
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| !token.kind().is_trivia())
            .map(|token| token.kind())
            .collect::<Vec<_>>();
        assert_eq!(
            token_kinds,
            vec![SyntaxKind::Ident, SyntaxKind::ColonColon, SyntaxKind::Ident]
        );
    }
}

#[test]
fn imports_only_and_trivia_only_units_are_clean() {
    for source in [
        "import a; import b;",
        " // trivia only\n /* nested /* x */ y */ ",
    ] {
        let parsed = parse(source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn reserved_keys_use_complete_identifier_tokens_not_prefixes() {
    let source = "record importx { exported: I64 } fn f(importx: I64, exported: I64) {}";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);
    assert_eq!(user_identifier_key("importx").as_deref(), Some("importx"));
    assert_eq!(user_identifier_key("exported").as_deref(), Some("exported"));
    assert!(user_identifier_key("import").is_none());
    assert!(user_identifier_key("export").is_none());
}

#[test]
fn double_colon_is_one_token_and_single_colon_remains_distinct() {
    let parsed = parse("record Holder { value: dep::Ticket }");
    assert!(parsed.errors().is_empty());
    let root = parsed.syntax();
    let token_kinds = root
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert!(token_kinds.contains(&SyntaxKind::Colon));
    assert!(token_kinds.contains(&SyntaxKind::ColonColon));
    assert_eq!(
        token_kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::ColonColon)
            .count(),
        1
    );
}

#[test]
fn malformed_qualification_and_export_import_recover_losslessly() {
    let malformed = [
        "record A { x: dep:Ticket }",
        "record A { x: dep:::Ticket }",
        "record A { x: dep:: }",
        "record A { x: dep::B::C }",
        "export import dep;",
        "import dep export fn f() {}",
        "fn f(x: I64) { let y: I64 = dep::value; }",
    ];

    for source in malformed {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly clean: {source}"
        );
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn malformed_constructs_do_not_swallow_later_top_level_elements() {
    let cases = [
        "record Broken { field: I64 import dep; export fn later() {}",
        "fn broken( import dep; export record Later {}",
        "fn broken() { return; import dep; export fn later() {}",
        "import dep export fn later() {}",
    ];

    for source in cases {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly clean: {source}"
        );
        assert_eq!(parsed.text(), source);
        let root = parsed.syntax();
        assert!(
            root.children()
                .any(|node| node.kind() == SyntaxKind::ImportDeclaration),
            "later import was swallowed: {source}"
        );
        assert!(
            root.children().any(|node| {
                matches!(
                    node.kind(),
                    SyntaxKind::RecordDefinition | SyntaxKind::FunctionDefinition
                ) && node
                    .children_with_tokens()
                    .filter_map(|element| element.into_token())
                    .any(|token| token.kind() == SyntaxKind::KwExport)
            }),
            "later exported item was swallowed: {source}"
        );
    }
}

#[test]
fn bare_or_nested_qualified_members_are_not_general_values_or_paths() {
    for source in [
        "fn f() { let x: I64 = dep::value; }",
        "fn f() { dep::nested::call(); }",
        "record R { value: dep::nested::Type }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly clean: {source}"
        );
        assert_eq!(parsed.text(), source);
    }
}
