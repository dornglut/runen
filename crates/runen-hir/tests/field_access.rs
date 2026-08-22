use runen_hir::{
    DiagnosticKind, FieldValueReceiver, ImportTarget, IntrinsicType, ModuleId, OwnedUse,
    RecordPatternScrutinee, SourceUnit, Statement, Type, Value, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, SyntaxKind, parse_source};

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
        .expect("result-bearing test function has returned value")
}

fn binding_receiver(value: &Value) -> (&FieldValueReceiver, &[usize], OwnedUse) {
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &value.kind
    else {
        panic!("expected field-value use");
    };
    (receiver, fields, *ownership)
}

fn has_kind(diagnostics: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.kind == kind)
}

fn count_kind(diagnostics: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.kind == kind)
        .count()
}

#[test]
fn resolves_paths_and_retains_duplicate_ownership() {
    let hir = build(
        "record Inner { pad: U8, value: I8 } \
         record Outer { first: I8, inner: Inner } \
         fn one(root: Outer) -> I8 { return root.first; } \
         fn nested(root: Outer) -> I8 { return root.inner.value; }",
    );

    let one = function(&hir, "one");
    let (receiver, fields, ownership) = binding_receiver(returned_value(one));
    let FieldValueReceiver::Binding { binding, ty } = receiver else {
        panic!("expected binding field receiver");
    };
    assert_eq!(*binding, one.parameters[0].binding);
    assert_eq!(*ty, Type::Record(hir.records[1].id));
    assert_eq!(fields, &[0]);
    assert_eq!(ownership, OwnedUse::Duplicate);
    assert_eq!(returned_value(one).ty, Type::Intrinsic(IntrinsicType::I8));

    let nested = function(&hir, "nested");
    let (receiver, fields, ownership) = binding_receiver(returned_value(nested));
    let FieldValueReceiver::Binding { binding, .. } = receiver else {
        panic!("expected binding field receiver");
    };
    assert_eq!(*binding, nested.parameters[0].binding);
    assert_eq!(fields, &[1, 1]);
    assert_eq!(ownership, OwnedUse::Duplicate);
}

#[test]
fn field_lookup_is_nominal_and_not_declaration_order_priority() {
    let hir = build(
        "record A { common: I8, other: U8 } \
         record B { other: U8, common: I8 } \
         fn a(root: A) -> I8 { return root.common; } \
         fn b(root: B) -> I8 { return root.common; }",
    );
    let ValueKind::FieldValueUse { fields: a, .. } = &returned_value(function(&hir, "a")).kind
    else {
        panic!("expected A field use");
    };
    let ValueKind::FieldValueUse { fields: b, .. } = &returned_value(function(&hir, "b")).kind
    else {
        panic!("expected B field use");
    };
    assert_eq!(a, &[0]);
    assert_eq!(b, &[1]);
}

#[test]
fn root_lookup_uses_active_binding_precedence_without_category_bypass() {
    let hir = build(
        "record root { value: U8 } record Box { value: I8 } \
         fn f(root: Box) -> I8 { return root.value; }",
    );
    let f = function(&hir, "f");
    let (receiver, fields, ownership) = binding_receiver(returned_value(f));
    let FieldValueReceiver::Binding { binding, .. } = receiver else {
        panic!("expected field use rooted in parameter");
    };
    assert_eq!(*binding, f.parameters[0].binding);
    assert_eq!(fields, &[0]);
    assert_eq!(ownership, OwnedUse::Duplicate);
    assert_eq!(returned_value(f).ty, Type::Intrinsic(IntrinsicType::I8));

    let wrong_category = errors("record root { value: I8 } fn g() -> I8 { return root.value; }");
    assert!(has_kind(
        &wrong_category,
        DiagnosticKind::ExpectedValueBinding
    ));
}

#[test]
fn producer_call_receiver_retains_producer_type_path_and_complete_duplicate_cleanup() {
    let hir = build(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn make() -> Outer { return Outer { inner: Inner { value: 7 } }; } \
         fn f() -> I8 { return make().inner.value; }",
    );
    let f = function(&hir, "f");
    let value = returned_value(f);
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &value.kind
    else {
        panic!("expected producer-backed field use");
    };
    let FieldValueReceiver::Producer {
        value: producer,
        cleanup,
    } = receiver
    else {
        panic!("expected producer receiver");
    };
    assert!(matches!(producer.kind, ValueKind::DirectCall { .. }));
    assert_eq!(producer.ty, Type::Record(hir.records[1].id));
    assert_eq!(fields, &[0, 0]);
    assert_eq!(*ownership, OwnedUse::Duplicate);
    assert_eq!(cleanup.paths, vec![Vec::<usize>::new()]);
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::I8));
}

#[test]
fn producer_construction_receiver_retains_complete_producer() {
    let hir = build(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn f() -> I8 { return Outer { inner: Inner { value: 9 } }.inner.value; }",
    );
    let value = returned_value(function(&hir, "f"));
    let ValueKind::FieldValueUse {
        receiver, fields, ..
    } = &value.kind
    else {
        panic!("expected producer-backed field use");
    };
    let FieldValueReceiver::Producer {
        value: producer, ..
    } = receiver
    else {
        panic!("expected producer receiver");
    };
    assert!(matches!(
        producer.kind,
        ValueKind::RecordConstruction { .. }
    ));
    assert_eq!(producer.ty, Type::Record(hir.records[1].id));
    assert_eq!(fields, &[0, 0]);
}

#[test]
fn producer_nonduplicable_selection_retains_canonical_remaining_frontier() {
    let hir = build(
        "record Token {} record Inner { token: Token, count: I8 } \
         record Outer { pad: I8, inner: Inner } \
         fn make() -> Outer { return Outer { pad: 1, inner: Inner { token: Token {}, count: 2 } }; } \
         fn f() -> Token { return make().inner.token; }",
    );
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &returned_value(function(&hir, "f")).kind
    else {
        panic!("expected producer-backed field use");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected producer receiver");
    };
    assert_eq!(fields, &[1, 0]);
    assert_eq!(*ownership, OwnedUse::Consume);
    assert_eq!(cleanup.paths, vec![vec![1, 1], vec![0]]);
}

#[test]
fn zero_leaf_receiver_cleanup_remains_retained_in_hir() {
    let hir = build(
        "record Token { value: I8 } record Empty {} record Box { token: Token, empty: Empty } \
         fn make() -> Box { return Box { token: Token { value: 1 }, empty: Empty {} }; } \
         fn f() -> Token { return make().token; }",
    );
    let ValueKind::FieldValueUse { receiver, .. } = &returned_value(function(&hir, "f")).kind
    else {
        panic!("expected producer-backed field use");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected producer receiver");
    };
    assert_eq!(cleanup.paths, vec![vec![1]]);
}

#[test]
fn producer_field_location_is_the_complete_field_use() {
    let source = "record Box { value: I8 } fn make() -> Box { return Box { value: 1 }; } fn f() -> I8 { return make().value; }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty());
    let syntax_range = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::FieldValueUse)
        .expect("field-value syntax node")
        .text_range();
    let hir =
        build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])]).expect("accepted HIR");
    let value = returned_value(function(&hir, "f"));
    assert_eq!(value.location.unit, 0);
    assert_eq!(value.location.range, syntax_range);
}

#[test]
fn producer_static_rejection_does_not_commit_receiver_consumption() {
    let final_type = errors(
        "record Ticket {} record Box { value: I8 } \
         fn make(ticket: Ticket) -> Box { return Box { value: 1 }; } fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: U8 = make(ticket).value; sink(ticket); }",
    );
    assert!(
        final_type
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(!has_kind(&final_type, DiagnosticKind::UnavailableBinding));

    let unknown_selector = errors(
        "record Ticket {} record Box { value: I8 } \
         fn make(ticket: Ticket) -> Box { return Box { value: 1 }; } fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: I8 = make(ticket).missing; sink(ticket); }",
    );
    assert!(has_kind(
        &unknown_selector,
        DiagnosticKind::UnknownRecordField
    ));
    assert!(!has_kind(
        &unknown_selector,
        DiagnosticKind::UnavailableBinding
    ));

    let non_record = errors(
        "record Ticket {} fn make(ticket: Ticket) -> I8 { return 1; } fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: I8 = make(ticket).value; sink(ticket); }",
    );
    assert!(has_kind(
        &non_record,
        DiagnosticKind::ExpectedRecordForFieldAccess
    ));
    assert!(!has_kind(&non_record, DiagnosticKind::UnavailableBinding));

    let no_result = errors(
        "record Ticket {} fn make(ticket: Ticket) {} fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: I8 = make(ticket).value; sink(ticket); }",
    );
    assert!(has_kind(
        &no_result,
        DiagnosticKind::NoResultCallUsedAsValue
    ));
    assert!(!has_kind(&no_result, DiagnosticKind::UnavailableBinding));
}

#[test]
fn construction_receiver_static_or_dynamic_rejection_rolls_back_initializer_consumption() {
    let final_type = errors(
        "record Ticket {} record Box { ticket: Ticket, value: I8 } fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: U8 = Box { ticket: ticket, value: 1 }.value; sink(ticket); }",
    );
    assert!(
        final_type
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(!has_kind(&final_type, DiagnosticKind::UnavailableBinding));

    let invalid_initializer = errors(
        "record Ticket {} record Box { ticket: Ticket, value: I8 } fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: I8 = Box { ticket: ticket, value: missing }.value; sink(ticket); }",
    );
    assert!(has_kind(
        &invalid_initializer,
        DiagnosticKind::UnresolvedName
    ));
    assert!(!has_kind(
        &invalid_initializer,
        DiagnosticKind::UnavailableBinding
    ));
}

#[test]
fn successful_producer_receivers_commit_call_and_constructor_consumption() {
    let diagnostics = errors(
        "record Ticket {} record Box { ticket: Ticket, value: I8 } \
         fn make(ticket: Ticket) -> Box { return Box { ticket: ticket, value: 1 }; } \
         fn sink(ticket: Ticket) {} \
         fn call_case(ticket: Ticket) { let value: I8 = make(ticket).value; sink(ticket); } \
         fn construction_case(ticket: Ticket) { let value: I8 = Box { ticket: ticket, value: 1 }.value; sink(ticket); }",
    );
    assert_eq!(
        count_kind(&diagnostics, DiagnosticKind::UnavailableBinding),
        2
    );
}

#[test]
fn invalid_receiver_producer_does_not_commit_speculative_argument_consumption() {
    let diagnostics = errors(
        "record Ticket {} record Box { value: I8 } \
         fn make(ticket: Ticket, count: I8) -> Box { return Box { value: count }; } \
         fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { let bad: I8 = make(ticket, missing).value; sink(ticket); }",
    );
    assert!(has_kind(&diagnostics, DiagnosticKind::UnresolvedName));
    assert!(!has_kind(&diagnostics, DiagnosticKind::UnavailableBinding));
}

#[test]
fn producer_bool_field_composes_as_existing_conditional_value() {
    let hir = build(
        "record Flag { ready: Bool } \
         fn make() -> Flag { return Flag { ready: true }; } \
         fn f() { if make().ready {} }",
    );
    let Statement::If { condition, .. } = &function(&hir, "f").body.statements[0] else {
        panic!("expected conditional");
    };
    assert_eq!(condition.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &condition.kind
    else {
        panic!("expected field-value condition");
    };
    let FieldValueReceiver::Producer {
        value: producer,
        cleanup,
    } = receiver
    else {
        panic!("expected producer receiver");
    };
    assert!(matches!(producer.kind, ValueKind::DirectCall { .. }));
    assert_eq!(fields, &[0]);
    assert_eq!(*ownership, OwnedUse::Duplicate);
    assert_eq!(cleanup.paths, vec![Vec::<usize>::new()]);
}

#[test]
fn producer_record_field_pattern_keeps_field_and_pattern_transients_distinct() {
    let hir = build(
        "record Token { value: I8 } record Inner { token: Token, count: I8 } \
         record Outer { inner: Inner, pad: I8 } \
         fn make() -> Outer { return Outer { inner: Inner { token: Token { value: 1 }, count: 2 }, pad: 3 }; } \
         fn f() { let Inner { token: moved, count: copied } = make().inner; }",
    );
    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected record destructuring");
    };
    let RecordPatternScrutinee::Producer {
        value,
        cleanup: pattern_cleanup,
    } = scrutinee
    else {
        panic!("expected producer pattern scrutinee");
    };
    let ValueKind::FieldValueUse {
        receiver,
        fields,
        ownership,
    } = &value.kind
    else {
        panic!("expected producer-backed field scrutinee value");
    };
    let FieldValueReceiver::Producer {
        value: producer,
        cleanup: field_cleanup,
    } = receiver
    else {
        panic!("expected producer field receiver");
    };
    assert!(matches!(producer.kind, ValueKind::DirectCall { .. }));
    assert_eq!(fields, &[0]);
    assert_eq!(*ownership, OwnedUse::Consume);
    assert_eq!(field_cleanup.paths, vec![vec![1]]);
    assert_eq!(pattern_cleanup.paths, vec![vec![1]]);
}

#[test]
fn whole_consumed_root_makes_descendant_field_unavailable() {
    let unavailable = errors(
        "record Box { value: I8 } \
         fn take(value: Box) {} \
         fn f(root: Box) -> I8 { take(root); return root.value; }",
    );
    assert!(has_kind(
        &unavailable,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn rejects_non_record_and_unknown_fields() {
    let non_record = errors("fn f(root: I8) -> I8 { return root.value; }");
    assert!(has_kind(
        &non_record,
        DiagnosticKind::ExpectedRecordForFieldAccess
    ));

    let unknown = errors(
        "record Box { value: I8 } record Other { missing: I8 } \
         fn f(root: Box) -> I8 { return root.missing; }",
    );
    assert!(has_kind(&unknown, DiagnosticKind::UnknownRecordField));
}

#[test]
fn nonduplicable_final_field_is_consumed_and_retained_in_hir() {
    let hir = build(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn f(root: Outer) -> Inner { return root.inner; }",
    );
    let f = function(&hir, "f");
    let (receiver, fields, ownership) = binding_receiver(returned_value(f));
    let FieldValueReceiver::Binding { binding, .. } = receiver else {
        panic!("expected binding receiver");
    };
    assert_eq!(*binding, f.parameters[0].binding);
    assert_eq!(fields, &[0]);
    assert_eq!(ownership, OwnedUse::Consume);
}

#[test]
fn nested_nonduplicable_field_consumes_exact_resolved_path() {
    let hir = build(
        "record Leaf { value: I8 } \
         record Inner { pad: I8, leaf: Leaf } \
         record Outer { first: I8, inner: Inner } \
         fn f(root: Outer) -> Leaf { return root.inner.leaf; }",
    );
    let ValueKind::FieldValueUse {
        fields, ownership, ..
    } = &returned_value(function(&hir, "f")).kind
    else {
        panic!("expected nested consuming field-value use");
    };
    assert_eq!(fields, &[1, 1]);
    assert_eq!(*ownership, OwnedUse::Consume);
}

#[test]
fn repeated_consumption_and_ancestor_whole_use_are_rejected() {
    let repeated = errors(
        "record Inner {} record Outer { inner: Inner } \
         fn sink(value: Inner) {} \
         fn f(root: Outer) { sink(root.inner); sink(root.inner); }",
    );
    assert!(has_kind(&repeated, DiagnosticKind::UnavailableFieldValue));

    let ancestor = errors(
        "record Inner {} record Outer { inner: Inner } \
         fn sink_inner(value: Inner) {} fn sink_outer(value: Outer) {} \
         fn f(root: Outer) { sink_inner(root.inner); sink_outer(root); }",
    );
    assert!(has_kind(&ancestor, DiagnosticKind::UnavailableBinding));
}

#[test]
fn disjoint_siblings_remain_available_after_consumption() {
    build(
        "record Left {} record Right {} \
         record Pair { left: Left, right: Right, count: I8 } \
         fn sink_left(value: Left) {} fn sink_right(value: Right) {} \
         fn f(root: Pair) -> I8 { \
             sink_left(root.left); \
             sink_right(root.right); \
             return root.count; \
         }",
    );
}

#[test]
fn partially_available_intermediate_allows_untouched_descendant() {
    build(
        "record Token {} record Inner { token: Token, value: I8 } \
         record Outer { inner: Inner } fn sink(value: Token) {} \
         fn f(root: Outer) -> I8 { sink(root.inner.token); return root.inner.value; }",
    );
}

#[test]
fn nonduplicable_intermediate_records_allow_deeper_duplicable_field() {
    let hir = build(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn f(root: Outer) -> I8 { return root.inner.value; }",
    );
    assert!(matches!(
        returned_value(function(&hir, "f")).kind,
        ValueKind::FieldValueUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
}

#[test]
fn repeated_duplicate_access_leaves_root_available_for_whole_consumption() {
    let hir = build(
        "record Box { value: I8 } \
         fn f(root: Box) -> Box { \
             let first: I8 = root.value; \
             let second: I8 = root.value; \
             return root; \
         }",
    );
    let f = function(&hir, "f");
    assert_eq!(f.body.statements.len(), 2);
    for statement in &f.body.statements {
        let Statement::Local { initializer, .. } = statement else {
            panic!("expected local");
        };
        assert!(matches!(
            initializer.kind,
            ValueKind::FieldValueUse {
                ownership: OwnedUse::Duplicate,
                ..
            }
        ));
    }
    assert!(matches!(
        returned_value(f).kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    ));
}

#[test]
fn immutable_partial_root_rejects_assignment_without_losing_disjoint_field() {
    let diagnostics = errors(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn sink_left(value: Left) {} fn sink_right(value: Right) {} \
         fn f(root: Pair) { \
             sink_left(root.left); \
             root = Pair { left: Left {}, right: Right {} }; \
             sink_right(root.right); \
         }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::ImmutableAssignmentTarget
    ));
    assert!(!has_kind(
        &diagnostics,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn mutable_partial_root_whole_replacement_restores_full_availability() {
    let hir = build(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn sink_left(value: Left) {} fn sink_pair(value: Pair) {} \
         fn f() { \
             let mut pair: Pair = Pair { left: Left {}, right: Right {} }; \
             sink_left(pair.left); \
             pair = Pair { left: Left {}, right: Right {} }; \
             sink_pair(pair); \
         }",
    );
    let f = function(&hir, "f");
    assert_eq!(f.body.statements.len(), 4);
    assert!(matches!(f.body.statements[2], Statement::Assignment { .. }));
}

#[test]
fn assignment_rhs_can_consume_target_field_before_successful_whole_reset() {
    let hir = build(
        "record Token {} record Holder { token: Token, count: I8 } \
         fn sink(value: Holder) {} \
         fn f() { \
             let mut holder: Holder = Holder { token: Token {}, count: 1 }; \
             holder = Holder { token: holder.token, count: 2 }; \
             sink(holder); \
         }",
    );
    let f = function(&hir, "f");
    let Statement::Local { binding, .. } = &f.body.statements[0] else {
        panic!("expected mutable holder local");
    };
    let Statement::Assignment { target, value, .. } = &f.body.statements[1] else {
        panic!("expected whole-binding assignment");
    };
    assert_eq!(*target, *binding);
    let ValueKind::RecordConstruction { fields, .. } = &value.kind else {
        panic!("assignment RHS must remain a record construction");
    };
    let ValueKind::FieldValueUse {
        receiver,
        ownership: OwnedUse::Consume,
        ..
    } = &fields[0].value.kind
    else {
        panic!("expected consuming field initializer");
    };
    assert!(matches!(
        receiver,
        FieldValueReceiver::Binding { binding: consumed, .. } if consumed == binding
    ));
    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected post-assignment call");
    };
    assert!(matches!(
        arguments[0].kind,
        ValueKind::BindingUse {
            binding: restored,
            ownership: OwnedUse::Consume,
        } if restored == *binding
    ));
}

#[test]
fn call_and_constructor_producers_observe_left_to_right_consumption() {
    let diagnostics = errors(
        "record Token {} record Pair { left: Token, right: Token } \
         record Holder { a: Token, b: Token } \
         fn two(a: Token, b: Token) {} \
         fn bad_call(root: Pair) { two(root.left, root.left); } \
         fn bad_constructor(root: Pair) -> Holder { \
             return Holder { a: root.left, b: root.left }; \
         }",
    );
    assert_eq!(
        count_kind(&diagnostics, DiagnosticKind::UnavailableFieldValue),
        2
    );
}

#[test]
fn rejected_required_type_does_not_apply_consumption_transition() {
    let whole = errors(
        "record Ticket {} fn needs_i8(value: I8) {} fn take(value: Ticket) {} \
         fn f(ticket: Ticket) { needs_i8(ticket); take(ticket); }",
    );
    assert!(
        whole
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(!has_kind(&whole, DiagnosticKind::UnavailableBinding));

    let field = errors(
        "record Inner {} record Outer { inner: Inner } \
         fn needs_i8(value: I8) {} fn take(value: Inner) {} \
         fn f(root: Outer) { needs_i8(root.inner); take(root.inner); }",
    );
    assert!(
        field
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(!has_kind(&field, DiagnosticKind::UnavailableFieldValue));
}

#[test]
fn direct_access_to_foreign_record_fields_is_rejected() {
    let foreign = parse("export record Foreign { value: I8 }");
    let local = parse("import ext; fn f(root: ext::Foreign) -> I8 { return root.value; }");
    let ext = ImportTarget::new("ext", ModuleId::new(1)).expect("valid alias");
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &foreign, &[]),
        SourceUnit::new(ModuleId::new(2), &local, &[ext]),
    ])
    .expect_err("foreign direct field access must be rejected");

    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
}

#[test]
fn qualified_call_receiver_resolves_before_foreign_field_access_rejects() {
    let foreign = parse(
        "export record Foreign { value: I8 } export fn make() -> Foreign { return Foreign { value: 1 }; }",
    );
    let local = parse("import ext; fn f() -> I8 { return ext::make().value; }");
    let ext = ImportTarget::new("ext", ModuleId::new(1)).expect("valid alias");
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &foreign, &[]),
        SourceUnit::new(ModuleId::new(2), &local, &[ext]),
    ])
    .expect_err("foreign field selection remains inaccessible");
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
    assert!(!has_kind(&diagnostics, DiagnosticKind::UnresolvedName));
    assert!(!has_kind(&diagnostics, DiagnosticKind::ExpectedFunction));
}

#[test]
fn nested_path_may_reach_foreign_record_but_cannot_select_inside_it() {
    let foreign = parse("export record Foreign { value: I8 }");
    let local = parse(
        "import ext; record Local { foreign: ext::Foreign } \
         fn f(root: Local) -> I8 { return root.foreign.value; }",
    );
    let ext = ImportTarget::new("ext", ModuleId::new(1)).expect("valid alias");
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &foreign, &[]),
        SourceUnit::new(ModuleId::new(2), &local, &[ext]),
    ])
    .expect_err("selector into foreign record must be rejected");

    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
}

#[test]
fn duplicable_field_values_require_exact_consumer_types() {
    let diagnostics = errors(
        "record Box { value: I8 } record Holder { value: U8 } \
         fn sink(value: U8) {} \
         fn bad_local(root: Box) { let value: U8 = root.value; } \
         fn bad_assignment(root: Box) { let mut target: U8 = 0; target = root.value; } \
         fn bad_call(root: Box) { sink(root.value); } \
         fn bad_return(root: Box) -> U8 { return root.value; } \
         fn bad_constructor(root: Box) -> Holder { return Holder { value: root.value }; }",
    );
    assert!(
        diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
            .count()
            >= 5
    );
}
