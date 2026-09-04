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
        &initializer.kind,
        ValueKind::ReferenceRoot {
            target,
            fields,
            permission: ReferencePermission::ExclusiveReplace,
        } if *target == match &f.body.statements[0] {
            Statement::Local { binding, .. } => *binding,
            _ => panic!("expected mutable ordinary local"),
        } && fields.is_empty()
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
fn replacement_root_requires_mutable_ordinary_local_for_complete_or_projected_target() {
    for source in [
        "fn f(x: I64) { let r: &mut I64 = &mut x; }",
        "fn f(seed: I64) { let x: I64 = seed; let r: &mut I64 = &mut x; }",
        "record Box { value: I64 } fn f(x: Box) { let r: &mut I64 = &mut x.value; }",
        "record Box { value: I64 } fn f(seed: Box) { let x: Box = seed; let r: &mut I64 = &mut x.value; }",
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
fn projected_replacement_root_and_reborrow_retain_exact_paths_and_permission() {
    let hir = compile(
        "record Inner { value: I64 }\
         record Outer { inner: Inner, other: I64 }\
         fn root(seed: Outer) {\
             let mut value: Outer = seed;\
             let selected: &mut I64 = &mut value.inner.value;\
         }\
         fn child(parent: &mut Outer) {\
             let selected: &mut I64 = &mut *parent.inner.value;\
         }",
    )
    .expect("projected replacement root and child must retain exact structural paths");

    let root = function(&hir, "root");
    let Statement::Local {
        binding: root_binding, ..
    } = &root.body.statements[0]
    else {
        panic!("expected mutable root local");
    };
    let Statement::Local { initializer, .. } = &root.body.statements[1] else {
        panic!("expected projected replacement-root local");
    };
    assert!(matches!(
        &initializer.kind,
        ValueKind::ReferenceRoot {
            target,
            fields,
            permission: ReferencePermission::ExclusiveReplace,
        } if *target == *root_binding && fields.as_slice() == [0, 0]
    ));

    let child = function(&hir, "child");
    let Statement::Local { initializer, .. } = &child.body.statements[0] else {
        panic!("expected projected replacement-child local");
    };
    assert!(matches!(
        &initializer.kind,
        ValueKind::ReferenceReborrow {
            reference,
            fields,
            permission: ReferencePermission::ExclusiveReplace,
        } if *reference == child.parameters[0].binding && fields.as_slice() == [0, 0]
    ));
}

#[test]
fn projected_replacement_root_observes_exact_structural_availability() {
    for source in [
        "record Token {} record Inner { token: Token, count: I64 } record Outer { inner: Inner, other: Token }\
         fn f(seed: Outer) { let mut root: Outer = seed; let moved: Token = root.inner.token; let r: &mut Token = &mut root.inner.token; }",
        "record Token {} record Inner { token: Token, count: I64 } record Outer { inner: Inner, other: Token }\
         fn f(seed: Outer) { let mut root: Outer = seed; let moved: Token = root.inner.token; let r: &mut Inner = &mut root.inner; }",
        "record Token {} record Inner { token: Token, count: I64 } record Outer { inner: Inner, other: Token }\
         fn f(seed: Outer) { let mut root: Outer = seed; let moved: Inner = root.inner; let r: &mut Token = &mut root.inner.token; }",
    ] {
        let errors = compile(source)
            .expect_err("equal, partial, or strict-ancestor consumption must reject projected root formation");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::InvalidReplacementReferenceTarget),
            "missing projected replacement availability diagnostic: {errors:?}"
        );
    }

    compile(
        "record Token {} record Inner { token: Token, count: I64 } record Outer { inner: Inner, other: Token }\
         fn f(seed: Outer) {\
             let mut root: Outer = seed;\
             let moved: Token = root.other;\
             let r: &mut Token = &mut root.inner.token;\
         }",
    )
    .expect("a disjoint consumed sibling must not block projected replacement-root formation");
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
        &initializer.kind,
        ValueKind::ReferenceReborrow {
            reference,
            fields,
            permission: ReferencePermission::Shared,
        } if *reference == f.parameters[0].binding && fields.is_empty()
    ));

    let Statement::Block(exclusive_block) = &f.body.statements[1] else {
        panic!("expected replacement child block");
    };
    let Statement::Local { initializer, .. } = &exclusive_block.statements[0] else {
        panic!("expected replacement child local");
    };
    assert!(matches!(
        &initializer.kind,
        ValueKind::ReferenceReborrow {
            reference,
            fields,
            permission: ReferencePermission::ExclusiveReplace,
        } if *reference == f.parameters[0].binding && fields.is_empty()
    ));

    let errors = compile("fn f(r: &I64) { let child: &mut I64 = &mut *r; }")
        .expect_err("Shared authority cannot be strengthened by reborrow");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReferencePermissionUnavailable));
}

#[test]
fn projected_shared_child_from_replacement_parent_is_bounded_to_selected_field() {
    let hir = compile(
        "record Ticket { value: I64, other: I64 }\
         fn f(r: &mut Ticket) {\
             { let child: &I64 = &*r.value; let copied: I64 = *child; }\
             let moved: Ticket = *r;\
             *r = moved;\
         }",
    )
    .expect("Shared projected child may borrow an admissible field of a nonduplicable replacement parent");
    let f = function(&hir, "f");
    let Statement::Block(block) = &f.body.statements[0] else {
        panic!("expected projected child block");
    };
    let Statement::Local { initializer, .. } = &block.statements[0] else {
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

    let errors = compile(
        "record Ticket { value: I64, other: I64 }\
         fn sink(r: &mut Ticket) {}\
         fn blocked(r: &mut Ticket) {\
             let child: &I64 = &*r.value;\
             sink(r);\
         }",
    )
    .expect_err("any active projected child prevents complete replacement-capable call transfer");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ReferencePermissionUnavailable));
}

#[test]
fn projected_replacement_children_move_and_reinitialize_exact_local_and_external_targets() {
    compile(
        "record Token {} record Pair { left: Token, right: Token }\
         fn local(seed: Pair) {\
             let mut root: Pair = seed;\
             {\
                 let parent: &mut Pair = &mut root;\
                 { let child: &mut Token = &mut *parent.left; let moved: Token = *child; *child = moved; }\
             }\
             let sibling: Token = root.right;\
         }\
         fn external(parent: &mut Pair) {\
             { let child: &mut Token = &mut *parent.left; let moved: Token = *child; *child = moved; }\
             { let sibling: &mut Token = &mut *parent.right; }\
         }",
    )
    .expect("projected replacement Move and reinitialization must operate on exact local/external backing paths");
}

#[test]
fn projected_replacement_reconstructs_descendants_and_preserves_disjoint_consumption() {
    compile(
        "record Token {} record Inner { token: Token, count: I64 } record Outer { inner: Inner, other: I64 }\
         fn f(parent: &mut Outer) {\
             let child: &mut Inner = &mut *parent.inner;\
             { let leaf: &mut Token = &mut *child.token; let moved: Token = *leaf; }\
             *child = Inner { token: Token {}, count: 7 };\
         }",
    )
    .expect("replacement of a projected child may reconstruct a descendant-consumed partial target");

    let errors = compile(
        "record Token {} record Inner { token: Token, count: I64 } record Outer { inner: Inner, other: Token }\
         fn f(seed: Outer) {\
             let mut root: Outer = seed;\
             let moved_other: Token = root.other;\
             { let inner: &mut Inner = &mut root.inner; *inner = Inner { token: Token {}, count: 7 }; }\
             let moved_again: Token = root.other;\
         }",
    )
    .expect_err("projected installation must preserve a disjoint consumed sibling");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableFieldValue));
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
fn projected_child_call_transfer_preserves_exact_target_and_restoration() {
    compile(
        "record Token {} record Pair { left: Token, right: Token }\
         fn sink(r: &mut Token) { let moved: Token = *r; *r = moved; }\
         fn f(parent: &mut Pair) {\
             sink(&mut *parent.left);\
             { let right: &mut Token = &mut *parent.right; }\
         }",
    )
    .expect("projected replacement call transfer must preserve the exact selected child target and return it restored");
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
fn rejected_projected_replacement_rolls_back_speculative_rhs_reference_state() {
    let errors = compile(
        "record copy Pair { left: I64, right: I64 }\
         fn take_and_value(r: &mut I64, value: I64) -> I64 { return value; }\
         fn f(parent: &mut Pair) {\
             let child: &mut I64 = &mut *parent.left;\
             *child = take_and_value(child, 1);\
             *child = 2;\
         }",
    )
    .expect_err("a rejected projected replacement must roll back speculative RHS carrier effects");
    assert_eq!(
        errors
            .iter()
            .filter(|error| error.kind == DiagnosticKind::ReferenceReplacementUnavailable)
            .count(),
        1,
        "post-error use of the destination child should observe rolled-back reference state: {errors:?}"
    );
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
