use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, BorrowKind,
    CallableInterface, ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LoanDecl,
    LoanId, LocalDecl, LocalId, NumericContract, Operand, Place, ReferenceAccess,
    ReferencePermission, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderAdmissionError, RealizationError, RealizedProgram,
};

fn function(
    name: &str,
    parameters: Vec<LocalId>,
    result: Option<runen_core_ir::TypeId>,
    body: Body,
) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

fn validate(types: TypeTable, functions: Vec<Function>) -> runen_core_ir::ValidatedProgram {
    validate_program(runen_core_ir::Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions,
    })
    .expect("coverage fixture must be valid Core")
}

fn assert_coverage_rejected(validated: runen_core_ir::ValidatedProgram) {
    assert!(matches!(
        RealizedProgram::new(&validated),
        Err(RealizationError::Coverage(_))
    ));
}

#[test]
fn admits_f32_basic_arithmetic_for_internal_transport() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let body = Body {
        locals: vec![LocalDecl::new("result", f32_ty, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![Statement::FloatAdd {
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(zero.clone()),
                right: Operand::Constant(zero),
                contract: NumericContract::Standard,
            }],
            Terminator::Return(None),
        )],
    };
    let validated = validate(types, vec![function("entry", Vec::new(), None, body)]);
    assert_eq!(
        RealizedProgram::new(&validated)
            .expect("direct F32 arithmetic must realize")
            .execute(FunctionId(0))
            .expect("direct F32 arithmetic fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn admits_f16_basic_arithmetic_for_internal_transport() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let zero = Value::F16(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let body = Body {
        locals: vec![LocalDecl::new("result", f16_ty, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![Statement::FloatAdd {
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(zero.clone()),
                right: Operand::Constant(zero),
                contract: NumericContract::Standard,
            }],
            Terminator::Return(None),
        )],
    };
    let validated = validate(types, vec![function("entry", Vec::new(), None, body)]);
    assert_eq!(
        RealizedProgram::new(&validated)
            .expect("direct F16 arithmetic must realize")
            .execute(FunctionId(0))
            .expect("direct F16 arithmetic fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn admits_reference_free_structural_types_and_projected_access() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let record_ty = types.push(TypeDef::structure(
        "Pair",
        vec![runen_core_ir::Field::new("value", i64_ty)],
    ));
    let body = Body {
        locals: vec![LocalDecl::new("pair", record_ty, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::Constant(Value::Struct(vec![Value::I64(1)])),
                },
                Statement::Read {
                    src: Place::local(LocalId(0)).field(0).into(),
                },
            ],
            Terminator::Return(None),
        )],
    };
    let validated = validate(types, vec![function("entry", Vec::new(), None, body)]);
    assert_eq!(
        RealizedProgram::new(&validated)
            .expect("reference-free structural fixture must realize")
            .execute(FunctionId(0))
            .expect("reference-free structural fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn admits_activation_local_safe_reference_types_and_operations() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let source = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let body = Body {
        locals: vec![
            LocalDecl::new("source", i64_ty, false),
            LocalDecl::new("reference", reference_ty, false),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: source.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Init {
                    dst: reference.clone(),
                    src: Operand::ReferenceRoot {
                        permission: ReferencePermission::Shared,
                        place: source,
                    },
                },
                Statement::ReferenceRead {
                    src: ReferenceAccess::new(reference),
                },
            ],
            Terminator::Return(None),
        )],
    };
    let validated = validate(types, vec![function("entry", Vec::new(), None, body)]);
    assert_eq!(
        RealizedProgram::new(&validated)
            .expect("activation-local Shared reference fixture must realize")
            .execute(FunctionId(0))
            .expect("activation-local Shared reference fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn admits_bounded_activation_local_raw_pointer_read() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let source = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let body = Body {
        locals: vec![
            LocalDecl::new("source", i64_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: source.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Init {
                    dst: pointer.clone(),
                    src: Operand::AddressOf(source.into()),
                },
                Statement::RawRead {
                    pointer: pointer.into(),
                },
            ],
            Terminator::Return(None),
        )],
    };
    let validated = validate(types, vec![function("entry", Vec::new(), None, body)]);
    assert_eq!(
        RealizedProgram::new(&validated)
            .expect("bounded activation-local raw pointer fixture must realize")
            .execute(FunctionId(0))
            .expect("bounded activation-local raw pointer fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn admits_first_order_callable_values_and_indirect_calls() {
    let mut types = TypeTable::new();
    let interface = CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None);
    let callable_ty = types.push(TypeDef::callable("Callable", interface));
    let entry = function(
        "entry",
        Vec::new(),
        None,
        Body {
            locals: vec![LocalDecl::new("callee", callable_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::FunctionValue(FunctionId(1)),
                    }],
                    Terminator::IndirectCall {
                        callable: callable_ty,
                        callee: Operand::Move(Place::local(LocalId(0)).into()),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        },
    );
    let target = function(
        "target",
        Vec::new(),
        None,
        Body {
            locals: Vec::new(),
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        },
    );
    let validated = validate(types, vec![entry, target]);
    assert_eq!(
        RealizedProgram::new(&validated)
            .expect("first-order callable fixture must realize")
            .execute(FunctionId(0))
            .expect("first-order callable fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn rejects_tracked_fixture_and_interior_mutability() {
    let mut tracked_types = TypeTable::new();
    let tracked = tracked_types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let tracked_body = Body {
        locals: vec![LocalDecl::new("tracked", tracked, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    };
    assert_coverage_rejected(validate(
        tracked_types,
        vec![function("entry", Vec::new(), None, tracked_body)],
    ));

    let mut interior_types = TypeTable::new();
    let interior = interior_types
        .push(TypeDef::scalar("Interior", ScalarType::I64).with_interior_mutability());
    let interior_body = Body {
        locals: vec![LocalDecl::new("value", interior, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    };
    assert_coverage_rejected(validate(
        interior_types,
        vec![function("entry", Vec::new(), None, interior_body)],
    ));
}

#[test]
fn supported_external_requires_provider_but_loan_facilities_remain_coverage_rejected() {
    let external_interface =
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None);
    let external_program = validate_program(runen_core_ir::Program {
        types: TypeTable::new(),
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(external_interface)],
        functions: vec![function(
            "entry",
            Vec::new(),
            None,
            Body {
                locals: Vec::new(),
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
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
            },
        )],
    })
    .expect("external-call fixture must be valid Core");
    assert_eq!(
        RealizedProgram::new(&external_program).err(),
        Some(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(0))
        ))
    );

    let mut loan_types = TypeTable::new();
    let loan_ty = loan_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let source = Place::local(LocalId(0));
    let loan_program = validate_program(runen_core_ir::Program {
        types: loan_types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![function(
            "entry",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("source", loan_ty, false)],
                loans: vec![LoanDecl::new("loan", loan_ty)],
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: source.clone(),
                            src: Operand::Constant(Value::I64(1)),
                        },
                        Statement::Borrow {
                            loan: LoanId(0),
                            kind: BorrowKind::Shared,
                            src: source.into(),
                        },
                        Statement::EndBorrow { loan: LoanId(0) },
                    ],
                    Terminator::Return(None),
                )],
            },
        )],
    })
    .expect("borrow fixture must be valid Core");
    assert_coverage_rejected(loan_program);
}

#[test]
fn explicit_entry_boundary_rejects_parameters_and_unknown_function_ids() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let validated = validate(
        types,
        vec![function(
            "with_parameter",
            vec![LocalId(0)],
            Some(i64_ty),
            Body {
                locals: vec![LocalDecl::new("value", i64_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            },
        )],
    );
    let realized = RealizedProgram::new(&validated).expect("supported parameter type may realize");
    assert_eq!(
        realized.execute(FunctionId(0)),
        Err(RealizationError::EntryHasParameters(FunctionId(0)))
    );
    assert_eq!(
        realized.execute(FunctionId(99)),
        Err(RealizationError::InvalidEntry(FunctionId(99)))
    );
}
