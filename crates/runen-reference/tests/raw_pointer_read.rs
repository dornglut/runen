use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Fault, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_body,
};
use runen_reference::{
    ExecutionReport, Machine, TerminalStatus, UndefinedBehavior, UndefinedBehaviorKind,
    VerificationEvent,
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
    Machine::new(validate_body(body).expect("language-valid raw-read fixture")).execute()
}

#[test]
fn raw_read_of_live_target_is_defined_and_non_consuming() {
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
            LocalDecl::new("target", tracked_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::TrackedFixture(7)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect("live target satisfies RawRead preconditions");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert!(report.verification_events.iter().any(|event| matches!(
        event,
        VerificationEvent::RawRead { target: read, .. } if read == &target
    )));
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 7, .. }))
            .count(),
        1,
        "RawRead must not consume or duplicate the non-copy pointee",
    );
}

#[test]
fn raw_read_of_never_initialized_target_is_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect_err("RawRead requires a fully-live target");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawReadTargetNotLive { .. }
    ));
}

#[test]
fn raw_read_of_dead_target_is_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("moved", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
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
            Statement::RawRead {
                pointer: Place::local(LocalId(2)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect_err("moved-out target is Dead");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawReadTargetNotLive { .. }
    ));
}

#[test]
fn raw_read_of_explicitly_dropped_target_is_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Drop {
                place: target.into(),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect_err("explicit destruction leaves a Dead raw target");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawReadTargetNotLive { .. }
    ));
}

#[test]
fn reinitialization_makes_existing_pointer_readable_again() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let report = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, true),
            LocalDecl::new("moved", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
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
                src: Operand::Move(target.clone().into()),
            },
            Statement::Assign {
                dst: target.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(2)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect("replacement in continuing storage restores a live target");

    let formed = report
        .verification_events
        .iter()
        .find_map(|event| match event {
            VerificationEvent::AddressOf { pointer, .. } => Some(pointer.clone()),
            _ => None,
        });
    let read = report
        .verification_events
        .iter()
        .find_map(|event| match event {
            VerificationEvent::RawRead {
                pointer,
                target: read,
            } if read == &target => Some(pointer.clone()),
            _ => None,
        });
    assert_eq!(read, formed);
}

#[test]
fn interior_replacement_preserves_existing_raw_read_target() {
    let mut types = TypeTable::new();
    let value_ty =
        types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    let pointer_ty = types.push(TypeDef::raw_pointer("interior_i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let report = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::InteriorAssign {
                dst: target.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect("interior replacement keeps the target storage region live");

    assert!(report.verification_events.iter().any(|event| matches!(
        event,
        VerificationEvent::RawRead { target: read, .. } if read == &target
    )));
}

#[test]
fn shared_target_loan_is_compatible_with_raw_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
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
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect("shared target loans are compatible with the raw shared read");
}

#[test]
fn exclusive_target_loan_makes_raw_read_undefined_behavior() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
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
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect_err("exclusive target loan violates RawRead compatibility");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawReadConflictsWithExclusiveLoan {
            loan: LoanId(0),
            ..
        }
    ));
}

#[test]
fn source_loan_end_does_not_revoke_later_raw_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
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
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect("ending formation loan does not erase pointer target metadata");
}

#[test]
fn partially_initialized_aggregate_target_is_not_raw_readable_as_a_whole() {
    let mut types = TypeTable::new();
    let leaf_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", leaf_ty), Field::new("right", leaf_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("pair_ptr", pair_ty));
    let pair = Place::local(LocalId(0));
    let error = execute(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
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
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return,
    )
    .expect_err("whole aggregate RawRead requires every leaf to be Live");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawReadTargetNotLive { .. }
    ));
}

#[test]
fn defined_fault_remains_defined_after_successful_raw_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let report = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Fault(Fault::new("expected")),
    )
    .expect("successful RawRead does not alter defined Fault semantics");

    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("expected".to_owned())
    );
}

#[test]
fn detected_undefined_behavior_does_not_run_defined_cleanup() {
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
            LocalDecl::new("moved", tracked_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::TrackedFixture(9)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(target.into()),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(2)).into(),
            },
        ],
        Terminator::Fault(Fault::new("must_not_be_reached")),
    )
    .expect_err("dead target enters UB before defined Fault");

    assert!(
        !error
            .verification_events
            .iter()
            .any(|event| matches!(event, VerificationEvent::DropTrackedFixture { .. }))
    );
}
