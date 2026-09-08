use runen_core_ir::{
    Function as CoreFunction, FunctionId as CoreFunctionId, Operand, Place, PlaceAccess,
    ScalarType, Statement as CoreStatement, Terminator, TypeKind, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, ModuleId, ReferencePermission, ReferenceReferent, SourceUnit, Type, ValueKind,
    build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn hir(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted generic HIR must lower to validated concrete Core")
}

fn functions_named<'a>(program: &'a runen_core_ir::Program, name: &str) -> Vec<&'a CoreFunction> {
    program
        .functions
        .iter()
        .filter(|function| function.name == name)
        .collect()
}

fn function_id(program: &runen_core_ir::Program, name: &str) -> CoreFunctionId {
    let matches = program
        .functions
        .iter()
        .enumerate()
        .filter(|(_, function)| function.name == name)
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "expected one Core function named {name}");
    CoreFunctionId(matches[0].0 as u32)
}

fn scalar_parameter(
    program: &runen_core_ir::Program,
    function: &CoreFunction,
    slot: usize,
) -> Option<ScalarType> {
    let ty = function.parameter_type(slot)?;
    match &program.types.get(ty)?.kind {
        TypeKind::Scalar(scalar) => Some(*scalar),
        TypeKind::Struct(_) | TypeKind::Reference { .. } | TypeKind::RawPointer(_) => None,
    }
}

fn direct(place: Place) -> PlaceAccess {
    PlaceAccess::Direct(place)
}

#[test]
fn non_generic_functions_remain_one_to_one_and_unreachable_generics_are_not_materialized() {
    let lowered = lower_source(
        "fn unused[T](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return value; }",
    );
    let program = lowered.as_program();

    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "root");
    assert_eq!(
        scalar_parameter(program, &program.functions[0], 0),
        Some(ScalarType::I64)
    );
}

#[test]
fn reachable_generic_application_materializes_one_exact_concrete_specialization() {
    let lowered = lower_source(
        "fn id[T](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return id[I64](value); }",
    );
    let program = lowered.as_program();
    let ids = functions_named(program, "id");

    assert_eq!(ids.len(), 1);
    assert_eq!(scalar_parameter(program, ids[0], 0), Some(ScalarType::I64));
    assert_eq!(
        ids[0]
            .result
            .and_then(|ty| program.types.get(ty))
            .map(|ty| &ty.kind),
        Some(&TypeKind::Scalar(ScalarType::I64))
    );
}

#[test]
fn distinct_concrete_applications_materialize_distinct_specializations() {
    let lowered = lower_source(
        "record Ticket { value: I64 } \
         fn id[T](value: T) -> T { return value; } \
         fn root(number: I64, ticket: Ticket) { \
             let copied: I64 = id[I64](number); \
             let moved: Ticket = id[Ticket](ticket); \
         }",
    );
    let program = lowered.as_program();
    let ids = functions_named(program, "id");

    assert_eq!(ids.len(), 2);
    let parameter_types = ids
        .iter()
        .map(|function| function.parameter_type(0).expect("id has one parameter"))
        .collect::<Vec<_>>();
    assert_ne!(parameter_types[0], parameter_types[1]);
    assert!(
        ids.iter()
            .any(|function| scalar_parameter(program, function, 0) == Some(ScalarType::I64))
    );
    assert!(ids.iter().any(|function| {
        let ty = function.parameter_type(0).expect("id has one parameter");
        matches!(program.types.get(ty).map(|ty| &ty.kind), Some(TypeKind::Struct(_)))
    }));
}

#[test]
fn generic_body_consume_remains_core_move_for_copyable_and_noncopyable_substitutions() {
    let lowered = lower_source(
        "record Ticket { value: I64 } \
         fn id[T](value: T) -> T { return value; } \
         fn root(number: I64, ticket: Ticket) { \
             let copied: I64 = id[I64](number); \
             let moved: Ticket = id[Ticket](ticket); \
         }",
    );
    let program = lowered.as_program();
    let ids = functions_named(program, "id");
    assert_eq!(ids.len(), 2);

    for id in ids {
        assert_eq!(
            id.body.blocks[0].statements[0],
            CoreStatement::Init {
                dst: Place::local(runen_core_ir::LocalId(1)),
                src: Operand::Move(direct(Place::local(runen_core_ir::LocalId(0)))),
            },
            "abstract generic Consume must not be re-selected after concrete substitution"
        );
    }
}

#[test]
fn generic_to_generic_type_argument_composes_before_specialization_lookup() {
    let lowered = lower_source(
        "fn id[T](value: T) -> T { return value; } \
         fn outer[U](value: U) -> U { return id[U](value); } \
         fn root(value: I64) -> I64 { return outer[I64](value); }",
    );
    let program = lowered.as_program();
    let outer = functions_named(program, "outer");
    assert_eq!(outer.len(), 1);
    assert_eq!(scalar_parameter(program, outer[0], 0), Some(ScalarType::I64));

    let Terminator::Call { function, .. } = &outer[0].body.blocks[0].terminator else {
        panic!("outer specialization must call the concrete id specialization");
    };
    let target = program
        .function(*function)
        .expect("composed generic call target exists");
    assert_eq!(target.name, "id");
    assert_eq!(scalar_parameter(program, target, 0), Some(ScalarType::I64));
}

#[test]
fn direct_and_mutual_generic_recursion_use_preallocated_specialization_ids() {
    let direct = lower_source(
        "fn recursive[T](value: T) -> T { return recursive[T](value); } \
         fn root(value: I64) -> I64 { return recursive[I64](value); }",
    );
    let direct_program = direct.as_program();
    let recursive_id = function_id(direct_program, "recursive");
    let recursive = direct_program
        .function(recursive_id)
        .expect("recursive specialization exists");
    let Terminator::Call { function, .. } = recursive.body.blocks[0].terminator else {
        panic!("recursive specialization must call itself");
    };
    assert_eq!(function, recursive_id);

    let mutual = lower_source(
        "fn left[T](value: T) -> T { return right[T](value); } \
         fn right[U](value: U) -> U { return left[U](value); } \
         fn root(value: I64) -> I64 { return left[I64](value); }",
    );
    let mutual_program = mutual.as_program();
    let left_id = function_id(mutual_program, "left");
    let right_id = function_id(mutual_program, "right");
    let left = mutual_program.function(left_id).expect("left exists");
    let right = mutual_program.function(right_id).expect("right exists");
    let Terminator::Call {
        function: left_target,
        ..
    } = left.body.blocks[0].terminator
    else {
        panic!("left must call right");
    };
    let Terminator::Call {
        function: right_target,
        ..
    } = right.body.blocks[0].terminator
    else {
        panic!("right must call left");
    };
    assert_eq!(left_target, right_id);
    assert_eq!(right_target, left_id);
}

#[test]
fn forged_generic_hir_invariants_are_rejected_instead_of_repaired() {
    let mut invalid_slot = hir(
        "fn id[T](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return id[I64](value); }",
    );
    let root_id = invalid_slot
        .functions
        .iter()
        .find(|function| function.name == "root")
        .expect("root exists")
        .id;
    invalid_slot
        .functions
        .iter_mut()
        .find(|function| function.name == "id")
        .expect("id exists")
        .type_parameters[0]
        .id
        .function = root_id;
    assert!(matches!(
        lower(&invalid_slot),
        Err(LoweringError::InvalidHirInvariant(_))
    ));

    let mut invalid_arity = hir(
        "fn id[T](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return id[I64](value); }",
    );
    let root = invalid_arity
        .functions
        .iter_mut()
        .find(|function| function.name == "root")
        .expect("root exists");
    let call = root
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("root returns a direct call");
    let ValueKind::DirectCall { type_arguments, .. } = &mut call.kind else {
        panic!("root must retain a direct call");
    };
    type_arguments.clear();
    assert!(matches!(
        lower(&invalid_arity),
        Err(LoweringError::InvalidHirInvariant(_))
    ));

    let mut unresolved_abstract = hir(
        "fn generic[T](value: T) {} \
         fn root(value: I64) {}",
    );
    let slot = unresolved_abstract
        .functions
        .iter()
        .find(|function| function.name == "generic")
        .expect("generic exists")
        .type_parameters[0]
        .id;
    unresolved_abstract
        .functions
        .iter_mut()
        .find(|function| function.name == "root")
        .expect("root exists")
        .parameters[0]
        .ty = Type::Parameter(slot);
    assert!(matches!(
        lower(&unresolved_abstract),
        Err(LoweringError::InvalidHirInvariant(_))
    ));

    let mut non_concrete_tuple = hir(
        "fn id[T](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return id[I64](value); }",
    );
    let root = non_concrete_tuple
        .functions
        .iter_mut()
        .find(|function| function.name == "root")
        .expect("root exists");
    let call = root
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("root returns a direct call");
    let ValueKind::DirectCall { type_arguments, .. } = &mut call.kind else {
        panic!("root must retain a direct call");
    };
    type_arguments[0] = Type::SafeReference {
        referent: ReferenceReferent::Intrinsic(IntrinsicType::I64),
        permission: ReferencePermission::Shared,
    };
    assert!(matches!(
        lower(&non_concrete_tuple),
        Err(LoweringError::InvalidHirInvariant(_))
    ));
}
