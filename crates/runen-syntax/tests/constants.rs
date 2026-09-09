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
fn constant_syntax_kind_appends_without_renumbering_existing_kinds() {
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::TraitRequirementClause).0,
        131
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::ConstDeclaration).0, 132);
}

#[test]
fn const_remains_a_contextual_user_identifier() {
    assert_eq!(user_identifier_key("const").as_deref(), Some("const"));

    let parsed = parse("const const: I64 = 1; fn const(const: I64) -> I64 { return const; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::ConstDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 1);
    assert_eq!(count(&parsed, SyntaxKind::IdentifierUse), 1);
}

#[test]
fn constants_parse_private_exported_and_all_literal_families() {
    let parsed = parse(
        r#"
const enabled: Bool = true;
export const signed: I64 = -42;
const unsigned: U8 = 255;
const ratio: F32 = -0.5;
"#,
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::ConstDeclaration), 4);
    assert_eq!(count(&parsed, SyntaxKind::BooleanLiteral), 1);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 2);
    assert_eq!(count(&parsed, SyntaxKind::DecimalFloatingLiteral), 1);
}

#[test]
fn constant_declarations_reject_non_intrinsic_types_and_non_literal_initializers() {
    for source in [
        "record R {} const bad: R = 1;",
        "const bad: &I64 = 1;",
        "const bad: I64 = other;",
        "const bad: I64 = 1 + 2;",
        "fn make() -> I64 { return 1; } const bad: I64 = make();",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
    }
}

#[test]
fn bare_qualified_member_is_a_value_but_calls_and_constructions_keep_their_shapes() {
    let parsed = parse(
        r#"
record R { field: I64 }
fn f() -> I64 {
    let a: I64 = dep::VALUE;
    let b: I64 = dep::call();
    let c: R = dep::R { field: dep::VALUE };
    return dep::VALUE;
}
"#,
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::QualifiedModuleMember), 5);
    assert_eq!(count(&parsed, SyntaxKind::IdentifierUse), 3);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 1);

    let mut value_uses = 0_usize;
    let mut call_targets = 0_usize;
    let mut construction_targets = 0_usize;
    for qualified in parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::QualifiedModuleMember)
    {
        match qualified
            .parent()
            .expect("qualified member has one represented owner")
            .kind()
        {
            SyntaxKind::IdentifierUse => value_uses += 1,
            SyntaxKind::DirectCall => call_targets += 1,
            SyntaxKind::RecordConstruction => construction_targets += 1,
            other => panic!("unexpected qualified-member parent {other:?}"),
        }
    }
    assert_eq!(value_uses, 3);
    assert_eq!(call_targets, 1);
    assert_eq!(construction_targets, 1);
}

#[test]
fn qualified_constants_parse_in_conditional_and_operator_value_positions() {
    let parsed = parse(
        r#"
fn f() -> I64 {
    if dep::ENABLED {}
    let x: I64 = dep::ONE + 2;
    if dep::ONE == 1 {}
    return dep::ONE;
}
"#,
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::QualifiedModuleMember), 4);
    assert_eq!(count(&parsed, SyntaxKind::IdentifierUse), 4);
    assert_eq!(count(&parsed, SyntaxKind::IfStatement), 2);
    assert_eq!(count(&parsed, SyntaxKind::AddValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);
}

#[test]
fn constant_value_shape_does_not_widen_targets_or_general_members() {
    for source in [
        "fn f() { dep::VALUE = 1; }",
        "fn f() { let x: I64 = dep::VALUE.field; }",
        "fn f() { let x: I64 = dep::nested::VALUE; }",
        "fn f() { let x: &I64 = &dep::VALUE; }",
        "fn f() { let x: raw I64 = raw &dep::VALUE; }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
    }
}
