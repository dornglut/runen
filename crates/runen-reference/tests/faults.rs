use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{Machine, TerminalStatus, VerificationEventKind};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn execute_fault(code: &str) -> TerminalStatus {
    let function = Function {
        name: "fault".into(),
        parameters: Vec::new(),
        result: None,
        shared_reference_result_origin: None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(Fault::new(code)),
            )],
        ),
    };
    let validated = validate_program(Program {
        types: TypeTable::new(),
        functions: vec![function],
    })
    .expect("explicit fault is valid Core");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault is not undefined behavior")
        .terminal
}

#[test]
fn top_level_explicit_fault_preserves_the_selected_reason() {
    assert_eq!(
        execute_fault("first-reason"),
        TerminalStatus::Faulted("first-reason".into())
    );
    assert_eq!(
        execute_fault("second-reason"),
        TerminalStatus::Faulted("second-reason".into())
    );
}

#[test]
fn explicit_fault_cleans_live_locals_once_in_reverse_declaration_order() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let function = Function {
        name: "fault_with_cleanup".into(),
        parameters: Vec::new(),
        result: None,
        shared_reference_result_origin: None,
        body: body(
            vec![
                LocalDecl::new("first", tracked, false),
                LocalDecl::new("second", tracked, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::TrackedFixture(2)),
                    },
                ],
                Terminator::Fault(Fault::new("cleanup")),
            )],
        ),
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("faulting cleanup program is valid");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault is not undefined behavior");

    assert_eq!(report.terminal, TerminalStatus::Faulted("cleanup".into()));
    assert_eq!(report.result, None);
    let drops = report
        .verification_events
        .iter()
        .filter_map(|event| match event.kind {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some(id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(drops, vec![2, 1]);
}

#[test]
fn result_bearing_top_level_fault_produces_no_normal_result() {
    let mut types = TypeTable::new();
    let result_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "faulting_result".into(),
        parameters: Vec::new(),
        result: Some(result_ty),
        shared_reference_result_origin: None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(Fault::new("no-result")),
            )],
        ),
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("result-bearing function may terminate by defined fault");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault is not undefined behavior");

    assert_eq!(report.terminal, TerminalStatus::Faulted("no-result".into()));
    assert_eq!(report.result, None);
}
