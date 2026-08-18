use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, LocalDecl, LocalId, Operand, Place, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value, validate_body,
};
use runen_reference::{Machine, RawPointerValue, VerificationEvent};

fn formed(events: &[VerificationEvent]) -> Vec<RawPointerValue> {
    events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::AddressOf { pointer, .. } => Some(pointer.clone()),
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
    let body = Body {
        types,
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
            Terminator::Return,
        )],
    };

    let events = Machine::new(validate_body(body).expect("valid storage-identity fixture"))
        .execute()
        .verification_events;
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 2);
    assert_eq!(pointers[0], pointers[1]);
}
