use runen_syntax::{SyntaxKind, parse_source};

fn parse(source: &str) -> runen_syntax::Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count_kind(parse: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parse
        .syntax()
        .descendants_with_tokens()
        .filter(|element| element.kind() == kind)
        .count()
}

#[test]
fn lexes_standalone_ampersand_without_splitting_boolean_conjunction() {
    let conjunction = parse("fn f(a: Bool, b: Bool) -> Bool { return a && b; }");
    assert!(conjunction.errors().is_empty(), "{:?}", conjunction.errors());
    assert_eq!(count_kind(&conjunction, SyntaxKind::AmpAmp), 1);
    assert_eq!(count_kind(&conjunction, SyntaxKind::Amp), 0);

    let reference = parse("fn f(x: I64) { let r: &I64 = &x; }");
    assert!(reference.errors().is_empty(), "{:?}", reference.errors());
    assert_eq!(count_kind(&reference, SyntaxKind::Amp), 2);
    assert_eq!(count_kind(&reference, SyntaxKind::AmpAmp), 0);
}

#[test]
fn parses_bounded_shared_reference_types_and_values() {
    let source = r#"
import dep;
fn refs(x: I64, r: &I64, foreign: &dep::Ticket) -> I64 {
    let a: &I64 = &x;
    let b: I64 = *r;
    let c: I64 = (*r);
    let d: I64 = *r * x;
    return d;
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count_kind(&parsed, SyntaxKind::SharedBorrowValue), 1);
    assert_eq!(count_kind(&parsed, SyntaxKind::SharedDereferenceValue), 3);
    assert_eq!(count_kind(&parsed, SyntaxKind::MulValue), 1);
}

#[test]
fn prefix_dereference_does_not_reinterpret_binary_multiplication() {
    let multiplication = parse("fn f(a: I64, b: I64) -> I64 { return a * b; }");
    assert!(multiplication.errors().is_empty(), "{:?}", multiplication.errors());
    assert_eq!(count_kind(&multiplication, SyntaxKind::MulValue), 1);
    assert_eq!(
        count_kind(&multiplication, SyntaxKind::SharedDereferenceValue),
        0
    );

    let dereference_then_multiply =
        parse("fn f(r: &I64, x: I64) -> I64 { return *r * x; }");
    assert!(
        dereference_then_multiply.errors().is_empty(),
        "{:?}",
        dereference_then_multiply.errors()
    );
    assert_eq!(
        count_kind(&dereference_then_multiply, SyntaxKind::SharedDereferenceValue),
        1
    );
    assert_eq!(count_kind(&dereference_then_multiply, SyntaxKind::MulValue), 1);
}

#[test]
fn rejects_unrepresented_reference_syntax() {
    for source in [
        "fn f(r: &&I64) {}",
        "fn f(r: &I64) { let x: I64 = **r; }",
        "fn f(r: &I64) { let x: I64 = *(r); }",
        "fn f(x: I64) { let r: &mut I64 = &x; }",
        "fn f(x: I64) { let r: &I64 = &x.value; }",
        "import dep; fn f(x: I64) { let r: &I64 = &dep::x; }",
        "fn f(r: &Bool) { if *r {} }",
        "fn f(x: Bool) { if &x {} }",
        "fn f(r: &Bool) { while (*r) {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty(), "must reject: {source}");
    }
}
