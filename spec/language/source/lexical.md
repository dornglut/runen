# Source Lexical Foundation

Status: **provisional normative; incomplete**

This document owns the source-text and identifier lexical foundation for Runen. It does not define concrete grammar, name resolution, literal value semantics, or an implementation representation.

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

Identifier-form tokens are case-sensitive. Identifier keys are compared without case folding.

Case alone does not imply visibility, declaration category, namespace, type/value status, or another semantic classification.

This revision does not adopt mathematical-compatibility, emoji, or another extended identifier profile beyond the rules above.

## Identifier keys and canonical equivalence

For an identifier-form token, its **lexical identifier key** is the token's Unicode Normalization Form C (NFC) form using Unicode 17.0.0 normalization data.

Two identifier-form spellings with the same lexical identifier key denote the same lexical identifier for any later source-language rule that consumes identifier identity.

An implementation MAY retain the original source spelling for diagnostics, formatting, or lossless syntax. Original spelling does not create a second lexical identifier identity distinct from its identifier key.

Deriving an identifier key does not silently delete or ignore Default_Ignorable_Code_Point characters. Only the NFC normalization relation established above determines canonical equivalence in this slice.

This document does not define declarations, scopes, namespaces, shadowing, modules, imports, lookup, or name resolution. Those later rules consume lexical identifier keys rather than redefining identifier equivalence.

## Keyword boundary

This revision does not reserve a language-wide keyword inventory.

The lexical foundation recognizes identifier-form tokens and their identifier keys. A later concrete grammar may assign special grammatical meaning to specified identifier keys in specified grammatical positions. Whether such a spelling is unavailable as a declaration name, contextual, or escapable is not defined by this revision.

## Comment, punctuation, and literal boundary

This revision does not define:

- comment or documentation-comment delimiters;
- punctuation or operator token spellings;
- numeric, string, byte, or character literal token forms;
- escape syntax or literal suffixes;
- literal value semantics.

Those concerns require later concrete lexical or grammar rules and their applicable semantic owners.
