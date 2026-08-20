use runen_core_ir::interprocedural::{BasicBlock, Body, Function, Program, Terminator};
use runen_core_ir::interprocedural_validation::{
    MirLocation, MirValidationErrorKind, validate_program,
};
use runen_core_ir::{
    BasicBlockId, FunctionId, LocalDecl, LocalId, Operand, Place, ScalarType, Statement, TypeDef,
    TypeTable, Value,
};

fn scalar_program_types() -> (TypeTable, runen_core_ir::TypeId) {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    (types, i64_ty)
}

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn validates_no_result_call_and_parameter_is_live_on_entry() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("arg", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(7)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "callee".into(),
        parameters: vec![LocalId(0)],
        result: None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(
                vec![Statement::Read {
                    src: Place::local(LocalId(0)).into(),
                }],
                Terminator::Return(None),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("valid parameter transfer and no-result call");
}

#[test]
fn result_call_initializes_destination_on_normal_successor() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("arg", i64_ty, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(9)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(1)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(i64_ty),
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("normal result transfer initializes destination");
}

#[test]
fn call_arguments_apply_move_effects_left_to_right() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![
                            Operand::Move(Place::local(LocalId(0)).into()),
                            Operand::Move(Place::local(LocalId(0)).into()),
                        ],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "take_two".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: None,
        body: body(
            vec![
                LocalDecl::new("left", i64_ty, false),
                LocalDecl::new("right", i64_ty, false),
            ],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("second move observes first argument consumption");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place)
            if *place == Place::local(LocalId(0))
    ));
}

#[test]
fn result_destination_must_still_be_never_initialized() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(5)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "produce".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(8)))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("call result is first initialization, not replacement");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::CallResultRequiresNeverInitialized(ref place)
            if *place == Place::local(LocalId(0))
    ));
}

#[test]
fn raw_pointer_containing_signatures_are_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("Ptr", i64_ty));
    let function = Function {
        name: "pointer_parameter".into(),
        parameters: vec![LocalId(0)],
        result: None,
        body: body(
            vec![LocalDecl::new("pointer", pointer_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("raw-pointer values do not cross activations in this slice");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallTransferUnsafe(pointer_ty)
    );
    assert_eq!(error.location, MirLocation::Function(FunctionId(0)));
}

#[test]
fn direct_and_mutual_recursive_call_graphs_are_valid() {
    let types = TypeTable::new();
    let left = Function {
        name: "left".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let right = Function {
        name: "right".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(0),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![left, right],
    })
    .expect("call-graph cycles are not a validation error");
}

#[test]
fn result_return_requires_an_owned_value() {
    let (types, i64_ty) = scalar_program_types();
    let function = Function {
        name: "bad_return".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("result-bearing function must return a value");
    assert_eq!(error.kind, MirValidationErrorKind::MissingReturnValue);
}

#[test]
fn parameter_local_designations_are_unique() {
    let (types, i64_ty) = scalar_program_types();
    let function = Function {
        name: "duplicate_parameter".into(),
        parameters: vec![LocalId(0), LocalId(0)],
        result: None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("one local cannot realize two parameter slots");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DuplicateParameter(LocalId(0))
    );
}

#[test]
fn body_points_include_function_identity() {
    let types = TypeTable::new();
    let first = Function {
        name: "first".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };
    let second = Function {
        name: "second".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Goto(BasicBlockId(9)),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![first, second],
    })
    .expect_err("invalid block in second function");
    assert!(matches!(
        error.location,
        MirLocation::Point(ref point)
            if point.function == FunctionId(1) && point.block == BasicBlockId(0)
    ));
}

#[test]
fn ordinary_source_scalar_tags_are_copyable_without_value_carriers() {
    let tags = [
        ScalarType::Bool,
        ScalarType::I8,
        ScalarType::I16,
        ScalarType::I32,
        ScalarType::I64,
        ScalarType::U8,
        ScalarType::U16,
        ScalarType::U32,
        ScalarType::U64,
        ScalarType::F16,
        ScalarType::F32,
        ScalarType::F64,
    ];
    let mut types = TypeTable::new();
    for (index, tag) in tags.into_iter().enumerate() {
        let ty = types.push(TypeDef::scalar(format!("scalar-{index}"), tag));
        assert!(types.is_copy(ty));
        assert!(types.is_call_transfer_safe(ty));
    }
}
