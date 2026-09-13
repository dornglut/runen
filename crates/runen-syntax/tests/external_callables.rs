use runen_syntax::{Parse, SyntaxKind, parse_source, user_identifier_key};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count(parse: &Parse, kind: SyntaxKind) -> usize {
    parse
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

#[test]
fn parses_private_and_exported_scalar_external_declarations() {
    let parsed = parse("external fn sink(); export external fn transform(I64, Bool,) -> U64;");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::ExternalFunctionDeclaration), 2);
    assert_eq!(count(&parsed, SyntaxKind::ParameterList), 2);
    assert_eq!(count(&parsed, SyntaxKind::ResultClause), 1);
    assert_eq!(count(&parsed, SyntaxKind::TypeRef), 3);
}

#[test]
fn external_remains_contextual_identifier_outside_item_introducer() {
    assert_eq!(user_identifier_key("external").as_deref(), Some("external"));
    let parsed = parse("fn external(external: I64) -> I64 { return external; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::ExternalFunctionDeclaration), 0);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 1);
}

#[test]
fn external_declaration_rejects_named_nonintrinsic_generic_and_body_shapes() {
    for source in [
        "external fn bad(value: I64);",
        "external fn bad(Name);",
        "external fn bad[T](I64);",
        "external fn bad(I64) { return; }",
        "external fn bad(I64) -> Name;",
    ] {
        assert!(
            !parse(source).errors().is_empty(),
            "unexpectedly valid: {source}"
        );
    }
}
