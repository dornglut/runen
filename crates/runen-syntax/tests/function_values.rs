use runen_syntax::{Parse, SyntaxKind, parse_source};

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
fn call_kind_keeps_existing_numeric_slot_but_is_semantically_neutral() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Call).0, 41);

    let parsed = parse("fn use(f: fn(I64) -> I64) -> I64 { f(1); return dep::apply[I64](1); }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::Call), 2);
    assert_eq!(count(&parsed, SyntaxKind::CallStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeArgumentList), 1);
}

#[test]
fn function_types_are_finite_recursive_type_refs_with_optional_result() {
    let source =
        "fn use(f: fn(I64, fn(Bool) -> I64,) -> Bool) -> fn(I64) { let g: fn(I64) = f; return g; }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);

    let function_type_refs = parsed
        .syntax()
        .descendants()
        .filter(|node| {
            node.kind() == SyntaxKind::TypeRef
                && node
                    .children_with_tokens()
                    .filter_map(|element| element.into_token())
                    .any(|token| token.kind() == SyntaxKind::KwFn)
        })
        .count();
    assert_eq!(function_type_refs, 4);
}

#[test]
fn function_types_do_not_widen_reference_or_generic_argument_grammar() {
    for source in [
        "fn bad(value: &fn(I64)) {}",
        "fn bad(value: raw fn(I64)) {}",
        "fn use() { generic[fn(I64)](); }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn arbitrary_postfix_callable_forms_remain_unrepresented() {
    for source in [
        "fn use(f: fn(I64) -> I64) -> I64 { return (f)(1); }",
        "fn use() -> I64 { return make()(1); }",
        "record R { handler: I64 } fn use(r: R) -> I64 { return r.handler(1); }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn malformed_function_type_remains_lossless_and_recovers_to_body() {
    let source = "fn bad(f: fn(I64 -> Bool) { return true; }";
    let parsed = parse(source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::Body), 1);
}
