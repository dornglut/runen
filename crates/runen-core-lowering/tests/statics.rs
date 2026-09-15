use runen_core_ir::{FunctionId, Operand, PersistentId, Statement, ValidatedProgram, Value};
use runen_core_lowering::lower;
use runen_hir::{ImportTarget, ModuleId, SourceUnit, build_typed_hir};
use runen_reference::{Machine, ObservedValue, TerminalStatus};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("static test source must produce accepted HIR");
    lower(&hir)
        .expect("accepted static HIR must lower to validated Core")
        .into_program()
}

fn lower_qualified(dependency: &str, caller: &str) -> ValidatedProgram {
    let dependency = parse(dependency);
    let caller = parse(caller);
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("qualified static source must produce accepted HIR");
    lower(&hir)
        .expect("accepted qualified static HIR must lower to validated Core")
        .into_program()
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a runen_core_ir::Function {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn execute_source(source: &str, entry_name: &str) -> runen_reference::ExecutionReport {
    let lowered = lower_source(source);
    let entry_index = lowered
        .as_program()
        .functions
        .iter()
        .position(|function| function.name == entry_name)
        .unwrap_or_else(|| panic!("missing Core entry function {entry_name}"));
    let entry = FunctionId(u32::try_from(entry_index).expect("test function index fits u32"));
    Machine::new(lowered, entry)
        .expect("static test entry has no parameters")
        .execute()
        .expect("lowered static execution is defined")
}

fn operands(function: &runen_core_ir::Function) -> Vec<&Operand> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| match statement {
            Statement::Init { src, .. } | Statement::Assign { src, .. } => Some(src),
            _ => None,
        })
        .collect()
}

#[test]
fn one_hir_static_becomes_one_exact_persistent_declaration_and_read() {
    let lowered = lower_source("static VALUE: I64 = -42; fn read() -> I64 { return VALUE; }");
    let program = lowered.as_program();
    assert_eq!(program.persistent.len(), 1);
    assert_eq!(program.persistent[0].initial, Value::I64(-42));

    assert!(
        operands(function(program, "read"))
            .iter()
            .any(|operand| matches!(operand, Operand::PersistentRead(PersistentId(0))))
    );
}

#[test]
fn same_valued_source_statics_lower_to_distinct_persistent_identities() {
    let lowered = lower_source(
        "static LEFT: I64 = 7; static RIGHT: I64 = 7; \
         fn left() -> I64 { return LEFT; } \
         fn right() -> I64 { return RIGHT; }",
    );
    let program = lowered.as_program();
    assert_eq!(program.persistent.len(), 2);
    assert_eq!(program.persistent[0].initial, Value::I64(7));
    assert_eq!(program.persistent[1].initial, Value::I64(7));
    assert!(
        operands(function(program, "left"))
            .iter()
            .any(|operand| matches!(operand, Operand::PersistentRead(PersistentId(0))))
    );
    assert!(
        operands(function(program, "right"))
            .iter()
            .any(|operand| matches!(operand, Operand::PersistentRead(PersistentId(1))))
    );
}

#[test]
fn qualified_static_read_keeps_declaration_identity_without_module_runtime_metadata() {
    let lowered = lower_qualified(
        "export static ANSWER: U64 = 18446744073709551615;",
        "import dep; fn entry() -> U64 { return dep::ANSWER; }",
    );
    let program = lowered.as_program();
    assert_eq!(program.persistent.len(), 1);
    assert_eq!(program.persistent[0].initial, Value::U64(u64::MAX));
    assert!(
        operands(function(program, "entry"))
            .iter()
            .any(|operand| matches!(operand, Operand::PersistentRead(PersistentId(0))))
    );
}

#[test]
fn shared_static_root_lowers_to_persistent_root_and_identity_roundtrip_executes() {
    let source = "static VALUE: I64 = 42; \
         fn identity(value: &I64) -> &I64 { return value; } \
         fn entry() -> I64 { \
             let root: &I64 = &VALUE; \
             let roundtrip: &I64 = identity(root); \
             return *roundtrip; \
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    assert!(
        operands(function(program, "entry"))
            .iter()
            .any(|operand| matches!(operand, Operand::PersistentSharedRoot(PersistentId(0))))
    );

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn generic_and_closure_static_reads_lower_to_the_same_persistent_identity() {
    let lowered = lower_source(
        "static VALUE: I64 = 9; \
         fn generic[T](ignored: T) -> I64 { return VALUE; } \
         fn entry(dummy: I64) -> I64 { \
             let c = fn[dummy]() -> I64 { return VALUE; }; \
             let via_generic: I64 = generic[I64](dummy); \
             return c() + via_generic; \
         }",
    );
    let program = lowered.as_program();
    assert_eq!(program.persistent.len(), 1);
    let persistent_reads = program
        .functions
        .iter()
        .flat_map(operands)
        .filter(|operand| matches!(operand, Operand::PersistentRead(PersistentId(0))))
        .count();
    assert_eq!(persistent_reads, 2);
}
