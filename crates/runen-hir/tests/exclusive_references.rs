use runen_hir::{
    AssignmentMutability, DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, ReferencePermission,
    ReferenceReferent, SourceUnit, Statement, Type, TypedCompilation, ValueKind, build_typed_hir,
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

fn exclusive_i64() -> Type {
    Type::SafeReference {
        referent: ReferenceReferent::Intrinsic(IntrinsicType::I64),
        permission: ReferencePermission::ExclusiveReplace,
    }
}

#[test]
fn replacement_reference_type_is_nonduplicable_and_type_mut_is_not_binding_mutability() {
    let hir = compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &mut I64 = &mut x;\
             let value: I64 = *r;\
         }",
    )
    .expect("immutable replacement-capable local over a mutable ordinary local must validate");
    let f = function(&hir, "f");
    let ty = exclusive_i64();
    assert!(!hir.type_is_duplicable(ty));

    let Statement::Local {
        binding: reference_binding,
        ty: local_ty,
        mutability,
        initializer,
        ..
    } = &f.body.statements[1]
    else {
        panic!("expected replacement-capable reference local");
    };
    assert_eq!(*local_ty, ty);
    assert_eq!(*mutability, AssignmentMutability::Immutable);
    assert!(matches!(
        initializer.kind,
        ValueKind::ReferenceRoot {
            target,
            permission: ReferencePermission::ExclusiveReplace,
        } if target == match &f.body.statements[0] {
            Statement::Local { binding, .. } => *binding,
            _ => panic!("expected mutable ordinary local"),
        }
    ));

    let Statement::Local {
        initializer: dereference,
        ..
    } = &f.body.statements[2]
    else {
        panic!("expected dereference result local");
    };
    assert!(matches!(
        dereference.kind,
        ValueKind::ReferenceDereference {
            reference,
            ownership: OwnedUse::Duplicate,
        } if reference == *reference_binding
    ));
}

#[test]
fn all_safe_reference_locals_remain_immutable() {
    let errors = compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let mut r: &mut I64 = &mut x;\
         }",
    )
    .expect_err("binding-level mutability on a safe-reference local must be rejected");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::MutableSafeReferenceLocal));
}

#[test]
fn replacement_reference_results_are_source_invalid() {
    let errors = compile("fn f(r: &mut I64) -> &mut I64 { return r; }")
        .expect_err("replacement-capable results are outside the accepted source contract");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReplacementReferenceResult));
}

#[test]
fn replacement_root_requires_complete_mutable_ordinary_local_not_parameter_or_immutable_local() {
    for source in [
        "fn f(x: I64) { let r: &mut I64 = &mut x; }",
        "fn f(seed: I64) { let x: I64 = seed; let r: &mut I64 = &mut x; }",
    ] {
        let errors = compile(source).expect_err("invalid replacement root target must be rejected");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::InvalidReplacementReferenceTarget),
            "missing replacement-target diagnostic: {errors:?}"
        );
    }
}

#[test]
fn replacement_reference_binding_use_moves_the_carrier() {
    let hir = compile("fn f(r: &mut I64) { let moved: &mut I64 = r; }")
        .expect("replacement-capable reference carriers move between immutable reference locals");
    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected moved replacement-reference local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::BindingUse {
            binding,
            ownership: OwnedUse::Consume,
        } if binding == f.parameters[0].binding
    ));
}

#[test]
fn nonduplicable_external_referent_must_be_restored_after_move() {
    let hir = compile(
        "record Ticket { value: I64 }\
         fn f(r: &mut Ticket) {\
             let ticket: Ticket = *r;\
             *r = ticket;\
         }",
    )
    .expect("move followed by source replacement restores the incoming external referent");
    let f = function(&hir, "f");
    let Statement::Local {
        binding: moved_binding,
        initializer,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected moved referent local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::ReferenceDereference {
            reference,
            ownership: OwnedUse::Consume,
        } if reference == f.parameters[0].binding
    ));
    assert!(matches!(
        &f.body.statements[1],
        Statement::ReferenceAssign { reference, value, .. }
            if *reference == f.parameters[0].binding
                && matches!(
                    value.kind,
                    ValueKind::BindingUse {
                        binding,
                        ownership: OwnedUse::Consume,
                    } if binding == *moved_binding
                )
    ));

    let errors = compile(
        "record Ticket { value: I64 }\
         fn f(r: &mut Ticket) { let ticket: Ticket = *r; }",
    )
    .expect_err("normal completion may not leave an incoming replacement referent consumed");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReferenceRestorationRequired));
}

#[test]
fn complete_reborrow_preserves_permission_without_strengthening() {
    let hir = compile(
        "fn f(r: &mut I64) {\
             { let shared: &I64 = &*r; let copied: I64 = *shared; }\
             { let child: &mut I64 = &mut *r; *child = 7; }\
         }",
    )
    .expect("complete Shared and replacement-capable child reborrows must validate");
    let f = function(&hir, "f");
    let Statement::Block(shared_block) = &f.body.statements[0] else {
        panic!("expected Shared reborrow block");
    };
    let Statement::Local { initializer, .. } = &shared_block.statements[0] else {
        panic!("expected Shared child local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::ReferenceReborrow {
            reference,
            permission: ReferencePermission::Shared,
        } if reference == f.parameters[0].binding
    ));

    let Statement::Block(exclusive_block) = &f.body.statements[1] else {
        panic!("expected replacement child block");
    };
    let Statement::Local { initializer, .. } = &exclusive_block.statements[0] else {
        panic!("expected replacement child local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::ReferenceReborrow {
            reference,
            permission: ReferencePermission::ExclusiveReplace,
        } if reference == f.parameters[0].binding
    ));

    let errors = compile("fn f(r: &I64) { let child: &mut I64 = &mut *r; }")
        .expect_err("Shared authority cannot be strengthened by reborrow");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReferencePermissionUnavailable));
}

#[test]
fn direct_temporary_and_explicit_child_calls_end_their_child_authority_on_normal_return() {
    compile(
        "fn sink(r: &mut I64) { *r = 7; }\
         fn root(seed: I64) {\
             let mut x: I64 = seed;\
             sink(&mut x);\
             let observed: I64 = x;\
         }\
         fn child(r: &mut I64) {\
             sink(&mut *r);\
             *r = 9;\
         }",
    )
    .expect("temporary replacement roots and explicit replacement children end after normal calls");
}

#[test]
fn nested_callee_may_move_and_restore_a_nonduplicable_child_referent() {
    compile(
        "record Ticket { value: I64 }\
         fn sink(r: &mut Ticket) {\
             let ticket: Ticket = *r;\
             *r = ticket;\
         }\
         fn outer(r: &mut Ticket) { sink(&mut *r); }",
    )
    .expect("callee may Move and restore a replacement child before normal return");
}

#[test]
fn defined_fault_has_no_synthetic_external_referent_restoration_obligation() {
    compile(
        "record Ticket { value: I64 }\
         fn f(r: &mut Ticket) {\
             let ticket: Ticket = *r;\
             fault;\
         }",
    )
    .expect("defined fault may leave an incoming replacement referent consumed");
}

#[test]
fn live_root_replacement_authority_blocks_direct_target_replacement_even_with_shared_child() {
    let errors = compile(
        "fn f(seed: I64, replacement: I64) {\
             let mut x: I64 = seed;\
             let root: &mut I64 = &mut x;\
             let child: &I64 = &*root;\
             x = replacement;\
         }",
    )
    .expect_err("Shared child does not reopen unrelated direct access to the root target");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::BorrowedAssignmentTarget));
}

#[test]
fn active_child_prevents_full_replacement_call_entry() {
    let errors = compile(
        "fn sink(r: &mut I64) {}\
         fn f(r: &mut I64) {\
             let child: &I64 = &*r;\
             sink(r);\
         }",
    )
    .expect_err("parent with an active child cannot promise full replacement capability to a call");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReferencePermissionUnavailable));
}

#[test]
fn reference_replacement_rechecks_destination_after_rhs_effects() {
    let errors = compile(
        "fn take_and_value(r: &mut I64, value: I64) -> I64 { return value; }\
         fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &mut I64 = &mut x;\
             *r = take_and_value(r, 1);\
         }",
    )
    .expect_err("RHS may not consume the destination carrier and then commit to a stale target");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReferenceReplacementUnavailable));
}

#[test]
fn fresh_reborrow_cannot_substitute_for_the_exact_shared_result_origin() {
    let errors = compile(
        "fn f(origin: &I64, r: &mut I64) -> &I64 {\
             let child: &I64 = &*r;\
             return child;\
         }",
    )
    .expect_err(
        "fresh child authority is not the callable's exact advertised Shared result origin",
    );
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::SharedReferenceResultOriginMismatch));
}

#[test]
fn raw_move_and_replacement_conflict_with_live_replacement_authority() {
    for source in [
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let p: raw I64 = raw &x;\
             let r: &mut I64 = &mut x;\
             unsafe { let moved: I64 = raw move p; }\
         }",
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let p: raw I64 = raw &x;\
             let r: &mut I64 = &mut x;\
             unsafe { raw assign p = 3; }\
         }",
    ] {
        let errors = compile(source)
            .expect_err("raw ownership operations require no active safe authority on the target");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::RawTargetSafeAuthorityConflict),
            "missing raw/safe-authority conflict: {errors:?}"
        );
    }
}

#[test]
fn exact_control_flow_rejects_unequal_external_referent_state() {
    let conditional = compile(
        "record Ticket { value: I64 }\
         fn f(flag: Bool, r: &mut Ticket) {\
             if flag { let ticket: Ticket = *r; } else {}\
         }",
    )
    .expect_err("two normal conditional outcomes require equal external referent state");
    assert!(has_diagnostic(&conditional, |kind| kind
        == DiagnosticKind::ConditionalReferenceStateMismatch));

    let loop_backedge = compile(
        "record Ticket { value: I64 }\
         fn f(flag: Bool, r: &mut Ticket) {\
             while flag { let ticket: Ticket = *r; }\
         }",
    )
    .expect_err("normal loop backedge must restore exact external referent state");
    assert!(has_diagnostic(&loop_backedge, |kind| kind
        == DiagnosticKind::LoopReferenceStateMismatch));
}

#[test]
fn caller_created_shared_child_can_round_trip_through_exact_identity_result() {
    compile(
        "fn id(r: &I64) -> &I64 { return r; }\
         fn f(r: &mut I64) {\
             let child: &I64 = &*r;\
             let returned: &I64 = id(child);\
             let copied: I64 = *returned;\
         }",
    )
    .expect(
        "caller-created Shared child identity may round-trip through an exact identity function",
    );
}
