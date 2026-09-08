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

fn child_texts(parsed: &Parse, list_kind: SyntaxKind, child_kind: SyntaxKind) -> Vec<String> {
    parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == list_kind)
        .flat_map(|list| {
            list.children()
                .filter(move |child| child.kind() == child_kind)
                .map(|child| child.text().to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn generic_syntax_kinds_append_without_renumbering_existing_kinds() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Less).0, 120);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::LBracket).0, 121);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::RBracket).0, 122);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::GenericTypeParameterList).0,
        123
    );
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::GenericTypeParameter).0,
        124
    );
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::GenericTypeArgumentList).0,
        125
    );
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::GenericTypeArgument).0,
        126
    );
}

#[test]
fn function_generic_parameters_are_nonempty_ordered_and_allow_trailing_comma() {
    let parsed = parse("fn map[T, U,](value: T) -> U { return value; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeParameterList), 1);
    assert_eq!(
        child_texts(
            &parsed,
            SyntaxKind::GenericTypeParameterList,
            SyntaxKind::GenericTypeParameter
        ),
        ["T", "U"]
    );

    let empty = parse("fn bad[]() {}");
    assert!(!empty.errors().is_empty());
    assert_eq!(count(&empty, SyntaxKind::GenericTypeParameterList), 1);
    assert_eq!(count(&empty, SyntaxKind::GenericTypeParameter), 0);
}

#[test]
fn generic_call_arguments_are_ordered_qualified_and_allow_trailing_comma() {
    let parsed = parse(
        "fn caller[T](value: I64) -> I64 { return dep::apply[I64, dep::Thing, T,](value); }",
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeArgumentList), 1);
    assert_eq!(
        child_texts(
            &parsed,
            SyntaxKind::GenericTypeArgumentList,
            SyntaxKind::GenericTypeArgument
        ),
        ["I64", "dep::Thing", "T"]
    );
    assert_eq!(count(&parsed, SyntaxKind::QualifiedModuleMember), 2);
}

#[test]
fn generic_calls_parse_in_every_existing_direct_call_receiving_context() {
    let parsed = parse(
        r#"
record R { field: I64 }
fn use(value: I64) {
    sink[I64](value);
    if pred[Bool](value) {}
    let selected: I64 = make[R](value).field;
    let R { field: bound } = make[R](value);
    if let R { field: 1 } = (dep::make[R](value)) {}
}
"#,
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeArgumentList), 5);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 5);
    assert_eq!(count(&parsed, SyntaxKind::CallStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::IfStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&parsed, SyntaxKind::RecordDestructuringDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::RefutableRecordSelectionStatement), 1);
}

#[test]
fn generic_call_list_is_nonempty_and_requires_the_ordinary_argument_list() {
    let empty = parse("fn f() { g[](); }");
    assert!(!empty.errors().is_empty());
    assert_eq!(count(&empty, SyntaxKind::GenericTypeArgumentList), 1);
    assert_eq!(count(&empty, SyntaxKind::GenericTypeArgument), 0);

    let incomplete = parse("fn f() { g[I64]; }");
    assert!(!incomplete.errors().is_empty());
    assert_eq!(count(&incomplete, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&incomplete, SyntaxKind::GenericTypeArgumentList), 1);
    assert_eq!(count(&incomplete, SyntaxKind::ArgumentList), 1);
}

#[test]
fn brackets_do_not_create_general_index_or_postfix_syntax() {
    for source in [
        "fn f(value: I64) { let x: I64 = value[I64]; }",
        "fn f(value: I64) { let x: I64 = (value)[I64]; }",
        "record R { field: I64 } fn f(value: R) { let x: I64 = value.field[I64]; }",
    ] {
        let parsed = parse(source);
        assert!(!parsed.errors().is_empty(), "source unexpectedly accepted: {source}");
    }
}

#[test]
fn missing_generic_list_close_recovers_at_following_parenthesis() {
    let declaration = parse("fn f[T(value: T) {}");
    assert!(!declaration.errors().is_empty());
    assert_eq!(count(&declaration, SyntaxKind::ParameterList), 1);

    let call = parse("fn f(value: I64) { g[I64(value); }");
    assert!(!call.errors().is_empty());
    assert_eq!(count(&call, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&call, SyntaxKind::ArgumentList), 1);
}
