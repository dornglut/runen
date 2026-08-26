use runen_syntax::{ExpectedSyntax, SyntaxErrorKind, SyntaxKind, parse_source};

#[test]
fn missing_pattern_close_after_rest_preserves_following_statement_and_top_level_boundary() {
    let source = "record Pair { left: I8 } fn f(root: Pair) { let Pair { .. = root; let later: I8 = 2; } record Next {}";
    let parsed = parse_source(source.as_bytes()).expect("valid UTF-8 test source");

    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().iter().any(|error| {
        error.kind() == SyntaxErrorKind::Expected(ExpectedSyntax::RightBrace)
    }));

    let root = parsed.syntax();
    assert_eq!(
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::RecordPatternRest)
            .count(),
        1
    );
    assert!(root.descendants().any(|node| {
        node.kind() == SyntaxKind::LocalDeclaration && node.text().to_string().contains("later")
    }));
    assert!(root.descendants().any(|node| {
        node.kind() == SyntaxKind::RecordDefinition && node.text().to_string().contains("Next")
    }));
}
