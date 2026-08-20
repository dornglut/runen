# Source Lexical Foundation

Status: **provisional normative; incomplete**

This document owns the source-text, whitespace/line-boundary, identifier-form, identifier-token extent, and lexical identifier-key foundation for Runen. The represented concrete reserved keys, punctuation, ordinary comments, and grammar are owned by [Source concrete syntax](concrete-syntax.md). Name resolution, literal value semantics, and implementation representation remain outside this owner.

## Source text

A **source unit** is one byte sequence presented for independent lexical processing. This term does not imply a file, module, package, or other source-organization relationship.

A Runen source unit MUST be valid UTF-8. A byte sequence that is not valid UTF-8 is invalid Runen source; an implementation MUST NOT make it valid by implicitly inserting replacement characters.

One U+FEFF BYTE ORDER MARK MAY occur at the beginning of a source unit. When present there, it is ignored before lexical processing. U+FEFF at any other position has no special byte-order-mark status under this rule.

After UTF-8 decoding and removal of an optional initial byte-order mark, lexical classification operates on Unicode scalar values.

Runen does not normalize the complete source unit as text.

## Whitespace and line boundaries

Runen uses the `Pattern_White_Space` property from Unicode 17.0.0 as its lexical whitespace classification.

Pattern whitespace is semantically inert under this lexical contract. It separates otherwise adjacent lexical material and is not part of an identifier-form token.

The following Pattern White Space characters are line-boundary characters:

- U+000A LINE FEED;
- U+000B LINE TABULATION;
- U+000C FORM FEED;
- U+000D CARRIAGE RETURN;
- U+0085 NEXT LINE;
- U+2028 LINE SEPARATOR;
- U+2029 PARAGRAPH SEPARATOR.

A U+000D CARRIAGE RETURN immediately followed by U+000A LINE FEED forms one logical line boundary for source-position purposes. This rule does not rewrite or normalize the stored source text.

An implementation MAY preserve the exact spelling and extent of whitespace for diagnostics, formatting, or lossless source tooling. Such preservation is not Runen program semantics.

## Identifier-form tokens

Runen's identifier profile is pinned to Unicode 17.0.0.

An **identifier-form token** is a non-empty sequence of Unicode scalar values satisfying both of the following:

- its first scalar has the Unicode 17.0.0 `XID_Start` property or is U+005F LOW LINE (`_`);
- every remaining scalar has the Unicode 17.0.0 `XID_Continue` property or is U+005F LOW LINE (`_`).

When lexical processing begins an identifier-form token at a source position, the token consumes the **maximal contiguous sequence** of Unicode scalar values that satisfies the rule above from that starting position. A later grammar MUST NOT split that maximal identifier-form token into shorter identifier-form tokens merely to obtain a keyword, type spelling, declaration name, or other grammatical interpretation.

Identifier-form tokens are case-sensitive. Identifier keys are compared without case folding.

Case alone does not imply visibility, declaration category, namespace, type/value status, or another semantic classification.

This revision does not adopt mathematical-compatibility, emoji, or another extended identifier profile beyond the rules above.

## Identifier keys and canonical equivalence

For an identifier-form token, its **lexical identifier key** is the token's Unicode Normalization Form C (NFC) form using Unicode 17.0.0 normalization data.

Two identifier-form spellings with the same lexical identifier key denote the same lexical identifier for any later source-language rule that consumes identifier identity.

An implementation MAY retain the original source spelling for diagnostics, formatting, or lossless syntax. Original spelling does not create a second lexical identifier identity distinct from its identifier key.

Deriving an identifier key does not silently delete or ignore Default_Ignorable_Code_Point characters. Only the NFC normalization relation established above determines canonical equivalence in this slice.

This document does not define declarations, scopes, namespaces, shadowing, modules, imports, lookup, or name resolution. Those rules consume lexical identifier keys rather than redefining identifier equivalence.

## Reserved-key boundary

This lexical foundation does not reserve a language-wide keyword inventory.

[Source concrete syntax](concrete-syntax.md) assigns grammatical meaning to, and reserves, the finite set of lexical identifier keys required by its represented concrete subset. That reservation consumes the lexical identifier keys defined here rather than creating a second identifier-equivalence relation.

Identifier keys not reserved by an applicable concrete grammar remain ordinary identifier keys under this document. Whether a future grammar reserves, contextually interprets, or permits escaping another key is not determined here.

## Concrete-token and literal boundary

[Source concrete syntax](concrete-syntax.md) owns the currently represented ordinary comment delimiters, punctuation tokens, reserved-key roles, and concrete grammar.

This lexical foundation does not define:

- documentation-comment semantics;
- numeric, string, byte, or character literal token forms;
- escape syntax or literal suffixes;
- literal value semantics;
- additional punctuation or operator token spellings.

Those concerns require their applicable semantic and concrete-syntax owners when accepted.
