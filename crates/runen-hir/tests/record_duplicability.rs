use runen_hir::{
    DiagnosticKind, Duplicability, FieldValueReceiver, ModuleId, OwnedUse, RecordPatternScrutinee,
    SourceUnit, Statement, Type, TypedCompilation, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, SyntaxKind, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn compile(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn record<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Record {
    hir.records
        .iter()
        .find(|record| record.name == name)
        .expect("record exists")
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .expect("function exists")
}

fn invalid_selection_count(errors: &[runen_hir::Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::InvalidRecordDuplicabilitySelection)
        .count()
}

#[test]
fn retains_independent_record_classifications_and_type_query() {
    let hir = compile(
        "record Plain { value: I8 }\
         record copy Point { x: I32, y: I32 }\
         record copy SelectedEmpty {}\
         record PlainEmpty {}\
         record copy Child { value: U8 }\
         record Parent { child: Child }\
         export record copy Public { hidden: I8, export visible: U8 }",
    )
    .expect("valid selected and unselected records");

    assert_eq!(record(&hir, "Plain").duplicability, Duplicability::NonDuplicable);
    assert_eq!(record(&hir, "Point").duplicability, Duplicability::Duplicable);
    assert_eq!(
        record(&hir, "SelectedEmpty").duplicability,
        Duplicability::Duplicable
    );
    assert_eq!(
        record(&hir, "PlainEmpty").duplicability,
        Duplicability::NonDuplicable
    );
    assert_eq!(record(&hir, "Child").duplicability, Duplicability::Duplicable);
    assert_eq!(
        record(&hir, "Parent").duplicability,
        Duplicability::NonDuplicable
    );
    assert_eq!(record(&hir, "Public").duplicability, Duplicability::Duplicable);

    assert!(hir.type_is_duplicable(Type::Intrinsic(runen_hir::IntrinsicType::I64)));
    assert!(hir.type_is_duplicable(Type::Record(record(&hir, "Point").id)));
    assert!(!hir.type_is_duplicable(Type::Record(record(&hir, "Plain").id)));

    assert_eq!(record(&hir, "Public").fields.len(), 2);
}

#[test]
fn selected_nested_records_are_order_independent() {
    for source in [
        "record copy Outer { inner: Inner } record copy Inner { leaf: Leaf } record copy Leaf { value: I8 }",
        "record copy Leaf { value: I8 } record copy Inner { leaf: Leaf } record copy Outer { inner: Inner }",
    ] {
        let hir = compile(source).expect("fully selected acyclic nesting is valid");
        for name in ["Outer", "Inner", "Leaf"] {
            let record = record(&hir, name);
            assert_eq!(record.duplicability, Duplicability::Duplicable);
            assert!(hir.type_is_duplicable(Type::Record(record.id)));
        }
    }
}

#[test]
fn rejects_selected_records_that_depend_on_unselected_records() {
    let errors = compile(
        "record Plain { value: I8 }\
         record copy Inner { plain: Plain }\
         record copy Outer { inner: Inner }",
    )
    .expect_err("invalid selected dependencies must reject the compilation");

    assert_eq!(invalid_selection_count(&errors), 2);
}

#[test]
fn invalid_selection_diagnostic_points_at_the_copy_token() {
    let source = "record Plain {} record copy Bad { plain: Plain }";
    let parsed = parse(source);
    let copy = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == SyntaxKind::KwCopy)
        .expect("copy token");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect_err("invalid selection must reject HIR");
    let error = errors
        .iter()
        .find(|error| error.kind == DiagnosticKind::InvalidRecordDuplicabilitySelection)
        .expect("dedicated duplicability diagnostic");
    assert_eq!(error.location.unit, 0);
    assert_eq!(error.location.range, copy.text_range());
}

#[test]
fn containment_cycle_rejection_precedes_duplicability_validation() {
    let errors = compile("record copy A { b: B } record copy B { a: A }")
        .expect_err("record containment cycle remains invalid");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::RecordContainmentCycle)
    );
    assert_eq!(invalid_selection_count(&errors), 0);
}

#[test]
fn selected_whole_binding_duplicates_while_unselected_record_consumes() {
    let selected = compile(
        "record copy Token { value: I8 }\
         fn take(value: Token) {}\
         fn f(value: Token) { take(value); take(value); }",
    )
    .expect("selected record binding can be used repeatedly");
    let calls = function(&selected, "f")
        .body
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Call { arguments, .. } => Some(arguments),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    for arguments in calls {
        assert!(matches!(
            arguments[0].kind,
            ValueKind::BindingUse {
                ownership: OwnedUse::Duplicate,
                ..
            }
        ));
    }

    let errors = compile(
        "record Token { value: I8 }\
         fn take(value: Token) {}\
         fn f(value: Token) { take(value); take(value); }",
    )
    .expect_err("unselected structurally simple record still consumes");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::UnavailableBinding)
    );
}

#[test]
fn selected_record_field_duplicates_from_binding_and_producer_receivers() {
    let hir = compile(
        "record copy Leaf { value: I8 }\
         record Holder { leaf: Leaf }\
         fn take(value: Leaf) {}\
         fn from_binding(holder: Holder) { take(holder.leaf); take(holder.leaf); }\
         fn from_producer() { take(Holder { leaf: Leaf { value: 1 } }.leaf); }",
    )
    .expect("selected record fields duplicate without consuming their paths");

    let binding_calls = function(&hir, "from_binding")
        .body
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Call { arguments, .. } => Some(arguments),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(binding_calls.len(), 2);
    for arguments in binding_calls {
        assert!(matches!(
            &arguments[0].kind,
            ValueKind::FieldValueUse {
                receiver: FieldValueReceiver::Binding { .. },
                ownership: OwnedUse::Duplicate,
                ..
            }
        ));
    }

    let producer_argument = match &function(&hir, "from_producer").body.statements[0] {
        Statement::Call { arguments, .. } => &arguments[0],
        _ => panic!("expected call statement"),
    };
    match &producer_argument.kind {
        ValueKind::FieldValueUse {
            receiver: FieldValueReceiver::Producer { cleanup, .. },
            fields,
            ownership,
        } => {
            assert_eq!(*ownership, OwnedUse::Duplicate);
            assert_eq!(fields, &[0]);
            assert_eq!(cleanup.paths, vec![Vec::<usize>::new()]);
        }
        _ => panic!("expected producer-backed field-value use"),
    }
}

#[test]
fn record_patterns_duplicate_selected_leaves_and_consume_unselected_siblings() {
    let hir = compile(
        "record copy Leaf { value: I8 }\
         record Token { value: I8 }\
         record Pair { leaf: Leaf, token: Token }\
         fn take_leaf(value: Leaf) {}\
         fn direct(pair: Pair) {\
             let Pair { leaf: leaf, token: token } = pair;\
             take_leaf(leaf); take_leaf(leaf);\
         }\
         fn producer() {\
             let Pair { leaf: leaf, token: token } = Pair { leaf: Leaf { value: 1 }, token: Token { value: 2 } };\
             take_leaf(leaf); take_leaf(leaf);\
         }",
    )
    .expect("record-aware pattern ownership is valid");

    let direct = match &function(&hir, "direct").body.statements[0] {
        Statement::RecordDestructure {
            scrutinee,
            bindings,
            ..
        } => {
            assert!(matches!(scrutinee, RecordPatternScrutinee::DirectRoot(_)));
            bindings
        }
        _ => panic!("expected direct-root destructuring"),
    };
    assert_eq!(direct.len(), 2);
    assert_eq!(direct[0].fields, vec![0]);
    assert_eq!(direct[0].ownership, OwnedUse::Duplicate);
    assert_eq!(direct[1].fields, vec![1]);
    assert_eq!(direct[1].ownership, OwnedUse::Consume);

    match &function(&hir, "producer").body.statements[0] {
        Statement::RecordDestructure {
            scrutinee: RecordPatternScrutinee::Producer { cleanup, .. },
            bindings,
            ..
        } => {
            assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
            assert_eq!(bindings[1].ownership, OwnedUse::Consume);
            assert_eq!(cleanup.paths, vec![vec![0]]);
        }
        _ => panic!("expected producer-backed destructuring"),
    }
}

#[test]
fn nested_pattern_leaf_uses_the_selected_record_classification() {
    let hir = compile(
        "record copy Leaf { value: I8 }\
         record Inner { leaf: Leaf }\
         record Outer { inner: Inner }\
         fn take(value: Leaf) {}\
         fn f(outer: Outer) {\
             let Outer { inner: Inner { leaf: leaf } } = outer;\
             take(leaf); take(leaf);\
         }",
    )
    .expect("nested selected leaf duplicates");

    match &function(&hir, "f").body.statements[0] {
        Statement::RecordDestructure { bindings, .. } => {
            assert_eq!(bindings.len(), 1);
            assert_eq!(bindings[0].fields, vec![0, 0]);
            assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
        }
        _ => panic!("expected record destructuring"),
    }
}

#[test]
fn selected_record_use_preserves_conditional_ownership_state() {
    let hir = compile(
        "record copy Token { value: I8 }\
         fn take(value: Token) {}\
         fn f(flag: Bool, token: Token) {\
             if flag { take(token); } else { take(token); }\
             take(token);\
         }",
    )
    .expect("selected record duplication leaves both conditional outcomes available");

    assert_eq!(function(&hir, "f").body.statements.len(), 2);
    assert!(matches!(function(&hir, "f").body.statements[0], Statement::If { .. }));
}
