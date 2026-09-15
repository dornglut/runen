use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Fault, Field, Function, FunctionId,
    LocalDecl, LocalId, Operand, Place, Program, ReferencePermission, SafeReferenceResultContract,
    ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value,
    validate_program,
};
use runen_core_wasm::{
    CoverageErrorKind, CoverageLocation, ExecutionOutcome, RealizationError, RealizedProgram,
    UnsupportedTypeCategory,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

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

fn validated(types: TypeTable, functions: Vec<Function>) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions,
    })
    .expect("higher-order callable fixture must be valid Core")
}

fn callable_type(
    types: &mut TypeTable,
    name: &str,
    parameters: Vec<TypeId>,
    result: Option<TypeId>,
) -> TypeId {
    types.push(TypeDef::callable(
        name,
        CallableInterface::new(parameters, result, SafeReferenceResultContract::None),
    ))
}

fn observed_to_value(value: ObservedValue) -> Value {
    match value {
        ObservedValue::Bool(value) => Value::Bool(value),
        ObservedValue::I8(value) => Value::I8(value),
        ObservedValue::I16(value) => Value::I16(value),
        ObservedValue::I32(value) => Value::I32(value),
        ObservedValue::I64(value) => Value::I64(value),
        ObservedValue::U8(value) => Value::U8(value),
        ObservedValue::U16(value) => Value::U16(value),
        ObservedValue::U32(value) => Value::U32(value),
        ObservedValue::U64(value) => Value::U64(value),
        ObservedValue::Struct(fields) => {
            Value::Struct(fields.into_iter().map(observed_to_value).collect())
        }
        other => panic!("unsupported higher-order callable observation: {other:?}"),
    }
}

fn reference_outcome(program: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(program, entry)
        .expect("higher-order callable entry must be admitted by reference machine")
        .execute()
        .expect("higher-order callable realization subset contains no Core UB operations");
    match report.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(report.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(program: &ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(program.clone(), entry);
    let actual = RealizedProgram::new(program)
        .expect("higher-order callable fixture must be inside Wasm realization coverage")
        .execute(entry)
        .expect("observable higher-order callable fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn higher_order_parameter_preserves_distinct_function_identity_and_copy_transport() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let unary = callable_type(&mut types, "Unary", vec![i64_ty], Some(i64_ty));
    let higher = callable_type(&mut types, "Higher", vec![unary, i64_ty], Some(i64_ty));

    let make_entry = |name: &str, target: FunctionId| {
        function(
            name,
            Vec::new(),
            Some(i64_ty),
            vec![
                LocalDecl::new("higher", higher, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::FunctionValue(FunctionId(2)),
                    }],
                    Terminator::IndirectCall {
                        callable: higher,
                        callee: Operand::Move(Place::local(LocalId(0)).into()),
                        arguments: vec![
                            Operand::FunctionValue(target),
                            Operand::Constant(Value::I64(41)),
                        ],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                ),
            ],
        )
    };

    let apply = function(
        "apply",
        vec![LocalId(0), LocalId(1)],
        Some(i64_ty),
        vec![
            LocalDecl::new("function", unary, false),
            LocalDecl::new("value", i64_ty, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable: unary,
                    callee: Operand::Copy(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Copy(Place::local(LocalId(1)).into())],
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
    let left = function(
        "left",
        vec![LocalId(0)],
        Some(i64_ty),
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(11)))),
        )],
    );
    let right = function(
        "right",
        vec![LocalId(0)],
        Some(i64_ty),
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(22)))),
        )],
    );
    let program = validated(
        types,
        vec![
            make_entry("left_entry", FunctionId(3)),
            make_entry("right_entry", FunctionId(4)),
            apply,
            left,
            right,
        ],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(11)))
    );
    assert_eq!(
        assert_differential(&program, FunctionId(1)),
        ExecutionOutcome::Returned(Some(Value::I64(22)))
    );
}

#[test]
fn callable_valued_indirect_result_copies_then_invokes() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let unary = callable_type(&mut types, "Unary", vec![i64_ty], Some(i64_ty));
    let factory = callable_type(&mut types, "Factory", Vec::new(), Some(unary));

    let entry = function(
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
    );
    let producer = function(
        "producer",
        Vec::new(),
        Some(unary),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(2)))),
        )],
    );
    let target = function(
        "target",
        vec![LocalId(0)],
        Some(i64_ty),
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
        )],
    );
    let program = validated(types, vec![entry, producer, target]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn multiple_callable_signature_levels_remain_scalar_carriers() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let unary = callable_type(&mut types, "Unary", vec![i64_ty], Some(i64_ty));
    let factory = callable_type(&mut types, "Factory", Vec::new(), Some(unary));
    let meta = callable_type(&mut types, "Meta", vec![factory], Some(unary));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("meta", meta, false),
            LocalDecl::new("returned", unary, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(1)),
                }],
                Terminator::IndirectCall {
                    callable: meta,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::FunctionValue(FunctionId(2))],
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable: unary,
                    callee: Operand::Move(Place::local(LocalId(1)).into()),
                    arguments: vec![Operand::Constant(Value::I64(7))],
                    destination: Some(Place::local(LocalId(2))),
                    target: BasicBlockId(2),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            ),
        ],
    );
    let invoke_factory = function(
        "invoke_factory",
        vec![LocalId(0)],
        Some(unary),
        vec![
            LocalDecl::new("factory", factory, false),
            LocalDecl::new("result", unary, false),
        ],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable: factory,
                    callee: Operand::Copy(Place::local(LocalId(0)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            ),
        ],
    );
    let factory_target = function(
        "factory_target",
        Vec::new(),
        Some(unary),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(3)))),
        )],
    );
    let unary_target = function(
        "unary_target",
        vec![LocalId(0)],
        Some(i64_ty),
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(73)))),
        )],
    );
    let program = validated(
        types,
        vec![entry, invoke_factory, factory_target, unary_target],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(73)))
    );
}

#[test]
fn faulting_callable_result_does_not_follow_normal_continuation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let unary = callable_type(&mut types, "Unary", vec![i64_ty], Some(i64_ty));
    let factory = callable_type(&mut types, "Factory", Vec::new(), Some(unary));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("factory", factory, false),
            LocalDecl::new("returned", unary, false),
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
                Vec::new(),
                Terminator::Fault(Fault::new("normal-continuation-ran")),
            ),
        ],
    );
    let producer = function(
        "producer",
        Vec::new(),
        Some(unary),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Fault(Fault::new("higher-order-result-fault")),
        )],
    );
    let program = validated(types, vec![entry, producer]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Faulted(Fault::new("higher-order-result-fault"))
    );
}

#[test]
fn cyclic_callable_signature_graph_executes_without_recursive_layout() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let a = types.push(TypeDef::callable(
        "A",
        CallableInterface::new(vec![TypeId(2)], None, SafeReferenceResultContract::None),
    ));
    let b = types.push(TypeDef::callable(
        "B",
        CallableInterface::new(vec![a], None, SafeReferenceResultContract::None),
    ));
    assert_eq!(a, TypeId(1));
    assert_eq!(b, TypeId(2));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![LocalDecl::new("a", a, false), LocalDecl::new("b", b, false)],
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
                ],
                Terminator::IndirectCall {
                    callable: a,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
            ),
        ],
    );
    let target_a = function(
        "target_a",
        vec![LocalId(0)],
        None,
        vec![LocalDecl::new("b", b, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );
    let target_b = function(
        "target_b",
        vec![LocalId(0)],
        None,
        vec![LocalDecl::new("a", a, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );
    let program = validated(types, vec![entry, target_a, target_b]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

fn local_callable_coverage_error(types: TypeTable, callable: TypeId) -> RealizationError {
    let program = validated(
        types,
        vec![function(
            "entry",
            Vec::new(),
            None,
            vec![LocalDecl::new("callable", callable, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    );
    match RealizedProgram::new(&program) {
        Err(error) => error,
        Ok(_) => panic!("fixture must remain outside realization coverage"),
    }
}

#[test]
fn nested_callable_with_direct_floating_component_is_admitted() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let inner = callable_type(&mut types, "Inner", vec![f32_ty], None);
    let outer = callable_type(&mut types, "Outer", vec![inner], None);
    let program = validated(
        types,
        vec![function(
            "entry",
            Vec::new(),
            None,
            vec![LocalDecl::new("callable", outer, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    );

    RealizedProgram::new(&program)
        .expect("nested callable with a direct floating component must be admitted");
}

#[test]
fn nested_callable_reports_safe_reference_tracked_and_interior_mutable_components() {
    let cases = [
        UnsupportedTypeCategory::SafeReference,
        UnsupportedTypeCategory::TrackedFixture,
        UnsupportedTypeCategory::InteriorMutable,
    ];

    for category in cases {
        let mut types = TypeTable::new();
        let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
        let unsupported = match category {
            UnsupportedTypeCategory::SafeReference => types.push(TypeDef::reference(
                "SharedI64",
                i64_ty,
                ReferencePermission::Shared,
            )),
            UnsupportedTypeCategory::TrackedFixture => {
                types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture))
            }
            UnsupportedTypeCategory::InteriorMutable => types
                .push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability()),
            _ => unreachable!(),
        };
        let inner = callable_type(&mut types, "Inner", vec![unsupported], None);
        let outer = callable_type(&mut types, "Outer", vec![inner], None);

        assert_eq!(
            local_callable_coverage_error(types, outer),
            RealizationError::Coverage(runen_core_wasm::CoverageError {
                location: CoverageLocation::Local {
                    function: FunctionId(0),
                    local: LocalId(0),
                },
                kind: CoverageErrorKind::UnsupportedCallableParameterType {
                    callable: inner,
                    parameter: 0,
                    ty: unsupported,
                    category,
                },
            })
        );
    }
}

#[test]
fn nested_callable_safe_reference_result_contract_remains_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let inner = types.push(TypeDef::callable(
        "Identity",
        CallableInterface::new(
            vec![shared],
            Some(shared),
            SafeReferenceResultContract::SharedIdentity { origin: 0 },
        ),
    ));
    let outer = callable_type(&mut types, "Outer", vec![inner], None);

    assert_eq!(
        local_callable_coverage_error(types, outer),
        RealizationError::Coverage(runen_core_wasm::CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedSafeReferenceResultContract,
        })
    );
}

#[test]
fn callable_hidden_inside_structural_component_remains_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let inner = callable_type(&mut types, "Inner", vec![i64_ty], Some(i64_ty));
    let wrapper = types.push(TypeDef::structure(
        "Wrapper",
        vec![Field::new("callable", inner)],
    ));
    let outer = callable_type(&mut types, "Outer", vec![wrapper], None);

    assert_eq!(
        local_callable_coverage_error(types, outer),
        RealizationError::Coverage(runen_core_wasm::CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedCallableParameterType {
                callable: outer,
                parameter: 0,
                ty: inner,
                category: UnsupportedTypeCategory::Callable,
            },
        })
    );
}
