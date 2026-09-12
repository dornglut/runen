use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, LocalDecl, LocalId, MirLocation,
    MirValidationErrorKind, Operand, PersistentDecl, PersistentId, Place, Program,
    ReferencePermission, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeId, TypeTable, Value, validate_program,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn no_result_function() -> Function {
    Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    }
}

#[test]
fn persistent_declarations_keep_program_identity_separate_from_initial_value() {
    let first = PersistentId(0);
    let second = PersistentId(1);
    assert_ne!(first, second);

    let declaration = PersistentDecl::new(TypeId(7), Value::I64(42));
    assert_eq!(declaration.ty, TypeId(7));
    assert_eq!(declaration.initial, Value::I64(42));
}

#[test]
fn persistent_read_is_typed_and_non_consuming() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("sum", i64_ty, false)],
            vec![BasicBlock::new(
                vec![Statement::IntegerAdd {
                    dst: Place::local(LocalId(0)),
                    left: Operand::PersistentRead(PersistentId(0)),
                    right: Operand::PersistentRead(PersistentId(0)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(21))],
        functions: vec![function],
    })
    .expect("repeated persistent scalar reads are valid and non-consuming");
}

#[test]
fn persistent_declaration_rejects_non_language_scalar_and_interior_mutability() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(tracked, Value::TrackedFixture(1))],
        functions: Vec::new(),
    })
    .expect_err("verification-only scalar is outside persistent storage domain");
    assert_eq!(error.location, MirLocation::Program);
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidPersistentType {
            persistent: PersistentId(0),
            ty: tracked,
        }
    );

    let mut types = TypeTable::new();
    let interior = types.push(TypeDef::scalar("I64", ScalarType::I64).with_interior_mutability());
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(interior, Value::I64(1))],
        functions: Vec::new(),
    })
    .expect_err("persistent Shared roots must not expose interior mutation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidPersistentType {
            persistent: PersistentId(0),
            ty: interior,
        }
    );
}

#[test]
fn persistent_initializer_must_match_exact_declared_type() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::Bool(true))],
        functions: Vec::new(),
    })
    .expect_err("persistent initializer must match its exact declaration type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::PersistentInitializerTypeMismatch {
            persistent: PersistentId(0),
            ty: i64_ty,
        }
    );
}

#[test]
fn persistent_root_is_shared_only_and_can_have_multiple_carriers() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("left", shared_i64, false),
                LocalDecl::new("right", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(1)).into(),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(0)).into(),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        functions: vec![function],
    })
    .expect("multiple Shared persistent roots are compatible");

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let function = Function {
        name: "invalid".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", replace_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::PersistentSharedRoot(PersistentId(0)),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        functions: vec![function],
    })
    .expect_err("persistent root has no replacement-capable representation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch {
            expected: replace_i64
        }
    );
}

#[test]
fn fresh_persistent_root_cannot_satisfy_shared_identity_result_contract() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let function = Function {
        name: "wrong_origin".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![LocalDecl::new("origin", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::PersistentSharedRoot(PersistentId(0)))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        functions: vec![function],
    })
    .expect_err("fresh persistent root is not the parameter-origin Shared identity");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::SharedIdentityResultMismatch
    );
}

#[test]
fn missing_persistent_identity_is_rejected_at_operand_site() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::PersistentRead(PersistentId(3)))),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        persistent: Vec::new(),
        functions: vec![function],
    })
    .expect_err("persistent operands must name an existing declaration");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidPersistent(PersistentId(3))
    );
}

#[test]
fn empty_persistent_sequence_preserves_existing_program_shape() {
    let mut types = TypeTable::new();
    let _ = types.push(TypeDef::scalar("I64", ScalarType::I64));
    validate_program(Program {
        types,
        persistent: Vec::new(),
        functions: vec![no_result_function()],
    })
    .expect("existing programs remain valid with an explicit empty persistent sequence");
}
