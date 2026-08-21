use runen_syntax::{ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count_kind(parse: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parse
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

#[test]
fn parses_empty_recursive_blocks_losslessly_without_semicolons() {
    let source = "fn f() { /* before */ {} { // nested\n { let x: I64 = 1; } } }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count_kind(&parsed, SyntaxKind::BlockStatement), 3);
}

#[test]
fn semicolon_after_block_is_not_part_of_the_block_statement() {
    let parsed = parse("fn f() { {}; }");

    assert_eq!(parsed.text(), "fn f() { {}; }");
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Statement)
    }));
    assert_eq!(count_kind(&parsed, SyntaxKind::BlockStatement), 1);
}

#[test]
fn nested_return_remains_outside_nested_block_grammar() {
    let parsed = parse("fn f() { { return; } }");

    assert_eq!(parsed.text(), "fn f() { { return; } }");
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Statement)
    }));
    assert_eq!(count_kind(&parsed, SyntaxKind::ReturnStatement), 0);
}

#[test]
fn block_is_not_accepted_as_a_value() {
    let parsed = parse("fn f() { let x: I64 = {}; }");

    assert_eq!(parsed.text(), "fn f() { let x: I64 = {}; }");
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::Value)
    }));
}

#[test]
fn malformed_statement_recovers_to_following_legal_block() {
    let source = "fn f() { broken { let x: I64 = 1; } }";
    let parsed = parse(source);

    assert_eq!(parsed.text(), source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(count_kind(&parsed, SyntaxKind::BlockStatement), 1);
    assert_eq!(count_kind(&parsed, SyntaxKind::LocalDeclaration), 1);
}

#[test]
fn missing_nested_and_root_closes_recover_to_next_top_level_item() {
    let source = "fn broken() { { let x: I64 = 1; fn ok() {}";
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
