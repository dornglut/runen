use runen_hir::{
    DiagnosticKind, FieldValueReceiver, ImportTarget, IntrinsicType, LiteralValue, ModuleId,
    OwnedUse, RecordPatternScrutinee, SourceUnit, Statement, Type, Value, ValueKind,
    build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn errors(source: &str) -> Vec<runen_hir::Diagnostic> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect_err("test source must be rejected")
}

fn cross_module(
    target_source: &str,
    source: &str,
) -> Result<runen_hir::TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let target = parse(target_source);
    let source = parse(source);
    assert!(target.errors().is_empty(), "{:?}", target.errors());
    assert!(source.errors().is_empty(), "{:?}", source.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &target, &[]),
        SourceUnit::new(ModuleId::new(1), &source, &imports),
    ])
}

fn function<'a>(hir: &'a runen_hir::TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .expect("named test function")
}

fn returned_value(function: &runen_hir::Function) -> &Value {
    function
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("result-bearing test function has a returned value")
}

#[test]
fn constructor_target_ignores_local_and_hir_retains_source_ordered_field_identity() {
    let hir = build(
        "record Pair { left: I8, right: U64 } \
         fn make(Pair: I8) -> Pair { \
             return Pair { right: 1, left: Pair }; \
         }",
    );
    let pair = hir
        .records
        .iter()
        .find(|record| record.name == "Pair")
        .unwrap();
    let value = returned_value(function(&hir, "make"));

    assert_eq!(value.ty, Type::Record(pair.id));
    let ValueKind::RecordConstruction { record, fields } = &value.kind else {
        panic!("expected record construction");
    };
    assert_eq!(*record, pair.id);
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].field, 1, "right is declaration field 1");
    assert_eq!(fields[1].field, 0, "left is declaration field 0");
    assert_eq!(fields[0].value.ty, Type::Intrinsic(IntrinsicType::U64));
    assert_eq!(
        fields[0].value.kind,
        ValueKind::Literal(LiteralValue::U64(1))
    );
    assert_eq!(fields[1].value.ty, Type::Intrinsic(IntrinsicType::I8));
    assert!(matches!(
        fields[1].value.kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
}

#[test]
fn constructor_target_reports_wrong_category_and_unresolved_names() {
    let diagnostics = errors(
        "record Box {} \
         fn helper() {} \
         fn wrong() -> Box { return helper {}; } \
         fn missing() -> Box { return Missing {}; }",
    );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::ExpectedRecordType)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnresolvedName)
    );
}

#[test]
fn qualified_target_uses_existing_lookup_and_erases_qualification_in_hir() {
    let hir = cross_module(
        "export record Empty {} export record Pair { export left: I8, export right: U64 }",
        "import dep; \
         fn empty() -> dep::Empty { return dep::Empty {}; } \
         fn pair() -> dep::Pair { return dep::Pair { right: 9, left: 3 }; }",
    )
    .expect("exported foreign records with accessible fields are constructible");

    let pair = hir
        .records
        .iter()
        .find(|record| record.name == "Pair")
        .expect("foreign Pair record");
    let value = returned_value(function(&hir, "pair"));
    assert_eq!(value.ty, Type::Record(pair.id));
    let ValueKind::RecordConstruction { record, fields } = &value.kind else {
        panic!("qualified target must remain ordinary record-construction HIR");
    };
    assert_eq!(*record, pair.id);
    assert_eq!(
        fields.iter().map(|field| field.field).collect::<Vec<_>>(),
        vec![1, 0]
    );
    assert_eq!(
        fields[0].value.kind,
        ValueKind::Literal(LiteralValue::U64(9))
    );
    assert_eq!(
        fields[1].value.kind,
        ValueKind::Literal(LiteralValue::I8(3))
    );

    assert!(matches!(
        returned_value(function(&hir, "empty")).kind,
        ValueKind::RecordConstruction { ref fields, .. } if fields.is_empty()
    ));
}

#[test]
fn qualified_target_preserves_lookup_failure_partition() {
    let unknown_alias = cross_module(
        "export record Pair { export left: I8 }",
        "import dep; fn f() -> dep::Pair { return other::Pair { left: 1 }; }",
    )
    .expect_err("undeclared alias must reject");
    assert!(
        unknown_alias
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnresolvedName)
    );

    let missing_member = cross_module(
        "export record Other {}",
        "import dep; fn f() -> dep::Other { return dep::Pair { }; }",
    )
    .expect_err("absent target member must reject");
    assert!(
        missing_member
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnresolvedName)
    );

    let private_target = cross_module(
        "record Pair { export left: I8 }",
        "import dep; fn f() { let value: I8 = dep::Pair { left: 1 }; }",
    )
    .expect_err("private foreign target must reject before construction");
    assert!(
        private_target
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::InaccessibleBinding)
    );

    let wrong_category = cross_module(
        "export fn Pair() {}",
        "import dep; fn f() { let value: I8 = dep::Pair {}; }",
    )
    .expect_err("exported function is not a constructor target");
    assert!(
        wrong_category
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::ExpectedRecordType)
    );
}

#[test]
fn qualified_initializer_access_resolves_identity_before_visibility() {
    let inaccessible = cross_module(
        "export record Pair { left: I8, export right: I8 }",
        "import dep; fn f() -> dep::Pair { return dep::Pair { left: 1, right: 2 }; }",
    )
    .expect_err("known private foreign field must be inaccessible");
    assert!(
        inaccessible
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::InaccessibleRecordField)
    );
    assert!(
        !inaccessible
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnknownRecordField)
    );

    let unknown = cross_module(
        "export record Pair { export left: I8 }",
        "import dep; fn f() -> dep::Pair { return dep::Pair { other: 1 }; }",
    )
    .expect_err("unknown foreign field must remain unknown");
    assert!(
        unknown
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnknownRecordField)
    );
    assert!(
        !unknown
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::InaccessibleRecordField)
    );
}

#[test]
fn foreign_private_fields_make_exhaustive_qualified_construction_impossible() {
    let named_private = cross_module(
        "export record Pair { hidden: I8, export shown: I8 }",
        "import dep; fn f() -> dep::Pair { return dep::Pair { hidden: 1, shown: 2 }; }",
    )
    .expect_err("naming private field must reject");
    assert!(
        named_private
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::InaccessibleRecordField)
    );
    assert!(
        !named_private
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::MissingRecordInitializer)
    );

    let omitted_private = cross_module(
        "export record Pair { hidden: I8, export shown: I8 }",
        "import dep; fn f() -> dep::Pair { return dep::Pair { shown: 2 }; }",
    )
    .expect_err("omitting private field must remain incomplete");
    assert!(
        omitted_private
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::MissingRecordInitializer)
    );
}

#[test]
fn structural_initializer_errors_are_diagnosed_before_any_producer_consumption() {
    let duplicate = errors(
        "record Token {} record Holder { item: Token } \
         fn f(token: Token) -> Token { \
             let bad: Holder = Holder { item: token, item: token }; \
             return token; \
         }",
    );
    assert!(
        duplicate
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::DuplicateRecordInitializer })
    );
    assert!(
        !duplicate
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::UnavailableBinding })
    );

    let unknown = errors(
        "record Token {} record Holder { item: Token } \
         fn f(token: Token) -> Token { \
             let bad: Holder = Holder { other: token }; \
             return token; \
         }",
    );
    assert!(
        unknown
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::UnknownRecordField })
    );
    assert!(
        unknown
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::MissingRecordInitializer })
    );
    assert!(
        !unknown
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::UnavailableBinding })
    );

    let missing = errors(
        "record Token {} record Holder { item: Token } \
         fn f(token: Token) -> Token { \
             let bad: Holder = Holder {}; \
             return token; \
         }",
    );
    assert!(
        missing
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::MissingRecordInitializer })
    );
    assert!(
        !missing
            .iter()
            .any(|diagnostic| { diagnostic.kind == DiagnosticKind::UnavailableBinding })
    );
}

#[test]
fn qualified_structural_rejection_does_not_commit_initializer_consumption() {
    for source in [
        "import dep; fn f(token: dep::Token) -> dep::Token { let bad: dep::Holder = dep::Holder { hidden: token }; return token; }",
        "import dep; fn f(token: dep::Token) -> dep::Token { let bad: dep::Holder = dep::Holder { other: token }; return token; }",
        "import dep; fn f(token: dep::Token) -> dep::Token { let bad: dep::Holder = dep::Holder { item: token, item: token }; return token; }",
    ] {
        let diagnostics = cross_module(
            "export record Token {} export record Holder { hidden: Token, export item: Token }",
            source,
        )
        .expect_err("structurally invalid qualified construction must reject");
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnavailableBinding),
            "initializer must not consume token before complete structural validity: {source}"
        );
    }
}

#[test]
fn constructor_result_requires_exact_outer_record_type() {
    let diagnostics = errors("record A {} record B {} fn f() -> A { return B {}; }");
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Record(expected),
            found: Type::Record(found),
        } if expected != found
    )));
}

#[test]
fn qualified_constructor_result_mismatch_precedes_initializer_consumption() {
    let diagnostics = cross_module(
        "export record Token {} export record Holder { export item: Token }",
        "import dep; record Local {} fn f(token: dep::Token) -> dep::Token { let bad: Local = dep::Holder { item: token }; return token; }",
    )
    .expect_err("qualified construction result must exactly match receiving type");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnavailableBinding)
    );
}

#[test]
fn nested_construction_calls_and_nonduplicable_field_consumption_are_retained() {
    let hir = build(
        "record Token {} \
         record Inner { token: Token } \
         record Outer { inner: Inner, number: I8 } \
         fn number() -> I8 { return 7; } \
         fn build(token: Token) -> Outer { \
             return Outer { number: number(), inner: Inner { token: token } }; \
         }",
    );

    let outer = returned_value(function(&hir, "build"));
    let ValueKind::RecordConstruction { fields, .. } = &outer.kind else {
        panic!("expected outer construction");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].field, 1);
    assert!(matches!(fields[0].value.kind, ValueKind::DirectCall { .. }));
    assert_eq!(fields[1].field, 0);

    let ValueKind::RecordConstruction {
        fields: inner_fields,
        ..
    } = &fields[1].value.kind
    else {
        panic!("expected nested construction");
    };
    assert_eq!(inner_fields.len(), 1);
    assert!(matches!(
        inner_fields[0].value.kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    ));
}

#[test]
fn qualified_constructor_retains_source_order_and_nonduplicable_consumption() {
    let hir = cross_module(
        "export record Token {} export record Pair { export right: Token, export left: Token }",
        "import dep; fn build(left: dep::Token, right: dep::Token) -> dep::Pair { return dep::Pair { left: left, right: right }; }",
    )
    .expect("qualified construction with exported fields must validate");

    let value = returned_value(function(&hir, "build"));
    let ValueKind::RecordConstruction { fields, .. } = &value.kind else {
        panic!("expected construction");
    };
    assert_eq!(
        fields.iter().map(|field| field.field).collect::<Vec<_>>(),
        vec![1, 0]
    );
    assert!(fields.iter().all(|field| matches!(
        field.value.kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    )));
}

#[test]
fn construction_composes_with_every_current_value_consumer() {
    let hir = build(
        "record Box { value: I8 } \
         record Outer { box: Box } \
         fn sink(value: Box) {} \
         fn all() -> Box { \
             let mut value: Box = Box { value: 1 }; \
             value = Box { value: 2 }; \
             let nested: Outer = Outer { box: Box { value: 3 } }; \
             sink(Box { value: 4 }); \
             return Box { value: 5 }; \
         }",
    );

    let all = function(&hir, "all");
    assert!(matches!(all.body.statements[0], Statement::Local { .. }));
    assert!(matches!(
        all.body.statements[1],
        Statement::Assignment { .. }
    ));
    assert!(matches!(all.body.statements[2], Statement::Local { .. }));
    assert!(matches!(all.body.statements[3], Statement::Call { .. }));
    assert!(matches!(
        returned_value(all).kind,
        ValueKind::RecordConstruction { .. }
    ));

    let Statement::Local { initializer, .. } = &all.body.statements[2] else {
        unreachable!();
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected outer local construction");
    };
    assert!(matches!(
        fields[0].value.kind,
        ValueKind::RecordConstruction { .. }
    ));
}

#[test]
fn qualified_construction_composes_with_current_value_consumers() {
    let hir = cross_module(
        "export record Box { export value: I8 }",
        "import dep; \
         record Outer { box: dep::Box } \
         fn sink(value: dep::Box) {} \
         fn all() -> dep::Box { \
             let mut value: dep::Box = dep::Box { value: 1 }; \
             value = dep::Box { value: 2 }; \
             let nested: Outer = Outer { box: dep::Box { value: 3 } }; \
             sink(dep::Box { value: 4 }); \
             return dep::Box { value: 5 }; \
         }",
    )
    .expect("qualified construction must remain the existing value producer");

    let all = function(&hir, "all");
    assert!(matches!(all.body.statements[0], Statement::Local { .. }));
    assert!(matches!(
        all.body.statements[1],
        Statement::Assignment { .. }
    ));
    assert!(matches!(all.body.statements[2], Statement::Local { .. }));
    assert!(matches!(all.body.statements[3], Statement::Call { .. }));
    assert!(matches!(
        returned_value(all).kind,
        ValueKind::RecordConstruction { .. }
    ));
}

#[test]
fn qualified_construction_field_receiver_retains_existing_ownership_and_condition_semantics() {
    let hir = cross_module(
        "export record Token {} export record Empty {} \
         export record Box { export token: Token, export empty: Empty, export count: I8 } \
         export record Flag { export ready: Bool }",
        "import dep; \
         fn take() -> dep::Token { \
             return dep::Box { token: dep::Token {}, empty: dep::Empty {}, count: 1 }.token; \
         } \
         fn cond() { if dep::Flag { ready: true }.ready {} }",
    )
    .expect("qualified construction must compose through existing field receiver semantics");

    let selected = returned_value(function(&hir, "take"));
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &selected.kind
    else {
        panic!("expected qualified-construction-backed field use");
    };
    let FieldValueReceiver::Producer {
        value: producer,
        cleanup,
    } = receiver
    else {
        panic!("expected producer receiver");
    };
    assert!(matches!(
        producer.kind,
        ValueKind::RecordConstruction { .. }
    ));
    assert_eq!(fields, &[0]);
    assert_eq!(*ownership, OwnedUse::Consume);
    assert_eq!(cleanup.paths, vec![vec![2], vec![1]]);

    let Statement::If { condition, .. } = &function(&hir, "cond").body.statements[0] else {
        panic!("expected conditional");
    };
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &condition.kind
    else {
        panic!("expected field-value condition");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected construction producer receiver");
    };
    assert_eq!(condition.ty, Type::Intrinsic(IntrinsicType::Bool));
    assert_eq!(fields, &[0]);
    assert_eq!(*ownership, OwnedUse::Duplicate);
    assert_eq!(cleanup.paths, vec![Vec::<usize>::new()]);
}

#[test]
fn qualified_construction_field_receiver_static_rejection_rolls_back_initializer_consumption() {
    let rejected = cross_module(
        "export record Token {} export record Box { export token: Token, export count: I8 }",
        "import dep; fn sink(token: dep::Token) {} \
         fn f(token: dep::Token) { \
             let bad: U8 = dep::Box { token: token, count: 1 }.count; \
             sink(token); \
         }",
    )
    .expect_err("outer field result mismatch must reject before constructor ownership commits");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(
        !rejected
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnavailableBinding)
    );

    let consumed = cross_module(
        "export record Token {} export record Box { export token: Token, export count: I8 }",
        "import dep; fn sink(token: dep::Token) {} \
         fn f(token: dep::Token) { \
             let value: I8 = dep::Box { token: token, count: 1 }.count; \
             sink(token); \
         }",
    )
    .expect_err("successful constructor receiver must commit initializer ownership");
    assert!(
        consumed
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnavailableBinding)
    );
}

#[test]
fn bare_foreign_qualified_construction_pattern_rejects_before_initializer_consumption() {
    let diagnostics = cross_module(
        "export record Token {} export record Pair { export token: Token }",
        "import dep; record Local {} \
         fn f(token: dep::Token) -> dep::Token { \
             let Local {} = dep::Pair { token: token }; \
             return token; \
         }",
    )
    .expect_err("foreign construction cannot match current same-module pattern head");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnavailableBinding)
    );
}

#[test]
fn qualified_construction_inside_field_producer_can_feed_same_module_pattern() {
    let home_module = ModuleId::new(1);
    let foreign_module = ModuleId::new(2);
    let home = parse(
        "import dep; export record Local {} \
         fn f() { let Local {} = dep::Wrapper { local: Local {} }.local; }",
    );
    let foreign = parse("import home; export record Wrapper { export local: home::Local }");
    assert!(home.errors().is_empty(), "{:?}", home.errors());
    assert!(foreign.errors().is_empty(), "{:?}", foreign.errors());
    let home_imports = [ImportTarget::new("dep", foreign_module).unwrap()];
    let foreign_imports = [ImportTarget::new("home", home_module).unwrap()];

    let hir = build_typed_hir(&[
        SourceUnit::new(home_module, &home, &home_imports),
        SourceUnit::new(foreign_module, &foreign, &foreign_imports),
    ])
    .expect("qualified construction field result may match a same-module pattern head");

    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected record destructuring");
    };
    let RecordPatternScrutinee::Producer { value, .. } = scrutinee else {
        panic!("expected producer-backed pattern scrutinee");
    };
    let ValueKind::FieldValueUse {
        receiver,
        ownership,
        ..
    } = &value.kind
    else {
        panic!("expected field-value scrutinee");
    };
    let FieldValueReceiver::Producer {
        value: producer, ..
    } = receiver
    else {
        panic!("expected construction producer receiver");
    };
    assert!(matches!(
        producer.kind,
        ValueKind::RecordConstruction { .. }
    ));
    assert_eq!(*ownership, OwnedUse::Consume);
}
