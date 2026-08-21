mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Fault, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
};
use runen_reference::{
    ExecutionReport, TerminalStatus, UndefinedBehavior, UndefinedBehaviorKind,
    VerificationEventKind, VerificationWriteKind,
};
use support::{event_kinds, machine, one_function_program};

fn execute(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
    terminator: Terminator,
) -> Result<ExecutionReport, UndefinedBehavior> {
    let program = one_function_program(
        types,
        Body {
            locals,
            loans,
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(statements, terminator)],
        },
    );
    machine(program).execute()
}

#[test]
fn raw_assign_replaces_live_target_with_existing_replacement_lifecycle() {
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
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
        ],
        Terminator::Return(None),
    )
    .expect("RawAssign has no overlapping target loan");

    let events = event_kinds(&report.verification_events);
    let old_drop = events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 1, .. }
            )
        })
        .expect("old value is destroyed during replacement");
    let write = events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEventKind::Write {
                    place,
                    kind: VerificationWriteKind::RawAssign,
                } if place == &target
            )
        })
        .expect("RawAssign write is instrumented");
    let new_drop = events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 2, .. }
            )
        })
        .expect("replacement value is destroyed during normal cleanup");

    assert!(old_drop < write && write < new_drop);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 1, .. }
            ))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 2, .. }
            ))
            .count(),
        1
    );
}

#[test]
fn raw_assign_initializes_never_initialized_target() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
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
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(7)),
            },
            Statement::Read {
                src: target.clone().into(),
            },
        ],
        Terminator::Return(None),
    )
    .expect("Never-initialized storage is a legal RawAssign replacement target");

    let events = event_kinds(&report.verification_events);
    assert!(events.iter().any(|event| matches!(
        event,
        VerificationEventKind::Write {
            place,
            kind: VerificationWriteKind::RawAssign,
        } if place == &target
    )));
}

#[test]
fn raw_assign_reinitializes_dead_target() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    execute(
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
                src: Operand::Move(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(2)).into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read { src: target.into() },
        ],
        Terminator::Return(None),
    )
    .expect("Dead continuing storage is a legal RawAssign replacement target");
}

#[test]
fn raw_assign_replaces_partial_aggregate_and_drops_only_live_old_fields() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![
            Field::new("left", tracked_ty),
            Field::new("right", tracked_ty),
        ],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("pair_ptr", pair_ty));
    let pair = Place::local(LocalId(0));
    let report = execute(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pair.clone().field(0),
                src: Operand::Constant(Value::TrackedFixture(10)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(pair.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::Struct(vec![
                    Value::TrackedFixture(20),
                    Value::TrackedFixture(30),
                ])),
            },
            Statement::Read { src: pair.into() },
        ],
        Terminator::Return(None),
    )
    .expect("partial target replacement is defined without overlapping loans");

    let events = event_kinds(&report.verification_events);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 10, .. }
            ))
            .count(),
        1,
        "only the old live left field is destroyed during replacement"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 20 | 30, .. }
            ))
            .count(),
        2,
        "both replacement fields are live and cleaned exactly once"
    );
}

#[test]
fn raw_assign_snapshots_target_before_source_move_and_evaluates_source_first() {
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
                src: Operand::Constant(Value::TrackedFixture(40)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Move(target.clone().into()),
            },
        ],
        Terminator::Return(None),
    )
    .expect("moving the target value out before writing it back is defined");

    let events = event_kinds(&report.verification_events);
    let move_index = events
        .iter()
        .position(|event| matches!(event, VerificationEventKind::Move(place) if place == &target))
        .expect("source Move is recorded");
    let write_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEventKind::Write {
                    place,
                    kind: VerificationWriteKind::RawAssign,
                } if place == &target
            )
        })
        .expect("RawAssign writes the snapshotted target");
    assert!(move_index < write_index);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 40, .. }
            ))
            .count(),
        1,
        "source-first Move prevents a spurious replacement drop; cleanup drops once"
    );
}

#[test]
fn overlapping_shared_target_loan_makes_raw_assign_undefined_behavior() {
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
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
        Terminator::Return(None),
    )
    .expect_err("raw target writes require exclusive compatibility");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawAssignConflictsWithLoan {
            loan: LoanId(0),
            ..
        }
    ));
}

#[test]
fn overlapping_exclusive_target_loan_makes_raw_assign_undefined_behavior() {
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
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
        Terminator::Return(None),
    )
    .expect_err("any overlapping active target loan blocks RawAssign");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawAssignConflictsWithLoan {
            loan: LoanId(0),
            ..
        }
    ));
}

#[test]
fn disjoint_exclusive_loan_does_not_block_raw_assign() {
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
        ],
        vec![LoanDecl::new("right_exclusive", value_ty)],
        vec![
            Statement::Init {
                dst: pair,
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(left.clone().into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: right.into(),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(3)),
            },
            Statement::Read { src: left.into() },
        ],
        Terminator::Return(None),
    )
    .expect("disjoint active loans do not constrain RawAssign target access");
}

#[test]
fn shared_loan_can_supply_pointer_value_for_raw_assign_execution() {
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
        ],
        vec![LoanDecl::new("pointer_shared", pointer_ty)],
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: pointer.into(),
            },
            Statement::RawAssign {
                pointer: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(4)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Read { src: target.into() },
        ],
        Terminator::Return(None),
    )
    .expect("pointer-value shared loan is independent of disjoint raw target access");
}

#[test]
fn ending_pointer_formation_loan_does_not_revoke_raw_assign() {
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
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(5)),
            },
        ],
        Terminator::Return(None),
    )
    .expect("formation-loan end neither grants nor revokes later RawAssign");
}

#[test]
fn raw_assign_does_not_require_ordinary_mutability_or_interior_marker() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    execute(
        types,
        vec![
            LocalDecl::new("immutable_unmarked_target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(6)),
            },
        ],
        Terminator::Return(None),
    )
    .expect("RawAssign has its own unsafe target-write gate");
}

#[test]
fn successful_raw_assign_preserves_pointer_target_metadata() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
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
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(7)),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
        Terminator::Return(None),
    )
    .expect("successful RawAssign leaves the pointer value unchanged");

    let events = event_kinds(&report.verification_events);
    let formed = events
        .iter()
        .find_map(|event| match event {
            VerificationEventKind::AddressOf { pointer, .. } => Some(pointer.clone()),
            _ => None,
        })
        .expect("pointer formation is instrumented");
    let read = events
        .iter()
        .find_map(|event| match event {
            VerificationEventKind::RawRead { pointer, .. } => Some(pointer.clone()),
            _ => None,
        })
        .expect("later RawRead observes the continuing pointer value");
    assert_eq!(formed, read);
}

#[test]
fn successful_raw_assign_can_be_followed_by_defined_fault_cleanup() {
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
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::TrackedFixture(80)),
            },
        ],
        Terminator::Fault(Fault::new("expected")),
    )
    .expect("defined Fault remains distinct from successful unsafe execution");

    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("expected".to_owned())
    );
    let events = event_kinds(&report.verification_events);
    assert!(events.iter().any(|event| matches!(
        event,
        VerificationEventKind::DropTrackedFixture { id: 80, .. }
    )));
}

#[test]
fn raw_assign_undefined_behavior_does_not_run_defined_cleanup() {
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
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::TrackedFixture(91)),
            },
        ],
        Terminator::Fault(Fault::new("must_not_be_reached")),
    )
    .expect_err("overlapping shared target loan enters UB before defined Fault");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawAssignConflictsWithLoan { .. }
    ));
    let events = event_kinds(&error.verification_events);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, VerificationEventKind::DropTrackedFixture { .. }))
    );
}
