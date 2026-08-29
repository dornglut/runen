use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ReferenceAccess, ReferencePermission, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn shared_reference_parameter_reads_suspended_caller_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
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
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::ReferenceRead {
                    src: ReferenceAccess::new(Place::local(LocalId(0))),
                }],
                Terminator::Return(None),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("a transferred Shared carrier may read its fully-Live external referent");
}

#[test]
fn exclusive_replace_parameter_may_move_then_restore_before_return() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "ExclusiveReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("reference", reference_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(9)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: Place::local(LocalId(0)),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let callee = Function {
        name: "round_trip".into(),
        parameters: vec![LocalId(0)],
        result: None,
        body: body(
            vec![
                LocalDecl::new("reference", reference_ty, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(0)))),
                    },
                    Statement::ReferenceAssign {
                        dst: ReferenceAccess::new(Place::local(LocalId(0))),
                        src: Operand::Move(Place::local(LocalId(1)).into()),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("normal Return is valid after an ExclusiveReplace referent is restored fully Live");
}

#[test]
fn reference_result_type_remains_outside_the_transfer_boundary() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let function = Function {
        name: "invalid_reference_result".into(),
        parameters: Vec::new(),
        result: Some(shared_i64),
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    assert!(
        validate_program(Program {
            types,
            functions: vec![function],
        })
        .is_err(),
        "safe-reference-containing results require a later accepted origin contract"
    );
}

#[test]
fn reference_parameter_referent_may_not_contain_raw_or_nested_reference_leaves() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let shared_raw = types.push(TypeDef::reference(
        "SharedRawI64",
        raw_i64,
        ReferencePermission::Shared,
    ));
    let shared_nested = types.push(TypeDef::reference(
        "SharedNestedI64",
        shared_i64,
        ReferencePermission::Shared,
    ));

    for invalid_parameter_type in [shared_raw, shared_nested] {
        let function = Function {
            name: "invalid_reference_parameter".into(),
            parameters: vec![LocalId(0)],
            result: None,
            body: body(
                vec![LocalDecl::new("parameter", invalid_parameter_type, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        };

        assert!(
            validate_program(Program {
                types: types.clone(),
                functions: vec![function],
            })
            .is_err(),
            "transferred reference referents must be raw-free and reference-free"
        );
    }
}
