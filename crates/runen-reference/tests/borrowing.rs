mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Fault, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable,
    Value,
};
use runen_reference::{
    ExecutionReport, TerminalStatus, VerificationEventKind, VerificationWriteKind,
};
use support::{event_kinds, machine, one_function_program};

fn defined_report(program: Program) -> ExecutionReport {
    machine(program)
        .execute()
        .expect("borrow fixture must have defined execution")
}

#[test]
fn exclusive_loan_controls_storage_across_move_and_replacement() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let taken = Place::local(LocalId(1));
    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("value", tracked, true),
                LocalDecl::new("taken", tracked, false),
            ],
            loans: vec![LoanDecl::new("exclusive", tracked)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    },
                    Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Exclusive,
                        src: value.clone().into(),
                    },
                    Statement::Init {
                        dst: taken.clone(),
                        src: Operand::Move(PlaceAccess::loan(LoanId(0))),
                    },
                    Statement::Assign {
                        dst: PlaceAccess::loan(LoanId(0)),
                        src: Operand::Constant(Value::TrackedFixture(2)),
                    },
                    Statement::Read {
                        src: PlaceAccess::loan(LoanId(0)),
                    },
                    Statement::EndBorrow { loan: LoanId(0) },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let report = defined_report(program);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(
        event_kinds(&report.verification_events),
        vec![
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::BorrowStart {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: value.clone(),
            },
            VerificationEventKind::Move(value.clone()),
            VerificationEventKind::Write {
                place: taken.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Assign,
            },
            VerificationEventKind::Read(value.clone()),
            VerificationEventKind::BorrowEnd(LoanId(0)),
            VerificationEventKind::DropTrackedFixture {
                place: taken,
                id: 1,
            },
            VerificationEventKind::DropTrackedFixture {
                place: value,
                id: 2,
            },
        ]
    );
}

#[test]
fn loan_relative_projection_resolves_to_concrete_subplace() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", scalar), Field::new("right", scalar)],
    ));
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let right = root.clone().field(1);
    let program = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("pair", pair, true)],
            loans: vec![LoanDecl::new("whole", pair)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: root.clone(),
                        src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
                    },
                    Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Exclusive,
                        src: root.into(),
                    },
                    Statement::Assign {
                        dst: PlaceAccess::loan(LoanId(0)).field(0),
                        src: Operand::Constant(Value::I64(3)),
                    },
                    Statement::Read {
                        src: PlaceAccess::loan(LoanId(0)).field(1),
                    },
                    Statement::EndBorrow { loan: LoanId(0) },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    assert!(events.contains(&VerificationEventKind::Write {
        place: left,
        kind: VerificationWriteKind::Assign,
    }));
    assert!(events.contains(&VerificationEventKind::Read(right)));
}

#[test]
fn defined_fault_ends_active_borrow_before_cleanup() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let program = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("value", tracked, false)],
            loans: vec![LoanDecl::new("shared", tracked)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::TrackedFixture(7)),
                    },
                    Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Shared,
                        src: value.clone().into(),
                    },
                ],
                Terminator::Fault(Fault::new("BORROW_FAULT")),
            )],
        },
    );

    let report = defined_report(program);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("BORROW_FAULT".into())
    );
    let events = event_kinds(&report.verification_events);
    assert!(events.contains(&VerificationEventKind::BorrowStart {
        loan: LoanId(0),
        kind: BorrowKind::Shared,
        place: value.clone(),
    }));
    assert!(!events.contains(&VerificationEventKind::BorrowEnd(LoanId(0))));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, VerificationEventKind::DropTrackedFixture { id: 7, .. }))
            .count(),
        1
    );
}
