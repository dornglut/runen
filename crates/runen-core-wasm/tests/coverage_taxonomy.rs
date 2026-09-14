use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, BorrowKind,
    CallableInterface, Field, Function, FunctionId, LoanDecl, LoanId, LocalDecl, LocalId,
    NumericContract, Operand, PersistentDecl, PersistentId, Place, Program, ReferenceAccess,
    ReferencePermission, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    CoverageError, CoverageErrorKind, CoverageLocation, RealizationError, RealizedProgram,
    UnsupportedOperandKind, UnsupportedStatementKind, UnsupportedTypeCategory,
};

fn function(name: &str, parameters: Vec<LocalId>, result: Option<TypeId>, body: Body) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

fn validated(
    types: TypeTable,
    persistent: Vec<PersistentDecl>,
    functions: Vec<Function>,
) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent,
        external_callables: Vec::new(),
        functions,
    })
    .expect("coverage taxonomy fixture must be valid Core")
}

fn coverage_error(program: &ValidatedProgram) -> CoverageError {
    match RealizedProgram::new(program) {
        Err(RealizationError::Coverage(error)) => error,
        Err(other) => panic!("expected coverage rejection, got realization error: {other:?}"),
        Ok(_) => panic!("expected coverage rejection, but realization was admitted"),
    }
}

fn empty_body(locals: Vec<LocalDecl>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    }
}

fn assert_local_type_category(types: TypeTable, ty: TypeId, category: UnsupportedTypeCategory) {
    let program = validated(
        types,
        Vec::new(),
        vec![function(
            "entry",
            Vec::new(),
            None,
            empty_body(vec![LocalDecl::new("value", ty, false)]),
        )],
    );
    assert_eq!(
        coverage_error(&program),
        CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedType { ty, category },
        }
    );
}

#[test]
fn passive_local_types_report_every_excluded_type_family() {
    let mut floating = TypeTable::new();
    let floating_ty = floating.push(TypeDef::scalar("F32", ScalarType::F32));
    assert_local_type_category(floating, floating_ty, UnsupportedTypeCategory::Floating);

    let mut raw = TypeTable::new();
    let raw_pointee = raw.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_ty = raw.push(TypeDef::raw_pointer("RawI64", raw_pointee));
    assert_local_type_category(raw, raw_ty, UnsupportedTypeCategory::RawPointer);

    let mut reference = TypeTable::new();
    let referent = reference.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = reference.push(TypeDef::reference(
        "SharedI64",
        referent,
        ReferencePermission::Shared,
    ));
    assert_local_type_category(
        reference,
        reference_ty,
        UnsupportedTypeCategory::SafeReference,
    );

    let mut callable = TypeTable::new();
    let callable_ty = callable.push(TypeDef::callable(
        "Callable",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    assert_local_type_category(callable, callable_ty, UnsupportedTypeCategory::Callable);

    let mut tracked = TypeTable::new();
    let tracked_ty = tracked.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    assert_local_type_category(tracked, tracked_ty, UnsupportedTypeCategory::TrackedFixture);

    let mut structural = TypeTable::new();
    let field_ty = structural.push(TypeDef::scalar("I64", ScalarType::I64));
    let structural_ty = structural.push(TypeDef::structure(
        "Pair",
        vec![Field::new("value", field_ty)],
    ));
    assert_local_type_category(
        structural,
        structural_ty,
        UnsupportedTypeCategory::StructuralAggregate,
    );

    let mut interior = TypeTable::new();
    let interior_ty =
        interior.push(TypeDef::scalar("Interior", ScalarType::I64).with_interior_mutability());
    assert_local_type_category(
        interior,
        interior_ty,
        UnsupportedTypeCategory::InteriorMutable,
    );
}

#[test]
fn valid_core_can_reach_every_excluded_statement_family() {
    let mut floating_types = TypeTable::new();
    let f32_ty = floating_types.push(TypeDef::scalar("F32", ScalarType::F32));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let floating_program = validated(
        floating_types,
        Vec::new(),
        vec![function(
            "floating",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("value", f32_ty, false)],
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
            },
        )],
    );
    assert_eq!(
        coverage_error(&floating_program).kind,
        CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::Floating)
    );

    let mut borrowing_types = TypeTable::new();
    let i64_ty = borrowing_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let source = Place::local(LocalId(0));
    let borrowing_program = validated(
        borrowing_types,
        Vec::new(),
        vec![function(
            "borrowing",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("source", i64_ty, false)],
                loans: vec![LoanDecl::new("loan", i64_ty)],
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
    );
    assert_eq!(
        coverage_error(&borrowing_program).kind,
        CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::Borrowing)
    );

    let mut reference_types = TypeTable::new();
    let referent = reference_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = reference_types.push(TypeDef::reference(
        "SharedI64",
        referent,
        ReferencePermission::Shared,
    ));
    let reference_program = validated(
        reference_types,
        Vec::new(),
        vec![function(
            "reference",
            vec![LocalId(0)],
            None,
            Body {
                locals: vec![LocalDecl::new("reference", shared, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::ReferenceRead {
                        src: ReferenceAccess::new(Place::local(LocalId(0))),
                    }],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&reference_program).kind,
        CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::Reference)
    );

    let mut raw_types = TypeTable::new();
    let pointee = raw_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = raw_types.push(TypeDef::raw_pointer("RawI64", pointee));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let raw_program = validated(
        raw_types,
        Vec::new(),
        vec![function(
            "raw",
            Vec::new(),
            None,
            Body {
                locals: vec![
                    LocalDecl::new("target", pointee, false),
                    LocalDecl::new("pointer", pointer_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(1),
                blocks: vec![
                    BasicBlock::new(
                        vec![Statement::RawRead {
                            pointer: pointer.clone().into(),
                        }],
                        Terminator::Return(None),
                    ),
                    BasicBlock::new(
                        vec![
                            Statement::Init {
                                dst: target.clone(),
                                src: Operand::Constant(Value::I64(1)),
                            },
                            Statement::Init {
                                dst: pointer,
                                src: Operand::AddressOf(target.into()),
                            },
                        ],
                        Terminator::Goto(BasicBlockId(0)),
                    ),
                ],
            },
        )],
    );
    assert_eq!(
        coverage_error(&raw_program).kind,
        CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::RawPointer)
    );

    let mut interior_types = TypeTable::new();
    let interior_ty = interior_types
        .push(TypeDef::scalar("Interior", ScalarType::I64).with_interior_mutability());
    let interior_program = validated(
        interior_types,
        Vec::new(),
        vec![function(
            "interior",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("value", interior_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(1)),
                        },
                        Statement::InteriorAssign {
                            dst: Place::local(LocalId(0)).into(),
                            src: Operand::Constant(Value::I64(2)),
                        },
                    ],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&interior_program).kind,
        CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::InteriorMutation)
    );
}

#[test]
fn representative_excluded_operand_families_have_stable_categories() {
    let mut floating_types = TypeTable::new();
    let f32_ty = floating_types.push(TypeDef::scalar("F32", ScalarType::F32));
    let floating_program = validated(
        floating_types,
        Vec::new(),
        vec![function(
            "floating_constant",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("value", f32_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::F32(BinaryFloatValue::Zero(
                            BinaryFloatSign::Positive,
                        ))),
                    }],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&floating_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::FloatingConstant)
    );

    let mut tracked_types = TypeTable::new();
    let tracked_ty = tracked_types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let tracked_program = validated(
        tracked_types,
        Vec::new(),
        vec![function(
            "tracked_constant",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("value", tracked_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    }],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&tracked_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::TrackedFixtureConstant)
    );

    let mut structural_types = TypeTable::new();
    let field_ty = structural_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = structural_types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("value", field_ty)],
    ));
    let structural_program = validated(
        structural_types,
        Vec::new(),
        vec![function(
            "structural_constant",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("value", pair_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::Struct(vec![Value::I64(1)])),
                    }],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&structural_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::StructuralConstant)
    );

    let mut persistent_types = TypeTable::new();
    let persistent_ty = persistent_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = persistent_types.push(TypeDef::reference(
        "SharedI64",
        persistent_ty,
        ReferencePermission::Shared,
    ));
    let persistent_program = validated(
        persistent_types,
        vec![PersistentDecl::new(persistent_ty, Value::I64(7))],
        vec![function(
            "persistent_root",
            Vec::new(),
            None,
            Body {
                locals: vec![LocalDecl::new("reference", shared, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    }],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&persistent_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::PersistentSharedRoot)
    );

    let mut callable_types = TypeTable::new();
    let callable_ty = callable_types.push(TypeDef::callable(
        "Callable",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let callable_program = validated(
        callable_types,
        Vec::new(),
        vec![
            function(
                "entry",
                Vec::new(),
                None,
                Body {
                    locals: vec![LocalDecl::new("callee", callable_ty, false)],
                    loans: Vec::new(),
                    entry: BasicBlockId(0),
                    blocks: vec![BasicBlock::new(
                        vec![Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::FunctionValue(FunctionId(1)),
                        }],
                        Terminator::Return(None),
                    )],
                },
            ),
            function("target", Vec::new(), None, empty_body(Vec::new())),
        ],
    );
    assert_eq!(
        coverage_error(&callable_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::FunctionValue)
    );

    let mut raw_types = TypeTable::new();
    let pointee = raw_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = raw_types.push(TypeDef::raw_pointer("RawI64", pointee));
    let raw_program = validated(
        raw_types,
        Vec::new(),
        vec![function(
            "address_of",
            Vec::new(),
            None,
            Body {
                locals: vec![
                    LocalDecl::new("target", pointee, false),
                    LocalDecl::new("pointer", pointer_ty, false),
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
                            src: Operand::AddressOf(Place::local(LocalId(0)).into()),
                        },
                    ],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&raw_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::AddressOf)
    );

    let mut reference_types = TypeTable::new();
    let referent = reference_types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = reference_types.push(TypeDef::reference(
        "SharedI64",
        referent,
        ReferencePermission::Shared,
    ));
    let reference_program = validated(
        reference_types,
        Vec::new(),
        vec![function(
            "reference_root",
            Vec::new(),
            None,
            Body {
                locals: vec![
                    LocalDecl::new("target", referent, false),
                    LocalDecl::new("reference", shared, false),
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
                    ],
                    Terminator::Return(None),
                )],
            },
        )],
    );
    assert_eq!(
        coverage_error(&reference_program).kind,
        CoverageErrorKind::UnsupportedOperand(UnsupportedOperandKind::ReferenceRoot)
    );
}
