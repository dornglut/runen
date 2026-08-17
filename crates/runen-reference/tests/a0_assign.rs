use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, LocalDecl, LocalId, Operand, Place, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value,
};
use runen_reference::{Machine, SemanticErrorKind, TraceEvent, WriteKind};

#[test]
fn assignment_drops_live_old_value_then_writes_replacement() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::Tracked));
    let place = Place::local(LocalId(0));

    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", tracked, true)],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: place.clone(),
                    src: Operand::Constant(Value::Tracked(1)),
                },
                Statement::Assign {
                    dst: place.clone(),
                    src: Operand::Constant(Value::Tracked(2)),
                },
            ],
            Terminator::Return,
        )],
    };

    let report = Machine::new(body)
        .execute()
        .expect("mutable assignment must execute");
    assert_eq!(
        report
            .trace
            .iter()
            .filter(|event| matches!(event, TraceEvent::DropTracked { id: 1, .. }))
            .count(),
        1
    );
    assert_eq!(
        report
            .trace
            .iter()
            .filter(|event| matches!(event, TraceEvent::DropTracked { id: 2, .. }))
            .count(),
        1
    );
    assert!(report.trace.contains(&TraceEvent::Write {
        place,
        kind: WriteKind::Assign,
    }));
}

#[test]
fn immutable_local_cannot_be_assigned_after_initialization() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));

    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", i64_ty, false)],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: place.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Assign {
                    dst: place,
                    src: Operand::Constant(Value::I64(2)),
                },
            ],
            Terminator::Return,
        )],
    };

    let error = Machine::new(body)
        .execute()
        .expect_err("immutable assignment must fail");
    assert_eq!(error.kind, SemanticErrorKind::AssignToImmutable(LocalId(0)));
}
