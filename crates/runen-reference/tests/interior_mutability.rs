use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_body,
};
use runen_reference::{
    ExecutionReport, Machine, TerminalStatus, VerificationEvent, VerificationWriteKind,
};

fn machine(body: Body) -> Machine {
    Machine::new(validate_body(body).expect("interior-mutability test MIR must validate"))
}

fn defined_report(body: Body) -> ExecutionReport {
    machine(body)
        .execute()
        .expect("interior-mutability fixture must have defined execution")
}

#[test]
fn shared_loan_survives_interior_replacement_and_observes_replacement_storage() {
    let mut types = TypeTable::new();
    let tracked = types.push(
        TypeDef::scalar("InteriorTracked", ScalarType::TrackedFixture).with_interior_mutability(),
    );
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
                    src: Operand::Constant(Value::TrackedFixture(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Shared,
                    src: value.clone().into(),
                },
                Statement::InteriorAssign {
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
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
            VerificationEvent::DropTrackedFixture {
                place: value.clone(),
                id: 1,
            },
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::InteriorAssign,
            },
            VerificationEvent::Read(value.clone()),
            VerificationEvent::BorrowEnd(LoanId(0)),
            VerificationEvent::DropTrackedFixture {
                place: value,
                id: 2,
            },
        ]
    );
}

#[test]
fn interior_assignment_preserves_source_first_replacement_order() {
    let mut types = TypeTable::new();
    let tracked = types.push(
        TypeDef::scalar("InteriorTracked", ScalarType::TrackedFixture).with_interior_mutability(),
    );
    let dst = Place::local(LocalId(0));
    let src = Place::local(LocalId(1));
    let body = Body {
        types,
        locals: vec![
            LocalDecl::new("dst", tracked, false),
            LocalDecl::new("src", tracked, false),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: dst.clone(),
                    src: Operand::Constant(Value::TrackedFixture(10)),
                },
                Statement::Init {
                    dst: src.clone(),
                    src: Operand::Constant(Value::TrackedFixture(20)),
                },
                Statement::InteriorAssign {
                    dst: dst.clone().into(),
                    src: Operand::Move(src.clone().into()),
                },
            ],
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    assert_eq!(
        report.verification_events,
        vec![
            VerificationEvent::Write {
                place: dst.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEvent::Write {
                place: src.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEvent::Move(src),
            VerificationEvent::DropTrackedFixture {
                place: dst.clone(),
                id: 10,
            },
            VerificationEvent::Write {
                place: dst.clone(),
                kind: VerificationWriteKind::InteriorAssign,
            },
            VerificationEvent::DropTrackedFixture { place: dst, id: 20 },
        ]
    );
}

#[test]
fn marked_aggregate_interior_assignment_drops_only_then_live_contents() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let aggregate = types.push(
        TypeDef::structure(
            "InteriorPair",
            vec![Field::new("left", tracked), Field::new("right", tracked)],
        )
        .with_interior_mutability(),
    );
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let right = root.clone().field(1);
    let taken = Place::local(LocalId(1));
    let body = Body {
        types,
        locals: vec![
            LocalDecl::new("pair", aggregate, false),
            LocalDecl::new("taken", tracked, false),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: root.clone(),
                    src: Operand::Constant(Value::Struct(vec![
                        Value::TrackedFixture(1),
                        Value::TrackedFixture(2),
                    ])),
                },
                Statement::Init {
                    dst: taken.clone(),
                    src: Operand::Move(left.clone().into()),
                },
                Statement::InteriorAssign {
                    dst: root.clone().into(),
                    src: Operand::Constant(Value::Struct(vec![
                        Value::TrackedFixture(3),
                        Value::TrackedFixture(4),
                    ])),
                },
            ],
            Terminator::Return,
        )],
    };

    let report = defined_report(body);
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEvent::DropTrackedFixture { place, id: 1 }
                    if place == &left
            ))
            .count(),
        0,
        "moved left value must not be destroyed by replacement"
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEvent::DropTrackedFixture { place, id: 2 }
                    if place == &right
            ))
            .count(),
        1,
        "the still-live right value is the old replacement destruction domain"
    );
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Write {
                place: root,
                kind: VerificationWriteKind::InteriorAssign,
            })
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 1, .. }))
            .count(),
        1,
        "the moved value is destroyed exactly once in its destination local"
    );
}

#[test]
fn interior_replacement_under_shared_borrow_is_cleaned_once_on_fault() {
    let mut types = TypeTable::new();
    let tracked = types.push(
        TypeDef::scalar("InteriorTracked", ScalarType::TrackedFixture).with_interior_mutability(),
    );
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
                    src: Operand::Constant(Value::TrackedFixture(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Shared,
                    src: value.clone().into(),
                },
                Statement::InteriorAssign {
                    dst: PlaceAccess::loan(LoanId(0)),
                    src: Operand::Constant(Value::TrackedFixture(2)),
                },
            ],
            Terminator::Fault(runen_core_ir::Fault::new("INTERIOR_FAULT")),
        )],
    };

    let report = defined_report(body);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("INTERIOR_FAULT".into())
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 1, .. }))
            .count(),
        1
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 2, .. }))
            .count(),
        1
    );
    assert!(
        !report
            .verification_events
            .contains(&VerificationEvent::BorrowEnd(LoanId(0)))
    );
}
