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
fn conjunction_kinds_append_without_renumbering_existing_syntax() {
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::IntegerComplementValue).0,
        95
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::AmpAmp).0, 96);
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::BooleanAndValue).0, 97);

    let source = "fn f(a: Bool, b: Bool) -> Bool { return a && b; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::BooleanAndValue), 1);
    assert!(nontrivia_kinds(&parsed).contains(&SyntaxKind::AmpAmp));
}

#[test]
fn conjunction_lexing_keeps_longest_match_with_standalone_ampersand() {
    let assign_like = parse("fn f(a: Bool, b: Bool) { let value: Bool = a &&= b; }");
    assert_eq!(
        nontrivia_kinds(&assign_like)
            .windows(2)
            .filter(|window| *window == [SyntaxKind::AmpAmp, SyntaxKind::Eq])
            .count(),
        1
    );
    assert!(!assign_like.errors().is_empty());

    let triple = parse("fn f(a: Bool, b: Bool) { let value: Bool = a &&& b; }");
    assert!(
        nontrivia_kinds(&triple)
            .windows(2)
            .any(|window| window == [SyntaxKind::AmpAmp, SyntaxKind::Amp])
    );
    assert!(triple.errors().is_empty(), "{:?}", triple.errors());
    assert_eq!(count(&triple, SyntaxKind::SharedBorrowValue), 1);

    let spaced = parse("fn f(a: Bool, b: Bool) { let value: Bool = a & & b; }");
    assert_eq!(
        nontrivia_kinds(&spaced)
            .iter()
            .filter(|kind| **kind == SyntaxKind::Amp)
            .count(),
        2
    );
    assert!(!spaced.errors().is_empty());
}

#[test]
fn equality_remains_tighter_than_conjunction_on_both_sides() {
    let source = "fn f(a: Bool, b: Bool, c: Bool) { let left: Bool = a == b && c; let right: Bool = a && b == c; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let conjunctions = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::BooleanAndValue)
        .collect::<Vec<_>>();
    assert_eq!(conjunctions.len(), 2);

    let first_children = conjunctions[0]
        .children()
        .map(|child| child.kind())
        .collect::<Vec<_>>();
    assert!(first_children.contains(&SyntaxKind::BooleanEqualityValue));
    assert_eq!(
        conjunctions[0]
            .children()
            .filter(|child| child.kind() == SyntaxKind::BooleanEqualityValue)
            .count(),
        1
    );
    assert_eq!(
        conjunctions[1]
            .children()
            .filter(|child| child.kind() == SyntaxKind::BooleanEqualityValue)
            .count(),
        1
    );
}

#[test]
fn ungrouped_conjunction_chains_are_rejected_but_explicit_grouping_nests() {
    let chained = parse("fn f(a: Bool, b: Bool, c: Bool) -> Bool { return a && b && c; }");
    assert_eq!(count(&chained, SyntaxKind::BooleanAndValue), 1);
    assert!(!chained.errors().is_empty());

    for source in [
        "fn f(a: Bool, b: Bool, c: Bool) -> Bool { return (a && b) && c; }",
        "fn f(a: Bool, b: Bool, c: Bool) -> Bool { return a && (b && c); }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::BooleanAndValue), 2);
        assert!(count(&parsed, SyntaxKind::GroupedValue) >= 1);
    }
}

#[test]
fn conditional_conjunction_preserves_standalone_record_construction_exclusion() {
    let valid = parse(
        "record Flag { ready: Bool } fn f(flag: Bool) { if Flag { ready: true }.ready && flag {} while flag && Flag { ready: true }.ready {} }",
    );
    assert_eq!(count(&valid, SyntaxKind::BooleanAndValue), 2);
    assert_eq!(count(&valid, SyntaxKind::RecordConstruction), 2);
    assert!(valid.errors().is_empty(), "{:?}", valid.errors());

    for source in [
        "record Flag { ready: Bool } fn f(flag: Bool) { if Flag { ready: true } && flag {} }",
        "record Flag { ready: Bool } fn f(flag: Bool) { if flag && Flag { ready: true } {} }",
        "record Flag { ready: Bool } fn f(flag: Bool) { while flag && !Flag { ready: true } {} }",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.text(), source);
        assert!(!parsed.errors().is_empty());
        assert_eq!(count(&parsed, SyntaxKind::RecordConstruction), 0);
    }
}

#[test]
fn conjunction_stays_a_value_and_does_not_expand_receiver_or_pattern_categories() {
    let field_source =
        "record Flag { ready: Bool } fn f(a: Bool, root: Flag) -> Bool { return a && root.ready; }";
    let field = parse(field_source);
    assert!(field.errors().is_empty(), "{:?}", field.errors());
    let conjunction = field
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::BooleanAndValue)
        .expect("conjunction value");
    assert!(
        conjunction
            .children()
            .any(|node| node.kind() == SyntaxKind::FieldValueUse)
    );

    let pattern_source = "record Flag { ready: Bool } fn f(a: Bool, b: Bool) { let Flag { ready: value } = a && b; }";
    let pattern = parse(pattern_source);
    assert_eq!(pattern.text(), pattern_source);
    assert!(!pattern.errors().is_empty());
    assert_eq!(count(&pattern, SyntaxKind::BooleanAndValue), 0);
}

#[test]
fn conjunction_composes_without_reinterpreting_existing_prefix_or_literal_forms() {
    let source = "fn f(a: Bool, b: Bool) -> Bool { return !a == !b && -1 == -1; }";
    let parsed = parse(source);
    assert_eq!(parsed.text(), source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::BooleanAndValue), 1);
    assert_eq!(count(&parsed, SyntaxKind::BooleanEqualityValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::BooleanNotValue), 2);
    assert_eq!(count(&parsed, SyntaxKind::DecimalIntegerLiteral), 2);
}
