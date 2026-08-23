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

fn nontrivia_kinds(parsed: &Parse) -> Vec<SyntaxKind> {
    parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.kind())
        .collect()
}

#[test]
fn operator_syntax_kinds_are_append_only_and_lossless() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::ContinueStatement).0, 81);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Bang).0, 82);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BooleanNotValue).0, 83);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::EqEq).0, 84);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BangEq).0, 85);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::BooleanEqualityValue).0,
        86
    );

    let source = "fn compare(a: Bool, b: Bool) -> Bool { return !a == b; } fn differ(a: Bool, b: Bool) -> Bool { return a != b; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let tokens = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| {
            matches!(
                token.kind(),
                SyntaxKind::Bang | SyntaxKind::EqEq | SyntaxKind::BangEq
            )
        })
        .map(|token| (token.kind(), token.text().to_owned()))
        .collect::<Vec<_>>();
    assert!(tokens.contains(&(SyntaxKind::Bang, "!".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::EqEq, "==".to_owned())));
    assert!(tokens.contains(&(SyntaxKind::BangEq, "!=".to_owned())));
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 2);
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
fn equality_lexing_uses_longest_match_and_keeps_nearby_forms_distinct() {
    let valid_source = "fn f(a: Bool, b: Bool) { let equal: Bool = a == b; let different: Bool = a != b; let negated: Bool = !a; }";
    let valid = parse(valid_source);
    assert_eq!(valid.text(), valid_source);
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    let valid_kinds = nontrivia_kinds(&valid);
    assert!(valid_kinds.contains(&SyntaxKind::EqEq));
    assert!(valid_kinds.contains(&SyntaxKind::BangEq));
    assert!(valid_kinds.contains(&SyntaxKind::Bang));
    assert!(valid_kinds.contains(&SyntaxKind::Eq));

    let spaced_source = "fn bad(a: Bool) { let value: Bool = a ! = a; }";
    let spaced = parse(spaced_source);
    assert_eq!(spaced.text(), spaced_source);
    assert!(!spaced.errors().is_empty());
    assert!(
        nontrivia_kinds(&spaced)
            .windows(2)
            .any(|window| window == [SyntaxKind::Bang, SyntaxKind::Eq])
    );

    for (source, first, second) in [
        (
            "fn bad(a: Bool) { let value: Bool = a === a; }",
            SyntaxKind::EqEq,
            SyntaxKind::Eq,
        ),
        (
            "fn bad(a: Bool) { let value: Bool = a !== a; }",
            SyntaxKind::BangEq,
            SyntaxKind::Eq,
        ),
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert!(
            nontrivia_kinds(&parsed)
                .windows(2)
                .any(|window| window == [first, second])
        );
    }
}

#[test]
fn equality_is_one_bounded_tier_above_prefix_negation() {
    let source = "fn f(a: Bool, b: Bool) { let left: Bool = !a == b; let right: Bool = a == !b; let different: Bool = !a != !b; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let equalities = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .collect::<Vec<_>>();
    assert_eq!(equalities.len(), 3);

    let first_children = equalities[0]
        .children()
        .filter(|child| {
            matches!(
                child.kind(),
                SyntaxKind::BooleanNotValue | SyntaxKind::IdentifierUse
            )
        })
        .map(|child| child.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        first_children,
        [SyntaxKind::BooleanNotValue, SyntaxKind::IdentifierUse]
    );

    let second_children = equalities[1]
        .children()
        .filter(|child| {
            matches!(
                child.kind(),
                SyntaxKind::BooleanNotValue | SyntaxKind::IdentifierUse
            )
        })
        .map(|child| child.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        second_children,
        [SyntaxKind::IdentifierUse, SyntaxKind::BooleanNotValue]
    );

    for negation in parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::BooleanNotValue)
    {
        assert_eq!(
            negation
                .descendants()
                .filter(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
                .count(),
            0,
            "prefix negation must not recursively absorb equality"
        );
    }
}

#[test]
fn equality_chains_are_syntax_invalid_and_never_gain_associativity() {
    for source in [
        "fn bad(a: Bool, b: Bool, c: Bool) { let value: Bool = a == b == c; }",
        "fn bad(a: Bool, b: Bool, c: Bool) { let value: Bool = a != b == c; }",
        "fn bad(a: Bool, b: Bool, c: Bool) { let value: Bool = a == b != c; }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 1);
    }
}

#[test]
fn ordinary_equality_composes_with_existing_prefix_atom_families() {
    let source = r#"
record Flag { ready: Bool }
fn make() -> Bool { return true; }
fn atoms(root: Flag) {
    let literals: Bool = true == false;
    let call_field: Bool = make() == !root.ready;
    let constructions: Bool = Flag { ready: true } != Flag { ready: false };
    let integers: Bool = -1 == 2;
}
"#;
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 4);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
    assert_eq!(count(&parsed, SyntaxKind::FieldValueUse), 1);
    assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 2);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 2);
}

#[test]
fn conditional_equality_reuses_conditional_prefix_restrictions_on_both_sides() {
    let valid_source = r#"
record Flag { ready: Bool }
fn choose(flag: Bool) {
    if Flag { ready: true }.ready == flag {}
    while flag != Flag { ready: false }.ready {}
    if !Flag { ready: true }.ready == !flag {}
}
"#;
    let valid = parse(valid_source);
    assert_eq!(valid.text(), valid_source);
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());
    assert_eq!(count(&valid, SyntaxKind::BooleanEqualityValue), 3);
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 3);
    assert_eq!(count(&valid, SyntaxKind::FieldValueUse), 3);

    for source in [
        "record Flag { ready: Bool } fn bad(flag: Bool) { if Flag { ready: true } == flag {} }",
        "record Flag { ready: Bool } fn bad(flag: Bool) { if flag == Flag { ready: true } {} }",
        "record Flag { ready: Bool } fn bad(flag: Bool) { while flag != !Flag { ready: true } {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
    }
}

#[test]
fn equality_is_not_reparsed_as_a_field_receiver_or_pattern_scrutinee() {
    let field_source = "record Flag { ready: Bool } fn f(a: Bool, root: Flag) { let value: Bool = a == root.ready; }";
    let field = parse(field_source);
    assert_eq!(field.text(), field_source);
    assert!(field.errors().is_empty(), "{:?}", field.errors());
    let equality = field
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
        .expect("Boolean equality value");
    assert!(
        equality
            .children()
            .any(|node| node.kind() == SyntaxKind::FieldValueUse)
    );
    let field_use = equality
        .children()
        .find(|node| node.kind() == SyntaxKind::FieldValueUse)
        .expect("right field-value operand");
    assert_eq!(
        field_use
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::BooleanEqualityValue)
            .count(),
        0
    );

    let pattern_source = "record Flag { ready: Bool } fn bad(a: Flag, b: Flag) { let Flag { ready: value } = a == b; }";
    let pattern = parse(pattern_source);
    assert_eq!(pattern.text(), pattern_source);
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::BooleanEqualityValue), 0);
}

#[test]
fn malformed_operator_forms_remain_lossless() {
    let numeric_source = "fn bad(flag: Bool) { let value: Bool = -!flag; }";
    let numeric = parse(numeric_source);
    assert_eq!(numeric.text(), numeric_source);
    assert!(!numeric.errors().is_empty());

    let leading_operator = "fn bad(flag: Bool) { let value: Bool = == flag; } fn after() {}";
    let leading = parse(leading_operator);
    assert_eq!(leading.text(), leading_operator);
    assert!(!leading.errors().is_empty());
    assert_eq!(count(&leading, SyntaxKind::FunctionDefinition), 2);
}

#[test]
fn missing_operator_operands_preserve_structural_recovery_boundaries() {
    let body_source = "fn bad(flag: Bool) { let first: Bool = flag == ; let second: Bool = true; if flag != {} sink(); while flag == ! {} }";
    let body = parse(body_source);
    assert_eq!(body.text(), body_source);
    assert!(!body.errors().is_empty());
    assert_eq!(count(&body, SyntaxKind::LocalDeclaration), 2);
    assert_eq!(count(&body, SyntaxKind::IfStatement), 1);
    assert_eq!(count(&body, SyntaxKind::WhileStatement), 1);
    assert_eq!(count(&body, SyntaxKind::CallStatement), 1);

    let item_source = "fn bad(flag: Bool) -> Bool { return flag == ; } fn after() {}";
    let item = parse(item_source);
    assert_eq!(item.text(), item_source);
    assert!(!item.errors().is_empty());
    assert_eq!(count(&item, SyntaxKind::FunctionDefinition), 2);
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
