use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, ReferencePermission, ReferenceReferent,
    SourceUnit, Statement, Type, TypedCompilation, ValueKind, build_typed_hir,
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
        borrow.kind,
        ValueKind::ReferenceRoot {
            target,
            permission: ReferencePermission::Shared,
        } if target == f.parameters[0].binding
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
fn shared_reference_result_requires_unique_exact_parameter_origin() {
    let missing = compile("fn f(x: I64) -> &I64 { return &x; }")
        .expect_err("Shared-reference result without an exact Shared parameter must be rejected");
    assert!(has_diagnostic(&missing, |kind| kind
        == DiagnosticKind::MissingSharedReferenceResultOrigin));

    let ambiguous = compile("fn f(a: &I64, b: &I64) -> &I64 { return a; }")
        .expect_err("multiple exact Shared parameters leave the elided result origin ambiguous");
    assert!(has_diagnostic(&ambiguous, |kind| kind
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
fn shared_reference_result_retains_direct_nonzero_and_local_duplicate_origins() {
    let hir = compile(
        "fn direct(r: &I64) -> &I64 { return r; }\
         fn nonzero(x: I64, r: &I64) -> &I64 { return r; }\
         fn local(r: &I64) -> &I64 { let s: &I64 = r; return s; }",
    )
    .expect("unique exact Shared parameter origins must validate");

    assert_eq!(
        function(&hir, "direct").shared_reference_result_origin,
        Some(0)
    );
    assert_eq!(
        function(&hir, "nonzero").shared_reference_result_origin,
        Some(1)
    );
    let local = function(&hir, "local");
    assert_eq!(local.shared_reference_result_origin, Some(0));
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
fn shared_reference_result_origin_composes_through_nested_recursive_and_mutual_calls() {
    let hir = compile(
        "fn inner(r: &I64) -> &I64 { return r; }\
         fn outer(r: &I64) -> &I64 { return inner(r); }\
         fn recursive(r: &I64) -> &I64 { return recursive(r); }\
         fn left(r: &I64) -> &I64 { return right(r); }\
         fn right(r: &I64) -> &I64 { return left(r); }",
    )
    .expect("call summaries must compose Shared result provenance without body expansion");

    for name in ["inner", "outer", "recursive", "left", "right"] {
        assert_eq!(function(&hir, name).shared_reference_result_origin, Some(0));
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
fn shared_reference_result_rejects_fresh_and_wrong_composed_origins() {
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
    assert_eq!(function(&hir, "id").shared_reference_result_origin, Some(0));

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
    assert!(!matches!(
        initializer.kind,
        ValueKind::ReferenceRoot { .. }
    ));
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
