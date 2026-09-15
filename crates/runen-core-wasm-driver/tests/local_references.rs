use runen_core_ir::Value;
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
fn projected_exclusive_replace_move_and_reinitialize_executes_through_driver() {
    let outcome = execute(
        "record Ticket { value: I64 }\
         record Holder { ticket: Ticket, other: I64 }\
         fn entry() -> I64 {\
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
             return holder.ticket.value;\
         }",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(73))));
}
