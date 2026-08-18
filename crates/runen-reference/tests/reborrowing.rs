use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Fault, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_body,
};
use runen_reference::{
    ExecutionReport, Machine, TerminalStatus, VerificationEvent, VerificationWriteKind,
};

fn machine(body: Body) -> Machine {
    Machine::new(validate_body(body).expect("reborrow test MIR must validate"))
}

fn defined_report(body: Body) -> ExecutionReport {
    machine(body)
        .execute()
        .expect("reborrow fixture must have defined execution")
}

#[test]
fn child_borrow_source_resolves_to_concrete_subplace() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", scalar), Field::new("right", scalar)],
    ));
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let body = Body {
        types,
        locals: vec![LocalDecl::new("pair", pair, true)],
        loans: vec![
            LoanDecl::new("parent", pair),
            LoanDecl::new("child", scalar),
        ],
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
                Statement::Borrow {
                    loan: LoanId(1),
                    kind: BorrowKind::Exclusive,
                    src: PlaceAccess::loan(LoanId(0)).field(0),
                },
                Statement::Assign {
                    dst: PlaceAccess::loan(LoanId(1)),
                    src: Operand::Constant(Value::I64(3)),
                },
                Statement::EndBorrow { loan: LoanId(1) },
                Statement::EndBorrow { loan: LoanId(0) },
            ],
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::BorrowStart {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                place: left.clone(),
            })
    );
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Write {
                place: left,
                kind: VerificationWriteKind::Assign,
            })
    );
}

#[test]
fn exclusive_child_controls_storage_across_move_and_replacement() {
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
        loans: vec![
            LoanDecl::new("parent", tracked),
            LoanDecl::new("child", tracked),
        ],
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
                Statement::Borrow {
                    loan: LoanId(1),
                    kind: BorrowKind::Exclusive,
                    src: PlaceAccess::loan(LoanId(0)),
                },
                Statement::Init {
                    dst: taken.clone(),
                    src: Operand::Move(PlaceAccess::loan(LoanId(1))),
                },
                Statement::Assign {
                    dst: PlaceAccess::loan(LoanId(1)),
                    src: Operand::Constant(Value::TrackedFixture(2)),
                },
                Statement::EndBorrow { loan: LoanId(1) },
                Statement::EndBorrow { loan: LoanId(0) },
            ],
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    let events = &report.verification_events;
    assert!(events.contains(&VerificationEvent::Move(value.clone())));
    assert!(events.contains(&VerificationEvent::Write {
        place: value.clone(),
        kind: VerificationWriteKind::Assign,
    }));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 1, .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 2, .. }))
            .count(),
        1
    );
    assert!(events.contains(&VerificationEvent::DropTrackedFixture {
        place: taken,
        id: 1
    }));
}

#[test]
fn explicit_nested_borrow_end_is_leaf_to_root() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", scalar, false)],
        loans: vec![
            LoanDecl::new("root", scalar),
            LoanDecl::new("child", scalar),
            LoanDecl::new("grandchild", scalar),
        ],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: value.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Exclusive,
                    src: value.clone().into(),
                },
                Statement::Borrow {
                    loan: LoanId(1),
                    kind: BorrowKind::Exclusive,
                    src: PlaceAccess::loan(LoanId(0)),
                },
                Statement::Borrow {
                    loan: LoanId(2),
                    kind: BorrowKind::Shared,
                    src: PlaceAccess::loan(LoanId(1)),
                },
                Statement::EndBorrow { loan: LoanId(2) },
                Statement::EndBorrow { loan: LoanId(1) },
                Statement::EndBorrow { loan: LoanId(0) },
            ],
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    let ends = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::BorrowEnd(loan) => Some(*loan),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(ends, vec![LoanId(2), LoanId(1), LoanId(0)]);
}

#[test]
fn defined_fault_terminates_nested_forest_before_cleanup() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", tracked, false)],
        loans: vec![
            LoanDecl::new("parent", tracked),
            LoanDecl::new("child", tracked),
        ],
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
                Statement::Borrow {
                    loan: LoanId(1),
                    kind: BorrowKind::Shared,
                    src: PlaceAccess::loan(LoanId(0)),
                },
            ],
            Terminator::Fault(Fault::new("NESTED_BORROW_FAULT")),
        )],
    };

    let report = defined_report(body);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("NESTED_BORROW_FAULT".into())
    );
    assert!(
        !report
            .verification_events
            .iter()
            .any(|event| matches!(event, VerificationEvent::BorrowEnd(_)))
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
