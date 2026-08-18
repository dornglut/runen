use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator,
    TypeDef, TypeId, TypeTable, Value, validate_body,
};

fn one_block(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
) -> Body {
    Body {
        types,
        locals,
        loans,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

#[test]
fn raw_move_requires_raw_pointer_operand() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let source = Place::local(LocalId(0));
    let destination = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", value_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: destination,
                src: Operand::RawMove(source.into()),
            },
        ],
    );

    let error = validate_body(body).expect_err("RawMove requires a stored raw-pointer value");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::RawMoveRequiresPointer(value_ty)
    );
}

#[test]
fn raw_move_result_type_must_match_enclosing_destination() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", bool_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
        ],
    );

    let error = validate_body(body).expect_err("RawMove produces its raw pointer pointee type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: bool_ty }
    );
}

#[test]
fn raw_move_requires_live_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        Vec::new(),
        vec![Statement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::RawMove(pointer.clone().into()),
        }],
    );

    let error = validate_body(body).expect_err("uninitialized pointer value is invalid MIR");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(pointer)
    );
}

#[test]
fn raw_move_rejects_moved_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("moved_pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Move(pointer.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::RawMove(pointer.clone().into()),
            },
        ],
    );

    let error = validate_body(body).expect_err("moved pointer value is invalid MIR");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(pointer)
    );
}

#[test]
fn exclusive_loan_over_pointer_storage_blocks_raw_move_at_validation() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("pointer_exclusive", pointer_ty)],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: pointer.clone().into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(pointer.clone().into()),
            },
        ],
    );

    let error = validate_body(body).expect_err("exclusive pointer-storage loan blocks RawMove");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: pointer,
            loan: LoanId(0),
        }
    );
}

#[test]
fn shared_loan_can_supply_raw_move_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let destination = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        vec![LoanDecl::new("pointer_shared", pointer_ty)],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: pointer.into(),
            },
            Statement::Init {
                dst: destination.clone(),
                src: Operand::RawMove(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Read {
                src: destination.into(),
            },
        ],
    );

    validate_body(body).expect("shared authority may obtain the stored pointer value for RawMove");
}

#[test]
fn defined_raw_move_leaves_exact_target_dead() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
            Statement::Read {
                src: target.clone().into(),
            },
        ],
    );

    let error = validate_body(body).expect_err("defined RawMove must leave its target Dead");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(target)
    );
}

#[test]
fn raw_move_transports_raw_pointer_target_metadata() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer_pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr_ptr", pointer_ty));
    let value = Place::local(LocalId(0));
    let inner = Place::local(LocalId(1));
    let outer = Place::local(LocalId(2));
    let moved = Place::local(LocalId(3));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", value_ty, false),
            LocalDecl::new("inner", pointer_ty, false),
            LocalDecl::new("outer", pointer_pointer_ty, false),
            LocalDecl::new("moved", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: inner.clone(),
                src: Operand::AddressOf(value.clone().into()),
            },
            Statement::Init {
                dst: outer,
                src: Operand::AddressOf(inner.into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::RawMove(Place::local(LocalId(2)).into()),
            },
            Statement::RawAssign {
                pointer: moved.into(),
                src: Operand::Constant(Value::I64(9)),
            },
            Statement::Read { src: value.into() },
        ],
    );

    validate_body(body).expect("RawMove must transport exact pointer target metadata");
}

#[test]
fn self_targeting_raw_move_snapshots_pointer_before_moving_same_storage() {
    let mut types = TypeTable::new();
    let pointer_ty = types.push(TypeDef::raw_pointer("self_ptr", TypeId(0)));
    assert_eq!(pointer_ty, TypeId(0));

    let pointer = Place::local(LocalId(0));
    let moved = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("moved", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(pointer.clone().into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::RawMove(pointer.clone().into()),
            },
            Statement::RawAssign {
                pointer: moved,
                src: Operand::AddressOf(pointer.clone().into()),
            },
            Statement::RawRead {
                pointer: pointer.into(),
            },
        ],
    );

    validate_body(body).expect("RawMove snapshots a self-targeting pointer before moving it");
}

#[test]
fn statically_evident_raw_move_ub_has_no_path_state_continuation() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
            // This read is statically and structurally valid, but there is no defined
            // execution reaching it because RawMove of NeverInitialized target is UB.
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("unsafe target failure is not a MIR-validation diagnostic");
}

#[test]
fn static_validation_still_checks_statements_after_statically_evident_raw_ub() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let bool_ty = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("destination", value_ty, false),
            LocalDecl::new("bool", bool_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::RawMove(Place::local(LocalId(1)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::Constant(Value::I64(7)),
            },
        ],
    );

    let error =
        validate_body(body).expect_err("whole-body static type validation remains mandatory");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: bool_ty }
    );
}
