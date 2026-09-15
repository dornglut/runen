use runen_core_ir::Value;
use runen_core_wasm::{ExecutionOutcome, RealizationError};
use runen_core_wasm_driver::{BuildError, ExecutionError, RealizedCompilation};
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

fn execute_named(source: &str, name: &str) -> ExecutionOutcome {
    let compilation = compilation(source);
    let selected = function(&compilation, name);
    RealizedCompilation::new(&compilation)
        .expect("accepted composition test must realize")
        .execute(selected)
        .expect("selected composition test function must execute")
}

#[test]
fn caller_selects_non_first_hir_function_without_core_order_knowledge() {
    let source = "fn first() -> I64 { return 11; } fn second() -> I64 { return 22; }";
    let compilation = compilation(source);
    let second = function(&compilation, "second");
    let realized = RealizedCompilation::new(&compilation).expect("program must realize");

    assert_eq!(
        realized.execute(second).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(22)))
    );
}

#[test]
fn declaration_reordering_does_not_change_hir_selected_behavior() {
    let first_order =
        "fn first() -> I64 { return 11; } fn selected() -> I64 { return 37; }";
    let second_order =
        "fn selected() -> I64 { return 37; } fn first() -> I64 { return 11; }";

    assert_eq!(
        execute_named(first_order, "selected"),
        ExecutionOutcome::Returned(Some(Value::I64(37)))
    );
    assert_eq!(
        execute_named(second_order, "selected"),
        ExecutionOutcome::Returned(Some(Value::I64(37)))
    );
}

#[test]
fn selected_ordinary_function_may_call_another_ordinary_function() {
    let outcome = execute_named(
        "fn helper() -> I64 { return 40; } fn entry() -> I64 { return helper() + 2; }",
        "entry",
    );

    assert_eq!(outcome, ExecutionOutcome::Returned(Some(Value::I64(42))));
}

#[test]
fn internal_generic_specialization_is_realized_but_generic_function_is_not_selectable() {
    let compilation = compilation(
        "fn id[T](value: T) -> T { return value; } \
         fn entry() -> I64 { return id[I64](9); }",
    );
    let generic = function(&compilation, "id");
    let entry = function(&compilation, "entry");
    let realized = RealizedCompilation::new(&compilation).expect("program must realize");

    assert_eq!(
        realized.execute(entry).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(9)))
    );
    assert_eq!(
        realized.execute(generic),
        Err(ExecutionError::FunctionNotSelectable(generic))
    );
}

#[test]
fn identity_absent_from_current_compilation_fails_without_global_provenance() {
    let foreign = compilation("fn first() -> I64 { return 1; } fn second() -> I64 { return 2; }");
    let absent = function(&foreign, "second");

    let current = compilation("fn only() -> I64 { return 7; }");
    let realized = RealizedCompilation::new(&current).expect("current program must realize");

    assert_eq!(
        realized.execute(absent),
        Err(ExecutionError::FunctionNotSelectable(absent))
    );
}

#[test]
fn external_declarations_are_rejected_before_provider_composition_is_exposed() {
    let compilation = compilation(
        "external fn transform(I64) -> I64; fn entry() -> I64 { return 1; }",
    );

    assert!(matches!(
        RealizedCompilation::new(&compilation),
        Err(BuildError::ExternalDeclarationsUnsupported)
    ));
}

#[test]
fn parameterized_entry_rejection_remains_owned_by_core_wasm() {
    let compilation = compilation("fn selected(value: I64) -> I64 { return value; }");
    let selected = function(&compilation, "selected");
    let realized = RealizedCompilation::new(&compilation).expect("program must realize");

    assert!(matches!(
        realized.execute(selected),
        Err(ExecutionError::Realization(
            RealizationError::EntryHasParameters(_)
        ))
    ));
}

#[test]
fn floating_result_observation_rejections_remain_owned_by_core_wasm() {
    let scalar = compilation("fn selected() -> F32 { return 1.0; }");
    let scalar_selected = function(&scalar, "selected");
    let scalar_realized = RealizedCompilation::new(&scalar).expect("scalar program must realize");
    assert!(matches!(
        scalar_realized.execute(scalar_selected),
        Err(ExecutionError::Realization(
            RealizationError::EntryResultUnsupported(_)
        ))
    ));

    let aggregate = compilation(
        "record Sample { value: F32 } \
         fn selected() -> Sample { return Sample { value: 1.0 }; }",
    );
    let aggregate_selected = function(&aggregate, "selected");
    let aggregate_realized =
        RealizedCompilation::new(&aggregate).expect("aggregate program must realize");
    assert!(matches!(
        aggregate_realized.execute(aggregate_selected),
        Err(ExecutionError::Realization(
            RealizationError::EntryResultUnsupported(_)
        ))
    ));
}
