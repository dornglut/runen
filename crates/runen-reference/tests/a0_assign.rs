mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, LocalDecl, LocalId, MirValidationErrorKind, Operand, Place,
    ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{VerificationEventKind, VerificationWriteKind};
use support::{event_kinds, machine, one_function_program};

#[test]
fn assignment_drops_live_old_value_then_writes_replacement() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let place = Place::local(LocalId(0));

    let program = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("value", tracked, true)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: place.clone(),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    },
                    Statement::Assign {
                        dst: place.clone().into(),
                        src: Operand::Constant(Value::TrackedFixture(2)),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let report = machine(program)
        .execute()
        .expect("A0 assignment fixture has defined execution");
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
    assert!(events.contains(&VerificationEventKind::Write {
        place,
        kind: VerificationWriteKind::Assign,
    }));
}

#[test]
fn immutable_local_assignment_is_rejected_before_execution() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));

    let program = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("value", i64_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: place.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Assign {
                        dst: place.into(),
                        src: Operand::Constant(Value::I64(2)),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let error =
        validate_program(program).expect_err("immutable assignment must fail MIR validation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}
