from pathlib import Path

p = Path("spec/language/source/types.md")
text = p.read_text()
old = "`SharedRef(T)` exists only when `T` satisfies the Shared-referent-admission relation from `references.md`: `T` is represented, source-duplicable, and its structural source value shape contains neither safe references nor raw pointers."
new = "`SharedRef(T)` exists only when `T` satisfies the Shared-referent-admission relation from `references.md`: `T` is one represented intrinsic scalar or nominal record source type, is source-duplicable, and its structural source value shape contains neither safe references nor raw pointers. Captureless function-value types are therefore not Shared referents in this slice even though they are duplicable."
if text.count(old) != 1:
    raise SystemExit(f"expected one stale SharedRef summary, found {text.count(old)}")
text = text.replace(old, new)
if "`T` is represented, source-duplicable" in text:
    raise SystemExit("stale broad SharedRef summary remains")
p.write_text(text)
