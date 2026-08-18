use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Fault, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable,
    Value, validate_body,
};
use runen_reference::{
    ExecutionReport, Machine, TerminalStatus, UndefinedBehavior, UndefinedBehaviorKind,
    VerificationEvent, VerificationWriteKind,
};

fn execute(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
    terminator: Terminator,
) -> Result<ExecutionReport, UndefinedBehavior> {
    let body = Body {
        types,
        locals,
        loans,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, terminator)],
    };
    Machine::new(validate_body(body).expect("language-valid RawMove fixture")).execute()
}

#[test]
fn raw_move_transfers_tracked_value_without_source_destruction() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("tracked_ptr", tracked_ty));
    let target = Place::local(LocalId(0));
    let destination = Place::local(LocalId(2));
    let report = execute(
        types,
        vec![
            LocalDecl::new("target", tracked_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", tracked_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: destination.clone(),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect("live unaliased target is a defined RawMove");

    let raw_move = report
        .verification_events
        .iter()
        .position(|event| matches!(event, VerificationEvent::RawMove { target: place, .. } if place == &target))
        .expect("raw ownership transfer is instrumented");
    let write = report
        .verification_events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEvent::Write {
                    place,
                    kind: VerificationWriteKind::Init,
                } if place == &destination
            )
        })
        .expect("enclosing Init writes the moved value");
    let drop = report
        .verification_events
        .iter()
        .position(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 1, .. }))
        .expect("moved value is eventually destroyed at its destination");

    assert!(raw_move < write && write < drop);
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 1, .. }))
            .count(),
        1,
        "RawMove does not destroy at the source and cleanup destroys the destination once"
    );
}

#[test]
fn raw_move_of_never_initialized_target_is_undefined_behavior_without_enclosing_write() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("tracked_ptr", tracked_ty));
    let destination = Place::local(LocalId(2));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", tracked_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", tracked_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: destination.clone(),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect_err("RawMove requires a fully Live target");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveTargetNotLive { .. }
    ));
    assert!(!error.verification_events.iter().any(|event| matches!(
        event,
        VerificationEvent::Write { place, .. } if place == &destination
    )));
    assert!(
        !error
            .verification_events
            .iter()
            .any(|event| matches!(event, VerificationEvent::DropTrackedFixture { .. }))
    );
}

#[test]
fn raw_move_of_dead_target_is_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("ordinary_move", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("raw_move", value_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(target.into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::RawMove(Place::local(LocalId(2)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect_err("Dead target cannot be RawMove'd");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveTargetNotLive { .. }
    ));
}

#[test]
fn raw_move_of_partial_aggregate_is_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", value_ty), Field::new("right", value_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("pair_ptr", pair_ty));
    let pair = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", pair_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pair.clone().field(0),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(pair.into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect_err("whole-target RawMove requires every target leaf Live");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveTargetNotLive { .. }
    ));
}

#[test]
fn overlapping_shared_target_loan_makes_raw_move_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("shared", value_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: target.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect_err("raw ownership transfer requires exclusive target compatibility");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveConflictsWithLoan {
            loan: LoanId(0),
            ..
        }
    ));
}

#[test]
fn overlapping_exclusive_target_loan_makes_raw_move_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("exclusive", value_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: target.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect_err("any overlapping active target loan blocks RawMove");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveConflictsWithLoan {
            loan: LoanId(0),
            ..
        }
    ));
}

#[test]
fn disjoint_exclusive_loan_does_not_block_raw_move() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", value_ty), Field::new("right", value_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pair = Place::local(LocalId(0));
    let left = pair.clone().field(0);
    let right = pair.clone().field(1);
    execute(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("right_exclusive", value_ty)],
        vec![
            Statement::Init {
                dst: pair,
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(left.into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: right.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect("disjoint active loans do not constrain RawMove target access");
}

#[test]
fn shared_loan_can_supply_pointer_value_for_raw_move_execution() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("pointer_shared", pointer_ty)],
        vec![
            Statement::Init {
                dst: target,
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: pointer.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(PlaceAccess::loan(LoanId(0))),
            },
        ],
        Terminator::Return,
    )
    .expect("pointer-value shared loan is independent of raw target access");
}

#[test]
fn ending_pointer_formation_loan_does_not_revoke_raw_move() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("source", value_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: target.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect("formation-loan end neither grants nor revokes later RawMove");
}

#[test]
fn assign_with_raw_move_from_same_target_is_source_first_without_double_drop() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("tracked_ptr", tracked_ty));
    let target = Place::local(LocalId(0));
    let report = execute(
        types,
        vec![
            LocalDecl::new("target", tracked_ty, true),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::TrackedFixture(40)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Assign {
                dst: target.clone().into(),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect("source RawMove leaves target Dead before ordinary replacement lifecycle");

    let raw_move = report
        .verification_events
        .iter()
        .position(|event| matches!(event, VerificationEvent::RawMove { target: place, .. } if place == &target))
        .expect("source RawMove occurs");
    let write = report
        .verification_events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEvent::Write {
                    place,
                    kind: VerificationWriteKind::Assign,
                } if place == &target
            )
        })
        .expect("outer Assign writes the moved value back");
    assert!(raw_move < write);
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 40, .. }))
            .count(),
        1,
        "replacement skips the already-moved source and cleanup destroys it once"
    );
}

#[test]
fn raw_move_transports_raw_pointer_metadata_at_runtime() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer_pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr_ptr", pointer_ty));
    let value = Place::local(LocalId(0));
    let moved = Place::local(LocalId(3));
    let report = execute(
        types,
        vec![
            LocalDecl::new("value", value_ty, false),
            LocalDecl::new("inner", pointer_ty, false),
            LocalDecl::new("outer", pointer_pointer_ty, false),
            LocalDecl::new("moved", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(value.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(Place::local(LocalId(1)).into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::RawMove(Place::local(LocalId(2)).into()),
            },
            Statement::RawAssign {
                pointer: moved.into(),
                src: Operand::Constant(Value::I64(9)),
            },
            Statement::Read { src: value.into() },
        ],
        Terminator::Return,
    )
    .expect("RawMove preserves a pointer-valued pointee's symbolic target");

    assert!(report.verification_events.iter().any(|event| matches!(
        event,
        VerificationEvent::RawPointerMove { place, .. } if place == &Place::local(LocalId(1))
    )));
}

#[test]
fn self_targeting_raw_move_snapshots_pointer_before_moving_same_storage() {
    let mut types = TypeTable::new();
    let pointer_ty = types.push(TypeDef::raw_pointer("self_ptr", TypeId(0)));
    assert_eq!(pointer_ty, TypeId(0));

    let pointer = Place::local(LocalId(0));
    let moved = Place::local(LocalId(1));
    execute(
        types,
        vec![
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("moved", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(pointer.clone().into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::RawMove(pointer.clone().into()),
            },
            Statement::RawAssign {
                pointer: moved.into(),
                src: Operand::AddressOf(pointer.clone().into()),
            },
            Statement::RawRead {
                pointer: pointer.into(),
            },
        ],
        Terminator::Return,
    )
    .expect("self-targeting RawMove snapshots the pointer before consuming its storage");
}

#[test]
fn raw_move_source_ub_prevents_enclosing_raw_assign_write() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let destination = Place::local(LocalId(0));
    let source = Place::local(LocalId(1));
    let error = execute(
        types,
        vec![
            LocalDecl::new("destination", value_ty, false),
            LocalDecl::new("source", value_ty, false),
            LocalDecl::new("destination_pointer", pointer_ty, false),
            LocalDecl::new("source_pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: destination.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(destination.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::AddressOf(source.into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(2)).into(),
                src: Operand::RawMove(Place::local(LocalId(3)).into()),
            },
        ],
        Terminator::Return,
    )
    .expect_err("NeverInitialized RawMove source enters UB before outer RawAssign");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveTargetNotLive { .. }
    ));
    assert!(!error.verification_events.iter().any(|event| matches!(
        event,
        VerificationEvent::Write {
            kind: VerificationWriteKind::RawAssign,
            ..
        }
    )));
}

#[test]
fn successful_raw_move_can_be_followed_by_defined_fault_cleanup() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("tracked_ptr", tracked_ty));
    let report = execute(
        types,
        vec![
            LocalDecl::new("target", tracked_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", tracked_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::TrackedFixture(80)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Fault(Fault::new("expected")),
    )
    .expect("defined Fault remains distinct from successful RawMove");

    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("expected".to_owned())
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 80, .. }))
            .count(),
        1
    );
}

#[test]
fn raw_move_undefined_behavior_does_not_run_defined_cleanup() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("tracked_ptr", tracked_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", tracked_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", tracked_ty, false),
        ],
        vec![LoanDecl::new("shared", tracked_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::TrackedFixture(90)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: target.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
        Terminator::Fault(Fault::new("must_not_be_reached")),
    )
    .expect_err("overlapping target loan enters UB before defined Fault");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveConflictsWithLoan { .. }
    ));
    assert!(
        !error
            .verification_events
            .iter()
            .any(|event| matches!(event, VerificationEvent::DropTrackedFixture { .. }))
    );
}
