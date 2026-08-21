mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
};
use runen_reference::{ExecutionReport, VerificationEventKind};
use support::{event_kinds, machine, one_function_program};

fn execute(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
) -> ExecutionReport {
    let program = one_function_program(
        types,
        Body {
            locals,
            loans,
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(statements, Terminator::Return(None))],
        },
    );

    machine(program)
        .execute()
        .expect("fixture satisfies RawRead unsafe target preconditions")
}

#[test]
fn shared_loan_can_supply_raw_read_pointer_value_during_execution() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));

    let report = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![LoanDecl::new("pointer_shared", pointer_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(target.clone().into()),
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

    let events = event_kinds(&report.verification_events);
    assert!(events.iter().any(|event| matches!(
        event,
        VerificationEventKind::RawRead { target: read, .. } if read == &target
    )));
}

#[test]
fn disjoint_exclusive_target_loan_does_not_block_raw_read() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", value_ty), Field::new("right", value_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pair = Place::local(LocalId(0));
    let left = pair.clone().field(0);
    let right = pair.clone().field(1);

    let report = execute(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![LoanDecl::new("right_exclusive", value_ty)],
        vec![
            Statement::Init {
                dst: pair,
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(left.clone().into()),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: right.into(),
            },
            Statement::RawRead {
                pointer: Place::local(LocalId(1)).into(),
            },
        ],
    );

    let events = event_kinds(&report.verification_events);
    assert!(events.iter().any(|event| matches!(
        event,
        VerificationEventKind::RawRead { target: read, .. } if read == &left
    )));
}
