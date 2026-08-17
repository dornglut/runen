use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId, Operand,
    Place, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_body,
};
use runen_reference::{Machine, TerminalStatus, VerificationEvent};

#[test]
fn defined_return_ends_active_borrow_before_cleanup() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", tracked, false)],
        loans: vec![LoanDecl::new("shared", tracked)],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: value.clone(),
                    src: Operand::Constant(Value::TrackedFixture(9)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Shared,
                    src: value.clone().into(),
                },
            ],
            Terminator::Return,
        )],
    };

    let report =
        Machine::new(validate_body(body).expect("return borrow MIR must validate")).execute();
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::BorrowStart {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            })
    );
    assert!(
        !report
            .verification_events
            .contains(&VerificationEvent::BorrowEnd(LoanId(0)))
    );
    assert_eq!(
        report
            .verification_events
            .iter()
            .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 9, .. }))
            .count(),
        1
    );
}
