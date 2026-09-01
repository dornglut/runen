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
fn direct_call_holds_multiple_shared_carriers_of_the_same_authority() {
    compile(
        "fn sum(a: &I64, b: &I64) -> I64 { return *a + *b; }\
         fn f(r: &I64) -> I64 { return sum(r, r); }",
    )
    .expect("left-to-right call validation must retain both Shared argument carriers until entry");
}

#[test]
fn failed_record_construction_rolls_back_an_earlier_local_move() {
    let errors = compile(
        "record Ticket { value: I64 }\
         record Pair { ticket: Ticket, number: I64 }\
         fn f(ticket: Ticket) {\
             let bad: Pair = Pair { ticket: ticket, number: true };\
             let recovered: Ticket = ticket;\
         }",
    )
    .expect_err("the later field type mismatch must reject the constructor");

    assert!(
        errors
            .iter()
            .any(|error| matches!(error.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed complete producer must not commit the earlier Ticket move: {errors:?}"
    );
}

#[test]
fn failed_record_construction_rolls_back_external_referent_move() {
    let errors = compile(
        "record Ticket { value: I64 }\
         record Pair { ticket: Ticket, number: I64 }\
         fn f(r: &mut Ticket) {\
             let bad: Pair = Pair { ticket: *r, number: true };\
         }",
    )
    .expect_err("the later field type mismatch must reject the constructor");

    assert!(
        errors
            .iter()
            .any(|error| matches!(error.kind, DiagnosticKind::TypeMismatch { .. }))
    );
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::ReferenceRestorationRequired),
        "failed complete producer must not leak the earlier external-referent Move: {errors:?}"
    );
}
