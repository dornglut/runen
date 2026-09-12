#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one occurrence, found {count}: {old!r}")
    p.write_text(text.replace(old, new, 1))

replace_once(
    "spec/language/core/functions.md",
    "When the normally returning activation is the outermost represented Core activation, its optional result is preserved while activation-local termination completes. Before that preserved optional result is delivered to the outer consumer, `persistent-storage.md` performs execution-terminal persistent cleanup and ends the persistent storage extents. Only then does the outer consumer receive the same optional result structure: no-result normal completion yields no value, an ordinary result-bearing normal completion yields the preserved owned result value, and a contract-bearing Shared-reference result preserves the exact authority/target/ancestry relation guaranteed by the selected callable interface. Persistent cleanup cannot legitimize a result reference whose target extent would end there; existing result contracts remain parameter-origin-only and storage-extent validity must hold. This fact defines Core execution structure and does not establish source entry-point semantics.",
    "When the normally returning activation is the outermost represented Core activation, its optional result is preserved while activation-local termination completes. Before terminal persistent cleanup may begin, the storage-extent validity relation from `references.md` applies to every persistent instance whose extent will end, including any safe-reference carrier preserved in that outer optional result. A preserved contract-bearing Shared-reference result whose target lies in any such persistent instance makes Core language validation fail at this terminal boundary; persistent cleanup cannot destroy, end, detach, re-root, or rewrite that result carrier to repair the violation. Only after this validity requirement holds does `persistent-storage.md` perform execution-terminal persistent cleanup and end the persistent storage extents. The outer consumer then receives the same optional result structure: no-result normal completion yields no value, an ordinary result-bearing normal completion yields the preserved owned result value, and a contract-bearing Shared-reference result preserves the exact authority/target/ancestry relation guaranteed by the selected callable interface. Existing callable result contracts remain parameter-origin-only; this terminal validity check adds no result-origin contract and no entry-point or outer-argument semantics. This fact defines Core execution structure and does not establish source entry-point semantics.",
)

replace_once(
    "spec/language/core/persistent-storage.md",
    "Persistent terminal cleanup requires that no live safe-reference carrier or active descendant authority remains whose target lies in a persistent instance whose extent is about to end. This is the existing non-dangling storage-extent validity law from `references.md` applied to this storage owner.",
    "Before persistent terminal cleanup may begin, no live safe-reference carrier or active descendant authority may remain whose target lies in a persistent instance whose extent is about to end. This includes any carrier preserved in the outermost activation's optional normal result. The requirement is the existing non-dangling storage-extent validity law from `references.md`; violation is a Core language-validation failure. Persistent cleanup does not destroy, end, detach, re-root, or rewrite a preserved result carrier merely to satisfy this precondition.",
)
