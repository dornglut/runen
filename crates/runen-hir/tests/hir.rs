use runen_hir::{
    Accessibility, DiagnosticKind, ImportTarget, ModuleId, OwnedUse, SourceUnit, Statement, Type,
    ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn unit<'a>(module: ModuleId, parse: &'a Parse) -> SourceUnit<'a> {
    SourceUnit::new(module, parse, &[])
}

fn has_diagnostic(
    errors: &[runen_hir::Diagnostic],
    predicate: impl Fn(DiagnosticKind) -> bool,
) -> bool {
    errors.iter().any(|error| predicate(error.kind))
}

#[test]
fn rejects_syntax_dirty_units_before_semantic_hir() {
    let source = parse("fn broken( {}");
    assert!(!source.errors().is_empty());
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("syntax-dirty unit must not produce HIR");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::SyntaxError(_)
    )));
}

#[test]
fn same_module_units_support_order_independent_forward_lookup() {
    let function = parse("fn id(value: Ticket) -> Ticket { return value; }");
    let record = parse("record Ticket {}");
    let module = ModuleId::new(7);

    let first = build_typed_hir(&[unit(module, &function), unit(module, &record)])
        .expect("forward lookup across same-module units must resolve");
    let second = build_typed_hir(&[unit(module, &record), unit(module, &function)])
        .expect("source-unit presentation order must not affect validity");

    assert_eq!(first.records.len(), 1);
    assert_eq!(first.functions.len(), 1);
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.functions.len(), 1);
    assert_eq!(
        first.functions[0].parameters[0].ty,
        Type::Record(first.records[0].id)
    );
    assert_eq!(
        second.functions[0].parameters[0].ty,
        Type::Record(second.records[0].id)
    );
}

#[test]
fn distinct_modules_do_not_share_unqualified_declarations() {
    let function = parse("fn id(value: Ticket) -> Ticket { return value; }");
    let record = parse("record Ticket {}");
    let errors = build_typed_hir(&[
        unit(ModuleId::new(1), &function),
        unit(ModuleId::new(2), &record),
    ])
    .expect_err("unqualified type lookup must stay in the current module");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));
}

#[test]
fn rejects_duplicate_module_bindings_across_categories() {
    let source = parse("record Clash {} fn Clash() {}");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("one module namespace forbids duplicate lexical keys");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::DuplicateModuleBinding));
}

#[test]
fn nominal_records_do_not_type_match_by_structure() {
    let source =
        parse("record A {} record B {} fn take(value: A) {} fn test(value: B) { take(value); }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("distinct nominal records must remain distinct types");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));
}

#[test]
fn rejects_duplicate_record_fields_and_containment_cycles() {
    let duplicate = parse("record A { x: I64, x: I64 }");
    let duplicate_errors = build_typed_hir(&[unit(ModuleId::new(1), &duplicate)])
        .expect_err("duplicate field keys are invalid");
    assert!(has_diagnostic(&duplicate_errors, |kind| kind
        == DiagnosticKind::DuplicateRecordField));

    let direct = parse("record A { a: A }");
    let direct_errors = build_typed_hir(&[unit(ModuleId::new(1), &direct)])
        .expect_err("direct record containment cycle is invalid");
    assert!(has_diagnostic(&direct_errors, |kind| kind
        == DiagnosticKind::RecordContainmentCycle));

    let mutual = parse("record A { b: B } record B { a: A }");
    let mutual_errors = build_typed_hir(&[unit(ModuleId::new(1), &mutual)])
        .expect_err("mutual direct record containment cycle is invalid");
    assert!(has_diagnostic(&mutual_errors, |kind| kind
        == DiagnosticKind::RecordContainmentCycle));
}

#[test]
fn resolves_signatures_and_rejects_duplicate_parameters() {
    let valid = parse("record Ticket {} fn f(a: I64, b: Ticket,) -> Ticket { return b; }");
    let hir = build_typed_hir(&[unit(ModuleId::new(1), &valid)])
        .expect("signature types must resolve");
    assert_eq!(hir.functions[0].parameters.len(), 2);
    assert_eq!(
        hir.functions[0].result,
        Some(Type::Record(hir.records[0].id))
    );

    let duplicate = parse("fn f(a: I64, a: I64) {}");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &duplicate)])
        .expect_err("parameter keys must be unique");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::DuplicateParameter));
}

#[test]
fn initializer_precedes_binding_and_locals_cannot_shadow_locals_or_parameters() {
    let self_reference = parse("fn f() { let x: I64 = x; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &self_reference)])
        .expect_err("local is not in scope inside its own initializer");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));

    let parameter_shadow = parse("fn f(x: I64) { let x: I64 = x; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &parameter_shadow)])
        .expect_err("local cannot shadow parameter");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::LocalShadowing));

    let local_reuse = parse("fn f(x: I64) { let y: I64 = x; let y: I64 = x; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &local_reuse)])
        .expect_err("same-scope local key cannot be reused");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::LocalShadowing));

    let module_shadow = parse("fn helper() {} fn f(helper: I64) {}");
    build_typed_hir(&[unit(ModuleId::new(1), &module_shadow)])
        .expect("function-local key may equal module declaration key");
}

#[test]
fn local_first_lookup_blocks_module_function_fallback() {
    let source = parse("fn helper() {} fn f(helper: I64) { helper(); }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("same-key local must block fallback to module function");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ExpectedFunction));
}

#[test]
fn intrinsic_uses_duplicate_but_nominal_record_uses_consume() {
    let intrinsic = parse("fn sink(v: I64) {} fn f(x: I64) -> I64 { sink(x); return x; }");
    let hir = build_typed_hir(&[unit(ModuleId::new(1), &intrinsic)])
        .expect("duplicable intrinsic may be used repeatedly");
    let f = hir
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
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
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &record)])
        .expect_err("concrete nominal record use consumes the binding");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableBinding));
}

#[test]
fn arguments_validate_left_to_right_including_nested_calls() {
    let source = parse(
        "record Ticket {} fn id(v: Ticket) -> Ticket { return v; } fn pair(a: Ticket, b: Ticket) {} fn test(x: Ticket) { pair(id(x), x); }",
    );
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("first nested argument consumes x before the second argument");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableBinding));
}

#[test]
fn direct_calls_require_exact_arity_and_type_without_conversion() {
    let arity = parse("fn target(x: I64) {} fn f(x: I64) { target(); }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &arity)])
        .expect_err("argument count must match exactly");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::ArgumentCount {
            expected: 1,
            found: 0
        }
    )));

    let ty = parse("fn target(x: I64) {} fn f(x: I32) { target(x); }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &ty)])
        .expect_err("I32 must not implicitly convert to I64");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));
}

#[test]
fn call_use_category_matches_result_structure() {
    let source = parse(
        "fn produce(x: I64) -> I64 { return x; } fn sink(x: I64) {} fn f(x: I64) { produce(x); let y: I64 = sink(x); }",
    );
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("result call cannot be statement and no-result call cannot be value");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ResultCallUsedAsStatement));
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::NoResultCallUsedAsValue));
}

#[test]
fn local_initializer_and_return_types_must_match_exactly() {
    let local = parse("fn f(x: I32) { let y: I64 = x; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &local)])
        .expect_err("initializer type must equal local type");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let returned = parse("fn f(x: I32) -> I64 { return x; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &returned)])
        .expect_err("return type must match exactly");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));
}

#[test]
fn return_and_fallthrough_match_result_structure() {
    let missing = parse("fn f() -> I64 {}");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &missing)])
        .expect_err("result-bearing function requires terminal value return");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::MissingResultReturn));

    let empty_return = parse("fn f() -> I64 { return; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &empty_return)])
        .expect_err("result-bearing return requires value");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ExpectedResultValue));

    let unexpected = parse("fn f(x: I64) { return x; }");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &unexpected)])
        .expect_err("no-result function cannot return value");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::UnexpectedResultValue));

    let fallthrough = parse("fn f() {}");
    build_typed_hir(&[unit(ModuleId::new(1), &fallthrough)])
        .expect("no-result function may fall through");
}

#[test]
fn direct_and_mutual_recursion_are_source_valid() {
    let direct = parse("fn f(x: I64) -> I64 { return f(x); }");
    build_typed_hir(&[unit(ModuleId::new(1), &direct)])
        .expect("direct recursion is source-valid");

    let mutual = parse("fn a(x: I64) -> I64 { return b(x); } fn b(x: I64) -> I64 { return a(x); }");
    build_typed_hir(&[unit(ModuleId::new(1), &mutual)])
        .expect("mutual recursion is source-valid");
}

#[test]
fn declaration_reordering_does_not_change_resolution_validity() {
    let first = parse("record Ticket {} fn sink(v: Ticket) {} fn test(v: Ticket) { sink(v); }");
    let second = parse("fn test(v: Ticket) { sink(v); } fn sink(v: Ticket) {} record Ticket {}");
    build_typed_hir(&[unit(ModuleId::new(1), &first)])
        .expect("first declaration order must validate");
    build_typed_hir(&[unit(ModuleId::new(1), &second)])
        .expect("reordered declarations must validate identically");
}

#[test]
fn import_target_constructor_uses_concrete_user_identifier_rules() {
    let module = ModuleId::new(2);
    assert!(ImportTarget::new("dep", module).is_some());
    assert!(ImportTarget::new("import", module).is_none());
    assert!(ImportTarget::new("Bool", module).is_none());
    assert!(ImportTarget::new("not-an-ident", module).is_none());
}

#[test]
fn import_relations_require_one_target_and_forbid_duplicate_aliases_and_self_import() {
    let missing = parse("import dep; fn f() {}");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &missing)])
        .expect_err("declared import requires one supplied target");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::MissingImportTarget));

    let duplicate = parse("import dep; import dep;");
    let targets = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &duplicate, &targets)])
        .expect_err("duplicate concrete aliases are invalid");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::DuplicateImportAlias));

    let duplicate_targets = [
        ImportTarget::new("dep", ModuleId::new(2)).unwrap(),
        ImportTarget::new("dep", ModuleId::new(3)).unwrap(),
    ];
    let single = parse("import dep;");
    let errors = build_typed_hir(&[SourceUnit::new(
        ModuleId::new(1),
        &single,
        &duplicate_targets,
    )])
    .expect_err("one concrete alias must not have multiple supplied targets");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::DuplicateImportTarget));

    let self_target = [ImportTarget::new("dep", ModuleId::new(1)).unwrap()];
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &single, &self_target)])
        .expect_err("source unit cannot import its own module");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::SelfImport));
}

#[test]
fn import_alias_conflicts_with_declaration_from_another_same_module_unit() {
    let imports = parse("import clash;");
    let declaration = parse("fn clash() {}");
    let targets = [ImportTarget::new("clash", ModuleId::new(2)).unwrap()];
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &imports, &targets),
        unit(ModuleId::new(1), &declaration),
    ])
    .expect_err("alias must conflict with complete same-module declaration namespace");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::ImportDeclarationConflict));
}

#[test]
fn distinct_aliases_may_target_one_module_and_resolve_one_nominal_record() {
    let target = parse("export record Ticket {}");
    let source = parse(
        "import left; import right; fn id(value: left::Ticket) -> right::Ticket { return value; }",
    );
    let target_module = ModuleId::new(2);
    let imports = [
        ImportTarget::new("left", target_module).unwrap(),
        ImportTarget::new("right", target_module).unwrap(),
    ];
    let hir = build_typed_hir(&[
        unit(target_module, &target),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
    .expect("two aliases may resolve to the same target module");
    assert_eq!(hir.functions[0].parameters[0].ty, hir.functions[0].result.unwrap());
}

#[test]
fn same_alias_key_in_distinct_units_may_target_different_modules() {
    let a = parse("export record A {}");
    let b = parse("export record B {}");
    let use_a = parse("import dep; fn use_a(value: dep::A) {}");
    let use_b = parse("import dep; fn use_b(value: dep::B) {}");
    let target_a = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let target_b = [ImportTarget::new("dep", ModuleId::new(4)).unwrap()];

    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &use_a, &target_a),
        unit(ModuleId::new(2), &a),
        SourceUnit::new(ModuleId::new(3), &use_b, &target_b),
        unit(ModuleId::new(4), &b),
    ])
    .expect("alias identity and target are source-unit-local");
}

#[test]
fn qualified_types_require_exported_records_and_work_in_all_current_type_positions() {
    let private = parse("record Ticket {}");
    let source = parse("import dep; fn f(value: dep::Ticket) {}");
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let errors = build_typed_hir(&[
        unit(ModuleId::new(2), &private),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
    .expect_err("private target record must not be visible cross-module");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::InaccessibleBinding));

    let exported = parse("export record Ticket {}");
    let source = parse(
        "import dep; record Holder { field: dep::Ticket } fn f(value: dep::Ticket) -> dep::Ticket { let out: dep::Ticket = value; return out; }",
    );
    let hir = build_typed_hir(&[
        unit(ModuleId::new(2), &exported),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
    .expect("qualified exported record must resolve in all represented type positions");
    let ticket = hir.records.iter().find(|record| record.name == "Ticket").unwrap().id;
    let holder = hir.records.iter().find(|record| record.name == "Holder").unwrap();
    let function = hir.functions.iter().find(|function| function.name == "f").unwrap();
    assert_eq!(holder.fields[0].ty, Type::Record(ticket));
    assert_eq!(function.parameters[0].ty, Type::Record(ticket));
    assert_eq!(function.result, Some(Type::Record(ticket)));
    let Statement::Local { ty, .. } = function.body.statements[0] else {
        panic!("expected local statement");
    };
    assert_eq!(ty, Type::Record(ticket));
}

#[test]
fn qualified_calls_require_exported_functions_and_support_nested_values() {
    let private = parse("fn hidden() {}");
    let source = parse("import dep; fn f() { dep::hidden(); }");
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let errors = build_typed_hir(&[
        unit(ModuleId::new(2), &private),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
    .expect_err("private target function must not be visible cross-module");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::InaccessibleBinding));

    let exported = parse(
        "export fn sink(value: I64) {} export fn id(value: I64) -> I64 { return value; }",
    );
    let source = parse(
        "import dep; fn f(x: I64) -> I64 { dep::sink(x); return dep::id(dep::id(x)); }",
    );
    let hir = build_typed_hir(&[
        unit(ModuleId::new(2), &exported),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
    .expect("exported qualified calls must resolve as statement and nested values");
    let f = hir.functions.iter().find(|function| function.name == "f").unwrap();
    let Statement::Call { function, .. } = f.body.statements[0] else {
        panic!("expected qualified call statement");
    };
    assert_eq!(hir.function(function).name, "sink");
    let returned = f.body.terminal_return.as_ref().unwrap().value.as_ref().unwrap();
    let ValueKind::DirectCall { function, .. } = returned.kind else {
        panic!("expected qualified result call");
    };
    assert_eq!(hir.function(function).name, "id");
}

#[test]
fn qualified_lookup_has_no_category_fallback_and_imports_are_not_unqualified_preludes() {
    let target = parse("export record R {} export fn make() {}");
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];

    let wrong_type = parse("import dep; fn f(value: dep::make) {}");
    let errors = build_typed_hir(&[
        unit(ModuleId::new(2), &target),
        SourceUnit::new(ModuleId::new(1), &wrong_type, &imports),
    ])
    .expect_err("qualified function cannot satisfy record type context");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ExpectedRecordType));

    let wrong_call = parse("import dep; fn f() { dep::R(); }");
    let errors = build_typed_hir(&[
        unit(ModuleId::new(2), &target),
        SourceUnit::new(ModuleId::new(1), &wrong_call, &imports),
    ])
    .expect_err("qualified record cannot satisfy function call context");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::ExpectedFunction));

    let unqualified = parse("import dep; fn f(value: R) {}");
    let errors = build_typed_hir(&[
        unit(ModuleId::new(2), &target),
        SourceUnit::new(ModuleId::new(1), &unqualified, &imports),
    ])
    .expect_err("imported module declarations must not be searched unqualified");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));
}

#[test]
fn local_same_as_alias_does_not_block_explicit_qualified_call() {
    let target = parse("export fn helper() {}");
    let source = parse("import dep; fn f(dep: I64) { dep::helper(); }");
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    build_typed_hir(&[
        unit(ModuleId::new(2), &target),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
    .expect("function-local lookup must not participate in explicit qualification");
}

#[test]
fn exported_function_cannot_expose_same_module_private_record() {
    let source = parse("record Private {} export fn expose(value: Private) {}");
    let errors = build_typed_hir(&[unit(ModuleId::new(1), &source)])
        .expect_err("exported signature must not expose module-private nominal type");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::PrivateTypeInExportedSignature));
}

#[test]
fn accessibility_is_retained_only_as_source_hir_intent() {
    let source = parse("record Private {} export record Public {} fn local() {} export fn public() {}");
    let hir = build_typed_hir(&[unit(ModuleId::new(1), &source)]).expect("source must validate");
    assert_eq!(
        hir.records.iter().find(|record| record.name == "Private").unwrap().accessibility,
        Accessibility::ModulePrivate
    );
    assert_eq!(
        hir.records.iter().find(|record| record.name == "Public").unwrap().accessibility,
        Accessibility::Exported
    );
    assert_eq!(
        hir.functions.iter().find(|function| function.name == "local").unwrap().accessibility,
        Accessibility::ModulePrivate
    );
    assert_eq!(
        hir.functions.iter().find(|function| function.name == "public").unwrap().accessibility,
        Accessibility::Exported
    );
}

#[test]
fn cyclic_module_alias_relations_are_valid_when_one_hop_lookups_resolve() {
    let a = parse("import b; export fn a() { b::b(); }");
    let b = parse("import a; export fn b() { a::a(); }");
    let a_imports = [ImportTarget::new("b", ModuleId::new(2)).unwrap()];
    let b_imports = [ImportTarget::new("a", ModuleId::new(1)).unwrap()];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &a, &a_imports),
        SourceUnit::new(ModuleId::new(2), &b, &b_imports),
    ])
    .expect("finite import cycle alone must not invalidate one-hop qualified lookup");
}

#[test]
fn extra_import_target_mapping_has_no_source_lookup_effect() {
    let source = parse("fn f() {}");
    let extra = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &source, &extra)])
        .expect("undeclared context mapping must be source-semantically inert");
}
