use runen_hir::{
    CallTarget, DiagnosticKind, Duplicability, ImportTarget, ModuleId, OwnedUse, SourceUnit,
    Statement, Type, TypedCompilation, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn closure_site_retains_fresh_opaque_type_capture_and_callable_interface() {
    let hir = build(
        "fn use(base: I64) -> I64 { \
             let add = fn[base](value: I64) -> I64 { return value + base; }; \
             return add(2); \
         }",
    )
    .expect("bounded closure is valid");
    let use_fn = function(&hir, "use");
    let [
        Statement::Closure {
            binding, closure, ..
        },
    ] = use_fn.body.statements.as_slice()
    else {
        panic!("expected one closure declaration");
    };
    let site = hir.closure(*closure);
    assert_eq!(site.captures.len(), 1);
    assert_eq!(site.captures[0].name, "base");
    assert_eq!(site.captures[0].ownership, OwnedUse::Duplicate);
    assert_eq!(site.duplicability, Duplicability::Duplicable);
    assert_eq!(site.parameters.len(), 1);
    assert_eq!(
        site.result,
        Some(Type::Intrinsic(runen_hir::IntrinsicType::I64))
    );

    let returned = use_fn
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns closure call");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("return must retain one call");
    };
    assert!(matches!(
        target,
        CallTarget::Closure {
            binding: target_binding,
            closure: target_closure,
        } if target_binding == binding && target_closure == closure
    ));
}

#[test]
fn equal_shape_closure_sites_keep_distinct_opaque_types() {
    let hir = build(
        "fn use(base: I64) { \
             let left = fn[base]() {}; \
             let right = fn[base]() {}; \
         }",
    )
    .expect("both closure sites are valid");
    assert_eq!(hir.closures.len(), 2);
    assert_ne!(hir.closures[0].id, hir.closures[1].id);
    assert_eq!(
        hir.closures[0].captures[0].ty,
        hir.closures[1].captures[0].ty
    );
}

#[test]
fn nonduplicable_capture_consumes_outer_root_and_makes_closure_one_shot() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) -> Box { \
             let take = fn[boxed]() -> I64 { return boxed.value; }; \
             return boxed; \
         }",
    )
    .expect_err("nonduplicable capture consumes the outer root");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));

    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) -> I64 { \
             let take = fn[boxed]() -> I64 { return boxed.value; }; \
             let first: I64 = take(); \
             return take(); \
         }",
    )
    .expect_err("nonduplicable closure call consumes its stored closure root");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
}

#[test]
fn rejected_capture_list_does_not_commit_prefix_consumption() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) -> I64 { \
             let rejected = fn[boxed, missing]() {}; \
             return boxed.value; \
         }",
    )
    .expect_err("missing capture rejects the declaration");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::InvalidClosureCapture
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "rejected closure formation must not consume the valid prefix capture"
    );
}

#[test]
fn invalid_closure_call_rolls_back_speculative_one_shot_snapshot() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) -> I64 { \
             let once = fn[boxed](value: I64) -> I64 { return value + boxed.value; }; \
             once(); \
             return once(1); \
         }",
    )
    .expect_err("first call has invalid arity");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::ArgumentCount {
            expected: 1,
            found: 0
        }
    )));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "invalid first call must not consume the one-shot closure"
    );
}

#[test]
fn captured_opaque_closure_is_category_final_and_non_callable() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) { \
             let inner = fn[boxed]() {}; \
             let outer = fn[inner]() { inner(); }; \
         }",
    )
    .expect_err("opaque closure-valued capture is not callable in this slice");
    assert!(has_diagnostic(&errors, DiagnosticKind::ExpectedFunction));
}

#[test]
fn closure_declarations_reject_generic_and_nested_activation_roots() {
    let errors = build("fn bad[T](value: T) { let c = fn[value]() {}; }")
        .expect_err("closures are excluded from generic functions");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ClosureInGenericFunction
    ));

    let errors = build(
        "fn bad(value: I64) { \
             let outer = fn[value]() { let inner = fn[value]() {}; }; \
         }",
    )
    .expect_err("nested closure declarations are excluded");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::NestedClosureDeclaration
    ));
}

#[test]
fn capture_and_explicit_parameter_keys_must_be_distinct() {
    let errors = build("fn bad(value: I64) { let c = fn[value](value: I64) {}; }")
        .expect_err("capture and parameter keys share one closure-local domain");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ClosureCaptureParameterConflict
    ));
}

#[test]
fn closure_body_is_a_fresh_activation_root_with_module_lookup_but_no_creator_local_fallback() {
    let errors = build(
        "fn helper() -> I64 { return 7; } \
         fn bad(hidden: I64, captured: I64) -> I64 { \
             let c = fn[captured]() -> I64 { return hidden; }; \
             return helper(); \
         }",
    )
    .expect_err("uncaptured creator locals must not enter closure lookup");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));

    build(
        "fn helper() -> I64 { return 7; } \
         fn ok(hidden: I64, captured: I64) -> I64 { \
             let c = fn[captured]() -> I64 { \
                 let hidden: I64 = helper(); \
                 return hidden + captured; \
             }; \
             return c(); \
         }",
    )
    .expect("fresh closure root keeps module lookup and permits reuse of uncaptured creator keys");
}

#[test]
fn closure_body_does_not_inherit_creator_loop_or_unsafe_context() {
    let loop_errors = build(
        "fn bad(flag: Bool, value: I64) { \
             while flag { \
                 let c = fn[value]() { break; }; \
                 break; \
             } \
         }",
    )
    .expect_err("closure-local break must not target the creator loop");
    assert!(has_diagnostic(
        &loop_errors,
        DiagnosticKind::BreakOutsideLoop
    ));

    let unsafe_errors = build(
        "fn bad(value: I64) { \
             unsafe { \
                 let c = fn[value]() { \
                     let pointer: raw I64 = raw &value; \
                     let moved: I64 = raw move pointer; \
                 }; \
             } \
         }",
    )
    .expect_err("creator unsafe admission must not cross the closure activation boundary");
    assert!(has_diagnostic(
        &unsafe_errors,
        DiagnosticKind::UnsafeOperationOutsideUnsafeBlock
    ));

    build(
        "fn ok(value: I64) { \
             let c = fn[value]() { \
                 let pointer: raw I64 = raw &value; \
                 unsafe { let moved: I64 = raw move pointer; } \
             }; \
         }",
    )
    .expect("a closure may establish its own bounded unsafe context");
}

#[test]
fn capture_bindings_allow_shared_and_raw_address_observation_but_not_mutation_or_replacement_roots()
{
    build(
        "record copy Pair { value: I64 } \
         fn ok(value: I64, pair: Pair) -> I64 { \
             let c = fn[value, pair]() -> I64 { \
                 let shared: &I64 = &value; \
                 let pointer: raw I64 = raw &value; \
                 let observed: I64 = pair.value; \
                 return *shared + observed; \
             }; \
             return c(); \
         }",
    )
    .expect("immutable captures retain ordinary Shared/raw-address/field observation");

    let assignment = build("fn bad(value: I64) { let c = fn[value]() { value = 2; }; }")
        .expect_err("capture bindings are immutable snapshots");
    assert!(has_diagnostic(
        &assignment,
        DiagnosticKind::ImmutableAssignmentTarget
    ));

    let replacement = build(
        "fn bad(value: I64) { \
             let c = fn[value]() { let replacement: &mut I64 = &mut value; }; \
         }",
    )
    .expect_err("capture roots cannot become replacement-capable roots");
    assert!(has_diagnostic(
        &replacement,
        DiagnosticKind::InvalidReplacementReferenceTarget
    ));
}

#[test]
fn capture_storage_is_never_a_safe_reference_result_origin() {
    let errors = build(
        "fn bad(value: I64) { \
             let c = fn[value]() -> &I64 { return &value; }; \
         }",
    )
    .expect_err("owned capture storage cannot satisfy a closure result-origin contract");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::MissingSharedReferenceResultOrigin
    ));
}

#[test]
fn closure_calls_participate_in_existing_condition_and_control_flow_state_relations() {
    build(
        "fn ok(value: I64) { \
             let predicate = fn[value]() -> Bool { return true; }; \
             if predicate() {} else {} \
             while predicate() { break; } \
         }",
    )
    .expect("duplicable Bool-returning closures may drive existing conditions repeatedly");

    let branch = build(
        "record Token {} \
         fn bad(flag: Bool) { \
             let token: Token = Token {}; \
             let once = fn[token]() -> Bool { return true; }; \
             if flag { let used: Bool = once(); } else {} \
         }",
    )
    .expect_err("one-sided one-shot closure consumption must fail the existing branch join");
    assert!(has_diagnostic(
        &branch,
        DiagnosticKind::ConditionalOwnershipMismatch
    ));

    let loop_condition = build(
        "record Token {} \
         fn bad() { \
             let token: Token = Token {}; \
             let once = fn[token]() -> Bool { return true; }; \
             while once() {} \
         }",
    )
    .expect_err("one-shot closure consumption cannot satisfy a repeated loop head");
    assert!(has_diagnostic(
        &loop_condition,
        DiagnosticKind::LoopOwnershipMismatch
    ));
}

#[test]
fn opaque_closure_capture_blocks_same_name_module_function_fallback() {
    let errors = build(
        "fn inner() {} \
         record Box { value: I64 } \
         fn bad(boxed: Box) { \
             let inner = fn[boxed]() {}; \
             let outer = fn[inner]() { inner(); }; \
         }",
    )
    .expect_err("captured opaque closure is a final local wrong-category target");
    assert!(has_diagnostic(&errors, DiagnosticKind::ExpectedFunction));
}

#[test]
fn generic_arguments_on_closure_target_reject_before_one_shot_snapshot() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) -> I64 { \
             let once = fn[boxed](value: I64) -> I64 { return value + boxed.value; }; \
             let invalid: I64 = once[I64](1); \
             return once(1); \
         }",
    )
    .expect_err("closure targets do not admit generic application");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnexpectedGenericTypeArguments
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "generic-target rejection must occur before consuming the one-shot closure"
    );
}

#[test]
fn capture_order_uniqueness_and_duplicable_record_transport_are_explicit() {
    let hir = build(
        "record copy Pair { value: I64 } \
         fn ok(left: I64, pair: Pair) -> I64 { \
             let c = fn[left, pair]() -> I64 { return left + pair.value; }; \
             return pair.value + c(); \
         }",
    )
    .expect("duplicable scalar/record captures leave their outer roots available");
    assert_eq!(
        hir.closures[0]
            .captures
            .iter()
            .map(|capture| capture.name.as_str())
            .collect::<Vec<_>>(),
        vec!["left", "pair"]
    );
    assert!(
        hir.closures[0]
            .captures
            .iter()
            .all(|capture| capture.ownership == OwnedUse::Duplicate)
    );

    let errors = build("fn bad(value: I64) { let c = fn[value, value]() {}; }")
        .expect_err("one outer binding cannot occur twice in a capture list");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::DuplicateClosureCapture
    ));
}

#[test]
fn safe_authority_capture_rejection_is_atomic_before_nonduplicable_prefix_commit() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box, value: I64) -> I64 { \
             let mut held: I64 = value; \
             let borrow: &mut I64 = &mut held; \
             let rejected = fn[boxed, held]() {}; \
             return boxed.value; \
         }",
    )
    .expect_err("exclusive authority over a later capture blocks Shared duplicate capture");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ReferencePermissionUnavailable
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed authority prevalidation must not consume the earlier nonduplicable capture"
    );
}

#[test]
fn later_closure_capture_consumes_an_earlier_nonduplicable_closure_root() {
    let errors = build(
        "record Box { value: I64 } \
         fn bad(boxed: Box) { \
             let inner = fn[boxed]() {}; \
             let outer = fn[inner]() {}; \
             inner(); \
         }",
    )
    .expect_err("capturing a nonduplicable earlier closure consumes that dedicated binding");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
}

#[test]
fn closure_body_retains_declaration_site_source_unit_alias_context() {
    let dependency = parse("export fn answer() -> I64 { return 42; }");
    let source = parse(
        "import dep; fn entry(seed: I64) -> I64 { \
             let c = fn[seed]() -> I64 { return dep::answer() + seed; }; \
             return c(); \
         }",
    );
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &source, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("closure body retains the declaration site's source-unit import aliases");
}

#[test]
fn final_call_entry_rejection_rolls_back_one_shot_snapshot_and_argument_effects() {
    let errors = build(
        "record Token {} \
         record Box { value: I64 } \
         fn bad(r: &mut Token, boxed: Box) { \
             let once = fn[boxed](arg: &mut Token) {}; \
             let moved: Token = *r; \
             once(r); \
             let mut other: Token = Token {}; \
             let other_ref: &mut Token = &mut other; \
             once(other_ref); \
         }",
    )
    .expect_err("the first call reaches final call-entry with an unavailable external referent");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ReferencePermissionUnavailable
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed final call-entry must roll back both the one-shot snapshot and argument binding use"
    );
}

#[test]
fn result_evidence_and_wrong_condition_type_do_not_double_consume_one_shot_closures() {
    build(
        "record Token {} \
         fn ok() -> Bool { \
             let token: Token = Token {}; \
             let once = fn[token]() -> I64 { return 42; }; \
             return once() == 42; \
         }",
    )
    .expect("comparison result-type evidence must not execute or consume its closure producer");

    let errors = build(
        "record Token {} \
         fn bad() -> I64 { \
             let token: Token = Token {}; \
             let once = fn[token]() -> I64 { return 42; }; \
             if once() {} \
             return once(); \
         }",
    )
    .expect_err("an I64-returning closure is not a Bool condition");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "rejected condition receiving must not commit the speculative one-shot snapshot"
    );
}
