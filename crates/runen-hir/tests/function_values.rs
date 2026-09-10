use runen_hir::{
    CallTarget, DiagnosticKind, FunctionTypeId, IntrinsicType, ModuleId,
    SafeReferenceResultContract, SourceUnit, Statement, Type, TypedCompilation, ValueKind,
    build_typed_hir,
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

fn function_type(ty: Type) -> FunctionTypeId {
    let Type::Function(id) = ty else {
        panic!("expected function type, found {ty:?}");
    };
    id
}

#[test]
fn equal_structural_function_types_share_one_canonical_handle_and_nested_types_resolve() {
    let hir = build(
        "fn left(value: I64) -> I64 { return value; } \
         fn right(value: I64) -> I64 { return value; } \
         fn use(a: fn(I64) -> I64, b: fn(I64) -> I64, nested: fn(fn(I64) -> I64) -> I64) {}",
    )
    .expect("concrete function types are valid");
    let use_fn = function(&hir, "use");
    let a = function_type(use_fn.parameters[0].ty);
    let b = function_type(use_fn.parameters[1].ty);
    assert_eq!(a, b);
    let nested = function_type(use_fn.parameters[2].ty);
    assert_ne!(a, nested);
    assert_eq!(hir.function_type(nested).parameters, &[Type::Function(a)]);
    assert_eq!(
        hir.function_type(a).result,
        Some(Type::Intrinsic(IntrinsicType::I64))
    );
}

#[test]
fn function_values_form_contextually_and_remain_distinct_payloads_under_equal_type() {
    let hir = build(
        "fn left(value: I64) -> I64 { return value; } \
         fn right(value: I64) -> I64 { return value; } \
         fn use() -> I64 { \
             let mut f: fn(I64) -> I64 = left; \
             let g: fn(I64) -> I64 = right; \
             f = g; \
             return f(7); \
         }",
    )
    .expect("function values use ordinary local transport");
    let use_fn = function(&hir, "use");
    let [
        Statement::Local {
            initializer: left, ..
        },
        Statement::Local {
            initializer: right, ..
        },
        Statement::Assignment {
            value: assigned, ..
        },
    ] = use_fn.body.statements.as_slice()
    else {
        panic!("expected two locals and one assignment");
    };
    let ValueKind::FunctionValue { function: left_id } = left.kind else {
        panic!("left initializer must be a function value");
    };
    let ValueKind::FunctionValue { function: right_id } = right.kind else {
        panic!("right initializer must be a function value");
    };
    assert_ne!(left_id, right_id);
    assert_eq!(left.ty, right.ty);
    assert!(matches!(assigned.kind, ValueKind::BindingUse { .. }));

    let returned = use_fn
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns an indirect call result");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("return must retain one call");
    };
    assert!(matches!(target, CallTarget::Indirect { .. }));
}

#[test]
fn indirect_no_result_call_uses_the_same_call_target_relation() {
    let hir = build("fn sink(value: I64) {} fn use(f: fn(I64)) { f(1); }")
        .expect("no-result indirect call is valid");
    let use_fn = function(&hir, "use");
    let [
        Statement::Call {
            target, arguments, ..
        },
    ] = use_fn.body.statements.as_slice()
    else {
        panic!("expected one call statement");
    };
    assert!(matches!(target, CallTarget::Indirect { .. }));
    assert_eq!(arguments.len(), 1);
}

#[test]
fn direct_calls_remain_direct_and_non_callable_locals_block_module_fallback() {
    let hir = build(
        "fn target(value: I64) -> I64 { return value; } fn good() -> I64 { return target(1); }",
    )
    .expect("ordinary direct call remains valid");
    let returned = function(&hir, "good")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("good returns a call");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("expected retained call");
    };
    assert!(matches!(target, CallTarget::Direct { .. }));

    let errors = build(
        "fn target(value: I64) -> I64 { return value; } fn bad(target: I64) -> I64 { return target(1); }",
    )
    .expect_err("local lookup is final for call classification");
    assert!(has_diagnostic(&errors, DiagnosticKind::ExpectedFunction));
}

#[test]
fn indirect_generic_arguments_and_generic_function_values_are_rejected() {
    let errors = build("fn use(f: fn(I64) -> I64) -> I64 { return f[I64](1); }")
        .expect_err("indirect calls do not accept generic arguments");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnexpectedGenericTypeArguments
    ));

    let errors = build(
        "fn id[T](value: T) -> T { return value; } fn bad() -> fn(I64) -> I64 { return id; }",
    )
    .expect_err("generic functions do not form function values");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::GenericFunctionValue
    ));
}

#[test]
fn abstract_components_and_function_valued_record_fields_remain_excluded() {
    let errors = build("fn bad[T](f: fn(T) -> I64) {}").expect_err("function types are concrete");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::InvalidGenericTypeParameterPosition
    ));

    let errors = build("record Bad { f: fn(I64) -> I64 }")
        .expect_err("record fields cannot be function-valued");
    assert!(has_diagnostic(&errors, DiagnosticKind::FunctionTypeField));
}

#[test]
fn shared_reference_function_type_contract_is_derived_once_and_retained() {
    let hir = build("fn use(f: fn(&I64) -> &I64) {}").expect("shared result has unique origin");
    let function_type = function_type(function(&hir, "use").parameters[0].ty);
    assert_eq!(
        hir.function_type(function_type)
            .safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
}

#[test]
fn unequal_function_type_structures_use_distinct_canonical_handles() {
    let hir = build(
        "fn use( \
             a: fn(I64) -> I64, \
             b: fn(Bool) -> I64, \
             c: fn(I64) -> Bool, \
             identity: fn(&I64) -> &I64, \
             child: fn(&mut I64) -> &I64 \
         ) {}",
    )
    .expect("distinct concrete function-type structures are valid");
    let use_fn = function(&hir, "use");
    let a = function_type(use_fn.parameters[0].ty);
    let b = function_type(use_fn.parameters[1].ty);
    let c = function_type(use_fn.parameters[2].ty);
    let identity = function_type(use_fn.parameters[3].ty);
    let child = function_type(use_fn.parameters[4].ty);
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(identity, child);
    assert_eq!(
        hir.function_type(identity).safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    assert_eq!(
        hir.function_type(child).safe_reference_result_contract,
        SafeReferenceResultContract::SharedDirectChild { origin: 0 }
    );
}
