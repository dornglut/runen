use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, ExternalCallableDecl, ExternalCallableId,
    Function, FunctionId, LocalDecl, LocalId, Operand, Place, Program, SafeReferenceResultContract,
    ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value,
    validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderBinding, ExternalScalarValue, RealizedProgram,
};

fn interface(parameters: Vec<TypeId>, result: Option<TypeId>) -> CallableInterface {
    CallableInterface::new(parameters, result, SafeReferenceResultContract::None)
}

fn function(
    name: &str,
    parameters: Vec<LocalId>,
    result: Option<TypeId>,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
) -> Function {
    Function {
        name: name.into(),
        parameters,
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

fn validated(
    types: TypeTable,
    external_callables: Vec<ExternalCallableDecl>,
    functions: Vec<Function>,
) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables,
        functions,
    })
    .expect("external provider composition fixture must be valid Core")
}

#[test]
fn directly_called_function_can_invoke_external_provider_and_return_normally() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let external = interface(Vec::new(), Some(i64_ty));
    let program = validated(
        types,
        vec![ExternalCallableDecl::new(external.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(i64_ty),
                vec![LocalDecl::new("result", i64_ty, false)],
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Call {
                            function: FunctionId(1),
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
            ),
            function(
                "worker",
                Vec::new(),
                Some(i64_ty),
                vec![LocalDecl::new("result", i64_ty, false)],
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
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
            ),
        ],
    );
    let invocation_count = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&invocation_count);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            move |arguments| {
                assert!(arguments.is_empty());
                seen.fetch_add(1, Ordering::SeqCst);
                ExternalScalarValue::I64(42)
            },
        )],
    )
    .expect("nested direct-call fixture must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
    assert_eq!(invocation_count.load(Ordering::SeqCst), 1);
}

#[test]
fn callable_valued_indirect_result_transport_survives_external_import_offset() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let unary = types.push(TypeDef::callable(
        "Unary",
        interface(vec![i64_ty], Some(i64_ty)),
    ));
    let factory = types.push(TypeDef::callable(
        "Factory",
        interface(Vec::new(), Some(unary)),
    ));
    let external = interface(Vec::new(), None);
    let program = validated(
        types,
        vec![ExternalCallableDecl::new(external.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(i64_ty),
                vec![
                    LocalDecl::new("factory", factory, false),
                    LocalDecl::new("returned", unary, false),
                    LocalDecl::new("copy", unary, false),
                    LocalDecl::new("result", i64_ty, false),
                ],
                vec![
                    BasicBlock::new(
                        vec![Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::FunctionValue(FunctionId(1)),
                        }],
                        Terminator::IndirectCall {
                            callable: factory,
                            callee: Operand::Move(Place::local(LocalId(0)).into()),
                            arguments: Vec::new(),
                            destination: Some(Place::local(LocalId(1))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        vec![Statement::Init {
                            dst: Place::local(LocalId(2)),
                            src: Operand::Copy(Place::local(LocalId(1)).into()),
                        }],
                        Terminator::IndirectCall {
                            callable: unary,
                            callee: Operand::Move(Place::local(LocalId(2)).into()),
                            arguments: vec![Operand::Constant(Value::I64(41))],
                            destination: Some(Place::local(LocalId(3))),
                            target: BasicBlockId(2),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(3)).into()))),
                    ),
                ],
            ),
            function(
                "producer",
                Vec::new(),
                Some(unary),
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::FunctionValue(FunctionId(2)))),
                )],
            ),
            function(
                "target",
                vec![LocalId(0)],
                Some(i64_ty),
                vec![LocalDecl::new("value", i64_ty, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
                )],
            ),
        ],
    );
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            external,
            |_| {},
        )],
    )
    .expect("callable-result/import composition fixture must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}
