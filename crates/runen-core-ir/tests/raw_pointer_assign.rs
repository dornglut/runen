use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator,
    TypeDef, TypeTable, Value, validate_body,
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
fn raw_assign_requires_raw_pointer_operand() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", value_ty, false)],
        Vec::new(),
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::RawAssign {
                pointer: value.into(),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    let error = validate_body(body).expect_err("RawAssign requires a raw-pointer value");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::RawAssignRequiresPointer(value_ty)
    );
}

#[test]
fn raw_assign_source_type_must_match_pointee() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::Bool(true)),
            },
        ],
    );

    let error = validate_body(body).expect_err("RawAssign source must match pointee type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: value_ty }
    );
    assert_ne!(value_ty, bool_ty);
}

#[test]
fn raw_assign_requires_live_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pointer", pointer_ty, false)],
        Vec::new(),
        vec![Statement::RawAssign {
            pointer: pointer.clone().into(),
            src: Operand::Constant(Value::I64(1)),
        }],
    );

    let error = validate_body(body).expect_err("uninitialized pointer value is invalid MIR");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(pointer)
    );
}

#[test]
fn raw_assign_rejects_moved_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("moved", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Move(pointer.clone().into()),
            },
            Statement::RawAssign {
                pointer: pointer.clone().into(),
                src: Operand::Constant(Value::I64(1)),
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
fn exclusive_loan_over_pointer_storage_blocks_raw_assign_at_validation() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![LoanDecl::new("pointer_exclusive", pointer_ty)],
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: pointer.clone().into(),
            },
            Statement::RawAssign {
                pointer: pointer.clone().into(),
                src: Operand::Constant(Value::I64(1)),
            },
        ],
    );

    let error = validate_body(body).expect_err("exclusive pointer-storage loan blocks RawAssign");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: pointer,
            loan: LoanId(0),
        }
    );
}

#[test]
fn raw_assign_makes_never_initialized_target_live_for_later_safe_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::I64(7)),
            },
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("defined RawAssign must update exact target path-state");
}

#[test]
fn raw_assign_reinitializes_dead_target_for_later_safe_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("moved", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(target.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(2)).into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("RawAssign must make the exact Dead target Live again");
}

#[test]
fn raw_assign_completes_partially_initialized_aggregate_for_later_safe_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", value_ty), Field::new("right", value_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("pair_ptr", pair_ty));
    let pair = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pair.clone().field(0),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(pair.clone().into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(1)).into(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(2), Value::I64(3)])),
            },
            Statement::Read { src: pair.into() },
        ],
    );

    validate_body(body).expect("RawAssign replacement must leave the complete target Live");
}

#[test]
fn shared_loan_can_supply_raw_assign_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![LoanDecl::new("pointer_shared", pointer_ty)],
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: pointer.into(),
            },
            Statement::RawAssign {
                pointer: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(4)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Read {
                src: Place::local(LocalId(0)).into(),
            },
        ],
    );

    validate_body(body).expect("shared authority may obtain the stored pointer value");
}

#[test]
fn pointer_copy_transports_exact_target_for_raw_assign() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("copy", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Copy(Place::local(LocalId(1)).into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(2)).into(),
                src: Operand::Constant(Value::I64(8)),
            },
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("pointer Copy must preserve exact target verification metadata");
}

#[test]
fn pointer_move_transports_exact_target_for_raw_assign() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
            LocalDecl::new("moved", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Move(Place::local(LocalId(1)).into()),
            },
            Statement::RawAssign {
                pointer: Place::local(LocalId(2)).into(),
                src: Operand::Constant(Value::I64(9)),
            },
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("pointer Move must transfer exact target verification metadata");
}

#[test]
fn aggregate_copy_transports_nested_pointer_target_for_raw_assign() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let holder_ty = types.push(TypeDef::structure(
        "Holder",
        vec![Field::new("pointer", pointer_ty)],
    ));
    let target = Place::local(LocalId(0));
    let holder = Place::local(LocalId(1));
    let copied = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("holder", holder_ty, false),
            LocalDecl::new("copied", holder_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: holder.field(0),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: copied.clone(),
                src: Operand::Copy(Place::local(LocalId(1)).into()),
            },
            Statement::RawAssign {
                pointer: copied.field(0).into(),
                src: Operand::Constant(Value::I64(10)),
            },
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("aggregate Copy must preserve nested raw-pointer targets");
}

#[test]
fn aggregate_move_transports_nested_pointer_target_for_raw_assign() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let holder_ty = types.push(TypeDef::structure(
        "Holder",
        vec![Field::new("pointer", pointer_ty)],
    ));
    let target = Place::local(LocalId(0));
    let holder = Place::local(LocalId(1));
    let moved = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("holder", holder_ty, false),
            LocalDecl::new("moved", holder_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: holder.field(0),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::Move(Place::local(LocalId(1)).into()),
            },
            Statement::RawAssign {
                pointer: moved.field(0).into(),
                src: Operand::Constant(Value::I64(12)),
            },
            Statement::Read { src: target.into() },
        ],
    );

    validate_body(body).expect("aggregate Move must transfer nested raw-pointer targets");
}

#[test]
fn pointer_replacement_installs_new_exact_target() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let first = Place::local(LocalId(0));
    let second = Place::local(LocalId(1));
    let pointer = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("first", value_ty, false),
            LocalDecl::new("second", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, true),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(first.into()),
            },
            Statement::Assign {
                dst: pointer.clone().into(),
                src: Operand::AddressOf(second.clone().into()),
            },
            Statement::RawAssign {
                pointer: pointer.into(),
                src: Operand::Constant(Value::I64(11)),
            },
            Statement::Read { src: second.into() },
        ],
    );

    validate_body(body).expect("pointer replacement must replace exact target metadata too");
}

#[test]
fn pointer_destruction_and_reinitialization_replace_exact_target_metadata() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let first = Place::local(LocalId(0));
    let second = Place::local(LocalId(1));
    let pointer = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("first", value_ty, false),
            LocalDecl::new("second", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, true),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(first.into()),
            },
            Statement::Drop {
                place: pointer.clone().into(),
            },
            Statement::Assign {
                dst: pointer.clone().into(),
                src: Operand::AddressOf(second.clone().into()),
            },
            Statement::RawAssign {
                pointer: pointer.into(),
                src: Operand::Constant(Value::I64(13)),
            },
            Statement::Read { src: second.into() },
        ],
    );

    validate_body(body)
        .expect("reinitializing pointer storage must install only the new exact target");
}

#[test]
fn pointer_target_metadata_participates_in_loop_state_repetition() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let first = Place::local(LocalId(0));
    let second = Place::local(LocalId(1));
    let pointer = Place::local(LocalId(2));
    let body = Body {
        types,
        locals: vec![
            LocalDecl::new("first", value_ty, false),
            LocalDecl::new("second", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, true),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: first.clone(),
                        src: Operand::Constant(Value::I64(0)),
                    },
                    Statement::Drop {
                        place: first.clone().into(),
                    },
                    Statement::Init {
                        dst: second.clone(),
                        src: Operand::Constant(Value::I64(0)),
                    },
                    Statement::Drop {
                        place: second.clone().into(),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(first.clone().into()),
                    },
                ],
                Terminator::Goto(BasicBlockId(1)),
            ),
            BasicBlock::new(
                vec![
                    Statement::RawAssign {
                        pointer: pointer.clone().into(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Read {
                        src: first.clone().into(),
                    },
                    Statement::Drop {
                        place: first.clone().into(),
                    },
                    Statement::Assign {
                        dst: pointer.into(),
                        src: Operand::AddressOf(second.into()),
                    },
                ],
                Terminator::Goto(BasicBlockId(1)),
            ),
        ],
    };

    let error = validate_body(body).expect_err(
        "second loop iteration must be validated because the pointer target changed",
    );
    assert_eq!(error.point.as_ref().map(|point| point.block), Some(BasicBlockId(1)));
    assert_eq!(error.point.as_ref().and_then(|point| point.statement), Some(1));
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(first)
    );
}
