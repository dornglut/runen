mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, LocalDecl, LocalId, Operand, Place, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value,
};
use runen_reference::{RawPointerValue, VerificationEvent, VerificationEventKind};
use support::{machine, one_function_program};

fn formed(events: &[VerificationEvent]) -> Vec<RawPointerValue> {
    events
        .iter()
        .filter_map(|event| match &event.kind {
            VerificationEventKind::AddressOf { pointer, .. } => Some(pointer.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn first_initialization_begins_value_lifetime_without_replacing_storage_instance() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("target", value_ty, false),
                LocalDecl::new("before", pointer_ty, false),
                LocalDecl::new("after", pointer_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::AddressOf(target.into()),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let events = machine(program)
        .execute()
        .expect("storage-identity fixture has defined execution")
        .verification_events;
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 2);
    assert_eq!(pointers[0], pointers[1]);
}
