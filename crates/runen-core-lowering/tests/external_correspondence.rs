use runen_core_lowering::lower;
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

#[test]
fn lowering_preserves_distinct_external_hir_identity_correspondence() {
    let compilation = compilation(
        "external fn first(I64) -> I64; \
         fn ordinary() -> I64 { return 0; } \
         external fn second(I64) -> I64;",
    );
    let first = function(&compilation, "first");
    let second = function(&compilation, "second");
    let ordinary = function(&compilation, "ordinary");

    let lowered = lower(&compilation).expect("accepted HIR must lower");
    let first_core = lowered
        .core_external_callable(first)
        .expect("first external declaration must retain correspondence");
    let second_core = lowered
        .core_external_callable(second)
        .expect("second external declaration must retain correspondence");

    assert_ne!(first_core, second_core);
    assert_eq!(lowered.core_external_callable(ordinary), None);
    assert!(lowered.core_function(first).is_none());
    assert!(lowered.core_function(second).is_none());
    assert!(lowered.core_function(ordinary).is_some());
}
