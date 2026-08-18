use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, LocalDecl, LocalId, Operand, Place, Statement, Terminator,
    TypeDef, TypeId, TypeTable, validate_body,
};

#[test]
fn self_targeting_raw_assign_snapshots_pointer_before_moving_same_storage() {
    let mut types = TypeTable::new();
    let pointer_ty = types.push(TypeDef::raw_pointer("self_ptr", TypeId(0)));
    assert_eq!(pointer_ty, TypeId(0));

    let pointer = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("pointer", pointer_ty, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: pointer.clone(),
                    src: Operand::AddressOf(pointer.clone().into()),
                },
                Statement::RawAssign {
                    pointer: pointer.clone().into(),
                    src: Operand::Move(pointer.clone().into()),
                },
                Statement::RawRead {
                    pointer: pointer.into(),
                },
            ],
            Terminator::Return,
        )],
    };

    validate_body(body).expect(
        "RawAssign snapshots a self-targeting pointer before source Move and writes it back Live",
    );
}
