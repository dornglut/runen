from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual != 1:
        raise SystemExit(f"{path}: expected 1 occurrence, found {actual}: {old!r}")
    p.write_text(text.replace(old, new))

fe = "spec/language/source/function-execution.md"
replace(fe,
    "It performs no source evaluation, binding `Move` or `Copy`, direct call, constructor initializer evaluation or record assembly, field receiver evaluation or field-value read/consumption, safe-reference formation/dereference/reborrow, raw operation, defined-fault/divergence observation, cleanup, runtime query, or producer-state transition.",
    "It performs no source evaluation, binding `Move` or `Copy`, source call, constructor initializer evaluation or record assembly, field receiver evaluation or field-value read/consumption, safe-reference formation/dereference/reborrow, raw operation, defined-fault/divergence observation, cleanup, runtime query, or producer-state transition.")
replace(fe,
    "If a direct call or record construction used as a producer-backed field receiver diverges before successful receiver production, no field-receiver transient or selected field result exists.",
    "If a source call or record construction used as a producer-backed field receiver diverges before successful receiver production, no field-receiver transient or selected field result exists.")

refs = "spec/language/source/references.md"
replace(refs,
    "No root formation selects a pattern path independently of its root binding, producer transient, direct-call result, record-construction transient, dereference result, arbitrary temporary, grouped value, or general source expression/place/lvalue.",
    "No root formation selects a pattern path independently of its root binding, producer transient, source-call result, record-construction transient, dereference result, arbitrary temporary, grouped value, or general source expression/place/lvalue.")

for path, forbidden in {
    fe: ["binding `Move` or `Copy`, direct call", "If a direct call or record construction used as a producer-backed field receiver"],
    refs: ["producer transient, direct-call result"],
}.items():
    text = Path(path).read_text()
    for phrase in forbidden:
        if phrase in text:
            raise SystemExit(f"{path}: residual remains: {phrase}")
