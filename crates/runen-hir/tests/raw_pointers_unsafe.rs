use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, RawPointerPointee, SourceUnit, Statement,
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
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn retains_raw_pointer_types_formation_copy_and_valid_retargeting() {
    let hir = compile(
        "record copy Point { x: I64 }\
         fn f(seed: I64, other: I64, point: Point) {\
             let scalar: raw I64 = raw &seed;\
             let copied: raw I64 = scalar;\
             let mut retarget: raw I64 = raw &seed;\
             retarget = raw &other;\
             let record_ptr: raw Point = raw &point;\
         }",
    )
    .expect("bounded raw-pointer locals and valid ordinary retargeting must validate");
    let f = function(&hir, "f");
    let scalar_ty = Type::RawPointer(RawPointerPointee::Intrinsic(IntrinsicType::I64));
    let point_ty = Type::RawPointer(RawPointerPointee::Record(hir.records[0].id));

    let Statement::Local {
        binding: scalar,
        ty,
        initializer,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected scalar raw-pointer local");
    };
    assert_eq!(*ty, scalar_ty);
    assert!(matches!(
        initializer.kind,
        ValueKind::RawAddressRoot { target } if target == f.parameters[0].binding
    ));

    let Statement::Local {
        ty,
        initializer,
        ..
    } = &f.body.statements[1]
    else {
        panic!("expected copied raw-pointer local");
    };
    assert_eq!(*ty, scalar_ty);
    assert!(matches!(
        initializer.kind,
        ValueKind::BindingUse {
            binding,
            ownership: OwnedUse::Duplicate,
        } if binding == *scalar
    ));

    let Statement::Assignment { value, .. } = &f.body.statements[3] else {
        panic!("expected ordinary pointer retarget assignment");
    };
    assert!(matches!(
        value.kind,
        ValueKind::RawAddressRoot { target } if target == f.parameters[1].binding
    ));

    let Statement::Local { ty, .. } = &f.body.statements[4] else {
        panic!("expected nominal raw-pointer local");
    };
    assert_eq!(*ty, point_ty);
}

#[test]
fn raw_address_formation_accepts_live_partial_unavailable_and_shared_targets() {
    compile(
        "record Ticket { value: I64 }\
         record Pair { left: Ticket, right: Ticket }\
         fn take_ticket(value: Ticket) {}\
         fn take_pair(value: Pair) {}\
         fn live(x: I64) { let p: raw I64 = raw &x; }\
         fn partial(pair: Pair) {\
             take_ticket(pair.left);\
             let p: raw Pair = raw &pair;\
         }\
         fn unavailable(pair: Pair) {\
             take_pair(pair);\
             let p: raw Pair = raw &pair;\
         }\
         fn shared(x: I64) {\
             let r: &I64 = &x;\
             let p: raw I64 = raw &x;\
             let observed: I64 = *r;\
         }",
    )
    .expect("raw address formation depends on active extent, not ownership availability or Shared authority");
}

#[test]
fn raw_move_requires_unsafe_full_target_availability_and_no_shared_authority() {
    compile(
        "record Ticket { value: I64 }\
         fn take(value: Ticket) {}\
         fn ok(ticket: Ticket) {\
             let p: raw Ticket = raw &ticket;\
             unsafe { let moved: Ticket = raw move p; take(moved); }\
         }",
    )
    .expect("unsafe RawMove from a fully available target must validate");

    let outside = compile(
        "fn f(x: I64) { let p: raw I64 = raw &x; let moved: I64 = raw move p; }",
    )
    .expect_err("RawMove outside unsafe must be rejected");
    assert!(has_diagnostic(
        &outside,
        DiagnosticKind::UnsafeOperationOutsideUnsafeBlock
    ));

    let unavailable = compile(
        "record Ticket { value: I64 }\
         fn take(value: Ticket) {}\
         fn f(ticket: Ticket) {\
             let p: raw Ticket = raw &ticket;\
             take(ticket);\
             unsafe { let moved: Ticket = raw move p; }\
         }",
    )
    .expect_err("RawMove from an unavailable target must be rejected");
    assert!(has_diagnostic(
        &unavailable,
        DiagnosticKind::RawMoveTargetUnavailable
    ));

    let stored_shared = compile(
        "fn f(x: I64) {\
             let p: raw I64 = raw &x;\
             let r: &I64 = &x;\
             unsafe { let moved: I64 = raw move p; }\
         }",
    )
    .expect_err("stored Shared authority must block RawMove");
    assert!(has_diagnostic(
        &stored_shared,
        DiagnosticKind::RawTargetSharedAuthorityConflict
    ));
}

#[test]
fn raw_move_sees_earlier_held_shared_call_argument_transients() {
    for source in [
        "fn sink(r: &I64, value: I64) {}\
         fn f(x: I64) {\
             let p: raw I64 = raw &x;\
             unsafe { sink(&x, raw move p); }\
         }",
        "fn id(r: &I64) -> &I64 { return r; }\
         fn sink(r: &I64, value: I64) {}\
         fn f(x: I64) {\
             let p: raw I64 = raw &x;\
             unsafe { sink(id(&x), raw move p); }\
         }",
    ] {
        let errors = compile(source)
            .expect_err("earlier held Shared call argument must block later RawMove to its target");
        assert!(
            has_diagnostic(&errors, DiagnosticKind::RawTargetSharedAuthorityConflict),
            "missing held-transient Shared conflict: {errors:?}"
        );
    }
}

#[test]
fn raw_assign_is_source_first_restores_target_and_allows_temporary_rhs_borrow_cleanup() {
    compile(
        "record Ticket { value: I64 }\
         fn take(value: Ticket) {}\
         fn restore_dead(ticket: Ticket) {\
             let p: raw Ticket = raw &ticket;\
             take(ticket);\
             unsafe { raw assign p = Ticket { value: 7 }; }\
             take(ticket);\
         }\
         fn round_trip(ticket: Ticket) {\
             let p: raw Ticket = raw &ticket;\
             unsafe { raw assign p = raw move p; }\
             take(ticket);\
         }\
         fn read(r: &I64) -> I64 { return *r; }\
         fn temporary_borrow(x: I64) {\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = read(&x); }\
         }",
    )
    .expect("RawAssign must be source-first, restore ownership, and check Shared authority after temporary RHS carriers clean up");
}

#[test]
fn raw_assign_replaces_partial_target_and_rejects_live_stored_shared_authority() {
    compile(
        "record Ticket { value: I64 }\
         record Pair { left: Ticket, right: Ticket }\
         fn take(value: Ticket) {}\
         fn take_pair(value: Pair) {}\
         fn f(pair: Pair) {\
             let p: raw Pair = raw &pair;\
             take(pair.left);\
             unsafe {\
                 raw assign p = Pair {\
                     left: Ticket { value: 1 },\
                     right: Ticket { value: 2 }\
                 };\
             }\
             take_pair(pair);\
         }",
    )
    .expect("RawAssign must replace a partially available target and restore complete ownership");

    let errors = compile(
        "fn f(x: I64) {\
             let p: raw I64 = raw &x;\
             let r: &I64 = &x;\
             unsafe { raw assign p = 9; }\
         }",
    )
    .expect_err("stored Shared carrier remaining after RHS must block RawAssign");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::RawTargetSharedAuthorityConflict
    ));
}

#[test]
fn raw_assign_requires_unsafe_and_raw_move_requires_raw_pointer_operand() {
    let outside = compile("fn f(x: I64) { let p: raw I64 = raw &x; raw assign p = 1; }")
        .expect_err("RawAssign outside unsafe must be rejected");
    assert!(has_diagnostic(
        &outside,
        DiagnosticKind::UnsafeOperationOutsideUnsafeBlock
    ));

    let wrong_operand = compile("fn f(x: I64) { unsafe { let moved: I64 = raw move x; } }")
        .expect_err("RawMove operand must be a raw-pointer binding");
    assert!(has_diagnostic(
        &wrong_operand,
        DiagnosticKind::ExpectedRawPointer
    ));
}

#[test]
fn lexical_target_domain_rejects_descendant_and_loop_body_retargets() {
    let descendant = compile(
        "fn f(x: I64) {\
             let mut p: raw I64 = raw &x;\
             { let y: I64 = 2; p = raw &y; }\
         }",
    )
    .expect_err("longer-lived pointer must not retarget to a descendant local");
    assert!(has_diagnostic(
        &descendant,
        DiagnosticKind::RawPointerTargetExtentMismatch
    ));

    let loop_body = compile(
        "fn f(flag: Bool, x: I64) {\
             let mut p: raw I64 = raw &x;\
             while flag { let y: I64 = 2; p = raw &y; break; }\
         }",
    )
    .expect_err("enclosing pointer must not acquire a loop-body-local origin");
    assert!(has_diagnostic(
        &loop_body,
        DiagnosticKind::RawPointerTargetExtentMismatch
    ));
}

#[test]
fn exact_pointer_origin_is_enforced_at_conditional_loop_break_and_continue_boundaries() {
    compile(
        "fn f(flag: Bool, a: I64, b: I64) {\
             let mut p: raw I64 = raw &a;\
             if flag { p = raw &b; p = raw &a; } else {}\
             if flag { p = raw &b; } else { fault; }\
         }",
    )
    .expect("restored two-normal origin and one-normal carried origin must validate");

    let conditional = compile(
        "fn f(flag: Bool, a: I64, b: I64) {\
             let mut p: raw I64 = raw &a;\
             if flag { p = raw &b; } else {}\
         }",
    )
    .expect_err("two normal conditional outcomes require exact equal pointer origins");
    assert!(has_diagnostic(
        &conditional,
        DiagnosticKind::ConditionalPointerOriginMismatch
    ));

    let backedge = compile(
        "fn f(flag: Bool, a: I64, b: I64) {\
             let mut p: raw I64 = raw &a;\
             while flag { p = raw &b; }\
         }",
    )
    .expect_err("normal loop backedge must restore exact pointer origin");
    assert!(has_diagnostic(
        &backedge,
        DiagnosticKind::LoopPointerOriginMismatch
    ));

    let break_errors = compile(
        "fn f(flag: Bool, a: I64, b: I64) {\
             let mut p: raw I64 = raw &a;\
             while flag { p = raw &b; break; }\
         }",
    )
    .expect_err("break must match exact loop continuation origin");
    assert!(has_diagnostic(
        &break_errors,
        DiagnosticKind::BreakPointerOriginMismatch
    ));

    let continue_errors = compile(
        "fn f(flag: Bool, a: I64, b: I64) {\
             let mut p: raw I64 = raw &a;\
             while flag { p = raw &b; continue; }\
         }",
    )
    .expect_err("continue must match exact loop-head origin");
    assert!(has_diagnostic(
        &continue_errors,
        DiagnosticKind::ContinuePointerOriginMismatch
    ));
}

#[test]
fn nested_unsafe_is_idempotent_and_raw_operations_remain_lexically_bounded() {
    compile(
        "fn f(x: I64) {\
             let p: raw I64 = raw &x;\
             unsafe { unsafe { raw assign p = 3; } }\
         }",
    )
    .expect("nested unsafe admission must be idempotent");

    let errors = compile(
        "fn f(x: I64) {\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = 3; }\
             raw assign p = 4;\
         }",
    )
    .expect_err("unsafe admission must end at the lexical wrapper boundary");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnsafeOperationOutsideUnsafeBlock
    ));
}

#[test]
fn rejects_raw_pointer_interfaces_recursive_pointees_and_raw_address_of_indirection() {
    let field = compile("record Holder { p: raw I64 }")
        .expect_err("raw-pointer record fields remain outside the source slice");
    assert!(has_diagnostic(&field, DiagnosticKind::RawPointerField));

    let parameter = compile("fn f(p: raw I64) {}")
        .expect_err("raw-pointer parameters remain outside the source slice");
    assert!(has_diagnostic(&parameter, DiagnosticKind::RawPointerParameter));

    let result = compile("fn f(x: I64) -> raw I64 { return raw &x; }")
        .expect_err("raw-pointer results remain outside the source slice");
    assert!(has_diagnostic(&result, DiagnosticKind::RawPointerResult));

    for source in [
        "fn f(x: I64) { let r: &I64 = &x; let p: raw I64 = raw &r; }",
        "fn f(x: I64) { let p: raw I64 = raw &x; let q: raw I64 = raw &p; }",
    ] {
        let errors = compile(source)
            .expect_err("raw address target must have a non-indirection pointee type");
        assert!(errors.iter().any(|error| matches!(
            error.kind,
            DiagnosticKind::InvalidRawPointerPointee { .. }
        )));
    }
}

#[test]
fn ordinary_assignment_preserves_existing_rhs_first_shared_diagnostic_order() {
    let errors = compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &I64 = &x;\
             x = missing;\
         }",
    )
    .expect_err("invalid RHS must be diagnosed before replacement admission");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::BorrowedAssignmentTarget),
        "replacement admission must not run when RHS production fails: {errors:?}"
    );
}
