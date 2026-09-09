use runen_syntax::{Parse, SyntaxKind, parse_source, user_identifier_key};

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
fn marker_trait_syntax_kinds_append_without_renumbering_existing_kinds() {
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::GenericTypeArgument).0,
        126
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::TraitDeclaration).0, 127);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::TraitImplementation).0,
        128
    );
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::ImplementationTarget).0,
        129
    );
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::TraitReference).0, 130);
    assert_eq!(
        rowan::SyntaxKind::from(SyntaxKind::TraitRequirementClause).0,
        131
    );
}

#[test]
fn trait_and_impl_remain_contextual_identifier_keys() {
    assert_eq!(user_identifier_key("trait").as_deref(), Some("trait"));
    assert_eq!(user_identifier_key("impl").as_deref(), Some("impl"));

    let source = "record R { trait: I64, impl: I64 } \
                  fn trait(impl: I64) -> I64 { let trait: I64 = impl; return trait; } \
                  fn impl(trait: I64) -> I64 { let impl: I64 = trait; return impl; } \
                  fn caller(value: I64) { trait(value); impl(value); }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::TraitDeclaration), 0);
    assert_eq!(count(&parsed, SyntaxKind::TraitImplementation), 0);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 2);
}

#[test]
fn parses_selected_marker_declarations_implementations_and_bounds() {
    let source = "trait Marker; \
                  export trait PublicMarker; \
                  trait impl; \
                  impl I64: Marker; \
                  impl Ticket: Marker; \
                  impl dep::Ticket: traits::PublicMarker; \
                  fn pass[T: Marker](value: T) -> T { return value; } \
                  fn pair[T: Marker + dep::OtherMarker, U](left: T, right: U) {}";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::TraitDeclaration), 3);
    assert_eq!(count(&parsed, SyntaxKind::TraitImplementation), 3);
    assert_eq!(count(&parsed, SyntaxKind::ImplementationTarget), 3);
    assert_eq!(count(&parsed, SyntaxKind::TraitRequirementClause), 2);
    assert_eq!(count(&parsed, SyntaxKind::TraitReference), 7);
    assert_eq!(count(&parsed, SyntaxKind::QualifiedModuleMember), 3);
}

#[test]
fn comma_still_separates_type_parameters_after_one_marker_bound() {
    let parsed = parse("trait A; fn f[T: A, B](left: T, right: B) {}");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeParameter), 2);
    assert_eq!(count(&parsed, SyntaxKind::TraitRequirementClause), 1);
    assert_eq!(count(&parsed, SyntaxKind::TraitReference), 1);
}

#[test]
fn selected_marker_syntax_rejects_non_selected_forms() {
    for source in [
        "trait Marker {}",
        "trait Marker; impl I64: Marker {}",
        "trait Marker; export impl I64: Marker;",
        "trait Marker; impl Marker for I64;",
        "trait Marker; trait Other; impl I64: Marker + Other;",
        "trait Marker; fn f[T: Marker +](value: T) {}",
        "trait Marker; impl &I64: Marker;",
        "trait Marker; impl raw I64: Marker;",
    ] {
        let parsed = parse(source);
        assert!(
            !parsed.errors().is_empty(),
            "source unexpectedly accepted: {source}"
        );
    }
}

#[test]
fn contextual_item_keys_do_not_become_nested_recovery_starters() {
    let source = "record R { trait: I64, impl: I64 } \
                  fn f(trait: I64, impl: I64) { \
                      let trait: I64 = impl; \
                      sink(trait); \
                  } \
                  trait Marker; \
                  impl I64: Marker;";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::RecordField), 2);
    assert_eq!(count(&parsed, SyntaxKind::TraitDeclaration), 1);
    assert_eq!(count(&parsed, SyntaxKind::TraitImplementation), 1);
}

#[test]
fn marker_bounds_do_not_change_direct_call_type_argument_grammar() {
    let parsed = parse(
        "trait Marker; \
         fn sink[T: Marker](value: T) {} \
         fn outer[U: Marker](value: U) { sink[U](value); }",
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeArgumentList), 1);
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeArgument), 1);
    assert_eq!(count(&parsed, SyntaxKind::DirectCall), 1);
}
