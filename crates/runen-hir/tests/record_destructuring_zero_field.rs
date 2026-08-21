use runen_hir::{ModuleId, SourceUnit, Statement, build_typed_hir};
use runen_syntax::parse_source;

#[test]
fn zero_field_pattern_does_not_require_whole_root_availability() {
    let source = "record Empty {} fn take(value: Empty) {} fn f(root: Empty) { take(root); let Empty {} = root; }";
    let parsed = parse_source(source.as_bytes()).expect("valid UTF-8 test source");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("zero-field pattern is an ownership no-op after whole-root consumption");
    let function = hir
        .functions
        .iter()
        .find(|function| function.name == "f")
        .expect("test function");

    let Statement::RecordDestructure { bindings, .. } = &function.body.statements[1] else {
        panic!("expected zero-field record destructuring");
    };
    assert!(bindings.is_empty());
}
