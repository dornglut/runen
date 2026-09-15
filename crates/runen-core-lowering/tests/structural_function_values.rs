use runen_core_ir::{FunctionId, ScalarType, Terminator, TypeId, TypeKind, ValidatedProgram};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_reference::{Machine, ObservedValue, TerminalStatus};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("structural function-value source must produce accepted HIR");
    lower(&hir)
        .expect("accepted structural function-value HIR must lower to validated Core")
        .into_program()
}

fn function_id(program: &runen_core_ir::Program, name: &str) -> FunctionId {
    let index = program
        .functions
        .iter()
        .position(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"));
    FunctionId(u32::try_from(index).expect("test function index fits u32"))
}

fn type_id(program: &runen_core_ir::Program, name: &str) -> TypeId {
    let index = (0..program.types.len())
        .find(|index| {
            let id = TypeId(u32::try_from(*index).expect("test type index fits u32"));
            program.types.get(id).is_some_and(|ty| ty.name == name)
        })
        .unwrap_or_else(|| panic!("missing Core type {name}"));
    TypeId(u32::try_from(index).expect("test type index fits u32"))
}

#[test]
fn record_parameter_and_result_function_value_lowers_and_executes() {
    let source = "record Pair { left: I64, right: I64 } \
         fn identity(value: Pair) -> Pair { return value; } \
         fn entry() -> I64 { \
             let f: fn(Pair) -> Pair = identity; \
             let pair: Pair = f(Pair { left: 20, right: 22 }); \
             return pair.left + pair.right; \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let pair = type_id(program, "Pair");
    let TypeKind::Struct(fields) = &program.types.get(pair).expect("Pair type exists").kind else {
        panic!("source record must lower to structural Core storage");
    };
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().all(|field| matches!(
        program.types.get(field.ty).map(|ty| &ty.kind),
        Some(TypeKind::Scalar(ScalarType::I64))
    )));

    let entry = &program.functions[function_id(program, "entry").0 as usize];
    let (callable, destination) = entry
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Terminator::IndirectCall {
                callable,
                arguments,
                destination,
                ..
            } => {
                assert_eq!(arguments.len(), 1);
                Some((*callable, destination.clone()))
            }
            _ => None,
        })
        .expect("record-bearing source function value must lower to Core IndirectCall");
    assert!(destination.is_some());
    let TypeKind::Scalar(ScalarType::Callable(interface)) = &program
        .types
        .get(callable)
        .expect("callable type exists")
        .kind
    else {
        panic!("indirect call must use a Core callable type");
    };
    assert_eq!(interface.parameters, vec![pair]);
    assert_eq!(interface.result, Some(pair));

    let entry_id = function_id(program, "entry");
    let report = Machine::new(lowered, entry_id)
        .expect("entry has no parameters")
        .execute()
        .expect("record-bearing function-value execution is defined");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}
