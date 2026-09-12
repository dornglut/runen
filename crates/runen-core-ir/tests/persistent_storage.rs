use runen_core_ir::{PersistentDecl, PersistentId, TypeId, Value};

#[test]
fn persistent_declarations_keep_program_identity_separate_from_initial_value() {
    let first = PersistentId(0);
    let second = PersistentId(1);
    assert_ne!(first, second);

    let declaration = PersistentDecl::new(TypeId(7), Value::I64(42));
    assert_eq!(declaration.ty, TypeId(7));
    assert_eq!(declaration.initial, Value::I64(42));
}
