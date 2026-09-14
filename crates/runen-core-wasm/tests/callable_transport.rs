use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Function, FunctionId, LocalDecl, LocalId,
    Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn function(
    name: &str,
    result: Option<runen_core_ir::TypeId>,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
) -> Function {
    Function {
        name: name.into(),
        parameters: Vec::new(),
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks,
        },
    }
}

#[test]
fn callable_copy_assignment_then_indirect_invocation_matches_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "Thunk",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));

    let entry = function(
        "entry",
        Some(i64_ty),
        vec![
            LocalDecl::new("original", callable, false),
            LocalDecl::new("selected", callable, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::FunctionValue(FunctionId(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::FunctionValue(FunctionId(2)),
                    },
                    Statement::Assign {
                        dst: Place::local(LocalId(1)).into(),
                        src: Operand::Copy(Place::local(LocalId(0)).into()),
                    },
                    Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    },
                ],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(1)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(2))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            ),
        ],
    );
    let selected_target = function(
        "selected_target",
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
        )],
    );
    let displaced_target = function(
        "displaced_target",
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(7)))),
        )],
    );

    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![entry, selected_target, displaced_target],
    })
    .expect("callable copy/assignment fixture must be valid Core");

    let reference = Machine::new(program.clone(), FunctionId(0))
        .expect("reference entry must be admitted")
        .execute()
        .expect("callable transport fixture has defined execution");
    assert_eq!(reference.terminal, TerminalStatus::Returned);
    assert_eq!(reference.result, Some(ObservedValue::I64(42)));

    let realized = RealizedProgram::new(&program)
        .expect("callable copy/assignment fixture must be inside realization coverage")
        .execute(FunctionId(0))
        .expect("callable copy/assignment fixture must execute");
    assert_eq!(realized, ExecutionOutcome::Returned(Some(Value::I64(42))));
}
