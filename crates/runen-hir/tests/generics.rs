use runen_hir::{
    CallTarget, DiagnosticKind, ImportTarget, IntrinsicType, ModuleId, OwnedUse,
    SafeReferenceResultContract, SourceUnit, Statement, Type, TypeParameterId, TypedCompilation,
    ValueKind, build_typed_hir,
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
fn retains_ordered_function_type_parameter_identity_and_abstract_signature() {
    let hir = build("fn choose[T, U](left: T, right: U) -> T { return left; }")
        .expect("generic signature and body are valid");
    let choose = function(&hir, "choose");

    assert_eq!(choose.type_parameters.len(), 2);
    assert_eq!(choose.type_parameters[0].name, "T");
    assert_eq!(choose.type_parameters[1].name, "U");
    assert_eq!(
        choose.type_parameters[0].id,
        TypeParameterId {
            function: choose.id,
            index: 0,
        }
    );
    assert_eq!(
        choose.type_parameters[1].id,
        TypeParameterId {
            function: choose.id,
            index: 1,
        }
    );
    assert_eq!(
        choose.parameters[0].ty,
        Type::Parameter(choose.type_parameters[0].id)
    );
    assert_eq!(
        choose.parameters[1].ty,
        Type::Parameter(choose.type_parameters[1].id)
    );
    assert_eq!(
        choose.result,
        Some(Type::Parameter(choose.type_parameters[0].id))
    );
}

#[test]
fn abstract_whole_binding_use_is_consuming_even_when_a_call_substitutes_i64() {
    let hir = build(
        "fn id[T](value: T) -> T { return value; } \
         fn caller(value: I64) -> I64 { return id[I64](value); }",
    )
    .expect("explicit concrete generic application is valid");

    let id = function(&hir, "id");
    let returned = id
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("generic id returns one value");
    let ValueKind::BindingUse { ownership, .. } = returned.kind else {
        panic!("generic id return must retain one binding use");
    };
    assert_eq!(ownership, OwnedUse::Consume);

    let caller = function(&hir, "caller");
    let call = caller
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("caller returns generic call result");
    let ValueKind::Call {
        target: CallTarget::Direct { type_arguments, .. },
        arguments,
        ..
    } = &call.kind
    else {
        panic!("caller return must retain direct call");
    };
    assert_eq!(type_arguments, &[Type::Intrinsic(IntrinsicType::I64)]);
    let ValueKind::BindingUse { ownership, .. } = arguments[0].kind else {
        panic!("caller argument must retain binding production");
    };
    assert_eq!(ownership, OwnedUse::Duplicate);
}

#[test]
fn second_abstract_use_is_rejected_after_the_first_consuming_call_argument() {
    let errors = build(
        "fn sink[T](value: T) {} \
         fn bad[T](value: T) { sink[T](value); sink[T](value); }",
    )
    .expect_err("the first abstract call argument consumes the complete binding root");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
}

#[test]
fn generic_call_composes_an_enclosing_abstract_slot_without_reidentifying_it() {
    let hir = build(
        "fn id[T](value: T) -> T { return value; } \
         fn outer[U](value: U) -> U { return id[U](value); }",
    )
    .expect("abstract generic application inside generic body is valid");
    let outer = function(&hir, "outer");
    let outer_slot = outer.type_parameters[0].id;
    let call = outer
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("outer returns call result");
    let ValueKind::Call {
        target: CallTarget::Direct { type_arguments, .. },
        arguments,
        ..
    } = &call.kind
    else {
        panic!("outer return must retain direct call");
    };
    assert_eq!(type_arguments, &[Type::Parameter(outer_slot)]);
    assert_eq!(call.ty, Type::Parameter(outer_slot));
    let ValueKind::BindingUse { ownership, .. } = arguments[0].kind else {
        panic!("outer abstract argument must retain binding use");
    };
    assert_eq!(ownership, OwnedUse::Consume);
}

#[test]
fn bare_type_parameter_shadows_same_named_nominal_in_signature_and_local_type_positions() {
    let hir = build(
        "record T {} \
         fn f[T](value: T) -> T { let local: T = value; return local; }",
    )
    .expect("type parameter shadows same-module record in bare admitted type positions");
    let f = function(&hir, "f");
    let slot = f.type_parameters[0].id;
    assert_eq!(f.parameters[0].ty, Type::Parameter(slot));
    assert_eq!(f.result, Some(Type::Parameter(slot)));
    let [Statement::Local { ty, .. }] = f.body.statements.as_slice() else {
        panic!("expected one ordinary local declaration");
    };
    assert_eq!(*ty, Type::Parameter(slot));
}

#[test]
fn inadmissible_reference_or_raw_parameter_position_does_not_fall_back_to_nominal() {
    for source in [
        "record T {} fn f[T](value: &T) {}",
        "record T {} fn f[T](value: &mut T) {}",
        "record T {} fn f[T](value: raw T) {}",
    ] {
        let errors = build(source).expect_err("abstract referent/pointee position is invalid");
        assert!(has_diagnostic(
            &errors,
            DiagnosticKind::InvalidGenericTypeParameterPosition
        ));
    }
}

#[test]
fn qualified_type_positions_and_arguments_never_select_a_local_type_parameter() {
    let dependency = parse("export record T {}");
    let caller = parse(
        "import dep; \
         fn id[U](value: U) -> U { return value; } \
         fn f[T](value: dep::T) -> dep::T { return id[dep::T](value); }",
    );
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("qualified type lookup remains in the module domain");

    let dep_t = hir.records[0].id;
    let f = function(&hir, "f");
    assert_eq!(f.parameters[0].ty, Type::Record(dep_t));
    assert_eq!(f.result, Some(Type::Record(dep_t)));
    let call = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("f returns one direct call");
    let ValueKind::Call {
        target: CallTarget::Direct { type_arguments, .. },
        ..
    } = &call.kind
    else {
        panic!("f return must retain a direct call");
    };
    assert_eq!(type_arguments, &[Type::Record(dep_t)]);
}

#[test]
fn duplicate_type_parameter_keys_are_rejected_before_body_validation() {
    let errors = build("fn f[T, T](value: T) { missing(); }")
        .expect_err("duplicate generic binder is invalid");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::DuplicateTypeParameter
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnresolvedName),
        "body validation must not run after invalid generic headers"
    );
}

#[test]
fn distinct_abstract_slots_remain_unequal_even_if_a_future_call_could_map_them_equal() {
    let errors = build("fn bad[T, U](value: T) -> U { return value; }")
        .expect_err("distinct abstract slot types are unequal during generic-body validation");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Parameter(_),
            found: Type::Parameter(_),
        }
    )));
}

#[test]
fn exact_generic_arity_and_presence_are_enforced_after_target_resolution() {
    let missing = build(
        "fn id[T](value: T) -> T { return value; } \
         fn f(value: I64) -> I64 { return id(value); }",
    )
    .expect_err("generic target requires explicit type arguments");
    assert!(has_diagnostic(
        &missing,
        DiagnosticKind::MissingGenericTypeArguments
    ));

    let unexpected = build(
        "fn id(value: I64) -> I64 { return value; } \
         fn f(value: I64) -> I64 { return id[I64](value); }",
    )
    .expect_err("non-generic target rejects explicit type arguments");
    assert!(has_diagnostic(
        &unexpected,
        DiagnosticKind::UnexpectedGenericTypeArguments
    ));

    let wrong_arity = build(
        "fn pair[T, U](left: T, right: U) {} \
         fn f(left: I64, right: Bool) { pair[I64](left, right); }",
    )
    .expect_err("generic arity must match exactly");
    assert!(has_diagnostic(
        &wrong_arity,
        DiagnosticKind::GenericTypeArgumentCount {
            expected: 2,
            found: 1,
        }
    ));
}

#[test]
fn generic_application_failure_is_transactional_before_ordinary_argument_effects() {
    let errors = build(
        "record Ticket {} \
         fn sink[T](value: T) {} \
         fn f(value: Ticket) { sink[Missing](value); sink[Ticket](value); }",
    )
    .expect_err("first generic type argument is unresolved");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed generic application must not consume the ordinary argument"
    );
}

#[test]
fn abstract_parameter_does_not_gain_concrete_field_or_scalar_operator_capability() {
    let field_errors = build("fn bad[T](value: T) -> I64 { return value.field; }")
        .expect_err("unconstrained abstract type has no record field capability");
    assert!(has_diagnostic(
        &field_errors,
        DiagnosticKind::ExpectedRecordForFieldAccess
    ));

    let operator_errors = build("fn bad[T](left: T, right: T) -> T { return left + right; }")
        .expect_err("unconstrained abstract type has no addition capability");
    assert!(operator_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::AdditionRequiresIntegerOrFloating {
            required: Type::Parameter(_),
        }
    )));
}

#[test]
fn direct_and_mutual_generic_recursion_retain_explicit_abstract_applications() {
    let hir = build(
        "fn recursive[T](value: T) -> T { return recursive[T](value); } \
         fn left[T](value: T) -> T { return right[T](value); } \
         fn right[U](value: U) -> U { return left[U](value); }",
    )
    .expect("direct and mutual generic recursion validate under exact explicit applications");

    for name in ["recursive", "left", "right"] {
        let function = function(&hir, name);
        let slot = function.type_parameters[0].id;
        let returned = function
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("recursive function returns one direct call");
        let ValueKind::Call {
            target: CallTarget::Direct { type_arguments, .. },
            ..
        } = &returned.kind
        else {
            panic!("recursive return must retain a direct call");
        };
        assert_eq!(type_arguments, &[Type::Parameter(slot)]);
    }
}

#[test]
fn abstract_parameters_do_not_widen_safe_reference_result_origin_derivation() {
    let hir = build("fn id[T](value: T, reference: &I64) -> &I64 { return reference; }").expect(
        "concrete Shared-reference result contract remains valid beside an abstract parameter",
    );
    assert_eq!(
        function(&hir, "id").safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 1 }
    );
}

#[test]
fn caller_private_record_is_an_admitted_argument_to_accessible_exported_generic() {
    let dependency = parse("export fn id[T](value: T) -> T { return value; }");
    let caller = parse(
        "import dep; record Local {} \
         fn f(value: Local) -> Local { return dep::id[Local](value); }",
    );
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("caller-local nominal type argument is resolved at the application site");

    let f = function(&hir, "f");
    let local = hir
        .records
        .iter()
        .find(|record| record.name == "Local")
        .expect("caller local record exists")
        .id;
    let call = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("caller returns generic result");
    let ValueKind::Call {
        target: CallTarget::Direct { type_arguments, .. },
        ..
    } = &call.kind
    else {
        panic!("caller return must retain generic direct call");
    };
    assert_eq!(type_arguments, &[Type::Record(local)]);
}

#[test]
fn generic_call_statement_retains_exact_type_arguments() {
    let hir = build(
        "fn sink[T, U](left: T, right: U) {} \
         fn f(left: I64, right: Bool) { sink[I64, Bool](left, right); }",
    )
    .expect("generic call statement is valid");
    let f = function(&hir, "f");
    let [
        Statement::Call {
            target: CallTarget::Direct { type_arguments, .. },
            ..
        },
    ] = f.body.statements.as_slice()
    else {
        panic!("expected one retained call statement");
    };
    assert_eq!(
        type_arguments,
        &[
            Type::Intrinsic(IntrinsicType::I64),
            Type::Intrinsic(IntrinsicType::Bool),
        ]
    );
}
