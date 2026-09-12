use runen_core_ir::{
    Function, FunctionId, LocalId, Operand, PlaceAccess, Projection, SafeReferenceResultContract,
    Statement as CoreStatement, Terminator, TypeId, TypeKind, ValidatedProgram,
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
        .expect("closure source must produce accepted HIR");
    lower(&hir).expect("accepted closure HIR must lower to validated Core")
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

fn type_id(program: &runen_core_ir::Program, name: &str) -> TypeId {
    let index = (0..program.types.len())
        .find(|index| {
            let id = TypeId(u32::try_from(*index).expect("test type index fits u32"));
            program.types.get(id).is_some_and(|ty| ty.name == name)
        })
        .unwrap_or_else(|| panic!("missing Core type {name}"));
    TypeId(u32::try_from(index).expect("test type index fits u32"))
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

fn execute_source(source: &str, entry_name: &str) -> ExecutionReport {
    let lowered = lower_source(source);
    let entry = function_id(lowered.as_program(), entry_name);
    Machine::new(lowered, entry)
        .expect("test entry has no parameters")
        .execute()
        .expect("lowered closure execution is defined")
}

#[test]
fn one_environment_type_and_wrapper_per_site_preserve_capture_order() {
    let lowered = lower_source(
        "fn entry() -> I64 { \
             let left: I64 = 20; \
             let right: I64 = 22; \
             let add = fn[left, right](value: I64) -> I64 { return value + (left + right); }; \
             return add(0); \
         }",
    );
    let program = lowered.as_program();

    let environment = type_id(program, "$closure-env-0");
    let TypeKind::Struct(fields) = &program.types.get(environment).unwrap().kind else {
        panic!("closure environment must be structural Core storage");
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        vec!["$capture0", "$capture1"]
    );

    let wrapper = function(program, "$closure-wrapper-0");
    assert_eq!(
        wrapper.parameters.len(),
        2,
        "explicit value parameter plus hidden environment"
    );
    assert_eq!(wrapper.parameter_type(1), Some(environment));
    let environment_local = wrapper.parameters[1];
    let entry_block = &wrapper.body.blocks[wrapper.body.entry.0 as usize];
    let projections = entry_block
        .statements
        .iter()
        .take(2)
        .map(|statement| match statement {
            CoreStatement::Init {
                src: Operand::Move(PlaceAccess::Direct(place)),
                ..
            } if place.local == environment_local => place.projections.clone(),
            other => panic!("wrapper capture initialization must move from environment: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        projections,
        vec![vec![Projection::Field(0)], vec![Projection::Field(1)]]
    );

    let report = execute_source(
        "fn entry() -> I64 { \
             let left: I64 = 20; \
             let right: I64 = 22; \
             let add = fn[left, right](value: I64) -> I64 { return value + (left + right); }; \
             return add(0); \
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn duplicable_closure_snapshots_by_copy_and_can_execute_repeatedly() {
    let source = "fn entry() -> I64 { \
         let base: I64 = 1; \
         let add = fn[base](value: I64) -> I64 { return value + base; }; \
         let first: I64 = add(20); \
         return add(first); \
     }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let entry = function(program, "entry");
    let closure_local = local_named(entry, "add");
    let snapshots = entry
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter(|statement| {
            matches!(
                statement,
                CoreStatement::Init {
                    src: Operand::Copy(PlaceAccess::Direct(place)),
                    ..
                } if place.local == closure_local && place.projections.is_empty()
            )
        })
        .count();
    assert_eq!(snapshots, 2);

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(22)));
}

#[test]
fn nonduplicable_closure_formation_and_invocation_use_source_selected_moves() {
    let source = "record Box { value: I64 } \
         fn entry() -> I64 { \
             let boxed: Box = Box { value: 41 }; \
             let once = fn[boxed](delta: I64) -> I64 { return boxed.value + delta; }; \
             return once(1); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let entry = function(program, "entry");
    let boxed = local_named(entry, "boxed");
    let once = local_named(entry, "once");

    assert!(
        entry
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|statement| matches!(
                statement,
                CoreStatement::Init {
                    src: Operand::Move(PlaceAccess::Direct(place)),
                    ..
                } if place.local == boxed && place.projections.is_empty()
            ))
    );
    assert!(
        entry
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|statement| matches!(
                statement,
                CoreStatement::Init {
                    src: Operand::Move(PlaceAccess::Direct(place)),
                    ..
                } if place.local == once && place.projections.is_empty()
            ))
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn closure_snapshot_precedes_nested_arguments_and_wrapper_call_is_direct() {
    let source = "fn left() -> I64 { return 20; } \
         fn right() -> I64 { return 22; } \
         fn entry() -> I64 { \
             let base: I64 = 0; \
             let combine = fn[base](left: I64, right: I64) -> I64 { return left + (right + base); }; \
             return combine(left(), right()); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let entry = function(program, "entry");
    let combine = local_named(entry, "combine");
    let entry_block = &entry.body.blocks[entry.body.entry.0 as usize];

    assert!(entry_block.statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(place)),
            ..
        } if place.local == combine && place.projections.is_empty()
    )));
    let Terminator::Call {
        function: left_target,
        target: after_left,
        ..
    } = entry_block.terminator
    else {
        panic!("first nested argument must execute after the closure snapshot");
    };
    assert_eq!(program.function(left_target).unwrap().name, "left");

    let right_block = &entry.body.blocks[after_left.0 as usize];
    let Terminator::Call {
        function: right_target,
        target: after_right,
        ..
    } = right_block.terminator
    else {
        panic!("second nested argument must follow the first");
    };
    assert_eq!(program.function(right_target).unwrap().name, "right");

    let wrapper_block = &entry.body.blocks[after_right.0 as usize];
    let Terminator::Call {
        function: wrapper_target,
        arguments,
        ..
    } = &wrapper_block.terminator
    else {
        panic!("closure call must lower to a direct Core wrapper call");
    };
    assert_eq!(
        program.function(*wrapper_target).unwrap().name,
        "$closure-wrapper-0"
    );
    assert_eq!(
        arguments.len(),
        3,
        "two source arguments plus hidden environment"
    );
    assert!(matches!(arguments.last(), Some(Operand::Move(_))));

    let report = execute_source(source, "entry");
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn hidden_environment_is_final_and_preserves_safe_reference_result_origin_indices() {
    let source = "fn entry() -> I64 { \
         let base: I64 = 1; \
         let value: I64 = 73; \
         let reference: &I64 = &value; \
         let identity = fn[base](reference: &I64) -> &I64 { return reference; }; \
         let returned: &I64 = identity(reference); \
         return *returned; \
     }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let wrapper = function(program, "$closure-wrapper-0");
    let environment = type_id(program, "$closure-env-0");
    assert_eq!(wrapper.parameters.len(), 2);
    assert_ne!(wrapper.parameter_type(0), Some(environment));
    assert_eq!(wrapper.parameter_type(1), Some(environment));
    assert_eq!(
        wrapper.safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(73)));
}

#[test]
fn later_closure_capture_of_earlier_closure_uses_nested_environment_ownership() {
    let source = "record Box { value: I64 } \
         fn entry() -> I64 { \
             let boxed: Box = Box { value: 1 }; \
             let inner = fn[boxed]() {}; \
             let answer: I64 = 42; \
             let outer = fn[inner, answer]() -> I64 { return answer; }; \
             return outer(); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let inner = type_id(program, "$closure-env-0");
    let outer = type_id(program, "$closure-env-1");
    let TypeKind::Struct(fields) = &program.types.get(outer).unwrap().kind else {
        panic!("outer closure environment must be structural");
    };
    assert_eq!(fields[0].ty, inner);
    assert_eq!(
        program
            .function(function_id(program, "$closure-wrapper-0"))
            .unwrap()
            .name,
        "$closure-wrapper-0"
    );
    assert_eq!(
        program
            .function(function_id(program, "$closure-wrapper-1"))
            .unwrap()
            .name,
        "$closure-wrapper-1"
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn closure_call_results_reuse_existing_field_pattern_and_comparison_producer_paths() {
    let field_source = "record copy Pair { value: I64 } \
         fn entry() -> I64 { \
             let seed: I64 = 42; \
             let make = fn[seed]() -> Pair { return Pair { value: seed }; }; \
             return make().value; \
         }";
    let report = execute_source(field_source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));

    let pattern_source = "record copy Pair { value: I64 } \
         fn entry() -> I64 { \
             let seed: I64 = 42; \
             let make = fn[seed]() -> Pair { return Pair { value: seed }; }; \
             let Pair { value: result } = make(); \
             return result; \
         }";
    let report = execute_source(pattern_source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));

    let comparison_source = "fn entry() -> Bool { \
         let seed: I64 = 42; \
         let produce = fn[seed]() -> I64 { return seed; }; \
         return produce() == 42; \
     }";
    let report = execute_source(comparison_source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::Bool(true)));
}

#[test]
fn argument_fault_occurs_after_one_shot_environment_snapshot_and_uses_existing_fault_cleanup() {
    let source = "record Box { value: I64 } \
         fn left() -> I64 { return 1; } \
         fn boom() -> I64 { fault; } \
         fn entry() -> I64 { \
             let boxed: Box = Box { value: 42 }; \
             let once = fn[boxed](left: I64, right: I64) -> I64 { return left + (right + boxed.value); }; \
             return once(left(), boom()); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let entry = function(program, "entry");
    let once = local_named(entry, "once");
    let entry_block = &entry.body.blocks[entry.body.entry.0 as usize];
    let held_environment = entry_block
        .statements
        .iter()
        .find_map(|statement| match statement {
            CoreStatement::Init {
                dst,
                src: Operand::Move(PlaceAccess::Direct(place)),
            } if place.local == once
                && place.projections.is_empty()
                && dst.projections.is_empty() =>
            {
                Some(dst.local)
            }
            _ => None,
        })
        .expect("one-shot environment is held before argument production");
    let Terminator::Call {
        function: left_target,
        destination: Some(left_result),
        target: after_left,
        ..
    } = &entry_block.terminator
    else {
        panic!("first argument producer must execute after environment snapshot");
    };
    assert_eq!(program.function(*left_target).unwrap().name, "left");
    assert!(
        left_result.local.0 > held_environment.0,
        "produced argument temporaries must be declared after the held environment so reverse Core cleanup destroys them first"
    );

    let boom_block = &entry.body.blocks[after_left.0 as usize];
    let Terminator::Call { function, .. } = &boom_block.terminator else {
        panic!("faulting second argument must follow the successful first argument");
    };
    assert_eq!(program.function(*function).unwrap().name, "boom");

    let entry_id = function_id(program, "entry");
    let report = Machine::new(lowered, entry_id)
        .expect("entry has no parameters")
        .execute()
        .expect("argument fault is defined");
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("source.explicit".to_owned())
    );
    assert_eq!(report.result, None);
}

#[test]
fn closure_body_fault_and_no_result_calls_use_existing_core_activation_relations() {
    let no_result = "fn entry() -> I64 { \
         let seed: I64 = 1; \
         let sink = fn[seed](value: I64) {}; \
         sink(42); \
         return 9; \
     }";
    let lowered = lower_source(no_result);
    let entry = function(lowered.as_program(), "entry");
    assert!(entry.body.blocks.iter().any(|block| matches!(
        block.terminator,
        Terminator::Call {
            destination: None,
            ..
        }
    )));
    let report = execute_source(no_result, "entry");
    assert_eq!(report.result, Some(ObservedValue::I64(9)));

    let faulting = "fn entry() -> I64 { \
         let seed: I64 = 1; \
         let fail = fn[seed]() -> I64 { fault; }; \
         return fail(); \
     }";
    let report = execute_source(faulting, "entry");
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("source.explicit".to_owned())
    );
    assert_eq!(report.result, None);
}

#[test]
fn captured_function_value_remains_an_existing_bounded_indirect_target_inside_wrapper() {
    let source = "fn plus_one(value: I64) -> I64 { return value + 1; } \
         fn entry() -> I64 { \
             let f: fn(I64) -> I64 = plus_one; \
             let apply = fn[f](value: I64) -> I64 { return f(value); }; \
             return apply(41); \
         }";
    let lowered = lower_source(source);
    let wrapper = function(lowered.as_program(), "$closure-wrapper-0");
    assert!(
        wrapper
            .body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::IndirectCall { .. }))
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn hidden_environment_preserves_shared_direct_child_result_contract() {
    let source = "fn entry() -> I64 { \
         let seed: I64 = 1; \
         let mut value: I64 = 84; \
         let reference: &mut I64 = &mut value; \
         let child = fn[seed](reference: &mut I64) -> &I64 { return &*reference; }; \
         let returned: &I64 = child(reference); \
         return *returned; \
     }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let wrapper = function(program, "$closure-wrapper-0");
    let environment = type_id(program, "$closure-env-0");
    assert_eq!(wrapper.parameters.len(), 2);
    assert_eq!(wrapper.parameter_type(1), Some(environment));
    assert_eq!(
        wrapper.safe_reference_result_contract,
        SafeReferenceResultContract::SharedDirectChild { origin: 0 }
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(84)));
}

#[test]
fn uninvoked_closure_retains_environment_storage_for_ordinary_reverse_cleanup() {
    let source = "record Box { value: I64 } \
         fn entry() -> I64 { \
             let left: Box = Box { value: 20 }; \
             let right: Box = Box { value: 22 }; \
             let held = fn[left, right]() {}; \
             return 42; \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let environment = type_id(program, "$closure-env-0");
    let TypeKind::Struct(fields) = &program.types.get(environment).unwrap().kind else {
        panic!("uninvoked closure still owns one structural environment");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "$capture0");
    assert_eq!(fields[1].name, "$capture1");

    let entry = function(program, "entry");
    let held = local_named(entry, "held");
    assert_eq!(entry.body.locals[held.0 as usize].ty, environment);
    assert!(
        !entry.body.blocks.iter().any(|block| matches!(
            block.terminator,
            Terminator::Call { function, .. }
                if program.function(function).is_some_and(|f| f.name == "$closure-wrapper-0")
        )),
        "uninvoked closure must retain its environment until ordinary activation cleanup rather than being eagerly destroyed"
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn generic_direct_call_specialization_is_discovered_from_closure_body() {
    let source = "fn id[T](value: T) -> T { return value; } \
         fn entry() -> I64 { \
             let seed: I64 = 41; \
             let c = fn[seed]() -> I64 { return id[I64](seed + 1); }; \
             return c(); \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let wrapper = function(program, "$closure-wrapper-0");
    assert!(wrapper.body.blocks.iter().any(|block| matches!(
        block.terminator,
        Terminator::Call { function, .. }
            if program.function(function).is_some_and(|target| target.name == "id")
    )));

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}
