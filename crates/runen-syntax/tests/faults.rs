use rowan::Language;
use runen_syntax::{
    ExpectedSyntax, RunenLanguage, SyntaxErrorKind, SyntaxKind, parse_source, user_identifier_key,
};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count_nodes(parse: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parse
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

fn count_tokens(parse: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parse
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == kind)
        .count()
}

#[test]
fn fault_kinds_are_appended_without_renumbering_existing_kinds() {
    assert_eq!(SyntaxKind::KwCopy as u16, 71);
    assert_eq!(SyntaxKind::KwFault as u16, 72);
    assert_eq!(SyntaxKind::FaultStatement as u16, 73);
    assert_eq!(
        RunenLanguage::kind_from_raw(rowan::SyntaxKind(72)),
        SyntaxKind::KwFault
    );
    assert_eq!(
        RunenLanguage::kind_from_raw(rowan::SyntaxKind(73)),
        SyntaxKind::FaultStatement
    );
}

#[test]
fn parses_root_and_nested_fault_statements_losslessly() {
    let source = "fn root() { fault; } fn nested() { { /* before */ fault ; } }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count_nodes(&parsed, SyntaxKind::FaultStatement), 2);
    assert_eq!(count_tokens(&parsed, SyntaxKind::KwFault), 2);
}

#[test]
fn fault_tail_is_syntax_represented_for_semantic_unreachable_validation() {
    let source = "fn f() { fault; let x: I64 = 1; return; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count_nodes(&parsed, SyntaxKind::FaultStatement), 1);
    assert_eq!(count_nodes(&parsed, SyntaxKind::LocalDeclaration), 1);
    assert_eq!(count_nodes(&parsed, SyntaxKind::ReturnStatement), 1);
}

#[test]
fn fault_is_reserved_but_longer_identifier_remains_user_identifier() {
    assert_eq!(user_identifier_key("fault"), None);
    assert_eq!(user_identifier_key("faulty"), Some("faulty".into()));

    let valid = parse("fn faulty() { let faulty: I64 = 1; }");
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());

    let reserved = parse("fn fault() {}");
    assert!(!reserved.errors().is_empty());
}

#[test]
fn missing_fault_semicolon_reports_and_preserves_following_statement() {
    let source = "fn f() { fault let x: I64 = 1; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Semicolon)
    }));
    assert_eq!(count_nodes(&parsed, SyntaxKind::FaultStatement), 1);
    assert_eq!(count_nodes(&parsed, SyntaxKind::LocalDeclaration), 1);
}

#[test]
fn malformed_statement_recovery_stops_before_following_fault() {
    let source = "fn f() { broken fault; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count_nodes(&parsed, SyntaxKind::FaultStatement), 1);
}

#[test]
fn fault_is_not_a_value() {
    let source = "fn f() { let x: I64 = fault; }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Value)
    }));
}

#[test]
fn return_specific_tail_rejection_is_unchanged() {
    let parsed = parse("fn f() { return; fault; }");

    assert!(parsed
        .errors()
        .iter()
        .any(|error| error.kind() == SyntaxErrorKind::UnexpectedAfterReturn));
    assert_eq!(count_nodes(&parsed, SyntaxKind::ReturnStatement), 1);
    assert_eq!(count_nodes(&parsed, SyntaxKind::FaultStatement), 0);
}
