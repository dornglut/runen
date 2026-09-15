use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Field, Function, FunctionId,
    LocalDecl, LocalId, Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value, validate_program,
};

use crate::{coverage, encoding};

#[test]
fn floating_aggregate_transport_reuses_existing_private_module_shape() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let inner_ty = types.push(TypeDef::structure(
        "Inner",
        vec![Field::new("half", f16_ty), Field::new("single", f32_ty)],
    ));
    let aggregate_ty = types.push(TypeDef::structure(
        "Aggregate",
        vec![Field::new("inner", inner_ty), Field::new("double", f64_ty)],
    ));

    let argument = Value::Struct(vec![
        Value::Struct(vec![
            Value::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative)),
            Value::F32(BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 1,
            }),
        ]),
        Value::F64(BinaryFloatValue::Infinity(BinaryFloatSign::Negative)),
    ]);

    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![
            Function {
                name: "entry".into(),
                parameters: Vec::new(),
                result: None,
                safe_reference_result_contract: SafeReferenceResultContract::None,
                body: Body {
                    locals: vec![
                        LocalDecl::new("argument", aggregate_ty, false),
                        LocalDecl::new("result", aggregate_ty, false),
                    ],
                    loans: Vec::new(),
                    entry: BasicBlockId(0),
                    blocks: vec![
                        BasicBlock::new(
                            vec![Statement::Init {
                                dst: Place::local(LocalId(0)),
                                src: Operand::Constant(argument),
                            }],
                            Terminator::Call {
                                function: FunctionId(1),
                                arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                                destination: Some(Place::local(LocalId(1))),
                                target: BasicBlockId(1),
                            },
                        ),
                        BasicBlock::new(
                            vec![Statement::Drop {
                                place: Place::local(LocalId(1)).into(),
                            }],
                            Terminator::Return(None),
                        ),
                    ],
                },
            },
            Function {
                name: "identity".into(),
                parameters: vec![LocalId(0)],
                result: Some(aggregate_ty),
                safe_reference_result_contract: SafeReferenceResultContract::None,
                body: Body {
                    locals: vec![LocalDecl::new("value", aggregate_ty, false)],
                    loans: Vec::new(),
                    entry: BasicBlockId(0),
                    blocks: vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                    )],
                },
            },
        ],
    })
    .expect("floating aggregate module-shape fixture must be valid Core");

    coverage::validate(&program).expect("floating aggregate fixture must be in realization coverage");
    let encoded = encoding::encode(&program).expect("floating aggregate fixture must encode");
    let module = &encoded.bytes[8..];

    assert_eq!(
        section_ids(module),
        vec![1, 3, 7, 10],
        "floating aggregate transport must not add imports, tables, memory, globals, elements, or data"
    );
    assert_eq!(
        export_kinds(section_payload(module, 7)),
        vec![0],
        "floating aggregate transport must add no public export kind"
    );
}

fn section_ids(bytes: &[u8]) -> Vec<u8> {
    let mut cursor = 0_usize;
    let mut ids = Vec::new();
    while cursor < bytes.len() {
        let section_id = bytes[cursor];
        cursor += 1;
        let payload_len = read_u32_leb(bytes, &mut cursor);
        let end = cursor
            .checked_add(payload_len)
            .expect("section payload length must fit usize");
        assert!(end <= bytes.len(), "section payload must fit module bytes");
        ids.push(section_id);
        cursor = end;
    }
    ids
}

fn section_payload(bytes: &[u8], wanted: u8) -> &[u8] {
    let mut cursor = 0_usize;
    while cursor < bytes.len() {
        let section_id = bytes[cursor];
        cursor += 1;
        let payload_len = read_u32_leb(bytes, &mut cursor);
        let start = cursor;
        let end = start
            .checked_add(payload_len)
            .expect("section payload length must fit usize");
        assert!(end <= bytes.len(), "section payload must fit module bytes");
        if section_id == wanted {
            return &bytes[start..end];
        }
        cursor = end;
    }
    panic!("missing expected Wasm section {wanted}");
}

fn export_kinds(bytes: &[u8]) -> Vec<u8> {
    let mut cursor = 0_usize;
    let count = read_u32_leb(bytes, &mut cursor);
    let mut kinds = Vec::with_capacity(count);
    for _ in 0..count {
        let name_len = read_u32_leb(bytes, &mut cursor);
        cursor = cursor
            .checked_add(name_len)
            .expect("export name length must fit usize");
        let kind = *bytes.get(cursor).expect("export kind must be present");
        cursor += 1;
        let _index = read_u32_leb(bytes, &mut cursor);
        kinds.push(kind);
    }
    assert_eq!(cursor, bytes.len(), "export section must be consumed exactly");
    kinds
}

fn read_u32_leb(bytes: &[u8], cursor: &mut usize) -> usize {
    let mut result = 0_usize;
    let mut shift = 0_u32;
    loop {
        let byte = *bytes
            .get(*cursor)
            .expect("section size LEB must be present in generated module");
        *cursor += 1;
        result |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return result;
        }
        shift += 7;
        assert!(shift < 35, "section size LEB must fit u32");
    }
}
