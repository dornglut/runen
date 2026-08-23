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
fn bang_is_append_only_and_lossless() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::ContinueStatement).0, 81);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Bang).0, 82);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BooleanNotValue).0, 83);

    let source = "fn negate(flag: Bool) -> Bool { return !flag; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let bang = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == SyntaxKind::Bang)
        .expect("Bang token");
    assert_eq!(bang.text(), "!");
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 1);
}

#[test]
fn repeated_prefix_negation_is_right_recursive_with_trivia_retained() {
    let source = "fn negate(flag: Bool) -> Bool { return ! /* outer */ ! flag; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let outer = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanNotValue)
        .expect("outer Boolean-not value");
    let inner = outer
        .children()
        .find(|node| node.kind() == SyntaxKind::BooleanNotValue)
        .expect("nested Boolean-not operand");
    assert!(
        inner
            .children()
            .any(|node| node.kind() == SyntaxKind::IdentifierUse)
    );
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 2);
}

#[test]
fn ordinary_prefix_negation_composes_with_every_represented_atom_family() {
    let source = r#"
record Flag { ready: Bool }
fn make() -> Bool { return true; }
fn atoms(root: Flag) {
    let literal: Bool = !true;
    let construction: Bool = !Flag { ready: true };
    let call: Bool = !make();
    let field: Bool = !root.ready;
    let integer: Bool = !-1;
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 5);
    assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 1);
}

#[test]
fn conditional_prefix_stays_recursive_and_keeps_arm_boundaries() {
    let source = r#"
record Flag { ready: Bool }
fn choose(flag: Bool) {
    if !flag {}
    while !!flag {}
    if !Flag { ready: true }.ready {}
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 4);
    assert_eq!(count(&parsed, SyntaxKind::IfStatement), 2);
    assert_eq!(count(&parsed, SyntaxKind::WhileStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 1);
    assert_eq!(count(&parsed, SyntaxKind::FieldValueUse), 1);
}

#[test]
fn conditional_prefix_never_admits_standalone_record_construction() {
    for source in [
        "record Flag { ready: Bool } fn bad() { if !Flag { ready: true } {} }",
        "record Flag { ready: Bool } fn bad() { while !!Flag { ready: true } {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
        assert!(count(&parsed, SyntaxKind::BooleanNotValue) >= 1);
    }
}

#[test]
fn boolean_not_is_not_reparsed_as_a_field_receiver_or_pattern_scrutinee() {
    let field_source = "fn use_field(root: Bool) { let value: Bool = !root.ready; }";
    let field = parse(field_source);
    assert_eq!(field.text(), field_source);
    assert!(field.errors().is_empty(), "{:?}", field.errors());
    let outer = field
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanNotValue)
        .expect("Boolean-not value");
    let field_use = outer
        .children()
        .find(|node| node.kind() == SyntaxKind::FieldValueUse)
        .expect("field use is the operand");
    assert_eq!(
        field_use
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::BooleanNotValue)
            .count(),
        0
    );

    let pattern_source = "record Flag { ready: Bool } fn make() -> Flag { return Flag { ready: true }; } fn bad() { let Flag { ready: value } = !make(); }";
    let pattern = parse(pattern_source);
    assert_eq!(pattern.text(), pattern_source);
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::BooleanNotValue), 0);
}

#[test]
fn malformed_prefix_forms_remain_lossless_and_do_not_create_inequality() {
    let inequality_source = "fn bad(flag: Bool) { let value: Bool = != flag; }";
    let inequality = parse(inequality_source);
    assert_eq!(inequality.text(), inequality_source);
    assert!(!inequality.errors().is_empty());
    let nontrivia = inequality
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.kind())
        .collect::<Vec<_>>();
    assert!(
        nontrivia
            .windows(2)
            .any(|window| window == [SyntaxKind::Bang, SyntaxKind::Eq])
    );

    let numeric_source = "fn bad(flag: Bool) { let value: Bool = -!flag; }";
    let numeric = parse(numeric_source);
    assert_eq!(numeric.text(), numeric_source);
    assert!(!numeric.errors().is_empty());
}

#[test]
fn missing_prefix_operands_preserve_structural_recovery_boundaries() {
    let body_source = "fn bad() { if ! {} let value: Bool = true; while ! {} sink(); }";
    let body = parse(body_source);
    assert_eq!(body.text(), body_source);
    assert!(!body.errors().is_empty());
    assert_eq!(count(&body, SyntaxKind::IfStatement), 1);
    assert_eq!(count(&body, SyntaxKind::WhileStatement), 1);
    assert_eq!(count(&body, SyntaxKind::LocalDeclaration), 1);
    assert_eq!(count(&body, SyntaxKind::CallStatement), 1);

    let item_source = "fn bad() -> Bool { return !; } fn after() {}";
    let item = parse(item_source);
    assert_eq!(item.text(), item_source);
    assert!(!item.errors().is_empty());
    assert_eq!(count(&item, SyntaxKind::FunctionDefinition), 2);
}
