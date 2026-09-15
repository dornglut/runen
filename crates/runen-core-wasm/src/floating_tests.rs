use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, SafeReferenceResultContract, ScalarType,
    Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};

use crate::scalar::{FloatingScalarValue, ScalarKind};
use crate::{coverage, encoding};

fn payload(residue: u64) -> i64 {
    i64::from_ne_bytes(residue.to_ne_bytes())
}

fn represented(value: BinaryFloatValue) -> FloatingScalarValue {
    FloatingScalarValue::Represented(value)
}

#[test]
fn private_floating_carrier_round_trips_all_format_boundaries_exactly() {
    let cases = [
        (
            ScalarKind::F16,
            vec![
                BinaryFloatValue::Zero(BinaryFloatSign::Positive),
                BinaryFloatValue::Zero(BinaryFloatSign::Negative),
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1,
                },
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Negative,
                    significand: (1_u64 << 10) - 1,
                },
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1_u64 << 10,
                    exponent: -14,
                },
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Negative,
                    significand: (1_u64 << 11) - 1,
                    exponent: 15,
                },
                BinaryFloatValue::Infinity(BinaryFloatSign::Positive),
                BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
            ],
        ),
        (
            ScalarKind::F32,
            vec![
                BinaryFloatValue::Zero(BinaryFloatSign::Positive),
                BinaryFloatValue::Zero(BinaryFloatSign::Negative),
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1,
                },
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Negative,
                    significand: (1_u64 << 23) - 1,
                },
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1_u64 << 23,
                    exponent: -126,
                },
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Negative,
                    significand: (1_u64 << 24) - 1,
                    exponent: 127,
                },
                BinaryFloatValue::Infinity(BinaryFloatSign::Positive),
                BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
            ],
        ),
        (
            ScalarKind::F64,
            vec![
                BinaryFloatValue::Zero(BinaryFloatSign::Positive),
                BinaryFloatValue::Zero(BinaryFloatSign::Negative),
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1,
                },
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Negative,
                    significand: (1_u64 << 52) - 1,
                },
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1_u64 << 52,
                    exponent: -1022,
                },
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Negative,
                    significand: (1_u64 << 53) - 1,
                    exponent: 1023,
                },
                BinaryFloatValue::Infinity(BinaryFloatSign::Positive),
                BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
            ],
        ),
    ];

    for (kind, values) in cases {
        for value in values {
            let residue = kind
                .floating_residue(represented(value))
                .expect("boundary value must fit the declared floating format");
            assert_eq!(
                kind.decode_floating(payload(residue)),
                Ok(represented(value))
            );
        }
    }
}

#[test]
fn floating_transport_reuses_only_existing_private_wasm_machinery() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let callable = types.push(TypeDef::callable(
        "FloatIdentity",
        CallableInterface::new(
            vec![f32_ty],
            Some(f32_ty),
            SafeReferenceResultContract::None,
        ),
    ));
    let external = ExternalCallableDecl::new(CallableInterface::new(
        vec![f32_ty],
        Some(f32_ty),
        SafeReferenceResultContract::None,
    ));
    let persistent = PersistentDecl::new(
        f32_ty,
        Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Negative)),
    );
    let program = validate_program(Program {
        types,
        persistent: vec![persistent],
        external_callables: vec![external],
        functions: vec![
            Function {
                name: "entry".into(),
                parameters: Vec::new(),
                result: None,
                safe_reference_result_contract: SafeReferenceResultContract::None,
                body: Body {
                    locals: vec![
                        LocalDecl::new("callee", callable, false),
                        LocalDecl::new("indirect_result", f32_ty, false),
                        LocalDecl::new("external_result", f32_ty, false),
                    ],
                    loans: Vec::new(),
                    entry: BasicBlockId(0),
                    blocks: vec![
                        BasicBlock::new(
                            vec![Statement::Init {
                                dst: Place::local(LocalId(0)),
                                src: Operand::FunctionValue(FunctionId(1)),
                            }],
                            Terminator::IndirectCall {
                                callable,
                                callee: Operand::Move(Place::local(LocalId(0)).into()),
                                arguments: vec![Operand::PersistentRead(PersistentId(0))],
                                destination: Some(Place::local(LocalId(1))),
                                target: BasicBlockId(1),
                            },
                        ),
                        BasicBlock::new(
                            Vec::new(),
                            Terminator::ExternalCall {
                                external: ExternalCallableId(0),
                                arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                                destination: Some(Place::local(LocalId(2))),
                                target: BasicBlockId(2),
                            },
                        ),
                        BasicBlock::new(Vec::new(), Terminator::Return(None)),
                    ],
                },
            },
            Function {
                name: "identity".into(),
                parameters: vec![LocalId(0)],
                result: Some(f32_ty),
                safe_reference_result_contract: SafeReferenceResultContract::None,
                body: Body {
                    locals: vec![LocalDecl::new("value", f32_ty, false)],
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
    .expect("floating module-shape fixture must be valid Core");
    coverage::validate(&program).expect("floating module-shape fixture must be admitted");
    let encoded = encoding::encode(&program).expect("floating module-shape fixture must encode");
    let module = &encoded.bytes[8..];

    assert_eq!(
        section_ids(module),
        vec![1, 2, 3, 4, 6, 7, 9, 10],
        "floating transport may compose only the existing import, table, global, and element machinery; memory/data sections stay absent"
    );
    assert_single_private_function_import(section_payload(module, 2));
    assert_private_two_function_table(section_payload(module, 4));
    assert_single_private_immutable_i64_global(section_payload(module, 6));
    assert_two_function_element_population(section_payload(module, 9), [1, 2]);
    assert_eq!(
        export_entries(section_payload(module, 7)),
        vec![(0, 1)],
        "only the shifted zero-parameter Runen entry is exported; provider import, callable table, and floating persistent global remain private"
    );
}

#[test]
fn floating_arithmetic_adds_no_wasm_sections_or_export_kinds() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let f32_zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let f64_zero = Value::F64(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![
                    LocalDecl::new("f32_result", f32_ty, false),
                    LocalDecl::new("f64_result", f64_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::FloatAdd {
                            dst: Place::local(LocalId(0)),
                            left: Operand::Constant(f32_zero.clone()),
                            right: Operand::Constant(f32_zero),
                            contract: runen_core_ir::NumericContract::Standard,
                        },
                        Statement::FloatMul {
                            dst: Place::local(LocalId(1)),
                            left: Operand::Constant(f64_zero.clone()),
                            right: Operand::Constant(f64_zero),
                            contract: runen_core_ir::NumericContract::Standard,
                        },
                        Statement::Drop {
                            place: Place::local(LocalId(0)).into(),
                        },
                        Statement::Drop {
                            place: Place::local(LocalId(1)).into(),
                        },
                    ],
                    Terminator::Return(None),
                )],
            },
        }],
    })
    .expect("floating arithmetic module-shape fixture must be valid Core");
    coverage::validate(&program).expect("F32/F64 arithmetic module-shape fixture must be admitted");
    let encoded =
        encoding::encode(&program).expect("floating arithmetic module-shape fixture must encode");
    let module = &encoded.bytes[8..];

    assert_eq!(
        section_ids(module),
        vec![1, 3, 7, 10],
        "transient F32/F64 arithmetic locals must not add imports, tables, memory, globals, elements, or data"
    );
    assert_eq!(
        export_entries(section_payload(module, 7)),
        vec![(0, 0)],
        "floating arithmetic must retain the existing function-only entry export"
    );
}

#[test]
fn arithmetic_nan_uses_the_existing_private_carrier_before_provider_decode() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let external = ExternalCallableDecl::new(CallableInterface::new(
        vec![f32_ty],
        None,
        SafeReferenceResultContract::None,
    ));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![external],
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", f32_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
                    BasicBlock::new(
                        vec![Statement::FloatDiv {
                            dst: Place::local(LocalId(0)),
                            left: Operand::Constant(zero.clone()),
                            right: Operand::Constant(zero),
                            contract: runen_core_ir::NumericContract::Standard,
                        }],
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            },
        }],
    })
    .expect("raw arithmetic NaN fixture must be valid Core");
    coverage::validate(&program).expect("raw arithmetic NaN fixture must be admitted");
    let encoded = encoding::encode(&program).expect("raw arithmetic NaN fixture must encode");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &encoded.bytes)
        .expect("raw arithmetic NaN fixture must compile");
    let mut store = wasmtime::Store::new(&engine, ());
    let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let captured = std::sync::Arc::clone(&observed);
    let import_type = wasmtime::FuncType::new(
        &engine,
        [wasmtime::ValType::I64],
        std::iter::empty::<wasmtime::ValType>(),
    );
    let import = wasmtime::Func::new(
        &mut store,
        import_type,
        move |_caller, parameters, results| {
            assert!(results.is_empty(), "observer import has no result");
            let [wasmtime::Val::I64(carrier)] = parameters else {
                panic!("observer import expects exactly one i64 carrier");
            };
            *captured.lock().unwrap() = Some(*carrier);
            Ok(())
        },
    );
    let instance = wasmtime::Instance::new(&mut store, &module, &[wasmtime::Extern::Func(import)])
        .expect("raw arithmetic NaN fixture must instantiate");
    let entry = instance
        .get_func(&mut store, &encoding::entry_export_name(FunctionId(0)))
        .expect("raw arithmetic NaN entry export must exist");
    let mut results = [wasmtime::Val::I32(-1), wasmtime::Val::I64(-1)];
    entry
        .call(&mut store, &[], &mut results)
        .expect("raw arithmetic NaN fixture must execute");
    assert!(matches!(results[0], wasmtime::Val::I32(0)));

    let expected = payload(
        ScalarKind::F32
            .floating_residue(FloatingScalarValue::NaNClass)
            .expect("existing private F32 NaN carrier must encode"),
    );
    assert_eq!(
        *observed.lock().unwrap(),
        Some(expected),
        "arithmetic NaN must reach the import boundary as exactly the existing private NaN carrier"
    );
}

fn assert_single_private_function_import(bytes: &[u8]) {
    let mut cursor = 0_usize;
    assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "exactly one import");
    let _module = read_name(bytes, &mut cursor);
    let _field = read_name(bytes, &mut cursor);
    assert_eq!(
        bytes.get(cursor),
        Some(&0x00),
        "provider import is a function"
    );
    cursor += 1;
    let _type_index = read_u32_leb(bytes, &mut cursor);
    assert_eq!(cursor, bytes.len(), "import section is consumed exactly");
}

fn assert_private_two_function_table(bytes: &[u8]) {
    let mut cursor = 0_usize;
    assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "exactly one table");
    assert_eq!(bytes.get(cursor), Some(&0x70), "table remains funcref");
    cursor += 1;
    assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "exact table limits");
    assert_eq!(read_u32_leb(bytes, &mut cursor), 2, "table minimum");
    assert_eq!(read_u32_leb(bytes, &mut cursor), 2, "table maximum");
    assert_eq!(cursor, bytes.len(), "table section is consumed exactly");
}

fn assert_single_private_immutable_i64_global(bytes: &[u8]) {
    let mut cursor = 0_usize;
    assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "one persistent global");
    assert_eq!(
        bytes.get(cursor),
        Some(&0x7e),
        "floating carrier global is i64"
    );
    cursor += 1;
    assert_eq!(
        bytes.get(cursor),
        Some(&0x00),
        "persistent global is immutable"
    );
}

fn assert_two_function_element_population(bytes: &[u8], expected: [usize; 2]) {
    let mut cursor = 0_usize;
    assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "one element segment");
    assert_eq!(
        read_u32_leb(bytes, &mut cursor),
        0,
        "active table-zero segment"
    );
    assert_eq!(bytes.get(cursor), Some(&0x41), "offset uses i32.const");
    cursor += 1;
    assert_eq!(bytes.get(cursor), Some(&0x00), "offset is zero");
    cursor += 1;
    assert_eq!(bytes.get(cursor), Some(&0x0b), "offset expression ends");
    cursor += 1;
    assert_eq!(read_u32_leb(bytes, &mut cursor), 2, "two function elements");
    assert_eq!(read_u32_leb(bytes, &mut cursor), expected[0]);
    assert_eq!(read_u32_leb(bytes, &mut cursor), expected[1]);
    assert_eq!(cursor, bytes.len(), "element section is consumed exactly");
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

fn export_entries(bytes: &[u8]) -> Vec<(u8, usize)> {
    let mut cursor = 0_usize;
    let count = read_u32_leb(bytes, &mut cursor);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let _name = read_name(bytes, &mut cursor);
        let kind = *bytes.get(cursor).expect("export kind must be present");
        cursor += 1;
        let index = read_u32_leb(bytes, &mut cursor);
        entries.push((kind, index));
    }
    assert_eq!(cursor, bytes.len(), "export section is consumed exactly");
    entries
}

fn read_name<'a>(bytes: &'a [u8], cursor: &mut usize) -> &'a str {
    let name_len = read_u32_leb(bytes, cursor);
    let end = cursor
        .checked_add(name_len)
        .expect("Wasm name length must fit usize");
    let name = std::str::from_utf8(
        bytes
            .get(*cursor..end)
            .expect("Wasm name bytes must fit section"),
    )
    .expect("generated Wasm names are UTF-8");
    *cursor = end;
    name
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
