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
fn closure_declaration_is_dedicated_and_lossless() {
    let source = "fn use(value: I64) -> I64 { let add = fn[value](x: I64) -> I64 { return x + value; }; return add(1); }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::ClosureDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::ClosureInitializer), 1);
    assert_eq!(count(&parsed, SyntaxKind::ClosureCaptures), 1);
    assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 0);
    assert_eq!(count(&parsed, SyntaxKind::Call), 1);
}

#[test]
fn closure_captures_preserve_order_and_optional_trailing_comma() {
    for source in [
        "fn use(a: I64) { let f = fn[a]() {}; }",
        "fn use(a: I64, b: I64) { let f = fn[a, b,]() {}; }",
    ] {
        let parsed = parse(source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(parsed.text(), source);
        assert_eq!(count(&parsed, SyntaxKind::ClosureDeclaration), 1);
    }
}

#[test]
fn closure_capture_list_is_nonempty_and_recovery_is_lossless() {
    for source in [
        "fn use() { let f = fn[]() {}; let later: I64 = 1; }",
        "fn use(a: I64) { let f = fn[a() {}; let later: I64 = 1; }",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
        assert_eq!(parsed.text(), source);
        assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 1);
    }
}

#[test]
fn closure_initializer_does_not_become_a_general_value_or_postfix_callee() {
    for source in [
        "fn use(a: I64) { let f: I64 = fn[a]() {}; }",
        "fn use(a: I64) { fn[a]() {}(); }",
        "fn use(a: I64) { return fn[a]() -> I64 { return a; }; }",
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
fn closure_syntax_remains_distinct_from_existing_let_function_and_function_type_forms() {
    let source = "fn module_fn(value: I64) -> I64 { return value; } \
                  fn use(value: I64, f: fn(I64) -> I64) { \
                      let ordinary: I64 = value; \
                      let c = fn[value](arg: I64) -> I64 { return arg + value; }; \
                  }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 2);
    assert_eq!(count(&parsed, SyntaxKind::LocalDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::ClosureDeclaration), 1);
    assert!(
        count(&parsed, SyntaxKind::TypeRef) >= 5,
        "structural fn(...) type syntax remains represented as TypeRef rather than closure syntax"
    );
}
