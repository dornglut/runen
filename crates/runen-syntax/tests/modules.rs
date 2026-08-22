use runen_syntax::{SyntaxKind, parse_source, user_identifier_key};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn round_trips_import_export_and_qualified_forms_losslessly() {
    let source = "/* head */ export record Holder { value: dep::Ticket } /* middle */ import dep; export fn use(value: dep::Ticket) -> dep::Ticket { dep::ping(); return value; }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);

    let root = parsed.syntax();
    let kinds = root.children().map(|node| node.kind()).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::RecordDefinition,
            SyntaxKind::ImportDeclaration,
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
fn exported_record_fields_are_lossless_and_bounded() {
    let source = "export record Holder { private: I8, /* field */ export public: dep::Ticket, export ready: Bool }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);

    let root = parsed.syntax();
    let record = root
        .children()
        .find(|node| node.kind() == SyntaxKind::RecordDefinition)
        .expect("record definition");
    assert_eq!(
        record
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::KwExport)
            .count(),
        1,
        "record export must remain distinct from field exports"
    );

    let fields = record
        .children()
        .filter(|node| node.kind() == SyntaxKind::RecordField)
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].text().to_string(), "private: I8");
    assert_eq!(
        fields[1].text().to_string(),
        "/* field */ export public: dep::Ticket"
    );
    assert_eq!(fields[2].text().to_string(), " export ready: Bool");

    let field_exports = fields
        .iter()
        .map(|field| {
            field
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .filter(|token| token.kind() == SyntaxKind::KwExport)
                .count()
        })
        .collect::<Vec<_>>();
    assert_eq!(field_exports, vec![0, 1, 1]);

    let qualified = fields[1]
        .children()
        .find(|node| node.kind() == SyntaxKind::TypeRef)
        .and_then(|ty| {
            ty.children()
                .find(|node| node.kind() == SyntaxKind::QualifiedModuleMember)
        });
    assert!(qualified.is_some());
}

#[test]
fn field_export_does_not_become_a_general_modifier() {
    for source in [
        "export import dep;",
        "fn f() { export value; }",
        "fn f() { let value: export = 1; }",
        "fn f() -> export { return 1; }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly clean: {source}"
        );
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn malformed_exported_field_recovers_without_swallowing_later_structure() {
    let source = "record Broken { export first I8, next: I16 } export fn later() {}";
    let parsed = parse(source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(parsed.text(), source);

    let root = parsed.syntax();
    let record = root
        .children()
        .find(|node| node.kind() == SyntaxKind::RecordDefinition)
        .expect("record definition retained");
    assert_eq!(
        record
            .children()
            .filter(|node| node.kind() == SyntaxKind::RecordField)
            .count(),
        2,
        "later field was swallowed"
    );
    assert!(root.children().any(|node| {
        node.kind() == SyntaxKind::FunctionDefinition
            && node
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .any(|token| token.kind() == SyntaxKind::KwExport)
    }));
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
