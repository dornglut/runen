use runen_hir::{
    DiagnosticKind, ImportTarget, IntrinsicType, ModuleId, OwnedUse, ReferencePermission,
    ReferenceReferent, SafeReferenceResultContract, SourceUnit, Statement, Type, TypedCompilation,
    ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn compile(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .expect("function exists")
}

fn has_diagnostic(
    errors: &[runen_hir::Diagnostic],
    predicate: impl Fn(DiagnosticKind) -> bool,
) -> bool {
    errors.iter().any(|error| predicate(error.kind))
}

fn shared_reference(referent: ReferenceReferent) -> Type {
    Type::SafeReference {
        referent,
        permission: ReferencePermission::Shared,
    }
}

#[test]
fn retains_exact_shared_reference_identity_and_duplicates_reference_bindings() {
    let hir = compile("fn f(x: I64, r: &I64) { let s: &I64 = r; }")
        .expect("Shared-reference parameter and immutable local must validate");
    let f = function(&hir, "f");
    let reference_ty = shared_reference(ReferenceReferent::Intrinsic(IntrinsicType::I64));

    assert_eq!(f.parameters[1].ty, reference_ty);
    assert!(hir.type_is_duplicable(reference_ty));

    let Statement::Local {
        ty, initializer, ..
    } = &f.body.statements[0]
    else {
        panic!("expected Shared-reference local");
    };
    assert_eq!(*ty, reference_ty);
    assert_eq!(initializer.ty, reference_ty);
    assert!(matches!(
        initializer.kind,
        ValueKind::BindingUse {
            binding,
            ownership: OwnedUse::Duplicate,
        } if binding == f.parameters[1].binding
    ));
}

#[test]
fn root_borrow_and_dereference_retain_exact_binding_identities() {
    let hir = compile("fn f(x: I64) { let r: &I64 = &x; let y: I64 = *r; }")
        .expect("root Shared borrow followed by copy dereference must validate");
    let f = function(&hir, "f");

    let Statement::Local {
        binding: reference_binding,
        initializer: borrow,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected reference local");
    };
    assert!(matches!(
        &borrow.kind,
        ValueKind::ReferenceRoot {
            target,
            fields,
            permission: ReferencePermission::Shared,
        } if *target == f.parameters[0].binding && fields.is_empty()
    ));
    assert_eq!(
        borrow.ty,
        shared_reference(ReferenceReferent::Intrinsic(IntrinsicType::I64))
    );

    let Statement::Local { initializer, .. } = &f.body.statements[1] else {
        panic!("expected dereference result local");
    };
    assert_eq!(initializer.ty, Type::Intrinsic(IntrinsicType::I64));
    assert!(matches!(
        initializer.kind,
        ValueKind::ReferenceDereference {
            reference,
            ownership: OwnedUse::Duplicate,
        } if reference == *reference_binding
    ));
}

#[test]
fn shared_field_roots_retain_exact_ordered_paths_and_final_referents() {
    let hir = compile(
        "record copy Inner { value: I64 }\
         record copy Outer { inner: Inner, other: I64 }\
         fn f(root: Outer) {\
             let direct: &Inner = &root.inner;\
             let nested: &I64 = &root.inner.value;\
         }",
    )
    .expect("Shared field roots must resolve through the canonical structural field path");
    let f = function(&hir, "f");
    let inner = hir.records[0].id;

    let Statement::Local {
        initializer: direct,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected direct field-root local");
    };
    assert_eq!(
        direct.ty,
        shared_reference(ReferenceReferent::Record(inner))
    );
    assert!(matches!(
        &direct.kind,
        ValueKind::ReferenceRoot {
            target,
            fields,
            permission: ReferencePermission::Shared,
        } if *target == f.parameters[0].binding && fields.as_slice() == [0]
    ));

    let Statement::Local {
        initializer: nested,
        ..
    } = &f.body.statements[1]
    else {
        panic!("expected nested field-root local");
    };
    assert_eq!(
        nested.ty,
        shared_reference(ReferenceReferent::Intrinsic(IntrinsicType::I64))
    );
    assert!(matches!(
        &nested.kind,
        ValueKind::ReferenceRoot {
            target,
            fields,
            permission: ReferencePermission::Shared,
        } if *target == f.parameters[0].binding && fields.as_slice() == [0, 0]
    ));
}

#[test]
fn shared_field_roots_use_existing_field_type_and_admission_diagnostics() {
    let non_record = compile("fn f(x: I64) { let r: &I64 = &x.value; }")
        .expect_err("field-root selector through a non-record must reject");
    assert!(has_diagnostic(&non_record, |kind| kind
        == DiagnosticKind::ExpectedRecordForFieldAccess));

    let unknown = compile(
        "record copy Pair { left: I64 }\
         fn f(root: Pair) { let r: &I64 = &root.missing; }",
    )
    .expect_err("unknown field-root selector must use the canonical field diagnostic");
    assert!(has_diagnostic(&unknown, |kind| kind == DiagnosticKind::UnknownRecordField));

    let mismatch = compile(
        "record copy Pair { left: I64 }\
         fn f(root: Pair) { let r: &I32 = &root.left; }",
    )
    .expect_err("the final selected field type must match the required Shared referent");
    assert!(has_diagnostic(&mismatch, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let invalid_referent = compile(
        "record Ticket { value: I64 }\
         record Holder { ticket: Ticket }\
         fn f(root: Holder) { let r: &Ticket = &root.ticket; }",
    )
    .expect_err("Shared admission is determined by the selected field referent");
    assert!(has_diagnostic(&invalid_referent, |kind| matches!(
        kind,
        DiagnosticKind::InvalidSafeReferenceReferent {
            permission: ReferencePermission::Shared,
            ..
        }
    )));
}

#[test]
fn shared_field_relative_reborrows_retain_exact_paths_and_final_referents() {
    let hir = compile(
        "record copy Inner { value: I64 }\
         record copy Outer { inner: Inner, other: I64 }\
         fn f(parent: &Outer) {\
             let complete: &Outer = &*parent;\
             let direct: &Inner = &*parent.inner;\
             let nested: &I64 = &*parent.inner.value;\
         }",
    )
    .expect("Shared field-relative reborrows must resolve from the parent referent");
    let f = function(&hir, "f");
    let inner = hir.records[0].id;

    for (index, expected_fields, expected_ty) in [
        (0, Vec::<usize>::new(), f.parameters[0].ty),
        (
            1,
            vec![0],
            shared_reference(ReferenceReferent::Record(inner)),
        ),
        (
            2,
            vec![0, 0],
            shared_reference(ReferenceReferent::Intrinsic(IntrinsicType::I64)),
        ),
    ] {
        let Statement::Local { initializer, .. } = &f.body.statements[index] else {
            panic!("expected reborrow local");
        };
        assert_eq!(initializer.ty, expected_ty);
        assert!(matches!(
            &initializer.kind,
            ValueKind::ReferenceReborrow {
                reference,
                fields,
                permission: ReferencePermission::Shared,
            } if *reference == f.parameters[0].binding && *fields == expected_fields
        ));
    }
}

#[test]
fn shared_field_relative_reborrow_reuses_field_and_referent_diagnostics() {
    let non_record = compile("fn f(r: &I64) { let child: &I64 = &*r.value; }")
        .expect_err("relative selector through a non-record referent must reject");
    assert!(has_diagnostic(&non_record, |kind| kind
        == DiagnosticKind::ExpectedRecordForFieldAccess));

    let unknown = compile(
        "record copy Pair { left: I64 }\
         fn f(r: &Pair) { let child: &I64 = &*r.missing; }",
    )
    .expect_err("unknown relative field must use the canonical field diagnostic");
    assert!(has_diagnostic(&unknown, |kind| kind == DiagnosticKind::UnknownRecordField));

    let mismatch = compile(
        "record copy Pair { left: I64 }\
         fn f(r: &Pair) { let child: &I32 = &*r.left; }",
    )
    .expect_err("selected relative field type must match the required Shared referent");
    assert!(has_diagnostic(&mismatch, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let invalid_referent = compile(
        "record Ticket { value: I64 }\
         record Holder { ticket: Ticket }\
         fn f(r: &mut Holder) { let child: &Ticket = &*r.ticket; }",
    )
    .expect_err("selected relative field must independently satisfy Shared referent admission");
    assert!(has_diagnostic(&invalid_referent, |kind| matches!(
        kind,
        DiagnosticKind::InvalidSafeReferenceReferent {
            permission: ReferencePermission::Shared,
            ..
        }
    )));
}

#[test]
fn shared_field_relative_reborrow_reuses_exact_field_accessibility() {
    let dep = parse("export record copy Public { export shown: I64, hidden: I64 }");
    let app_ok = parse("import dep; fn f(r: &dep::Public) { let child: &I64 = &*r.shown; }");
    let dep_target = ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias");
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &dep, &[]),
        SourceUnit::new(ModuleId::new(1), &app_ok, std::slice::from_ref(&dep_target)),
    ])
    .expect("foreign exported relative field is accessible");

    let app_hidden = parse("import dep; fn f(r: &dep::Public) { let child: &I64 = &*r.hidden; }");
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &dep, &[]),
        SourceUnit::new(
            ModuleId::new(1),
            &app_hidden,
            std::slice::from_ref(&dep_target),
        ),
    ])
    .expect_err("foreign private relative field remains inaccessible");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::InaccessibleRecordField));
}

#[test]
fn projected_replacement_forms_reuse_canonical_field_type_and_permission_diagnostics() {
    let root_non_record = compile(
        "fn f(seed: I64) { let mut x: I64 = seed; let r: &mut I64 = &mut x.value; }",
    )
    .expect_err("replacement root selector through a non-record must reject canonically");
    assert!(has_diagnostic(&root_non_record, |kind| kind
        == DiagnosticKind::ExpectedRecordForFieldAccess));

    let root_unknown = compile(
        "record Box { value: I64 }\
         fn f(seed: Box) { let mut x: Box = seed; let r: &mut I64 = &mut x.missing; }",
    )
    .expect_err("unknown replacement-root selector must use the canonical field diagnostic");
    assert!(has_diagnostic(&root_unknown, |kind| kind == DiagnosticKind::UnknownRecordField));

    let root_mismatch = compile(
        "record Box { value: I64 }\
         fn f(seed: Box) { let mut x: Box = seed; let r: &mut I32 = &mut x.value; }",
    )
    .expect_err("replacement-root selected type must match the exact receiving referent");
    assert!(has_diagnostic(&root_mismatch, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let child_non_record = compile("fn f(r: &mut I64) { let child: &mut I64 = &mut *r.value; }")
        .expect_err("replacement child selector through a non-record must reject canonically");
    assert!(has_diagnostic(&child_non_record, |kind| kind
        == DiagnosticKind::ExpectedRecordForFieldAccess));

    let child_unknown = compile(
        "record Box { value: I64 }\
         fn f(r: &mut Box) { let child: &mut I64 = &mut *r.missing; }",
    )
    .expect_err("unknown replacement-child selector must use the canonical field diagnostic");
    assert!(has_diagnostic(&child_unknown, |kind| kind == DiagnosticKind::UnknownRecordField));

    let child_mismatch = compile(
        "record Box { value: I64 }\
         fn f(r: &mut Box) { let child: &mut I32 = &mut *r.value; }",
    )
    .expect_err("replacement-child selected type must match the exact receiving referent");
    assert!(has_diagnostic(&child_mismatch, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let strengthened = compile(
        "record copy Box { value: I64 }\
         fn f(r: &Box) { let child: &mut I64 = &mut *r.value; }",
    )
    .expect_err("projected child may not strengthen a Shared parent to replacement permission");
    assert!(has_diagnostic(&strengthened, |kind| kind
        == DiagnosticKind::ReferencePermissionUnavailable));
}

#[test]
fn projected_replacement_forms_reuse_exact_field_accessibility() {
    compile(
        "record Local { hidden: I64 }\
         fn root(seed: Local) { let mut value: Local = seed; let r: &mut I64 = &mut value.hidden; }\
         fn child(r: &mut Local) { let selected: &mut I64 = &mut *r.hidden; }",
    )
    .expect("same-module private fields remain accessible to projected replacement forms");

    let dep = parse("export record Public { export shown: I64, hidden: I64 }");
    let dep_target = ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias");
    let app_ok = parse(
        "import dep;\
         fn root(seed: dep::Public) { let mut value: dep::Public = seed; let r: &mut I64 = &mut value.shown; }\
         fn child(r: &mut dep::Public) { let selected: &mut I64 = &mut *r.shown; }",
    );
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &dep, &[]),
        SourceUnit::new(ModuleId::new(1), &app_ok, std::slice::from_ref(&dep_target)),
    ])
    .expect("foreign exported fields remain accessible to projected replacement forms");

    for app_hidden in [
        parse(
            "import dep; fn root(seed: dep::Public) { let mut value: dep::Public = seed; let r: &mut I64 = &mut value.hidden; }",
        ),
        parse("import dep; fn child(r: &mut dep::Public) { let selected: &mut I64 = &mut *r.hidden; }"),
    ] {
        let errors = build_typed_hir(&[
            SourceUnit::new(ModuleId::new(2), &dep, &[]),
            SourceUnit::new(
                ModuleId::new(1),
                &app_hidden,
                std::slice::from_ref(&dep_target),
            ),
        ])
        .expect_err("foreign private projected replacement field remains inaccessible");
        assert!(has_diagnostic(&errors, |kind| kind
            == DiagnosticKind::InaccessibleRecordField));
    }
}

#[test]
fn projected_descendant_does_not_create_a_shared_direct_child_result_contract() {
    let errors = compile(
        "record copy Pair { left: I64, right: I64 }\
         fn f(parent: &mut Pair) -> &I64 { return &*parent.left; }",
    )
    .expect_err("projected descendant result must not widen the complete-target SharedDirectChild contract");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::MissingSharedReferenceResultOrigin));
}

#[test]
fn accepts_intrinsic_and_selected_record_referents_but_rejects_nonduplicable_records() {
    let hir = compile(
        "record copy Point { x: I64 }\
         fn f(value: I64, point: Point) {\
             let scalar_ref: &I64 = &value;\
             let point_ref: &Point = &point;\
             let scalar: I64 = *scalar_ref;\
             let copied: Point = *point_ref;\
         }",
    )
    .expect("intrinsic and selected-record Shared referents must validate");
    let point = hir.records[0].id;
    assert!(hir.type_is_duplicable(shared_reference(ReferenceReferent::Record(point))));

    let errors = compile("record Ticket { value: I64 } fn f(r: &Ticket) {}")
        .expect_err("non-duplicable nominal referent must be rejected");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::InvalidSafeReferenceReferent {
            permission: ReferencePermission::Shared,
            ..
        }
    )));
}

#[test]
fn shared_reference_result_requires_deterministic_exact_parameter_origin() {
    let missing = compile("fn f(x: I64) -> &I64 { return &x; }").expect_err(
        "Shared-reference result without an exact reference candidate must be rejected",
    );
    assert!(has_diagnostic(&missing, |kind| kind
        == DiagnosticKind::MissingSharedReferenceResultOrigin));

    let ambiguous_shared = compile("fn f(a: &I64, b: &I64) -> &I64 { return a; }")
        .expect_err("multiple exact Shared parameters leave the elided result origin ambiguous");
    assert!(has_diagnostic(&ambiguous_shared, |kind| kind
        == DiagnosticKind::AmbiguousSharedReferenceResultOrigin));

    let ambiguous_replacement = compile("fn f(a: &mut I64, b: &mut I64) -> &I64 { return &*a; }")
        .expect_err("multiple exact replacement parameters leave the derived origin ambiguous");
    assert!(has_diagnostic(&ambiguous_replacement, |kind| kind
        == DiagnosticKind::AmbiguousSharedReferenceResultOrigin));

    let field = compile("record Holder { value: &I64 }")
        .expect_err("Shared-reference record field remains outside the source slice");
    assert!(has_diagnostic(&field, |kind| kind == DiagnosticKind::SafeReferenceField));

    let mutable = compile("fn f(x: I64) { let mut r: &I64 = &x; }")
        .expect_err("Shared-reference ordinary local must remain immutable");
    assert!(has_diagnostic(&mutable, |kind| kind
        == DiagnosticKind::MutableSafeReferenceLocal));
}

#[test]
fn shared_reference_result_retains_identity_contract_and_shared_precedence() {
    let hir = compile(
        "fn direct(r: &I64) -> &I64 { return r; }\
         fn nonzero(x: I64, r: &I64) -> &I64 { return r; }\
         fn local(r: &I64) -> &I64 { let s: &I64 = r; return s; }\
         fn mixed(shared: &I64, replacement: &mut I64) -> &I64 { return shared; }",
    )
    .expect("identity contracts and Shared precedence must validate");

    assert_eq!(
        function(&hir, "direct").safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    assert_eq!(
        function(&hir, "nonzero").safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 1 }
    );
    assert_eq!(
        function(&hir, "mixed").safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    let local = function(&hir, "local");
    assert_eq!(
        local.safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    let Statement::Local { initializer, .. } = &local.body.statements[0] else {
        panic!("expected copied Shared-reference local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::BindingUse {
            binding,
            ownership: OwnedUse::Duplicate,
        } if binding == local.parameters[0].binding
    ));
}

#[test]
fn projected_shared_authority_survives_transport_call_identity_and_reborrow() {
    compile(
        "record copy Pair { left: I64, right: I64 }\
         fn id(r: &I64) -> &I64 { return r; }\
         fn f(root: Pair) -> I64 {\
             let projected: &I64 = &root.left;\
             let duplicate: &I64 = projected;\
             let child: &I64 = &*duplicate;\
             let returned: &I64 = id(child);\
             return *returned;\
         }",
    )
    .expect("projected Shared target must survive carrier transport, reborrow, and identity calls");
}

#[test]
fn field_relative_child_survives_transport_and_identity_call_without_becoming_identity_origin() {
    let hir = compile(
        "record copy Pair { left: I64, right: I64 }\
         fn id(r: &I64) -> &I64 { return r; }\
         fn f(parent: &Pair) -> I64 {\
             let child: &I64 = &*parent.left;\
             let duplicate: &I64 = child;\
             let returned: &I64 = id(duplicate);\
             return *returned;\
         }",
    )
    .expect("field-relative child authority must survive local and call transport");
    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected projected child local");
    };
    assert!(matches!(
        &initializer.kind,
        ValueKind::ReferenceReborrow {
            reference,
            fields,
            permission: ReferencePermission::Shared,
        } if *reference == f.parameters[0].binding && fields.as_slice() == [0]
    ));
}

#[test]
fn field_relative_child_cannot_satisfy_unrelated_identity_or_direct_child_result_contract() {
    for source in [
        "record copy Pair { left: I64 }\
         fn f(origin: &I64, parent: &Pair) -> &I64 { return &*parent.left; }",
        "record copy Pair { left: I64 }\
         fn f(origin: &mut I64, parent: &Pair) -> &I64 { return &*parent.left; }",
    ] {
        let errors = compile(source)
            .expect_err("projected child must not widen identity or direct-child result contracts");
        assert!(has_diagnostic(&errors, |kind| kind
            == DiagnosticKind::SharedReferenceResultOriginMismatch));
    }
}

#[test]
fn callee_local_shared_field_root_cannot_satisfy_external_identity_result_contract() {
    let errors = compile(
        "record copy Pair { left: I64 }\
         fn f(origin: &I64, root: Pair) -> &I64 { return &root.left; }",
    )
    .expect_err("callee-local field-root authority is not the advertised external identity origin");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::SharedReferenceResultOriginMismatch));
}

#[test]
fn shared_direct_child_contract_accepts_exact_reborrow_moved_parent_and_child_transport() {
    let hir = compile(
        "fn direct(r: &mut I64) -> &I64 { return &*r; }\
         fn moved(r: &mut I64) -> &I64 { let parent: &mut I64 = r; return &*parent; }\
         fn transported(r: &mut I64) -> &I64 {\
             let child: &I64 = &*r;\
             let duplicate: &I64 = child;\
             return duplicate;\
         }",
    )
    .expect(
        "exact direct children remain valid across parent movement and Shared carrier transport",
    );

    for name in ["direct", "moved", "transported"] {
        assert_eq!(
            function(&hir, name).safe_reference_result_contract,
            SafeReferenceResultContract::SharedDirectChild { origin: 0 }
        );
    }
}

#[test]
fn shared_reference_identity_composes_through_nested_recursive_and_mutual_calls() {
    let hir = compile(
        "fn inner(r: &I64) -> &I64 { return r; }\
         fn outer(r: &I64) -> &I64 { return inner(r); }\
         fn recursive(r: &I64) -> &I64 { return recursive(r); }\
         fn left(r: &I64) -> &I64 { return right(r); }\
         fn right(r: &I64) -> &I64 { return left(r); }",
    )
    .expect("call summaries must compose Shared identity provenance without body expansion");

    for name in ["inner", "outer", "recursive", "left", "right"] {
        assert_eq!(
            function(&hir, name).safe_reference_result_contract,
            SafeReferenceResultContract::SharedIdentity { origin: 0 }
        );
    }

    let outer = function(&hir, "outer");
    let returned = outer
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("outer returns the nested call result");
    assert!(matches!(returned.kind, ValueKind::DirectCall { .. }));

    let recursive = function(&hir, "recursive");
    let returned = recursive
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("recursive function returns its recursive call result");
    assert!(matches!(
        returned.kind,
        ValueKind::DirectCall { function, .. } if function == recursive.id
    ));
}

#[test]
fn shared_direct_child_composes_through_identity_nested_and_recursive_calls() {
    let hir = compile(
        "fn id(r: &I64) -> &I64 { return r; }\
         fn inner(r: &mut I64) -> &I64 { return &*r; }\
         fn through_identity(r: &mut I64) -> &I64 { return id(&*r); }\
         fn nested(r: &mut I64) -> &I64 { return inner(r); }\
         fn recursive(r: &mut I64) -> &I64 { return recursive(r); }\
         fn left(r: &mut I64) -> &I64 { return right(r); }\
         fn right(r: &mut I64) -> &I64 { return left(r); }",
    )
    .expect("direct-child summaries must compose from callable contracts without body expansion");

    assert_eq!(
        function(&hir, "id").safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    for name in [
        "inner",
        "through_identity",
        "nested",
        "recursive",
        "left",
        "right",
    ] {
        assert_eq!(
            function(&hir, name).safe_reference_result_contract,
            SafeReferenceResultContract::SharedDirectChild { origin: 0 }
        );
    }
}

#[test]
fn shared_reference_identity_rejects_fresh_and_wrong_composed_origins() {
    for source in [
        "fn f(r: &I64, x: I64) -> &I64 { return &x; }",
        "fn f(r: &I64, x: I64) -> &I64 { let s: &I64 = &x; return s; }",
        "fn id(r: &I64) -> &I64 { return r; }\
         fn f(r: &I64, x: I64) -> &I64 { return id(&x); }",
    ] {
        let errors = compile(source)
            .expect_err("Shared result identity must match the advertised parameter origin");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::SharedReferenceResultOriginMismatch),
            "missing Shared result origin mismatch: {errors:?}"
        );
    }
}

#[test]
fn shared_direct_child_rejects_fresh_wrong_parent_and_grandchild_results() {
    for source in [
        "fn f(r: &mut I64, x: I64) -> &I64 { return &x; }",
        "fn f(r: &mut I64) -> &I64 {\
             let mut x: I64 = 1;\
             let other: &mut I64 = &mut x;\
             return &*other;\
         }",
        "fn f(r: &mut I64) -> &I64 {\
             let replacement_child: &mut I64 = &mut *r;\
             return &*replacement_child;\
         }",
        "fn f(r: &mut I64) -> &I64 {\
             let shared_child: &I64 = &*r;\
             return &*shared_child;\
         }",
    ] {
        let errors = compile(source).expect_err(
            "direct-child result must have the exact activation origin as direct parent",
        );
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::SharedReferenceResultOriginMismatch),
            "missing direct-child result origin mismatch: {errors:?}"
        );
    }
}

#[test]
fn returned_call_result_local_preserves_caller_target_protection() {
    let errors = compile(
        "fn id(r: &I64) -> &I64 { return r; }\
         fn f(seed: I64, replacement: I64) {\
             let mut x: I64 = seed;\
             let returned: &I64 = id(&x);\
             x = replacement;\
         }",
    )
    .expect_err("stored returned Shared carrier must keep protecting its caller-local target");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::BorrowedAssignmentTarget));
}

#[test]
fn exported_shared_reference_results_preserve_nominal_accessibility() {
    let hir = compile(
        "export record copy Public { value: I64 }\
         export fn id(r: &Public) -> &Public { return r; }",
    )
    .expect("exported Shared result may expose an exported duplicable nominal referent");
    assert_eq!(
        function(&hir, "id").safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );

    let errors = compile(
        "record copy Hidden { value: I64 }\
         export fn id(r: &Hidden) -> &Hidden { return r; }",
    )
    .expect_err("private nominal referent must not leak through parameter or result signature");
    let accessibility_errors = errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::PrivateTypeInExportedSignature)
        .count();
    assert_eq!(
        accessibility_errors, 2,
        "both exported parameter and exported Shared result accessibility must be checked"
    );
}

#[test]
fn exported_reference_parameter_preserves_private_nominal_referent_accessibility() {
    let errors = compile("record copy Hidden { value: I64 } export fn f(r: &Hidden) {}")
        .expect_err("private nominal referent must not leak through an exported signature");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::PrivateTypeInExportedSignature));
}

#[test]
fn borrow_and_dereference_require_exact_lookup_and_types() {
    let unresolved = compile("fn f() { let r: &I64 = &missing; }")
        .expect_err("borrow target must resolve through function-local lookup");
    assert!(has_diagnostic(&unresolved, |kind| kind == DiagnosticKind::UnresolvedName));

    let mismatch = compile("fn f(x: I64) { let r: &I32 = &x; }")
        .expect_err("root borrow must match the exact referent type");
    assert!(has_diagnostic(&mismatch, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let non_reference = compile("fn f(x: I64) { let y: I64 = *x; }")
        .expect_err("dereference requires a safe-reference binding");
    assert!(has_diagnostic(&non_reference, |kind| kind
        == DiagnosticKind::ExpectedSafeReference));

    let dereference_mismatch = compile("fn f(r: &I64) { let y: I32 = *r; }")
        .expect_err("dereference producer has the exact referent type");
    assert!(has_diagnostic(&dereference_mismatch, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));
}

#[test]
fn stored_local_origin_rejects_target_replacement_after_rhs_validation() {
    for source in [
        "fn f(seed: I64, replacement: I64) {\
             let mut x: I64 = seed;\
             let r: &I64 = &x;\
             x = replacement;\
         }",
        "fn id(v: I64) -> I64 { return v; }\
         fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &I64 = &x;\
             x = id(*r);\
         }",
    ] {
        let errors = compile(source)
            .expect_err("stored Shared carrier must protect its target at replacement time");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::BorrowedAssignmentTarget),
            "missing borrowed-target diagnostic: {errors:?}"
        );
    }
}

#[test]
fn normally_completed_call_temporary_borrow_does_not_persist_into_assignment() {
    compile(
        "fn read_ref(r: &I64) -> I64 { return *r; }\
         fn f(seed: I64) {\
             let mut x: I64 = seed;\
             x = read_ref(&x);\
             x = 17;\
         }",
    )
    .expect("temporary call borrow must end before assignment replacement");
}

#[test]
fn external_reference_parameters_duplicate_and_dereference_without_local_target_identity() {
    let hir = compile("fn f(r: &I64) -> I64 { let s: &I64 = r; return *s; }")
        .expect("external Shared-reference parameter may be copied and dereferenced");
    let f = function(&hir, "f");

    let Statement::Local {
        binding: local_reference,
        initializer,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected copied reference local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::BindingUse {
            binding,
            ownership: OwnedUse::Duplicate,
        } if binding == f.parameters[0].binding
    ));

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("result-bearing function returns dereferenced value");
    assert!(matches!(
        returned.kind,
        ValueKind::ReferenceDereference {
            reference,
            ownership: OwnedUse::Duplicate,
        } if reference == *local_reference
    ));
    assert!(!matches!(initializer.kind, ValueKind::ReferenceRoot { .. }));
}

#[test]
fn lexical_reference_cleanup_precedes_outer_assignment_and_control_flow_join() {
    compile(
        "fn f(seed: I64, flag: Bool) {\
             let mut x: I64 = seed;\
             { let r: &I64 = &x; let y: I64 = *r; }\
             if flag { let a: &I64 = &x; let av: I64 = *a; }\
             else { let b: &I64 = &x; let bv: I64 = *b; }\
             x = 23;\
         }",
    )
    .expect("branch-local carriers must be cleaned before the enclosing join and assignment");
}

#[test]
fn field_root_cleanup_precedes_control_join_and_whole_root_assignment() {
    compile(
        "record copy Pair { left: I64, right: I64 }\
         fn f(seed: Pair, flag: Bool) {\
             let mut root: Pair = seed;\
             if flag { let left: &I64 = &root.left; }\
             else { let right: &I64 = &root.right; }\
             root = seed;\
         }",
    )
    .expect(
        "branch-local projected carriers must be cleaned before exact-state join and assignment",
    );
}
