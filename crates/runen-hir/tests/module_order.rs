use runen_hir::{ImportTarget, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

#[test]
fn qualified_resolution_ignores_import_item_and_unit_presentation_order() {
    let target =
        parse("export record Ticket {} export fn id(value: Ticket) -> Ticket { return value; }");
    let import_first = parse(
        "import dep; fn use_ticket(value: dep::Ticket) -> dep::Ticket { return dep::id(value); }",
    );
    let item_first = parse(
        "fn use_ticket(value: dep::Ticket) -> dep::Ticket { return dep::id(value); } import dep;",
    );
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).unwrap()];

    let first = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &target, &[]),
        SourceUnit::new(ModuleId::new(1), &import_first, &imports),
    ])
    .expect("qualified lookup must resolve with import before item and target unit first");

    let second = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &item_first, &imports),
        SourceUnit::new(ModuleId::new(2), &target, &[]),
    ])
    .expect("qualified lookup must resolve with item before import and importing unit first");

    for hir in [&first, &second] {
        let ticket = hir
            .records
            .iter()
            .find(|record| record.name == "Ticket")
            .expect("target record exists")
            .id;
        let function = hir
            .functions
            .iter()
            .find(|function| function.name == "use_ticket")
            .expect("importing function exists");

        assert_eq!(function.parameters[0].ty, Type::Record(ticket));
        assert_eq!(function.result, Some(Type::Record(ticket)));

        let returned = function
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("importing function returns one value");
        let ValueKind::DirectCall { function, .. } = &returned.kind else {
            panic!("qualified result call must remain a resolved direct call");
        };
        assert_eq!(hir.function(*function).name, "id");
    }
}
