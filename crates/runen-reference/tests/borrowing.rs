use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Fault, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_body,
};
use runen_reference::{
    ExecutionReport, Machine, TerminalStatus, VerificationEvent, VerificationWriteKind,
};

fn machine(body: Body) -> Machine {
    Machine::new(validate_body(body).expect("borrow test MIR must pass validation"))
}

fn defined_report(body: Body) -> ExecutionReport {
    machine(body)
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
    let body = Body {
        types,
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
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(
        report.verification_events,
        vec![
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEvent::BorrowStart {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: value.clone(),
            },
            VerificationEvent::Move(value.clone()),
            VerificationEvent::Write {
                place: taken.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Assign,
            },
            VerificationEvent::Read(value.clone()),
            VerificationEvent::BorrowEnd(LoanId(0)),
            VerificationEvent::DropTrackedFixture {
                place: taken,
                id: 1,
            },
            VerificationEvent::DropTrackedFixture {
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
    let body = Body {
        types,
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
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Write {
                place: left,
                kind: VerificationWriteKind::Assign,
            })
    );
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Read(right))
    );
}

#[test]
fn defined_fault_ends_active_borrow_before_cleanup() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
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
    };

    let report = defined_report(body);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("BORROW_FAULT".into())
    );
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::BorrowStart {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value.clone(),
            })
    );
    assert!(
        !report
            .verification_events
            .contains(&VerificationEvent::BorrowEnd(LoanId(0)))
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 7, .. }))
            .count(),
        1
    );
}
