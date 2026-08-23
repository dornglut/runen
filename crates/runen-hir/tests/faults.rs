use runen_hir::{DiagnosticKind, ModuleId, SourceUnit, Statement, TypedCompilation, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn retains_payload_free_root_fault_and_no_normal_continuation() {
    let hir = build("fn f() { fault; }").expect("payload-free fault is valid");
    let f = function(&hir, "f");

    assert!(!f.body.has_normal_continuation);
    assert!(f.body.terminal_return.is_none());
    let [Statement::Fault { location }] = f.body.statements.as_slice() else {
        panic!("expected one retained fault statement");
    };
    assert_eq!(location.unit, 0);
}

#[test]
fn result_bearing_function_may_end_only_by_fault() {
    let hir = build("fn f() -> I64 { fault; }").expect("abnormal path needs no result value");
    let f = function(&hir, "f");

    assert!(!f.body.has_normal_continuation);
    assert!(f.body.terminal_return.is_none());
    assert!(matches!(f.body.statements.as_slice(), [Statement::Fault { .. }]));
}

#[test]
fn nested_faulting_block_has_no_normal_cleanup() {
    let hir = build("record Ticket {} fn make() -> Ticket { return Ticket {}; } fn f() { { let ticket: Ticket = make(); fault; } }")
        .expect("faulting nested block is valid");
    let f = function(&hir, "f");
    let [Statement::Block(block)] = f.body.statements.as_slice() else {
        panic!("expected one nested block");
    };

    assert!(!block.has_normal_continuation);
    assert!(block.normal_cleanup.is_empty());
    assert!(matches!(block.statements.last(), Some(Statement::Fault { .. })));
}

#[test]
fn statement_after_fault_is_unreachable_and_not_semantically_validated() {
    let errors = build("fn f() { fault; missing(); }")
        .expect_err("following statement must be unreachable");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnreachableStatement));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnresolvedName),
        "unreachable sibling must not be validated as an ordinary continuation"
    );
}

#[test]
fn terminal_return_after_fault_is_unreachable() {
    let errors = build("fn f() { fault; return; }")
        .expect_err("terminal return after fault must be unreachable");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnreachableStatement));
}

#[test]
fn ownership_transition_before_fault_remains_accepted() {
    build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { sink(value); fault; }",
    )
    .expect("fault itself performs no new source ownership use");
}

#[test]
fn omitted_else_is_the_sole_normal_outcome_after_faulting_then_arm() {
    build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { if flag { fault; } sink(value); }",
    )
    .expect("false outcome keeps the unchanged post-condition ownership state");
}

#[test]
fn explicit_fault_and_normal_arm_use_only_the_normal_arm_state() {
    build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { \
             if flag { fault; } else {} \
             sink(value); \
         }",
    )
    .expect("faulting arm is excluded from normal ownership comparison");
}

#[test]
fn fault_fault_and_return_fault_conditionals_have_zero_normal_continuation() {
    let both_fault = build(
        "fn f(flag: Bool) -> I64 { if flag { fault; } else { fault; } }",
    )
    .expect("two faulting arms complete result-bearing function abnormally");
    assert!(!function(&both_fault, "f").body.has_normal_continuation);

    let mixed = build(
        "fn f(flag: Bool) -> I64 { if flag { return 1; } else { fault; } }",
    )
    .expect("return/fault arms have zero normal outcome");
    assert!(!function(&mixed, "f").body.has_normal_continuation);
}

#[test]
fn call_to_faulting_callee_remains_statically_normally_continuable() {
    let hir = build("fn die() { fault; } fn f() { die(); let x: I64 = 1; }")
        .expect("callee fault possibility is dynamic, not interprocedural completion inference");
    let f = function(&hir, "f");

    assert!(f.body.has_normal_continuation);
    assert!(matches!(f.body.statements[0], Statement::Call { .. }));
    assert!(matches!(f.body.statements[1], Statement::Local { .. }));
}
