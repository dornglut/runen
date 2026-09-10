use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Field, Function, FunctionId, LocalDecl,
    LocalId, Operand, Place, Program, SafeReferenceResultContract, Statement, Terminator, TypeDef,
    TypeTable, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn execute(program: Program) -> runen_reference::ExecutionReport {
    let validated = validate_program(program).expect("callable storage fixture must validate");
    Machine::new(validated, FunctionId(0))
        .expect("callable storage fixture has a zero-parameter entry")
        .execute()
        .expect("callable storage fixture has defined execution")
}

fn no_result_target() -> Function {
    Function {
        name: "target".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    }
}

#[test]
fn callable_leaf_inside_aggregate_uses_ordinary_copy_move_and_cleanup() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "NoArgs",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let wrapper = types.push(TypeDef::structure(
        "Wrapper",
        vec![Field::new("function", callable)],
    ));

    let entry = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("source", wrapper, false),
                LocalDecl::new("copy", wrapper, false),
                LocalDecl::new("moved", wrapper, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)).field(0),
                        src: Operand::FunctionValue(FunctionId(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Copy(Place::local(LocalId(0)).into()),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::Move(Place::local(LocalId(1)).into()),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(0)).into(),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(2)).into(),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![entry, no_result_target()],
    });
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, None);
}

#[test]
fn callable_value_passes_and_returns_through_existing_call_transfer() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "NoArgs",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));

    let entry = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(callable),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("returned", callable, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::FunctionValue(FunctionId(2))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        ),
    };
    let identity = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(callable),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("function", callable, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![entry, identity, no_result_target()],
    });
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::Function(FunctionId(2))));
}
