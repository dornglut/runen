use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, LocalDecl, LocalId, MirValidationErrorKind, Operand,
    Place, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, Value, validate_body,
};

fn one_block(types: TypeTable, locals: Vec<LocalDecl>, statements: Vec<Statement>) -> Body {
    Body {
        types,
        locals,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

#[test]
fn valid_body_is_admitted() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(Value::I64(1)),
        }],
    );

    let validated = validate_body(body).expect("well-formed body must be admitted");
    assert_eq!(validated.as_body().entry, BasicBlockId(0));
}

#[test]
fn invalid_entry_block_is_rejected() {
    let body = Body {
        types: TypeTable::new(),
        locals: Vec::new(),
        entry: BasicBlockId(1),
        blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return)],
    };

    let error = validate_body(body).expect_err("entry outside the body must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidEntryBlock(BasicBlockId(1))
    );
}

#[test]
fn invalid_goto_target_is_rejected() {
    let body = Body {
        types: TypeTable::new(),
        locals: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            Vec::new(),
            Terminator::Goto(BasicBlockId(7)),
        )],
    };

    let error = validate_body(body).expect_err("unknown target must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidTargetBlock(BasicBlockId(7))
    );
}

#[test]
fn unknown_local_type_is_rejected() {
    let body = one_block(
        TypeTable::new(),
        vec![LocalDecl::new("value", TypeId(9), false)],
        Vec::new(),
    );

    let error = validate_body(body).expect_err("unknown local type must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::UnknownType(TypeId(9)));
}

#[test]
fn statement_referencing_unknown_local_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Read {
            src: Place::local(LocalId(8)),
        }],
    );

    let error = validate_body(body).expect_err("unknown local reference must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::InvalidLocal(LocalId(8)));
}

#[test]
fn struct_field_referencing_unknown_type_is_rejected() {
    let mut types = TypeTable::new();
    let aggregate = types.push(TypeDef::structure(
        "Broken",
        vec![Field::new("field", TypeId(9))],
    ));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", aggregate, false)],
        Vec::new(),
    );

    let error = validate_body(body).expect_err("unknown field type must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::UnknownType(TypeId(9)));
}

#[test]
fn recursive_by_value_type_is_rejected() {
    let mut types = TypeTable::new();
    let recursive = TypeId(0);
    assert_eq!(
        types.push(TypeDef::structure(
            "Recursive",
            vec![Field::new("next", recursive)]
        )),
        recursive
    );

    let body = one_block(
        types,
        vec![LocalDecl::new("value", recursive, false)],
        Vec::new(),
    );

    let error = validate_body(body).expect_err("recursive value type must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::RecursiveType(recursive)
    );
}

#[test]
fn invalid_projection_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Read {
            src: Place::local(LocalId(0)).field(0),
        }],
    );

    let place = Place::local(LocalId(0)).field(0);
    let error = validate_body(body).expect_err("field projection from scalar must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::InvalidProjection(place));
}

#[test]
fn operand_type_mismatch_is_rejected() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", bool_ty, false)],
        vec![Statement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(Value::I64(1)),
        }],
    );

    let error = validate_body(body).expect_err("typed MIR mismatch must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: bool_ty }
    );
}

#[test]
fn copy_of_noncopy_type_is_rejected_at_admission() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::Tracked));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", tracked, false),
            LocalDecl::new("target", tracked, false),
        ],
        vec![Statement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Copy(Place::local(LocalId(0))),
        }],
    );

    let error = validate_body(body).expect_err("non-copy Copy must be rejected before execution");
    assert_eq!(error.kind, MirValidationErrorKind::CopyOfNonCopy(tracked));
}

#[test]
fn assignment_through_immutable_local_is_rejected_at_admission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Assign {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(Value::I64(2)),
        }],
    );

    let error =
        validate_body(body).expect_err("immutable assignment must be rejected before execution");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}
