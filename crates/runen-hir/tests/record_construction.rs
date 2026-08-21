use runen_hir::{
    DiagnosticKind, IntrinsicType, LiteralValue, ModuleId, OwnedUse, SourceUnit, Statement, Type,
    Value, ValueKind, build_typed_hir,
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
    let pair = hir.records.iter().find(|record| record.name == "Pair").unwrap();
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
fn structural_initializer_errors_are_diagnosed_before_any_producer_consumption() {
    let duplicate = errors(
        "record Token {} record Holder { item: Token } \
         fn f(token: Token) -> Token { \
             let bad: Holder = Holder { item: token, item: token }; \
             return token; \
         }",
    );
    assert!(duplicate.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::DuplicateRecordInitializer
    }));
    assert!(!duplicate.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::UnavailableBinding
    }));

    let unknown = errors(
        "record Token {} record Holder { item: Token } \
         fn f(token: Token) -> Token { \
             let bad: Holder = Holder { other: token }; \
             return token; \
         }",
    );
    assert!(unknown.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::UnknownRecordField
    }));
    assert!(unknown.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::MissingRecordInitializer
    }));
    assert!(!unknown.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::UnavailableBinding
    }));

    let missing = errors(
        "record Token {} record Holder { item: Token } \
         fn f(token: Token) -> Token { \
             let bad: Holder = Holder {}; \
             return token; \
         }",
    );
    assert!(missing.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::MissingRecordInitializer
    }));
    assert!(!missing.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::UnavailableBinding
    }));
}

#[test]
fn constructor_result_requires_exact_outer_record_type() {
    let diagnostics = errors(
        "record A {} record B {} fn f() -> A { return B {}; }",
    );
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Record(_),
            found: Type::Record(_),
        }
    )));
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
    assert!(matches!(all.body.statements[1], Statement::Assignment { .. }));
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
