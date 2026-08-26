use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ScalarType, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus, VerificationEventKind};

#[test]
fn outer_result_bearing_entry_preserves_result_across_entry_cleanup() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let entry = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(tracked),
        body: Body {
            locals: vec![
                LocalDecl::new("result", tracked, false),
                LocalDecl::new("temporary", tracked, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    runen_core_ir::Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::TrackedFixture(41)),
                    },
                    runen_core_ir::Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::TrackedFixture(42)),
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        },
    };

    let validated = validate_program(Program {
        types,
        functions: vec![entry],
    })
    .expect("result-bearing zero-parameter entry is valid");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("entry return is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::TrackedFixture(41)));

    let drops = report
        .verification_events
        .iter()
        .filter_map(|event| match event.kind {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some((event.activation.0, id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(drops, vec![(1, 42)]);
}
