use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, ExternalCallableDecl, ExternalCallableId,
    Fault, Field, Function, FunctionId, LocalDecl, LocalId, Operand, Place, Program,
    ReferenceAccess, ReferencePermission, SafeReferenceResultContract, ScalarType, Statement,
    Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    CoverageError, CoverageErrorKind, CoverageLocation, ExecutionOutcome, RealizationError,
    RealizedProgram, UnsupportedStatementKind, UnsupportedTypeCategory,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn function(
    name: &str,
    parameters: Vec<LocalId>,
    result: Option<TypeId>,
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

fn validate(
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
    .expect("reference realization fixture must be valid Core")
}

fn reference_outcome(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(validated, entry)
        .expect("reference differential entry must be admitted")
        .execute()
        .expect("accepted reference fixture must execute in reference semantics");
    match report.terminal {
        TerminalStatus::Returned => ExecutionOutcome::Returned(match report.result {
            None => None,
            Some(ObservedValue::I64(value)) => Some(Value::I64(value)),
            other => panic!("unexpected reference differential result: {other:?}"),
        }),
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(validated.clone(), entry);
    let actual = RealizedProgram::new(&validated)
        .expect("reference fixture must be inside bounded Core-Wasm coverage")
        .execute(entry)
        .expect("accepted reference fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn shared_scalar_root_copy_and_dereference_match_reference_semantics() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let original = Place::local(LocalId(1));
    let copy = Place::local(LocalId(2));
    let result = Place::local(LocalId(3));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("original", shared_i64, false),
                LocalDecl::new("copy", shared_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(41)),
                    },
                    Statement::Init {
                        dst: original.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target,
                        },
                    },
                    Statement::Init {
                        dst: copy.clone(),
                        src: Operand::Copy(original.clone().into()),
                    },
                    Statement::Drop {
                        place: original.into(),
                    },
                    Statement::Init {
                        dst: result.clone(),
                        src: Operand::ReferenceCopy(ReferenceAccess::new(copy.clone())),
                    },
                    Statement::Drop { place: copy.into() },
                ],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, Vec::new(), vec![entry]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(41)))
    );
}

#[test]
fn dynamic_reference_handle_flow_selects_each_runtime_target() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("left_result", i64_ty, false),
                LocalDecl::new("right_result", i64_ty, false),
                LocalDecl::new("sum", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(Value::Bool(true))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(Value::Bool(false))],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::IntegerAdd {
                        dst: Place::local(LocalId(2)),
                        left: Operand::Move(Place::local(LocalId(0)).into()),
                        right: Operand::Move(Place::local(LocalId(1)).into()),
                    }],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
                ),
            ],
        },
    );

    let helper = function(
        "select",
        vec![LocalId(0)],
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("condition", bool_ty, false),
                LocalDecl::new("left", i64_ty, false),
                LocalDecl::new("right", i64_ty, false),
                LocalDecl::new("selected", shared_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::Constant(Value::I64(11)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(2)),
                            src: Operand::Constant(Value::I64(22)),
                        },
                    ],
                    Terminator::Branch {
                        condition: Operand::Copy(Place::local(LocalId(0)).into()),
                        true_target: BasicBlockId(1),
                        false_target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(1)),
                        },
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(2)),
                        },
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(4)),
                            src: Operand::ReferenceCopy(ReferenceAccess::new(Place::local(
                                LocalId(3),
                            ))),
                        },
                        Statement::Drop {
                            place: Place::local(LocalId(3)).into(),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(4)).into()))),
                ),
            ],
        },
    );

    assert_eq!(
        assert_differential(
            validate(types, Vec::new(), vec![entry, helper]),
            FunctionId(0),
        ),
        ExecutionOutcome::Returned(Some(Value::I64(33)))
    );
}

#[test]
fn nested_reference_fault_cleanup_matches_reference_semantics() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let fault = Fault::new("nested-reference-fault");
    let entry = function(
        "entry",
        Vec::new(),
        None,
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("parent", shared_i64, false),
                LocalDecl::new("child", shared_i64, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(7)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(1))),
                        },
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(1)).into(),
                    },
                ],
                Terminator::Fault(fault.clone()),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, Vec::new(), vec![entry]), FunctionId(0)),
        ExecutionOutcome::Faulted(fault)
    );
}

#[test]
fn reference_containing_aggregate_remains_a_coverage_rejection() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let holder_ty = types.push(TypeDef::structure(
        "Holder",
        vec![Field::new("reference", shared_i64)],
    ));
    let entry = function(
        "entry",
        Vec::new(),
        None,
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("holder", holder_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(5)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)).field(0),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(0)),
                        },
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );
    let validated = validate(types, Vec::new(), vec![entry]);

    assert_eq!(
        RealizedProgram::new(&validated).err(),
        Some(RealizationError::Coverage(CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(1),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: shared_i64,
                category: UnsupportedTypeCategory::SafeReference,
            },
        }))
    );
}

#[test]
fn external_reference_transfer_remains_a_coverage_rejection() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let external = ExternalCallableDecl::new(CallableInterface::new(
        vec![shared_i64],
        None,
        SafeReferenceResultContract::None,
    ));
    let entry = function(
        "entry",
        Vec::new(),
        None,
        Body {
            locals: Vec::new(),
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        },
    );
    let validated = validate(types, vec![external], vec![entry]);

    assert_eq!(
        RealizedProgram::new(&validated).err(),
        Some(RealizationError::Coverage(CoverageError {
            location: CoverageLocation::ExternalCallable(ExternalCallableId(0)),
            kind: CoverageErrorKind::UnsupportedExternalParameterType {
                external: ExternalCallableId(0),
                parameter: 0,
                ty: shared_i64,
                category: UnsupportedTypeCategory::SafeReference,
            },
        }))
    );
}

#[test]
fn reference_interior_assignment_remains_a_statement_coverage_rejection() {
    let mut types = TypeTable::new();
    let interior_ty =
        types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    let shared_interior = types.push(TypeDef::reference(
        "SharedInteriorI64",
        interior_ty,
        ReferencePermission::Shared,
    ));
    let entry = function(
        "entry",
        Vec::new(),
        None,
        Body {
            locals: vec![
                LocalDecl::new("target", interior_ty, false),
                LocalDecl::new("reference", shared_interior, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::ReferenceInteriorAssign {
                        dst: ReferenceAccess::new(Place::local(LocalId(1))),
                        src: Operand::Constant(Value::I64(2)),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );
    let validated = validate(types, Vec::new(), vec![entry]);

    assert_eq!(
        RealizedProgram::new(&validated).err(),
        Some(RealizationError::Coverage(CoverageError {
            location: CoverageLocation::Statement {
                function: FunctionId(0),
                block: BasicBlockId(0),
                statement: 2,
            },
            kind: CoverageErrorKind::UnsupportedStatement(
                UnsupportedStatementKind::InteriorMutation,
            ),
        }))
    );
}
