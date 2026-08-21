mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
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
        .expect("interior-mutability fixture must have defined execution")
}

#[test]
fn shared_loan_survives_interior_replacement_and_observes_replacement_storage() {
    let mut types = TypeTable::new();
    let tracked = types.push(
        TypeDef::scalar("InteriorTracked", ScalarType::TrackedFixture).with_interior_mutability(),
    );
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
                Terminator::Return(None),
            )],
        },
    );

    let report = defined_report(program);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    let events = event_kinds(&report.verification_events);
    assert_eq!(
        events,
        vec![
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::BorrowStart {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
            VerificationEventKind::DropTrackedFixture {
                place: value.clone(),
                id: 1,
            },
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::InteriorAssign,
            },
            VerificationEventKind::Read(value.clone()),
            VerificationEventKind::BorrowEnd(LoanId(0)),
            VerificationEventKind::DropTrackedFixture {
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
    let program = one_function_program(
        types,
        Body {
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
                Terminator::Return(None),
            )],
        },
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    assert_eq!(
        events,
        vec![
            VerificationEventKind::Write {
                place: dst.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::Write {
                place: src.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::Move(src),
            VerificationEventKind::DropTrackedFixture {
                place: dst.clone(),
                id: 10,
            },
            VerificationEventKind::Write {
                place: dst.clone(),
                kind: VerificationWriteKind::InteriorAssign,
            },
            VerificationEventKind::DropTrackedFixture { place: dst, id: 20 },
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
    let program = one_function_program(
        types,
        Body {
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
                Terminator::Return(None),
            )],
        },
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { place, id: 1 }
                    if place == &left
            ))
            .count(),
        0,
        "moved left value must not be destroyed by replacement"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { place, id: 2 }
                    if place == &right
            ))
            .count(),
        1,
        "the still-live right value is the old replacement destruction domain"
    );
    assert!(events.contains(&VerificationEventKind::Write {
        place: root,
        kind: VerificationWriteKind::InteriorAssign,
    }));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 1, .. }
            ))
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
        },
    );

    let report = defined_report(program);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("INTERIOR_FAULT".into())
    );
    let events = event_kinds(&report.verification_events);
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
    assert!(!events.contains(&VerificationEventKind::BorrowEnd(LoanId(0))));
}
