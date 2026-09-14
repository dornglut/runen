use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LoanDecl, LocalDecl, LocalId,
    Operand, PersistentDecl, PersistentId, Place, Program, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable,
    Value, validate_program,
};
use runen_core_wasm::{
    CoverageError, CoverageErrorKind, CoverageLocation, ExecutionOutcome, RealizationError,
    RealizedProgram, UnsupportedOperandKind, UnsupportedTerminatorKind, UnsupportedTypeCategory,
};

fn body(locals: Vec<LocalDecl>, loans: Vec<LoanDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans,
        entry: BasicBlockId(0),
        blocks,
    }
}

fn function(
    name: &str,
    parameters: Vec<LocalId>,
    result: Option<TypeId>,
    contract: SafeReferenceResultContract,
    body: Body,
) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        safe_reference_result_contract: contract,
        body,
    }
}

fn validated(
    types: TypeTable,
    persistent: Vec<PersistentDecl>,
    external_callables: Vec<ExternalCallableDecl>,
    functions: Vec<Function>,
) -> runen_core_ir::ValidatedProgram {
    validate_program(Program {
        types,
        persistent,
        external_callables,
        functions,
    })
    .expect("coverage diagnostic fixture must be valid Core")
}

fn coverage_error(program: &runen_core_ir::ValidatedProgram) -> CoverageError {
    match RealizedProgram::new(program) {
        Err(RealizationError::Coverage(error)) => error,
        Err(other) => panic!("expected coverage rejection, got realization error: {other:?}"),
        Ok(_) => panic!("expected coverage rejection, but realization was admitted"),
    }
}

#[test]
fn unused_unsupported_type_definitions_do_not_affect_realization_coverage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    types.push(TypeDef::scalar("F32", ScalarType::F32));
    types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    types.push(TypeDef::callable(
        "Callable",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    types.push(TypeDef::structure(
        "Pair",
        vec![runen_core_ir::Field::new("value", i64_ty)],
    ));
    types.push(TypeDef::scalar("Interior", ScalarType::I64).with_interior_mutability());

    let program = validated(
        types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "entry",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                Vec::new(),
                Vec::new(),
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        )],
    );

    let outcome = RealizedProgram::new(&program)
        .expect("unused unsupported type definitions are outside consumed coverage")
        .execute(FunctionId(0))
        .expect("admitted empty program must execute");
    assert_eq!(outcome, ExecutionOutcome::Returned(None));
}

#[test]
fn passive_unsupported_type_roles_have_precise_locations() {
    let mut local_types = TypeTable::new();
    let local_float = local_types.push(TypeDef::scalar("F32", ScalarType::F32));
    let local_program = validated(
        local_types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "local",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                vec![LocalDecl::new("value", local_float, false)],
                Vec::new(),
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        )],
    );
    assert_eq!(
        coverage_error(&local_program),
        CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: local_float,
                category: UnsupportedTypeCategory::Floating,
            },
        }
    );

    let mut parameter_types = TypeTable::new();
    let parameter_float = parameter_types.push(TypeDef::scalar("F32", ScalarType::F32));
    let parameter_program = validated(
        parameter_types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "parameter",
            vec![LocalId(0)],
            None,
            SafeReferenceResultContract::None,
            body(
                vec![LocalDecl::new("value", parameter_float, false)],
                Vec::new(),
                vec![BasicBlock::new(
                    vec![Statement::Drop {
                        place: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                )],
            ),
        )],
    );
    assert_eq!(
        coverage_error(&parameter_program),
        CoverageError {
            location: CoverageLocation::Parameter {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: parameter_float,
                category: UnsupportedTypeCategory::Floating,
            },
        }
    );

    let mut result_types = TypeTable::new();
    let result_float = result_types.push(TypeDef::scalar("F32", ScalarType::F32));
    let result_program = validated(
        result_types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "result",
            vec![LocalId(0)],
            Some(result_float),
            SafeReferenceResultContract::None,
            body(
                vec![LocalDecl::new("value", result_float, false)],
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        )],
    );
    assert_eq!(
        coverage_error(&result_program),
        CoverageError {
            location: CoverageLocation::Result {
                function: FunctionId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: result_float,
                category: UnsupportedTypeCategory::Floating,
            },
        }
    );
}

#[test]
fn callable_result_has_a_precise_result_location() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "Thunk",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));
    let program = validated(
        types,
        Vec::new(),
        Vec::new(),
        vec![
            function(
                "producer",
                Vec::new(),
                Some(callable),
                SafeReferenceResultContract::None,
                body(
                    Vec::new(),
                    Vec::new(),
                    vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::FunctionValue(FunctionId(1)))),
                    )],
                ),
            ),
            function(
                "target",
                Vec::new(),
                Some(i64_ty),
                SafeReferenceResultContract::None,
                body(
                    Vec::new(),
                    Vec::new(),
                    vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Constant(Value::I64(1)))),
                    )],
                ),
            ),
        ],
    );

    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Result {
                function: FunctionId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: callable,
                category: UnsupportedTypeCategory::Callable,
            },
        }
    );
}

#[test]
fn unsupported_persistent_type_has_a_persistent_location() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let program = validated(
        types,
        vec![PersistentDecl::new(
            f32_ty,
            Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive)),
        )],
        Vec::new(),
        vec![function(
            "entry",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                Vec::new(),
                Vec::new(),
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        )],
    );

    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Persistent(PersistentId(0)),
            kind: CoverageErrorKind::UnsupportedType {
                ty: f32_ty,
                category: UnsupportedTypeCategory::Floating,
            },
        }
    );
}

#[test]
fn persistent_shared_root_is_rejected_at_its_consuming_statement() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let program = validated(
        types,
        vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        Vec::new(),
        vec![function(
            "entry",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                vec![LocalDecl::new("reference", shared_i64, false)],
                Vec::new(),
                vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    }],
                    Terminator::Return(None),
                )],
            ),
        )],
    );

    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Statement {
                function: FunctionId(0),
                block: BasicBlockId(0),
                statement: 0,
            },
            kind: CoverageErrorKind::UnsupportedOperand(
                UnsupportedOperandKind::PersistentSharedRoot,
            ),
        }
    );
}

#[test]
fn active_external_call_has_a_terminator_diagnostic() {
    let external_interface =
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None);
    let external_program = validated(
        TypeTable::new(),
        Vec::new(),
        vec![ExternalCallableDecl::new(external_interface)],
        vec![function(
            "external",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                Vec::new(),
                Vec::new(),
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: Vec::new(),
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            ),
        )],
    );
    assert_eq!(
        coverage_error(&external_program),
        CoverageError {
            location: CoverageLocation::Terminator {
                function: FunctionId(0),
                block: BasicBlockId(0),
            },
            kind: CoverageErrorKind::UnsupportedTerminator(UnsupportedTerminatorKind::ExternalCall,),
        }
    );
}

#[test]
fn higher_order_indirect_call_rejects_at_the_consuming_terminator() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let inner = types.push(TypeDef::callable(
        "Inner",
        CallableInterface::new(
            vec![i64_ty],
            Some(i64_ty),
            SafeReferenceResultContract::None,
        ),
    ));
    let outer = types.push(TypeDef::callable(
        "Outer",
        CallableInterface::new(vec![inner], Some(i64_ty), SafeReferenceResultContract::None),
    ));
    let program = validated(
        types,
        Vec::new(),
        Vec::new(),
        vec![
            function(
                "entry",
                Vec::new(),
                Some(i64_ty),
                SafeReferenceResultContract::None,
                body(
                    vec![
                        LocalDecl::new("callee", outer, false),
                        LocalDecl::new("argument", inner, false),
                        LocalDecl::new("result", i64_ty, false),
                    ],
                    Vec::new(),
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
                                callable: outer,
                                callee: Operand::Move(Place::local(LocalId(0)).into()),
                                arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                                destination: Some(Place::local(LocalId(2))),
                                target: BasicBlockId(1),
                            },
                        ),
                        BasicBlock::new(
                            Vec::new(),
                            Terminator::Return(Some(Operand::Move(
                                Place::local(LocalId(2)).into(),
                            ))),
                        ),
                    ],
                ),
            ),
            function(
                "outer_target",
                vec![LocalId(0)],
                Some(i64_ty),
                SafeReferenceResultContract::None,
                body(
                    vec![LocalDecl::new("argument", inner, false)],
                    Vec::new(),
                    vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
                    )],
                ),
            ),
            function(
                "inner_target",
                vec![LocalId(0)],
                Some(i64_ty),
                SafeReferenceResultContract::None,
                body(
                    vec![LocalDecl::new("argument", i64_ty, false)],
                    Vec::new(),
                    vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                    )],
                ),
            ),
        ],
    );

    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Terminator {
                function: FunctionId(0),
                block: BasicBlockId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: outer,
                category: UnsupportedTypeCategory::Callable,
            },
        }
    );
}

#[test]
fn projected_access_does_not_hide_unsupported_nested_parameter_type() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![
            runen_core_ir::Field::new("supported", i64_ty),
            runen_core_ir::Field::new("unsupported", f32_ty),
        ],
    ));
    let program = validated(
        types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "projected",
            vec![LocalId(0)],
            None,
            SafeReferenceResultContract::None,
            body(
                vec![LocalDecl::new("pair", pair_ty, false)],
                Vec::new(),
                vec![BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).field(0).into(),
                    }],
                    Terminator::Return(None),
                )],
            ),
        )],
    );

    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Parameter {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: f32_ty,
                category: UnsupportedTypeCategory::Floating,
            },
        }
    );
}

#[test]
fn passive_external_and_loan_facilities_keep_program_or_function_categories() {
    let external_program = validated(
        TypeTable::new(),
        Vec::new(),
        vec![ExternalCallableDecl::new(CallableInterface::new(
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
        ))],
        vec![function(
            "entry",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                Vec::new(),
                Vec::new(),
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        )],
    );
    assert_eq!(
        coverage_error(&external_program),
        CoverageError {
            location: CoverageLocation::Program,
            kind: CoverageErrorKind::ExternalCallables,
        }
    );

    let mut loan_types = TypeTable::new();
    let loan_ty = loan_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let loan_program = validated(
        loan_types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "entry",
            Vec::new(),
            None,
            SafeReferenceResultContract::None,
            body(
                Vec::new(),
                vec![LoanDecl::new("loan", loan_ty)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        )],
    );
    assert_eq!(
        coverage_error(&loan_program),
        CoverageError {
            location: CoverageLocation::Function(FunctionId(0)),
            kind: CoverageErrorKind::LoanDeclarations,
        }
    );
}

#[test]
fn safe_reference_result_contract_has_a_function_level_category() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let program = validated(
        types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "identity",
            vec![LocalId(0)],
            Some(shared_i64),
            SafeReferenceResultContract::SharedIdentity { origin: 0 },
            body(
                vec![LocalDecl::new("reference", shared_i64, false)],
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        )],
    );

    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Function(FunctionId(0)),
            kind: CoverageErrorKind::UnsupportedSafeReferenceResultContract,
        }
    );
}
