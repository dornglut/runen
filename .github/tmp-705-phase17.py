from pathlib import Path

path = Path("crates/runen-core-lowering/tests/external_callables.rs")
text = path.read_text()
old = "    assert_eq!(interface.result.is_some(), true);\n"
new = "    assert!(interface.result.is_some());\n"
if text.count(old) != 1:
    raise SystemExit("external callable result assertion anchor mismatch")
path.write_text(text.replace(old, new, 1))
