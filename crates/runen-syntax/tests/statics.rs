use runen_syntax::{Parse, SyntaxKind, parse_source, user_identifier_key};

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

#[test]
fn static_syntax_kind_appends_without_renumbering_existing_kinds() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::ClosureCaptures).0, 135);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::StaticDeclaration).0, 136);
}

#[test]
fn static_remains_a_contextual_user_identifier() {
    assert_eq!(user_identifier_key("static").as_deref(), Some("static"));

    let parsed = parse(
        "static static: I64 = 1; fn static(static: I64) -> I64 { return static; }",
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::StaticDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 1);
    assert_eq!(count(&parsed, SyntaxKind::IdentifierUse), 1);
}

#[test]
fn statics_parse_private_exported_and_all_admitted_scalar_literal_shapes() {
    let parsed = parse(
        r#"
static enabled: Bool = true;
export static i8_value: I8 = -1;
static i16_value: I16 = -2;
static i32_value: I32 = -3;
static i64_value: I64 = -4;
static u8_value: U8 = 1;
static u16_value: U16 = 2;
static u32_value: U32 = 3;
static u64_value: U64 = 4;
static f16_value: F16 = 0.5;
static f32_value: F32 = -1.5;
static f64_value: F64 = 2.5;
"#,
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::StaticDeclaration), 12);
    assert_eq!(count(&parsed, SyntaxKind::BooleanLiteral), 1);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 8);
    assert_eq!(count(&parsed, SyntaxKind::DecimalFloatingLiteral), 3);
}

#[test]
fn static_declarations_reject_non_intrinsic_types_and_non_literal_initializers() {
    for source in [
        "record R {} static bad: R = 1;",
        "static bad: &I64 = 1;",
        "static bad: raw I64 = 1;",
        "static bad: I64 = other;",
        "static bad: I64 = 1 + 2;",
        "static bad: I64 = (1);",
        "fn make() -> I64 { return 1; } static bad: I64 = make();",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
    }
}

#[test]
fn malformed_static_declaration_is_lossless_and_recovers_to_following_item() {
    let source = "static broken I64 = 1; fn next() {}";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count(&parsed, SyntaxKind::StaticDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 1);
}

#[test]
fn qualified_shared_root_has_only_the_bounded_static_shape() {
    let parsed = parse("fn f() { let value: &I64 = &dep::VALUE; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::SafeReferenceValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::QualifiedModuleMember), 1);

    for source in [
        "fn f() { let value: &I64 = &dep::nested::VALUE; }",
        "fn f() { let value: &mut I64 = &mut dep::VALUE; }",
        "fn f() { let value: &I64 = &*dep::VALUE; }",
        "fn f() { let value: raw I64 = raw &dep::VALUE; }",
        "fn f() { let value: &I64 = &dep::VALUE.field; }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
    }
}
