use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_body,
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
fn live_raw_pointer_value_is_valid_for_raw_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(target.into()),
            },
            Statement::RawRead {
                pointer: pointer.into(),
            },
        ],
    );

    validate_body(body).expect("live raw-pointer value is valid MIR for RawRead");
}

#[test]
fn shared_loan_can_supply_raw_read_pointer_value() {
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
            Statement::RawRead {
                pointer: PlaceAccess::loan(LoanId(0)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("shared loan retains authority to read the stored pointer value");
}

#[test]
fn raw_read_requires_raw_pointer_type() {
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
            Statement::RawRead {
                pointer: value.into(),
            },
        ],
    );

    let error = validate_body(body).expect_err("RawRead of non-pointer MIR must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::RawReadRequiresPointer(value_ty)
    );
}

#[test]
fn raw_read_requires_live_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pointer", pointer_ty, false)],
        Vec::new(),
        vec![Statement::RawRead {
            pointer: pointer.clone().into(),
        }],
    );

    let error = validate_body(body).expect_err("uninitialized pointer value is invalid MIR");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(pointer)
    );
}

#[test]
fn raw_read_rejects_moved_pointer_value() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let moved = Place::local(LocalId(2));
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
                dst: moved,
                src: Operand::Move(pointer.clone().into()),
            },
            Statement::RawRead {
                pointer: pointer.clone().into(),
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
fn exclusive_loan_over_pointer_storage_blocks_raw_read_at_validation() {
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
        vec![LoanDecl::new("pointer_loan", pointer_ty)],
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
            Statement::RawRead {
                pointer: pointer.clone().into(),
            },
        ],
    );

    let error = validate_body(body).expect_err("exclusive pointer-storage loan blocks RawRead");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: pointer,
            loan: LoanId(0),
        }
    );
}
