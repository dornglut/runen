use runen_core_ir::{Fault, Value};
use runen_core_wasm::{ExecutionOutcome, RealizationError};
use runen_core_wasm_driver::{BuildError, RealizedCompilation};
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
        .expect("accepted activation-local reference program must realize")
        .execute(entry)
        .expect("activation-local reference entry must execute")
}

#[test]
fn projected_shared_root_executes_through_driver() {
    let outcome = execute(
        "record copy Pair { left: I64, right: I64 }\
         fn entry() -> I64 {\
             let root: Pair = Pair { left: 83, right: 17 };\
             let r: &I64 = &root.left;\
             return *r;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(83))));
}

#[test]
fn projected_shared_reborrow_executes_through_driver() {
    let outcome = execute(
        "record copy Pair { left: I64, right: I64 }\
         fn entry() -> I64 {\
             let root: Pair = Pair { left: 83, right: 17 };\
             let parent: &Pair = &root;\
             let child: &I64 = &*parent.right;\
             return *child;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(17))));
}

#[test]
fn nested_reference_cleanup_and_defined_fault_execute_through_driver() {
    let outcome = execute(
        "fn entry() {\
             let x: I64 = 5;\
             {\
                 let r: &I64 = &x;\
                 {\
                     let s: &I64 = r;\
                     let value: I64 = *s;\
                     fault;\
                 }\
             }\
         }",
    );

    assert_eq!(
        outcome,
        ExecutionOutcome::Faulted(Fault::new("source.explicit"))
    );
}

#[test]
fn projected_exclusive_replace_move_and_reinitialize_executes_through_driver() {
    let outcome = execute(
        "record Ticket { value: I64 }\
         record Holder { ticket: Ticket, other: I64 }\
         record copy Snapshot { updated: I64, untouched: I64 }\
         fn entry() -> Snapshot {\
             let mut holder: Holder = Holder {\
                 ticket: Ticket { value: 11 },\
                 other: 5\
             };\
             {\
                 let root: &mut Holder = &mut holder;\
                 {\
                     let child: &mut Ticket = &mut *root.ticket;\
                     let old: Ticket = *child;\
                     *child = Ticket { value: 73 };\
                 }\
             }\
             return Snapshot {\
                 updated: holder.ticket.value,\
                 untouched: holder.other\
             };\
         }",
    );

    assert_eq!(
        outcome,
        ExecutionOutcome::Returned(Some(Value::Struct(vec![Value::I64(73), Value::I64(5),])))
    );
}

#[test]
fn persistent_shared_static_root_executes_through_existing_driver() {
    let outcome = execute(
        "static VALUE: I64 = 89;\
         fn entry() -> I64 {\
             let r: &I64 = &VALUE;\
             return *r;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(89))));
}

#[test]
fn persistent_shared_static_root_does_not_widen_reference_call_transfer() {
    let compilation = compilation(
        "static VALUE: I64 = 97;\
         fn read(r: &I64) -> I64 { return *r; }\
         fn entry() -> I64 { return read(&VALUE); }",
    );

    assert!(matches!(
        RealizedCompilation::new(&compilation),
        Err(BuildError::Realization(RealizationError::Coverage(_)))
    ));
}
