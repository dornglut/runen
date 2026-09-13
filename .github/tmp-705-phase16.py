from pathlib import Path

path = Path("crates/runen-reference/src/interprocedural.rs")
text = path.read_text()
old = '''enum ExternalProvider {
    NoResult(Box<dyn FnMut(&[ExternalScalarValue])>),
    ScalarResult(Box<dyn FnMut(&[ExternalScalarValue]) -> ExternalScalarValue>),
}
'''
new = '''type NoResultExternalProvider = Box<dyn FnMut(&[ExternalScalarValue])>;
type ScalarResultExternalProvider =
    Box<dyn FnMut(&[ExternalScalarValue]) -> ExternalScalarValue>;

enum ExternalProvider {
    NoResult(NoResultExternalProvider),
    ScalarResult(ScalarResultExternalProvider),
}
'''
if text.count(old) != 1:
    raise SystemExit("ExternalProvider complexity anchor mismatch")
path.write_text(text.replace(old, new, 1))
