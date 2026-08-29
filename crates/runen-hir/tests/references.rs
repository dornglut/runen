use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, ReferenceReferent, SourceUnit, Statement,
    Type, TypedCompilation, ValueKind, build_typed_hir,
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

#[test]
fn retains_exact_shared_reference_identity_and_duplicates_reference_bindings() {
    let hir = compile("fn f(x: I64, r: &I64) { let s: &I64 = r; }")
        .expect("Shared-reference parameter and immutable local must validate");
    let f = function(&hir, "f");
    let reference_ty = Type::SharedReference(ReferenceReferent::Intrinsic(IntrinsicType::I64));

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
        ValueKind::SharedBorrowRoot { target } if target == f.parameters[0].binding
    ));
    assert_eq!(
        borrow.ty,
        Type::SharedReference(ReferenceReferent::Intrinsic(IntrinsicType::I64))
    );

    let Statement::Local { initializer, .. } = &f.body.statements[1] else {
        panic!("expected dereference result local");
    };
    assert_eq!(initializer.ty, Type::Intrinsic(IntrinsicType::I64));
    assert!(matches!(
        initializer.kind,
        ValueKind::SharedDereferenceCopy { reference } if reference == *reference_binding
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
    assert!(hir.type_is_duplicable(Type::SharedReference(ReferenceReferent::Record(point))));

    let errors = compile("record Ticket { value: I64 } fn f(r: &Ticket) {}")
        .expect_err("non-duplicable nominal referent must be rejected");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::InvalidSharedReferenceReferent { .. }
    )));
}

#[test]
fn restricts_shared_reference_types_to_parameters_and_immutable_locals() {
    let result = compile("fn f(x: I64) -> &I64 { return &x; }")
        .expect_err("Shared-reference result type is outside the first source slice");
    assert!(has_diagnostic(&result, |kind| kind
        == DiagnosticKind::SharedReferenceResult));

    let field = compile("record Holder { value: &I64 }")
        .expect_err("Shared-reference record field is outside the first source slice");
    assert!(has_diagnostic(&field, |kind| kind == DiagnosticKind::SharedReferenceField));

    let mutable = compile("fn f(x: I64) { let mut r: &I64 = &x; }")
        .expect_err("Shared-reference ordinary local must be immutable");
    assert!(has_diagnostic(&mutable, |kind| kind
        == DiagnosticKind::MutableSharedReferenceLocal));
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
        .expect_err("dereference requires a Shared-reference binding");
    assert!(has_diagnostic(&non_reference, |kind| kind
        == DiagnosticKind::ExpectedSharedReference));

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
        "fn copy(r: &I64) -> I64 { return *r; }\
         fn f(seed: I64) {\
             let mut x: I64 = seed;\
             x = copy(&x);\
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
        ValueKind::SharedDereferenceCopy { reference } if reference == *local_reference
    ));
    assert!(!matches!(
        initializer.kind,
        ValueKind::SharedBorrowRoot { .. }
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
