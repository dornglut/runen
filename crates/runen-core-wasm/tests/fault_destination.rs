use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeTable,
    validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, TerminalStatus};

fn result_function(name: &str, result_ty: runen_core_ir::TypeId, body: Body) -> Function {
    Function {
        name: name.into(),
        parameters: Vec::new(),
        result: Some(result_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

fn call_then_return_result(
    name: &str,
    result_ty: runen_core_ir::TypeId,
    callee: FunctionId,
) -> Function {
    result_function(
        name,
        result_ty,
        Body {
            locals: vec![LocalDecl::new("result", result_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: callee,
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        },
    )
}

#[test]
fn faulting_result_calls_do_not_initialize_destinations_or_follow_normal_targets() {
    let mut types = TypeTable::new();
    let u64_ty = types.push(TypeDef::scalar("U64", ScalarType::U64));
    let fault = Fault::new("result-call-fault");

    let entry = call_then_return_result("entry", u64_ty, FunctionId(1));
    let middle = call_then_return_result("middle", u64_ty, FunctionId(2));
    let leaf = result_function(
        "leaf",
        u64_ty,
        Body {
            locals: Vec::new(),
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(fault.clone()),
            )],
        },
    );

    let validated = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![entry, middle, leaf],
    })
    .expect("result-bearing fault propagation fixture must be valid Core");

    let reference = Machine::new(validated.clone(), FunctionId(0))
        .expect("entry is zero-parameter")
        .execute()
        .expect("explicit Core fault is a defined execution outcome");
    assert_eq!(reference.terminal, TerminalStatus::Faulted(fault.code.clone()));
    assert_eq!(reference.result, None);

    let realized = RealizedProgram::new(&validated)
        .expect("fixture is inside bounded Wasm realization coverage")
        .execute(FunctionId(0))
        .expect("defined Core fault must not become a backend failure");
    assert_eq!(realized, ExecutionOutcome::Faulted(fault));
}
