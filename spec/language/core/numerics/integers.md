# Core Integer Semantics

Status: **provisional normative; incomplete**

Fixed-width integer arithmetic MUST have language-defined semantics.

Signed overflow MUST NOT become undefined behavior merely because a backend uses machine integers, and debug or release mode MUST NOT change language meaning.

Checked, wrapping, and saturating operations are part of the intended arithmetic model.

The default overflow behavior of plain fixed-width arithmetic is not defined by this revision.