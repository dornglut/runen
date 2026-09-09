#[test]
fn emit_builder_source_for_local_reconstruction() {
    panic!(
        "RUNEN_BUILDER_SOURCE_BEGIN\n{}\nRUNEN_BUILDER_SOURCE_END",
        include_str!("../src/build.rs")
    );
}
