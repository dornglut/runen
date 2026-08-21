use runen_hir::{
    Block, DiagnosticKind, ModuleId, OwnedUse, SourceUnit, Statement, TypedCompilation, ValueKind,
    build_typed_hir,
};
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

fn block(statement: &Statement) -> &Block {
    let Statement::Block(block) = statement else {
        panic!("expected nested block statement");
    };
    block
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn retains_nested_structure_cleanup_order_and_distinct_sibling_bindings() {
    let hir = build(
        "fn f(root: I64) { \
            { let a: I64 = root; let b: I64 = a; } \
            { let a: I64 = root; } \
        }",
    )
    .expect("disjoint sibling blocks may reuse one local key");
    let f = function(&hir, "f");

    let first = block(&f.body.statements[0]);
    let Statement::Local {
        binding: first_a, ..
    } = &first.statements[0]
    else {
        panic!("expected first sibling local a");
    };
    let Statement::Local {
        binding: first_b, ..
    } = &first.statements[1]
    else {
        panic!("expected first sibling local b");
    };
    assert_eq!(first.normal_cleanup, vec![*first_b, *first_a]);

    let second = block(&f.body.statements[1]);
    let Statement::Local {
        binding: second_a, ..
    } = &second.statements[0]
    else {
        panic!("expected second sibling local a");
    };
    assert_ne!(*first_a, *second_a);
    assert_eq!(second.normal_cleanup, vec![*second_a]);
}

#[test]
fn active_ancestor_shadowing_is_rejected_and_child_binding_ends_at_exit() {
    let shadowing = build("fn f(x: I64) { { let x: I64 = x; } }")
        .expect_err("active ancestor key reuse must be rejected");
    assert!(has_diagnostic(&shadowing, DiagnosticKind::LocalShadowing));

    let escaped = build("fn f() { { let x: I64 = 1; } let y: I64 = x; }")
        .expect_err("child local must not resolve after child scope exit");
    assert!(has_diagnostic(&escaped, DiagnosticKind::UnresolvedName));
}

#[test]
fn ancestor_availability_transitions_propagate_through_child_exit() {
    let consumed = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { { sink(value); } sink(value); }",
    )
    .expect_err("consuming a non-duplicable ancestor in a child must persist afterward");
    assert!(has_diagnostic(
        &consumed,
        DiagnosticKind::UnavailableBinding
    ));

    build("fn sink(value: I64) {} fn f(value: I64) { { sink(value); } sink(value); }")
        .expect("duplicating an intrinsic ancestor in a child must leave it available");
}

#[test]
fn ancestor_assignment_state_persists_after_child_exit() {
    let hir = build(
        "fn f(seed: I64) { \
            let mut x: I64 = seed; \
            { x = 2; } \
            let y: I64 = x; \
        }",
    )
    .expect("child assignment to mutable ancestor must persist after exit");
    let f = function(&hir, "f");

    let Statement::Local {
        binding: x_binding, ..
    } = &f.body.statements[0]
    else {
        panic!("expected root mutable local");
    };
    let child = block(&f.body.statements[1]);
    let Statement::Assignment { target, .. } = &child.statements[0] else {
        panic!("expected child assignment");
    };
    assert_eq!(*target, *x_binding);

    let Statement::Local { initializer, .. } = &f.body.statements[2] else {
        panic!("expected post-block local");
    };
    let ValueKind::BindingUse { binding, ownership } = initializer.kind else {
        panic!("post-block initializer must resolve the ancestor binding");
    };
    assert_eq!(binding, *x_binding);
    assert_eq!(ownership, OwnedUse::Duplicate);
}

#[test]
fn consumed_child_local_is_omitted_from_normal_cleanup() {
    let hir = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { { let child: Ticket = value; sink(child); } }",
    )
    .expect("consumed child local is valid and needs no normal-exit cleanup");
    let f = function(&hir, "f");
    let child = block(&f.body.statements[0]);

    assert!(child.normal_cleanup.is_empty());
}

#[test]
fn recursively_nested_blocks_retain_independent_cleanup_selections() {
    let hir = build("fn f() { { let outer: I64 = 1; { let inner: I64 = 2; } } }")
        .expect("recursive blocks must validate");
    let f = function(&hir, "f");
    let outer = block(&f.body.statements[0]);
    let Statement::Local {
        binding: outer_binding,
        ..
    } = &outer.statements[0]
    else {
        panic!("expected outer local");
    };
    let inner = block(&outer.statements[1]);
    let Statement::Local {
        binding: inner_binding,
        ..
    } = &inner.statements[0]
    else {
        panic!("expected inner local");
    };

    assert_eq!(inner.normal_cleanup, vec![*inner_binding]);
    assert_eq!(outer.normal_cleanup, vec![*outer_binding]);
}
