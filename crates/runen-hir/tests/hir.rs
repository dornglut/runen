use runen_hir::{
    DiagnosticKind, ModuleId, OwnedUse, SourceUnit, Statement, Type, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], predicate: impl Fn(DiagnosticKind) -> bool) -> bool {
    errors.iter().any(|error| predicate(error.kind))
}

#[test]
fn rejects_syntax_dirty_units_before_semantic_hir() {
    let source = parse("fn broken( {}");
    assert!(!source.errors().is_empty());
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source)])
        .expect_err("syntax-dirty unit must not produce HIR");
    assert!(has_diagnostic(&errors, |kind| matches!(kind, DiagnosticKind::SyntaxError(_))));
}

#[test]
fn same_module_units_support_order_independent_forward_lookup() {
    let function = parse("fn id(value: Ticket) -> Ticket { return value; }");
    let record = parse("record Ticket {}");
    let module = ModuleId::new(7);

    let first = build_typed_hir(&[
        SourceUnit::new(module, &function),
        SourceUnit::new(module, &record),
    ])
    .expect("forward lookup across same-module units must resolve");
    let second = build_typed_hir(&[
        SourceUnit::new(module, &record),
        SourceUnit::new(module, &function),
    ])
    .expect("source-unit presentation order must not affect validity");

    assert_eq!(first.records.len(), 1);
    assert_eq!(first.functions.len(), 1);
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.functions.len(), 1);
    assert_eq!(first.functions[0].parameters[0].ty, Type::Record(first.records[0].id));
    assert_eq!(second.functions[0].parameters[0].ty, Type::Record(second.records[0].id));
}

#[test]
fn distinct_modules_do_not_share_unqualified_declarations() {
    let function = parse("fn id(value: Ticket) -> Ticket { return value; }");
    let record = parse("record Ticket {}");
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &function),
        SourceUnit::new(ModuleId::new(2), &record),
    ])
    .expect_err("unqualified type lookup must stay in the current module");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));
}

#[test]
fn rejects_duplicate_module_bindings_across_categories() {
    let source = parse("record Clash {} fn Clash() {}");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source)])
        .expect_err("one module namespace forbids duplicate lexical keys");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::DuplicateModuleBinding));
}

#[test]
fn nominal_records_do_not_type_match_by_structure() {
    let source = parse(
        "record A {} record B {} fn take(value: A) {} fn test(value: B) { take(value); }",
    );
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source)])
        .expect_err("distinct nominal records must remain distinct types");
    assert!(has_diagnostic(&errors, |kind| matches!(kind, DiagnosticKind::TypeMismatch { .. })));
}

#[test]
fn rejects_duplicate_record_fields_and_containment_cycles() {
    let duplicate = parse("record A { x: I64, x: I64 }");
    let duplicate_errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &duplicate)])
        .expect_err("duplicate field keys are invalid");
    assert!(has_diagnostic(&duplicate_errors, |kind| kind == DiagnosticKind::DuplicateRecordField));

    let direct = parse("record A { a: A }");
    let direct_errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &direct)])
        .expect_err("direct record containment cycle is invalid");
    assert!(has_diagnostic(&direct_errors, |kind| kind == DiagnosticKind::RecordContainmentCycle));

    let mutual = parse("record A { b: B } record B { a: A }");
    let mutual_errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &mutual)])
        .expect_err("mutual direct record containment cycle is invalid");
    assert!(has_diagnostic(&mutual_errors, |kind| kind == DiagnosticKind::RecordContainmentCycle));
}

#[test]
fn resolves_signatures_and_rejects_duplicate_parameters() {
    let valid = parse("record Ticket {} fn f(a: I64, b: Ticket,) -> Ticket { return b; }");
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &valid)])
        .expect("signature types must resolve");
    assert_eq!(hir.functions[0].parameters.len(), 2);
    assert_eq!(hir.functions[0].result, Some(Type::Record(hir.records[0].id)));

    let duplicate = parse("fn f(a: I64, a: I64) {}");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &duplicate)])
        .expect_err("parameter keys must be unique");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::DuplicateParameter));
}

#[test]
fn initializer_precedes_binding_and_locals_cannot_shadow_locals_or_parameters() {
    let self_reference = parse("fn f() { let x: I64 = x; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &self_reference)])
        .expect_err("local is not in scope inside its own initializer");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));

    let parameter_shadow = parse("fn f(x: I64) { let x: I64 = x; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parameter_shadow)])
        .expect_err("local cannot shadow parameter");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::LocalShadowing));

    let local_reuse = parse("fn f(x: I64) { let y: I64 = x; let y: I64 = x; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &local_reuse)])
        .expect_err("same-scope local key cannot be reused");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::LocalShadowing));

    let module_shadow = parse("fn helper() {} fn f(helper: I64) {}");
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &module_shadow)])
        .expect("function-local key may equal module declaration key");
}

#[test]
fn local_first_lookup_blocks_module_function_fallback() {
    let source = parse("fn helper() {} fn f(helper: I64) { helper(); }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source)])
        .expect_err("same-key local must block fallback to module function");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ExpectedFunction));
}

#[test]
fn intrinsic_uses_duplicate_but_nominal_record_uses_consume() {
    let intrinsic = parse("fn sink(v: I64) {} fn f(x: I64) -> I64 { sink(x); return x; }");
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &intrinsic)])
        .expect("duplicable intrinsic may be used repeatedly");
    let f = hir.functions.iter().find(|function| function.name == "f").unwrap();
    let Statement::Call { arguments, .. } = &f.body.statements[0] else {
        panic!("expected call statement");
    };
    let ValueKind::BindingUse { ownership, .. } = arguments[0].kind else {
        panic!("expected binding use");
    };
    assert_eq!(ownership, OwnedUse::Duplicate);

    let record = parse(
        "record Ticket {} fn sink(v: Ticket) {} fn f(x: Ticket) -> Ticket { sink(x); return x; }",
    );
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &record)])
        .expect_err("concrete nominal record use consumes the binding");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableBinding));
}

#[test]
fn arguments_validate_left_to_right_including_nested_calls() {
    let source = parse(
        "record Ticket {} fn id(v: Ticket) -> Ticket { return v; } fn pair(a: Ticket, b: Ticket) {} fn test(x: Ticket) { pair(id(x), x); }",
    );
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source)])
        .expect_err("first nested argument consumes x before the second argument");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableBinding));
}

#[test]
fn direct_calls_require_exact_arity_and_type_without_conversion() {
    let arity = parse("fn target(x: I64) {} fn f(x: I64) { target(); }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &arity)])
        .expect_err("argument count must match exactly");
    assert!(has_diagnostic(&errors, |kind| matches!(kind, DiagnosticKind::ArgumentCount { expected: 1, found: 0 })));

    let ty = parse("fn target(x: I64) {} fn f(x: I32) { target(x); }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &ty)])
        .expect_err("I32 must not implicitly convert to I64");
    assert!(has_diagnostic(&errors, |kind| matches!(kind, DiagnosticKind::TypeMismatch { .. })));
}

#[test]
fn call_use_category_matches_result_structure() {
    let source = parse(
        "fn produce(x: I64) -> I64 { return x; } fn sink(x: I64) {} fn f(x: I64) { produce(x); let y: I64 = sink(x); }",
    );
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source)])
        .expect_err("result call cannot be statement and no-result call cannot be value");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ResultCallUsedAsStatement));
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::NoResultCallUsedAsValue));
}

#[test]
fn local_initializer_and_return_types_must_match_exactly() {
    let local = parse("fn f(x: I32) { let y: I64 = x; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &local)])
        .expect_err("initializer type must equal local type");
    assert!(has_diagnostic(&errors, |kind| matches!(kind, DiagnosticKind::TypeMismatch { .. })));

    let returned = parse("fn f(x: I32) -> I64 { return x; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &returned)])
        .expect_err("return type must match exactly");
    assert!(has_diagnostic(&errors, |kind| matches!(kind, DiagnosticKind::TypeMismatch { .. })));
}

#[test]
fn return_and_fallthrough_match_result_structure() {
    let missing = parse("fn f() -> I64 {}");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &missing)])
        .expect_err("result-bearing function requires terminal value return");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::MissingResultReturn));

    let empty_return = parse("fn f() -> I64 { return; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &empty_return)])
        .expect_err("result-bearing return requires value");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ExpectedResultValue));

    let unexpected = parse("fn f(x: I64) { return x; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &unexpected)])
        .expect_err("no-result function cannot return value");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnexpectedResultValue));

    let fallthrough = parse("fn f() {}");
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &fallthrough)])
        .expect("no-result function may fall through");
}

#[test]
fn direct_and_mutual_recursion_are_source_valid() {
    let direct = parse("fn f(x: I64) -> I64 { return f(x); }");
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &direct)])
        .expect("direct recursion is source-valid");

    let mutual = parse(
        "fn a(x: I64) -> I64 { return b(x); } fn b(x: I64) -> I64 { return a(x); }",
    );
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &mutual)])
        .expect("mutual recursion is source-valid");
}

#[test]
fn declaration_reordering_does_not_change_resolution_validity() {
    let first = parse(
        "record Ticket {} fn sink(v: Ticket) {} fn test(v: Ticket) { sink(v); }",
    );
    let second = parse(
        "fn test(v: Ticket) { sink(v); } fn sink(v: Ticket) {} record Ticket {}",
    );
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &first)])
        .expect("first declaration order must validate");
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &second)])
        .expect("reordered declarations must validate identically");
}
