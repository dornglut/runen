use runen_core_ir::{Body, Function, FunctionId, Program, TypeTable, validate_program};
use runen_reference::{ActivationId, Machine, VerificationEvent, VerificationEventKind};

pub fn one_function_program(types: TypeTable, body: Body) -> Program {
    Program {
        types,
        functions: vec![Function {
            name: "fixture".into(),
            parameters: Vec::new(),
            result: None,
            body,
        }],
    }
}

pub fn machine(program: Program) -> Machine {
    let validated = validate_program(program).expect("historical one-function fixture must validate");
    Machine::new(validated, FunctionId(0)).expect("one-function fixture has zero parameters")
}

pub fn event_kinds(events: &[VerificationEvent]) -> Vec<VerificationEventKind> {
    assert!(
        events
            .iter()
            .all(|event| event.activation == ActivationId(1)),
        "historical one-function fixture must remain within entry activation"
    );
    events.iter().map(|event| event.kind.clone()).collect()
}
