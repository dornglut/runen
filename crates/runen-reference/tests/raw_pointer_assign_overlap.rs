mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId, Operand,
    Place, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, Value,
};
use runen_reference::{UndefinedBehaviorKind, VerificationEventKind, VerificationWriteKind};
use support::{event_kinds, machine, one_function_program};

#[test]
fn self_targeting_raw_assign_executes_from_snapshotted_pointer_value() {
    let mut types = TypeTable::new();
    let pointer_ty = types.push(TypeDef::raw_pointer("self_ptr", TypeId(0)));
    assert_eq!(pointer_ty, TypeId(0));

    let pointer = Place::local(LocalId(0));
    let program = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("pointer", pointer_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(pointer.clone().into()),
                    },
                    Statement::RawAssign {
                        pointer: pointer.clone().into(),
                        src: Operand::Move(pointer.clone().into()),
                    },
                    Statement::RawRead {
                        pointer: pointer.clone().into(),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let report = machine(program)
        .execute()
        .expect("self-targeting RawAssign has no active target loan");
    let events = event_kinds(&report.verification_events);

    let formed = events
        .iter()
        .find_map(|event| match event {
            VerificationEventKind::AddressOf { pointer, .. } => Some(pointer.clone()),
            _ => None,
        })
        .expect("self-target formation is instrumented");
    let moved = events
        .iter()
        .position(|event| matches!(event, VerificationEventKind::Move(place) if place == &pointer))
        .expect("source pointer Move is instrumented");
    let write = events
        .iter()
        .position(|event| {
            matches!(
                event,
                VerificationEventKind::Write {
                    place,
                    kind: VerificationWriteKind::RawAssign,
                } if place == &pointer
            )
        })
        .expect("RawAssign writes the snapshotted self target");
    let read = events
        .iter()
        .find_map(|event| match event {
            VerificationEventKind::RawRead { pointer, .. } => Some(pointer.clone()),
            _ => None,
        })
        .expect("written-back pointer remains readable");

    assert!(moved < write);
    assert_eq!(formed, read);
}

#[test]
fn raw_assign_evaluates_source_before_detecting_target_loan_ub() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("target", value_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
            ],
            loans: vec![LoanDecl::new("shared", value_ty)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
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
                        src: target.clone().into(),
                    },
                    Statement::RawAssign {
                        pointer: Place::local(LocalId(1)).into(),
                        src: Operand::Copy(target.clone().into()),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let error = machine(program)
        .execute()
        .expect_err("overlapping shared target loan makes RawAssign undefined");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawAssignConflictsWithLoan {
            loan: LoanId(0),
            ..
        }
    ));
    let events = event_kinds(&error.verification_events);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, VerificationEventKind::Copy(place) if place == &target)),
        "source Copy must execute before the raw target conflict is checked"
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            VerificationEventKind::Write {
                kind: VerificationWriteKind::RawAssign,
                ..
            }
        )),
        "UB prevents the RawAssign replacement write"
    );
}
