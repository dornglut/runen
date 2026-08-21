use runen_core_ir::{
    BasicBlockId, Function as CoreFunction, FunctionId as CoreFunctionId, LocalId, Operand, Place,
    PlaceAccess, Program, ScalarType, Statement as CoreStatement, Terminator, TypeKind,
    ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR");
    lower(&hir).expect("accepted HIR must lower to valid Core")
}

fn lower_units(module: ModuleId, sources: &[&str]) -> ValidatedProgram {
    let parses = sources
        .iter()
        .map(|source| parse(source))
        .collect::<Vec<_>>();
    let units = parses
        .iter()
        .map(|parse| SourceUnit::new(module, parse, &[]))
        .collect::<Vec<_>>();
    let hir = build_typed_hir(&units).expect("test units must produce accepted HIR");
    lower(&hir).expect("accepted HIR must lower to valid Core")
}

fn function<'a>(program: &'a Program, name: &str) -> &'a CoreFunction {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn function_name(program: &Program, id: CoreFunctionId) -> &str {
    &program
        .function(id)
        .expect("lowered call target must exist")
        .name
}

fn direct(place: Place) -> PlaceAccess {
    PlaceAccess::Direct(place)
}

#[test]
fn maps_every_represented_intrinsic_to_the_corresponding_core_scalar_tag() {
    let lowered = lower_source(
        "fn all(\
            b: Bool, i8v: I8, i16v: I16, i32v: I32, i64v: I64,\
            u8v: U8, u16v: U16, u32v: U32, u64v: U64,\
            f16v: F16, f32v: F32, f64v: F64\
        ) {}",
    );
    let program = lowered.as_program();
    let function = function(program, "all");
    let expected = [
        ScalarType::Bool,
        ScalarType::I8,
        ScalarType::I16,
        ScalarType::I32,
        ScalarType::I64,
        ScalarType::U8,
        ScalarType::U16,
        ScalarType::U32,
        ScalarType::U64,
        ScalarType::F16,
        ScalarType::F32,
        ScalarType::F64,
    ];

    assert_eq!(program.types.len(), expected.len());
    assert_eq!(function.parameters.len(), expected.len());
    for (slot, expected) in expected.into_iter().enumerate() {
        let ty = function
            .parameter_type(slot)
            .expect("lowered parameter slot has a type");
        let TypeKind::Scalar(actual) = &program
            .types
            .get(ty)
            .expect("lowered scalar type exists")
            .kind
        else {
            panic!("intrinsic source type lowered to non-scalar Core type");
        };
        assert_eq!(*actual, expected);
    }
}

#[test]
fn preserves_nominal_record_identity_field_order_and_dependency_resolution() {
    let forward = lower_source(
        "record A { inner: Inner, flag: Bool } \
         record B { inner: Inner, flag: Bool } \
         record Inner { value: I64 } \
         fn take(a: A, b: B) {}",
    );
    let dependency_first = lower_source(
        "record Inner { value: I64 } \
         record A { inner: Inner, flag: Bool } \
         record B { inner: Inner, flag: Bool } \
         fn take(a: A, b: B) {}",
    );

    for lowered in [&forward, &dependency_first] {
        let program = lowered.as_program();
        let take = function(program, "take");
        let a = take.parameter_type(0).unwrap();
        let b = take.parameter_type(1).unwrap();
        assert_ne!(
            a, b,
            "equal record structure must remain nominally distinct"
        );

        for record_ty in [a, b] {
            let TypeKind::Struct(fields) = &program.types.get(record_ty).unwrap().kind else {
                panic!("record source type must lower to a structural Core type");
            };
            assert_eq!(
                fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["inner", "flag"]
            );
            let inner_ty = fields[0].ty;
            let TypeKind::Struct(inner_fields) = &program.types.get(inner_ty).unwrap().kind else {
                panic!("record dependency must lower before its consumer");
            };
            assert_eq!(inner_fields.len(), 1);
            assert_eq!(inner_fields[0].name, "value");
        }
    }
}

#[test]
fn uses_hir_duplicate_and_consume_instead_of_core_structural_copyability() {
    let lowered = lower_source(
        "record Ticket { value: I64 } \
         fn return_ticket(t: Ticket) -> Ticket { return t; } \
         fn return_i64(x: I64) -> I64 { return x; }",
    );
    let program = lowered.as_program();

    let record_function = function(program, "return_ticket");
    let record_ty = record_function.parameter_type(0).unwrap();
    assert!(
        program.types.is_copy(record_ty),
        "Core structural copyability is intentionally broader than current source nominal duplicability"
    );
    assert_eq!(
        record_function.body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)))),
        }
    );

    let scalar_function = function(program, "return_i64");
    assert_eq!(
        scalar_function.body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Copy(direct(Place::local(LocalId(0)))),
        }
    );
}

#[test]
fn materializes_earlier_simple_argument_before_later_nested_call() {
    let lowered = lower_source(
        "fn id(v: I64) -> I64 { return v; } \
         fn pair(a: I64, b: I64) {} \
         fn test(x: I64, y: I64) { pair(x, id(y)); }",
    );
    let program = lowered.as_program();
    let test = function(program, "test");

    assert_eq!(
        test.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec!["x", "y", "$tmp0", "$tmp1", "$tmp2"]
    );
    assert_eq!(
        test.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Copy(direct(Place::local(LocalId(0)))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::Copy(direct(Place::local(LocalId(1)))),
            },
        ]
    );

    let Terminator::Call {
        function: nested_target,
        arguments,
        destination: Some(destination),
        target: BasicBlockId(1),
    } = &test.body.blocks[0].terminator
    else {
        panic!("nested result call must terminate the first Core block");
    };
    assert_eq!(function_name(program, *nested_target), "id");
    assert_eq!(
        arguments,
        &vec![Operand::Move(direct(Place::local(LocalId(3))))]
    );
    assert_eq!(*destination, Place::local(LocalId(4)));

    let Terminator::Call {
        function: outer_target,
        arguments,
        destination: None,
        target: BasicBlockId(2),
    } = &test.body.blocks[1].terminator
    else {
        panic!("outer no-result call must follow the nested call continuation");
    };
    assert_eq!(function_name(program, *outer_target), "pair");
    assert_eq!(
        arguments,
        &vec![
            Operand::Move(direct(Place::local(LocalId(2)))),
            Operand::Move(direct(Place::local(LocalId(4)))),
        ]
    );
    assert_eq!(test.body.blocks[2].terminator, Terminator::Return(None));
}

#[test]
fn predeclares_source_locals_before_temporaries_and_transfers_initializers_once() {
    let lowered = lower_source(
        "fn id(v: I64) -> I64 { return v; } \
         fn f(p: I64, q: I64) -> I64 { \
             let a: I64 = id(p); \
             let b: I64 = q; \
             return id(a); \
         }",
    );
    let program = lowered.as_program();
    let function = function(program, "f");

    assert_eq!(
        function
            .body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "p", "q", "a", "b", "$tmp0", "$tmp1", "$tmp2", "$tmp3", "$tmp4"
        ]
    );

    let first_continuation = &function.body.blocks[1];
    assert_eq!(
        first_continuation.statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(2)),
            src: Operand::Move(direct(Place::local(LocalId(5)))),
        },
        "call result must transfer exactly once into source local a"
    );
    assert_eq!(
        first_continuation.statements[1],
        CoreStatement::Init {
            dst: Place::local(LocalId(6)),
            src: Operand::Copy(direct(Place::local(LocalId(1)))),
        }
    );
    assert_eq!(
        first_continuation.statements[2],
        CoreStatement::Init {
            dst: Place::local(LocalId(3)),
            src: Operand::Move(direct(Place::local(LocalId(6)))),
        },
        "simple initializer must transfer its produced temporary exactly once"
    );

    let Terminator::Call {
        destination: Some(destination),
        target: BasicBlockId(2),
        ..
    } = &first_continuation.terminator
    else {
        panic!("return-value producer must be lowered as a result call");
    };
    assert_eq!(*destination, Place::local(LocalId(8)));
    assert_eq!(
        function.body.blocks[2].terminator,
        Terminator::Return(Some(Operand::Move(direct(Place::local(LocalId(8))))))
    );
}

#[test]
fn lowers_no_result_calls_returns_and_fallthrough_without_unit_value() {
    let lowered = lower_source("fn a() {} fn b() { return; } fn c() { a(); }");
    let program = lowered.as_program();
    assert_eq!(program.types.len(), 12);

    assert_eq!(function(program, "a").result, None);
    assert_eq!(
        function(program, "a").body.blocks[0].terminator,
        Terminator::Return(None)
    );
    assert_eq!(
        function(program, "b").body.blocks[0].terminator,
        Terminator::Return(None)
    );

    let c = function(program, "c");
    let Terminator::Call {
        destination: None,
        target: BasicBlockId(1),
        ..
    } = &c.body.blocks[0].terminator
    else {
        panic!("no-result source call must have no Core destination");
    };
    assert_eq!(c.body.blocks[1].terminator, Terminator::Return(None));
}

#[test]
fn maps_all_functions_before_bodies_so_mutual_recursion_resolves() {
    let lowered = lower_source(
        "fn a(x: I64) -> I64 { return b(x); } \
         fn b(x: I64) -> I64 { return a(x); }",
    );
    let program = lowered.as_program();

    for (caller, callee) in [("a", "b"), ("b", "a")] {
        let function = function(program, caller);
        let Terminator::Call {
            function: target, ..
        } = &function.body.blocks[0].terminator
        else {
            panic!("recursive source call must lower to direct Core Call");
        };
        assert_eq!(function_name(program, *target), callee);
    }
}

#[test]
fn source_unit_presentation_order_does_not_change_resolved_call_target() {
    let caller = "fn caller(x: I64) -> I64 { return callee(x); }";
    let callee = "fn callee(x: I64) -> I64 { return x; }";
    let module = ModuleId::new(9);

    let first = lower_units(module, &[caller, callee]);
    let second = lower_units(module, &[callee, caller]);

    for lowered in [&first, &second] {
        let program = lowered.as_program();
        let caller = function(program, "caller");
        let Terminator::Call {
            function: target, ..
        } = &caller.body.blocks[0].terminator
        else {
            panic!("resolved HIR call must remain a direct Core call");
        };
        assert_eq!(function_name(program, *target), "callee");
    }
}

#[test]
fn opaque_module_identity_is_erased_after_hir_resolution() {
    let source = "record Ticket {} fn id(value: Ticket) -> Ticket { return value; }";
    let first = lower_units(ModuleId::new(1), &[source]);
    let second = lower_units(ModuleId::new(99), &[source]);

    assert_eq!(first.as_program(), second.as_program());
}
