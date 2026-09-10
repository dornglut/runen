use std::collections::BTreeSet;

use runen_core_ir::{
    CallableInterface, Function, FunctionId, LocalId, Operand, PlaceAccess, SafeReferenceResultContract,
    ScalarType, Statement as CoreStatement, Terminator, TypeId, TypeKind, ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_reference::{ExecutionReport, Machine, ObservedValue, TerminalStatus};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("function-value source must produce accepted HIR");
    lower(&hir).expect("accepted function-value HIR must lower to validated Core")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a Function {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn function_id(program: &runen_core_ir::Program, name: &str) -> FunctionId {
    let index = program
        .functions
        .iter()
        .position(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"));
    FunctionId(u32::try_from(index).expect("test function index fits u32"))
}

fn local_named(function: &Function, name: &str) -> LocalId {
    let index = function
        .body
        .locals
        .iter()
        .position(|local| local.name == name)
        .unwrap_or_else(|| panic!("missing Core local {name}"));
    LocalId(u32::try_from(index).expect("test local index fits u32"))
}

fn callable_types(program: &runen_core_ir::Program) -> Vec<(TypeId, CallableInterface)> {
    (0..program.types.len())
        .filter_map(|index| {
            let id = TypeId(u32::try_from(index).expect("test type index fits u32"));
            let definition = program.types.get(id).expect("enumerated Core type exists");
            match &definition.kind {
                TypeKind::Scalar(ScalarType::Callable(interface)) => Some((id, interface.clone())),
                TypeKind::Scalar(_) | TypeKind::Struct(_) => None,
            }
        })
        .collect()
}

fn execute_source(source: &str, entry_name: &str) -> ExecutionReport {
    let lowered = lower_source(source);
    let entry = function_id(lowered.as_program(), entry_name);
    Machine::new(lowered, entry)
        .expect("test entry has no parameters")
        .execute()
        .expect("lowered function-value execution is defined")
}

#[test]
fn equal_source_function_types_share_one_core_callable_type_but_function_payloads_remain_distinct() {
    let source =
        "fn left(value: I64) -> I64 { return value + 1; } \
         fn right(value: I64) -> I64 { return value + 2; } \
         fn entry() -> I64 { \
             let mut f: fn(I64) -> I64 = left; \
             let g: fn(I64) -> I64 = right; \
             f = g; \
             return f(40); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    assert_eq!(callable_types(program).len(), 1);

    let entry = function(program, "entry");
    let payloads = entry
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(|statement| match statement {
            CoreStatement::Init {
                src: Operand::FunctionValue(function),
                ..
            } => Some(*function),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(payloads.len(), 2);
    assert_ne!(payloads[0], payloads[1]);
    assert_eq!(
        payloads
            .iter()
            .map(|id| program.function(*id).expect("function value target exists").name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["left", "right"])
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn used_nested_source_function_types_map_recursively_without_materializing_unrelated_types() {
    let lowered = lower_source(
        "fn apply(f: fn(I64) -> I64, value: I64) -> I64 { return f(value); } \
         fn higher(h: fn(fn(I64) -> I64) -> I64) {}",
    );
    let program = lowered.as_program();
    let callables = callable_types(program);
    assert_eq!(callables.len(), 2);

    let (outer_id, outer) = callables
        .iter()
        .find(|(_, interface)| {
            interface.parameters.len() == 1
                && matches!(
                    program.types.get(interface.parameters[0]).map(|ty| &ty.kind),
                    Some(TypeKind::Scalar(ScalarType::Callable(_)))
                )
        })
        .expect("one callable interface consumes the nested callable type");
    let inner_id = outer.parameters[0];
    assert_ne!(*outer_id, inner_id);
    assert!(callables.iter().any(|(id, _)| *id == inner_id));
    assert_eq!(outer.result, Some(
        function(program, "higher")
            .result
            .expect("higher callable parameter's interface returns I64")
    ));
}

#[test]
fn function_values_transport_through_locals_parameters_and_results_using_existing_call_relations() {
    let source =
        "fn plus_one(value: I64) -> I64 { return value + 1; } \
         fn pass(f: fn(I64) -> I64) -> fn(I64) -> I64 { return f; } \
         fn entry() -> I64 { \
             let f: fn(I64) -> I64 = plus_one; \
             let g: fn(I64) -> I64 = pass(f); \
             return g(41); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let pass_id = function_id(program, "pass");
    let entry = function(program, "entry");
    assert!(entry.body.blocks.iter().any(|block| matches!(
        block.terminator,
        Terminator::Call { function, .. } if function == pass_id
    )));
    assert!(entry.body.blocks.iter().any(|block| matches!(
        block.terminator,
        Terminator::IndirectCall { destination: Some(_), .. }
    )));

    let pass = function(program, "pass");
    assert_eq!(pass.parameter_type(0), pass.result);

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn indirect_callee_snapshot_precedes_nested_arguments_and_arguments_remain_left_to_right() {
    let source =
        "fn combine(left: I64, right: I64) -> I64 { return left + right; } \
         fn left() -> I64 { return 20; } \
         fn right() -> I64 { return 22; } \
         fn entry() -> I64 { \
             let f: fn(I64, I64) -> I64 = combine; \
             return f(left(), right()); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let entry = function(program, "entry");
    let f = local_named(entry, "f");
    let entry_block = &entry.body.blocks[entry.body.entry.0 as usize];

    let held_callee = entry_block
        .statements
        .iter()
        .find_map(|statement| match statement {
            CoreStatement::Init {
                dst,
                src: Operand::Copy(PlaceAccess::Direct(source)),
            } if source.local == f
                && source.projections.is_empty()
                && dst.projections.is_empty() => Some(dst.local),
            _ => None,
        })
        .expect("callee binding is copied into an operation-owned temporary before arguments");

    let Terminator::Call {
        function: first,
        target: after_first,
        ..
    } = &entry_block.terminator
    else {
        panic!("first nested argument must be the first emitted call");
    };
    assert_eq!(program.function(*first).expect("first target exists").name, "left");

    let second_block = &entry.body.blocks[after_first.0 as usize];
    let Terminator::Call {
        function: second,
        target: after_second,
        ..
    } = &second_block.terminator
    else {
        panic!("second nested argument must follow the first call continuation");
    };
    assert_eq!(program.function(*second).expect("second target exists").name, "right");

    let call_block = &entry.body.blocks[after_second.0 as usize];
    let Terminator::IndirectCall {
        callee,
        arguments,
        destination,
        ..
    } = &call_block.terminator
    else {
        panic!("indirect call must follow both nested argument producers");
    };
    assert_eq!(arguments.len(), 2);
    assert!(destination.is_some());
    assert!(matches!(
        callee,
        Operand::Move(PlaceAccess::Direct(place))
            if place.local == held_callee && place.projections.is_empty()
    ));

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn no_result_indirect_call_has_no_destination_and_uses_the_existing_normal_continuation() {
    let source =
        "fn sink(value: I64) {} \
         fn entry() -> I64 { \
             let f: fn(I64) = sink; \
             f(1); \
             return 9; \
         }";
    let lowered = lower_source(source);
    let entry = function(lowered.as_program(), "entry");
    let indirect = entry.body.blocks.iter().find_map(|block| match &block.terminator {
        Terminator::IndirectCall {
            destination,
            target,
            ..
        } => Some((destination, target)),
        _ => None,
    });
    let Some((destination, target)) = indirect else {
        panic!("entry must contain one indirect no-result call");
    };
    assert!(destination.is_none());
    assert!((target.0 as usize) < entry.body.blocks.len());

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(9)));
}

#[test]
fn indirect_fault_propagates_through_the_existing_reference_machine() {
    let source =
        "fn boom() -> I64 { fault; } \
         fn entry() -> I64 { \
             let f: fn() -> I64 = boom; \
             return f(); \
         }";
    let report = execute_source(source, "entry");
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("source.explicit".to_owned())
    );
    assert_eq!(report.result, None);
}

#[test]
fn indirect_shared_identity_and_direct_child_contracts_execute_through_existing_reference_summaries() {
    let identity_source =
        "fn identity(reference: &I64) -> &I64 { return reference; } \
         fn entry() -> I64 { \
             let value: I64 = 73; \
             let reference: &I64 = &value; \
             let f: fn(&I64) -> &I64 = identity; \
             let returned: &I64 = f(reference); \
             return *returned; \
         }";
    let identity = lower_source(identity_source);
    let identity_program = identity.as_program();
    let identity_callable = callable_types(identity_program);
    assert_eq!(identity_callable.len(), 1);
    assert_eq!(
        identity_callable[0].1.safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    let report = execute_source(identity_source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(73)));

    let child_source =
        "fn child(reference: &mut I64) -> &I64 { return &*reference; } \
         fn entry() -> I64 { \
             let mut value: I64 = 84; \
             let reference: &mut I64 = &mut value; \
             let f: fn(&mut I64) -> &I64 = child; \
             let returned: &I64 = f(reference); \
             return *returned; \
         }";
    let child = lower_source(child_source);
    let child_program = child.as_program();
    let child_callable = callable_types(child_program);
    assert_eq!(child_callable.len(), 1);
    assert_eq!(
        child_callable[0].1.safe_reference_result_contract,
        SafeReferenceResultContract::SharedDirectChild { origin: 0 }
    );
    let report = execute_source(child_source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(84)));
}

#[test]
fn mutual_indirect_call_graph_lowers_without_dynamic_target_set_discovery() {
    let lowered = lower_source(
        "fn left(value: I64) -> I64 { \
             let f: fn(I64) -> I64 = right; \
             return f(value); \
         } \
         fn right(value: I64) -> I64 { \
             let f: fn(I64) -> I64 = left; \
             return f(value); \
         }",
    );
    let program = lowered.as_program();
    for name in ["left", "right"] {
        let function = function(program, name);
        assert_eq!(
            function
                .body
                .blocks
                .iter()
                .filter(|block| matches!(block.terminator, Terminator::IndirectCall { .. }))
                .count(),
            1,
            "{name} must lower one bounded indirect recursive edge"
        );
    }
    assert_eq!(program.functions.len(), 2, "no target-set specialization is introduced");
}
