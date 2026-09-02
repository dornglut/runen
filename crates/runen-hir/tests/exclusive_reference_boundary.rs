use runen_hir::{DiagnosticKind, ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::parse_source;

fn compile(source: &str) -> Result<runen_hir::TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse_source(source.as_bytes()).expect("valid UTF-8 test source");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn replacement_root_rejects_overlapping_shared_authority() {
    let errors = compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let shared: &I64 = &x;\
             let replacement: &mut I64 = &mut x;\
         }",
    )
    .expect_err("replacement root requires no overlapping safe authority");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::InvalidReplacementReferenceTarget),
        "missing overlapping-root rejection: {errors:?}"
    );
}

#[test]
fn replacement_reference_carrier_cannot_be_used_twice() {
    let errors = compile(
        "fn f(r: &mut I64) {\
             let first: &mut I64 = r;\
             let second: &mut I64 = r;\
         }",
    )
    .expect_err("replacement-reference carrier transport is Move, never copy");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "second carrier use must observe the first Move: {errors:?}"
    );
}

#[test]
fn explicit_return_requires_incoming_replacement_referent_restored() {
    let errors = compile(
        "record Ticket { value: I64 }\
         fn f(r: &mut Ticket) -> I64 {\
             let ticket: Ticket = *r;\
             return 1;\
         }",
    )
    .expect_err("normal Return may not leave the incoming external referent consumed");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ReferenceRestorationRequired),
        "explicit Return must apply restoration after result effects: {errors:?}"
    );
}

#[test]
fn raw_address_coexists_with_shared_authority_but_not_replacement_authority() {
    compile(
        "fn shared(seed: I64) {\
             let x: I64 = seed;\
             let shared: &I64 = &x;\
             let pointer: raw I64 = raw &x;\
         }",
    )
    .expect("raw address formation uses the Shared direct-compatibility requirement");

    let errors = compile(
        "fn exclusive(seed: I64) {\
             let mut x: I64 = seed;\
             let replacement: &mut I64 = &mut x;\
             let pointer: raw I64 = raw &x;\
         }",
    )
    .expect_err("raw address formation conflicts with a live replacement authority");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::RawTargetSafeAuthorityConflict),
        "missing raw-address/replacement-authority conflict: {errors:?}"
    );
}

#[test]
fn shared_child_restricts_parent_to_shared_relative_capability() {
    compile(
        "fn f(r: &mut I64) {\
             let child: &I64 = &*r;\
             let copied_from_parent: I64 = *r;\
             let copied_from_child: I64 = *child;\
         }",
    )
    .expect("a Shared child leaves its replacement parent with Shared relative capability");
}

#[test]
fn replacement_child_suspends_parent_reference_access() {
    let errors = compile(
        "fn f(r: &mut I64) {\
             let child: &mut I64 = &mut *r;\
             let copied_from_parent: I64 = *r;\
         }",
    )
    .expect_err("a live replacement child suspends parent reference-relative access");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ReferencePermissionUnavailable),
        "missing suspended-parent authority diagnostic: {errors:?}"
    );
}

#[test]
fn conditional_rejects_unequal_live_reference_authority_state() {
    let errors = compile(
        "fn f(flag: Bool, r: &mut I64) {\
             if flag { let moved: &mut I64 = r; } else {}\
         }",
    )
    .expect_err("two normal conditional outcomes require equal live reference authority state");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ConditionalReferenceStateMismatch),
        "missing conditional reference-state mismatch: {errors:?}"
    );
}
