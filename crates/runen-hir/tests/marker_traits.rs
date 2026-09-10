use runen_hir::{
    CallTarget, DiagnosticKind, ImportTarget, IntrinsicType, MarkerImplementationTarget, ModuleId,
    OwnedUse, SourceUnit, Type, TypedCompilation, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

#[test]
fn retains_nominal_traits_implementations_and_exact_requirement_sets() {
    let hir = build(
        "trait A; trait B; record Ticket {} \
         impl I64: A; impl Ticket: B; \
         fn pair[T: B + A, U](left: T, right: U) {} \
         fn pair_reordered[V: A + B](value: V) {}",
    )
    .expect("marker declarations, implementations and requirements are valid");

    assert_eq!(hir.marker_traits.len(), 2);
    assert_eq!(hir.marker_implementations.len(), 2);
    assert_eq!(hir.modules[0].marker_traits.len(), 2);
    let a = hir
        .marker_traits
        .iter()
        .find(|marker| marker.name == "A")
        .expect("A exists")
        .id;
    let b = hir
        .marker_traits
        .iter()
        .find(|marker| marker.name == "B")
        .expect("B exists")
        .id;
    let pair = function(&hir, "pair");
    assert_eq!(pair.type_parameters[0].requirements.len(), 2);
    assert!(pair.type_parameters[0].requirements.contains(&a));
    assert!(pair.type_parameters[0].requirements.contains(&b));
    assert!(pair.type_parameters[1].requirements.is_empty());
    let pair_reordered = function(&hir, "pair_reordered");
    assert_eq!(
        pair.type_parameters[0].requirements, pair_reordered.type_parameters[0].requirements,
        "requirement source order is not semantic"
    );
    assert!(hir.marker_implementations.iter().any(|implementation| {
        implementation.trait_id == a
            && implementation.target == MarkerImplementationTarget::Intrinsic(IntrinsicType::I64)
    }));
}

#[test]
fn same_spelled_markers_in_distinct_modules_have_distinct_nominal_identities() {
    let left = parse("export trait Marker;");
    let right = parse("export trait Marker;");
    let caller = parse("import a; import b; fn f[T: a::Marker + b::Marker](value: T) {}");
    let imports = [
        ImportTarget::new("a", ModuleId::new(2)).expect("valid alias"),
        ImportTarget::new("b", ModuleId::new(3)).expect("valid alias"),
    ];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &left, &[]),
        SourceUnit::new(ModuleId::new(3), &right, &[]),
    ])
    .expect("same spelling in distinct modules denotes distinct nominal marker identities");

    let left_id = hir
        .marker_traits
        .iter()
        .find(|marker| marker.module == ModuleId::new(2) && marker.name == "Marker")
        .expect("left marker exists")
        .id;
    let right_id = hir
        .marker_traits
        .iter()
        .find(|marker| marker.module == ModuleId::new(3) && marker.name == "Marker")
        .expect("right marker exists")
        .id;
    assert_ne!(left_id, right_id);

    let requirements = &function(&hir, "f").type_parameters[0].requirements;
    assert_eq!(requirements.len(), 2);
    assert!(requirements.contains(&left_id));
    assert!(requirements.contains(&right_id));
}

#[test]
fn trait_declarations_share_the_existing_module_namespace() {
    for source in [
        "trait Name; record Name {}",
        "fn Name() {} trait Name;",
        "trait Name; trait Name;",
    ] {
        let errors = build(source).expect_err("duplicate module binding must be rejected");
        assert!(has_diagnostic(
            &errors,
            DiagnosticKind::DuplicateModuleBinding
        ));
    }
}

#[test]
fn marker_trait_binding_conflicts_with_import_alias() {
    let local = parse("trait dep; import dep; fn f() {}");
    let target = parse("export trait Marker;");
    assert!(local.errors().is_empty(), "{:?}", local.errors());
    assert!(target.errors().is_empty(), "{:?}", target.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &local, &imports),
        SourceUnit::new(ModuleId::new(2), &target, &[]),
    ])
    .expect_err("trait binding participates in ordinary import-alias conflict");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ImportDeclarationConflict
    ));
}

#[test]
fn marker_reference_lookup_uses_module_domain_not_generic_slot_domain() {
    let hir = build("trait T; fn f[T: T](value: T) -> T { return value; }")
        .expect("bound-side T denotes module marker while type-position T denotes slot");
    let marker = hir.marker_traits[0].id;
    let f = function(&hir, "f");
    assert!(f.type_parameters[0].requirements.contains(&marker));
    assert_eq!(f.parameters[0].ty, Type::Parameter(f.type_parameters[0].id));

    let errors = build("record R {} fn f[T: R](value: T) {}")
        .expect_err("wrong-category module binding selected for marker reference is final");
    assert!(has_diagnostic(&errors, DiagnosticKind::ExpectedMarkerTrait));
    assert!(!has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
}

#[test]
fn qualified_marker_lookup_honors_accessibility_and_wrong_category_finality() {
    let defs = parse("export trait Public; trait Private; export record R {}");
    let imports = [ImportTarget::new("defs", ModuleId::new(2)).expect("valid alias")];

    let good = parse("import defs; fn f[T: defs::Public](value: T) {}");
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &good, &imports),
        SourceUnit::new(ModuleId::new(2), &defs, &[]),
    ])
    .expect("qualified exported marker reference is accessible");
    assert_eq!(function(&hir, "f").type_parameters[0].requirements.len(), 1);

    let private = parse("import defs; fn f[T: defs::Private](value: T) {}");
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &private, &imports),
        SourceUnit::new(ModuleId::new(2), &defs, &[]),
    ])
    .expect_err("qualified private marker binding is inaccessible");
    assert!(has_diagnostic(&errors, DiagnosticKind::InaccessibleBinding));

    let wrong_category = parse("import defs; fn f[T: defs::R](value: T) {}");
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &wrong_category, &imports),
        SourceUnit::new(ModuleId::new(2), &defs, &[]),
    ])
    .expect_err("qualified wrong-category binding is final");
    assert!(has_diagnostic(&errors, DiagnosticKind::ExpectedMarkerTrait));
    assert!(!has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
}

#[test]
fn duplicate_resolved_marker_requirement_is_rejected_independent_of_spelling() {
    let defs = parse("export trait Marker;");
    let caller = parse("import a; import b; fn f[T: a::Marker + b::Marker](value: T) {}");
    assert!(defs.errors().is_empty(), "{:?}", defs.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [
        ImportTarget::new("a", ModuleId::new(2)).expect("valid alias"),
        ImportTarget::new("b", ModuleId::new(2)).expect("valid alias"),
    ];
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &defs, &[]),
    ])
    .expect_err("two spellings resolving to one marker identity are a duplicate requirement");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::DuplicateMarkerRequirement
    ));
}

#[test]
fn exported_generic_rejects_private_marker_requirement_but_private_function_allows_it() {
    let private = build("trait Marker; fn f[T: Marker](value: T) {}");
    assert!(
        private.is_ok(),
        "private signature may mention private marker"
    );

    let errors = build("trait Marker; export fn f[T: Marker](value: T) {}")
        .expect_err("exported signature may not expose private marker");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::PrivateMarkerTraitInExportedSignature
    ));
}

#[test]
fn implementation_targets_are_only_intrinsics_or_nominal_records() {
    let hir = build(
        "trait Marker; record Ticket {} fn NotAType() {} \
         impl I64: Marker; impl Ticket: Marker;",
    )
    .expect("intrinsic and nominal-record implementation targets are admitted");
    assert_eq!(hir.marker_implementations.len(), 2);

    let errors = build("trait Marker; fn F() {} impl F: Marker;")
        .expect_err("function binding cannot be an implementation target");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::InvalidMarkerImplementationTarget
    ));
}

#[test]
fn foreign_trait_and_foreign_record_implementation_is_admitted_when_both_resolve() {
    let traits = parse("export trait Marker;");
    let types = parse("export record Ticket {}");
    let implementations = parse("import tr; import ty; impl ty::Ticket: tr::Marker;");
    let impl_imports = [
        ImportTarget::new("tr", ModuleId::new(2)).expect("valid alias"),
        ImportTarget::new("ty", ModuleId::new(3)).expect("valid alias"),
    ];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &traits, &[]),
        SourceUnit::new(ModuleId::new(3), &types, &[]),
        SourceUnit::new(ModuleId::new(4), &implementations, &impl_imports),
    ])
    .expect("first slice has no orphan restriction");
    assert_eq!(hir.marker_implementations.len(), 1);
}

#[test]
fn duplicate_exact_implementation_pair_is_compilation_global_and_identity_based() {
    let defs = parse("export trait Marker; export record Ticket {}");
    let first = parse("import a; impl a::Ticket: a::Marker;");
    let second = parse("import b; impl b::Ticket: b::Marker;");
    let first_imports = [ImportTarget::new("a", ModuleId::new(2)).expect("valid alias")];
    let second_imports = [ImportTarget::new("b", ModuleId::new(2)).expect("valid alias")];

    for reversed in [false, true] {
        let units = if reversed {
            vec![
                SourceUnit::new(ModuleId::new(2), &defs, &[]),
                SourceUnit::new(ModuleId::new(4), &second, &second_imports),
                SourceUnit::new(ModuleId::new(3), &first, &first_imports),
            ]
        } else {
            vec![
                SourceUnit::new(ModuleId::new(2), &defs, &[]),
                SourceUnit::new(ModuleId::new(3), &first, &first_imports),
                SourceUnit::new(ModuleId::new(4), &second, &second_imports),
            ]
        };
        let errors = build_typed_hir(&units)
            .expect_err("same resolved pair declared anywhere twice violates exact coherence");
        assert!(has_diagnostic(
            &errors,
            DiagnosticKind::DuplicateMarkerImplementation
        ));
    }
}

#[test]
fn distinct_implementation_pairs_do_not_conflict() {
    let hir = build(
        "trait A; trait B; record Ticket {} \
         impl Ticket: A; impl Ticket: B; impl I64: A;",
    )
    .expect("distinct exact propositions coexist");
    assert_eq!(hir.marker_implementations.len(), 3);
}

#[test]
fn concrete_generic_argument_requires_exact_global_implementation() {
    let accepted = build(
        "trait Marker; impl I64: Marker; \
         fn id[T: Marker](value: T) -> T { return value; } \
         fn caller(value: I64) -> I64 { return id[I64](value); }",
    )
    .expect("exact concrete implementation discharges marker obligation");
    let caller = function(&accepted, "caller");
    let returned = caller
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("caller returns one value");
    let ValueKind::Call {
        target: CallTarget::Direct { type_arguments, .. },
        ..
    } = &returned.kind
    else {
        panic!("caller return remains direct call");
    };
    assert_eq!(type_arguments, &[Type::Intrinsic(IntrinsicType::I64)]);

    let errors = build(
        "trait Marker; fn id[T: Marker](value: T) -> T { return value; } \
         fn caller(value: I64) -> I64 { return id[I64](value); }",
    )
    .expect_err("missing concrete implementation rejects application");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::UnsatisfiedMarkerRequirement {
            argument: Type::Intrinsic(IntrinsicType::I64),
            ..
        }
    )));
}

#[test]
fn abstract_generic_evidence_propagates_only_from_enclosing_declared_requirement_set() {
    let accepted = build(
        "trait Marker; \
         fn sink[T: Marker](value: T) {} \
         fn outer[U: Marker](value: U) { sink[U](value); }",
    )
    .expect("enclosing exact marker requirement discharges abstract application");
    let outer = function(&accepted, "outer");
    assert_eq!(outer.body.statements.len(), 1);

    let errors = build(
        "trait Marker; impl I64: Marker; \
         fn sink[T: Marker](value: T) {} \
         fn outer[U](value: U) { sink[U](value); } \
         fn caller(value: I64) { outer[I64](value); }",
    )
    .expect_err("future concrete implementation cannot supply missing abstract evidence");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::UnsatisfiedMarkerRequirement {
            argument: Type::Parameter(_),
            ..
        }
    )));
}

#[test]
fn marker_obligation_failure_precedes_and_does_not_commit_argument_producer_effects() {
    let errors = build(
        "trait Marker; record Ticket {} \
         fn sink[T: Marker](value: T) {} \
         fn plain[T](value: T) {} \
         fn f(value: Ticket) { sink[Ticket](value); plain[Ticket](value); }",
    )
    .expect_err("first application lacks marker evidence");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::UnsatisfiedMarkerRequirement {
            argument: Type::Record(_),
            ..
        }
    )));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed marker application must not consume its ordinary argument"
    );
}

#[test]
fn marker_requirement_does_not_widen_generic_body_operational_capabilities() {
    let hir = build("trait Copy; fn id[T: Copy](value: T) -> T { return value; }")
        .expect("marker spelling Copy still leaves an abstract value opaque");
    let id = function(&hir, "id");
    let returned = id
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("id returns value");
    let ValueKind::BindingUse { ownership, .. } = returned.kind else {
        panic!("marker-bounded return remains ordinary binding use");
    };
    assert_eq!(ownership, OwnedUse::Consume);

    let errors =
        build("trait Copy; fn bad[T: Copy](left: T, right: T) -> T { return left + right; }")
            .expect_err("marker named Copy does not grant scalar addition");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::AdditionRequiresIntegerOrFloating {
            required: Type::Parameter(_),
        }
    )));
}

#[test]
fn coherent_implementation_is_global_evidence_without_importing_impl_module() {
    let defs = parse("export trait Marker; export record Ticket {}");
    let impl_unit = parse("import defs; impl defs::Ticket: defs::Marker;");
    let app = parse(
        "import defs; \
         fn id[T: defs::Marker](value: T) -> T { return value; } \
         fn use_ticket(value: defs::Ticket) -> defs::Ticket { \
             return id[defs::Ticket](value); \
         }",
    );
    let impl_imports = [ImportTarget::new("defs", ModuleId::new(2)).expect("valid alias")];
    let app_imports = [ImportTarget::new("defs", ModuleId::new(2)).expect("valid alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &app, &app_imports),
        SourceUnit::new(ModuleId::new(2), &defs, &[]),
        SourceUnit::new(ModuleId::new(3), &impl_unit, &impl_imports),
    ])
    .expect("application consults global proposition without importing impl module");
    assert_eq!(hir.marker_implementations.len(), 1);
    assert!(function(&hir, "use_ticket").body.terminal_return.is_some());
}
