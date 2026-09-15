use runen_core_ir::{Fault, Value};
use runen_core_wasm::ExecutionOutcome;
use runen_core_wasm_driver::RealizedCompilation;
use runen_hir::{FunctionId, ModuleId, SourceUnit, TypedCompilation, build_typed_hir};
use runen_syntax::parse_source;

fn compilation(source: &str) -> TypedCompilation {
    let parsed = parse_source(source.as_bytes()).expect("test source must be valid UTF-8");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted typed HIR")
}

fn function(compilation: &TypedCompilation, name: &str) -> FunctionId {
    compilation
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
        .id
}

fn execute(source: &str) -> ExecutionOutcome {
    let compilation = compilation(source);
    let entry = function(&compilation, "entry");
    RealizedCompilation::new(&compilation)
        .expect("accepted activation-local raw-pointer program must realize")
        .execute(entry)
        .expect("activation-local raw-pointer entry must execute")
}

#[test]
fn scalar_raw_move_executes_through_existing_driver() {
    let outcome = execute(
        "fn entry() -> I64 {\
             let x: I64 = 41;\
             let p: raw I64 = raw &x;\
             let mut result: I64 = 0;\
             unsafe { result = raw move p; }\
             return result;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(41))));
}

#[test]
fn pointer_retarget_and_copy_select_the_exact_source_binding() {
    let outcome = execute(
        "fn entry() -> I64 {\
             let x: I64 = 11;\
             let y: I64 = 17;\
             let mut p: raw I64 = raw &x;\
             p = raw &y;\
             let q: raw I64 = p;\
             unsafe { raw assign q = 29; }\
             return x + y;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(40))));
}

#[test]
fn raw_assign_from_raw_move_round_trips_through_driver() {
    let outcome = execute(
        "fn entry() -> I64 {\
             let x: I64 = 53;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = raw move p; }\
             return x;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(53))));
}

#[test]
fn raw_assign_restores_an_unavailable_aggregate_root() {
    let outcome = execute(
        "record Ticket { value: I64 }\
         fn take(value: Ticket) {}\
         fn entry() -> I64 {\
             let ticket: Ticket = Ticket { value: 61 };\
             let p: raw Ticket = raw &ticket;\
             take(ticket);\
             unsafe { raw assign p = Ticket { value: 67 }; }\
             return ticket.value;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(67))));
}

#[test]
fn raw_assign_restores_a_partially_moved_aggregate_root() {
    let outcome = execute(
        "record Ticket { value: I64 }\
         record Pair { left: Ticket, right: Ticket }\
         fn take(value: Ticket) {}\
         fn entry() -> I64 {\
             let pair: Pair = Pair {\
                 left: Ticket { value: 1 },\
                 right: Ticket { value: 2 }\
             };\
             let p: raw Pair = raw &pair;\
             take(pair.left);\
             unsafe {\
                 raw assign p = Pair {\
                     left: Ticket { value: 3 },\
                     right: Ticket { value: 71 }\
                 };\
             }\
             return pair.right.value;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(71))));
}

#[test]
fn raw_assign_call_result_writes_only_after_normal_return() {
    let outcome = execute(
        "fn produce() -> I64 { return 7; }\
         fn entry() -> I64 {\
             let x: I64 = 1;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = produce(); }\
             return x;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(7))));
}

#[test]
fn raw_assign_faulting_rhs_never_executes_the_write_continuation() {
    let outcome = execute(
        "fn fail() -> I64 { fault; }\
         fn entry() -> I64 {\
             let x: I64 = 5;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = fail(); }\
             return x;\
         }",
    );

    assert_eq!(
        outcome,
        ExecutionOutcome::Faulted(Fault::new("source.explicit"))
    );
}

#[test]
fn caller_pointer_survives_nested_raw_pointer_activation() {
    let outcome = execute(
        "fn helper() -> I64 {\
             let y: I64 = 5;\
             let q: raw I64 = raw &y;\
             let mut result: I64 = 0;\
             unsafe { result = raw move q; }\
             return result;\
         }\
         fn entry() -> I64 {\
             let x: I64 = 41;\
             let p: raw I64 = raw &x;\
             let nested: I64 = helper();\
             let mut local: I64 = 0;\
             unsafe { local = raw move p; }\
             return nested + local;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(46))));
}
